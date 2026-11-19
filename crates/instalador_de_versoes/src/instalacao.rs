//! Junta todas as etapas: informacoes da versao -> arquivos do jogo -> Java.

use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

use anyhow::{anyhow, bail, Context, Result};
use reqwest::blocking::Client;

use crate::baixador::{
    baixar_arquivos_em_paralelo, baixar_bytes_verificados, calcular_sha1_dos_bytes,
    ArquivoParaBaixar, ModoDeVerificacao, ProgressoDoDownload,
};
use crate::modelos_da_mojang::{
    DetalhesDaVersao, IndiceDeAssets, ManifestoDeVersoes, ENDERECO_DOS_ASSETS,
    ENDERECO_DO_MANIFESTO_DE_VERSOES,
};
use crate::pastas::PastasDoHonkai;
use crate::regras::{biblioteca_nativa_e_desta_arquitetura, regras_permitem_uso};
use crate::runtime_java::instalar_java;
use crate::versoes_validadas::versao_esta_validada;

/// O que a interface mostra enquanto a instalacao anda.
#[derive(Debug, Clone)]
pub enum EtapaDaInstalacao {
    BaixandoInformacoesDaVersao,
    BaixandoArquivosDoJogo(ProgressoDoDownload),
    BaixandoJava(ProgressoDoDownload),
    Concluida,
}

/// Resultado da instalacao: tudo que o inicializador precisa para abrir o jogo.
#[derive(Debug, Clone)]
pub struct VersaoInstalada {
    pub detalhes: DetalhesDaVersao,
    pub caminho_do_jar_do_cliente: PathBuf,
    pub caminhos_das_bibliotecas: Vec<PathBuf>,
    pub caminho_do_executavel_java: PathBuf,
    pub caminho_da_configuracao_de_log: Option<PathBuf>,
}

pub fn instalar_versao(
    cliente: &Client,
    pastas: &PastasDoHonkai,
    id_da_versao: &str,
    modo: ModoDeVerificacao,
    ao_progredir: &(dyn Fn(EtapaDaInstalacao) + Sync),
) -> Result<VersaoInstalada> {
    if !versao_esta_validada(id_da_versao) {
        bail!("A versão {id_da_versao} ainda não foi validada pelo Honkai Client.");
    }

    ao_progredir(EtapaDaInstalacao::BaixandoInformacoesDaVersao);
    let detalhes = obter_detalhes_da_versao(cliente, pastas, id_da_versao)?;
    let indice_de_assets = obter_indice_de_assets(cliente, pastas, &detalhes)?;

    let (arquivos_do_jogo, caminhos_das_bibliotecas) =
        listar_arquivos_do_jogo(pastas, &detalhes, &indice_de_assets);
    baixar_arquivos_em_paralelo(cliente, &arquivos_do_jogo, modo, &|progresso| {
        ao_progredir(EtapaDaInstalacao::BaixandoArquivosDoJogo(progresso))
    })?;

    let caminho_do_executavel_java = instalar_java(
        cliente,
        pastas,
        &detalhes.versao_do_java.componente,
        modo,
        &|progresso| ao_progredir(EtapaDaInstalacao::BaixandoJava(progresso)),
    )?;

    let caminho_da_configuracao_de_log = detalhes
        .configuracao_de_log
        .as_ref()
        .and_then(|log| log.cliente.as_ref())
        .map(|log| pastas.arquivo_de_configuracao_de_log(&log.arquivo.id));

    ao_progredir(EtapaDaInstalacao::Concluida);
    Ok(VersaoInstalada {
        caminho_do_jar_do_cliente: pastas.arquivo_jar_da_versao(id_da_versao),
        detalhes,
        caminhos_das_bibliotecas,
        caminho_do_executavel_java,
        caminho_da_configuracao_de_log,
    })
}

/// Busca o JSON da versao na Mojang e guarda uma copia.
/// Sem internet, usa a copia guardada para o jogador conseguir jogar offline.
fn obter_detalhes_da_versao(
    cliente: &Client,
    pastas: &PastasDoHonkai,
    id_da_versao: &str,
) -> Result<DetalhesDaVersao> {
    let arquivo_local = pastas.arquivo_json_da_versao(id_da_versao);
    let bytes = match baixar_json_da_versao(cliente, id_da_versao) {
        Ok(bytes) => {
            fs::create_dir_all(pastas.pasta_da_versao(id_da_versao))?;
            fs::write(&arquivo_local, &bytes)?;
            bytes
        }
        Err(erro) => fs::read(&arquivo_local).map_err(|_| {
            erro.context("Sem internet e a versão ainda não foi baixada neste computador")
        })?,
    };
    serde_json::from_slice(&bytes).context("Arquivo da versão em formato inesperado")
}

fn baixar_json_da_versao(cliente: &Client, id_da_versao: &str) -> Result<Vec<u8>> {
    let manifesto: ManifestoDeVersoes = cliente
        .get(ENDERECO_DO_MANIFESTO_DE_VERSOES)
        .send()
        .context("Não foi possível falar com os servidores da Mojang")?
        .error_for_status()?
        .json()
        .context("Lista de versões da Mojang em formato inesperado")?;
    let entrada = manifesto
        .versoes
        .iter()
        .find(|versao| versao.id == id_da_versao)
        .ok_or_else(|| anyhow!("A Mojang não tem a versão {id_da_versao}"))?;
    baixar_bytes_verificados(cliente, &entrada.endereco, Some(&entrada.sha1))
}

fn obter_indice_de_assets(
    cliente: &Client,
    pastas: &PastasDoHonkai,
    detalhes: &DetalhesDaVersao,
) -> Result<IndiceDeAssets> {
    let referencia = &detalhes.indice_de_assets;
    let arquivo_local = pastas.arquivo_do_indice_de_assets(&referencia.id);

    let copia_local_valida = fs::read(&arquivo_local)
        .ok()
        .filter(|bytes| calcular_sha1_dos_bytes(bytes).eq_ignore_ascii_case(&referencia.sha1));

    let bytes = match copia_local_valida {
        Some(bytes) => bytes,
        None => {
            let bytes =
                baixar_bytes_verificados(cliente, &referencia.endereco, Some(&referencia.sha1))?;
            if let Some(pasta) = arquivo_local.parent() {
                fs::create_dir_all(pasta)?;
            }
            fs::write(&arquivo_local, &bytes)?;
            bytes
        }
    };
    serde_json::from_slice(&bytes).context("Índice de assets em formato inesperado")
}

/// Monta a lista de tudo que precisa existir no disco e o classpath das bibliotecas.
fn listar_arquivos_do_jogo(
    pastas: &PastasDoHonkai,
    detalhes: &DetalhesDaVersao,
    indice_de_assets: &IndiceDeAssets,
) -> (Vec<ArquivoParaBaixar>, Vec<PathBuf>) {
    let mut arquivos = Vec::new();
    let mut caminhos_das_bibliotecas = Vec::new();

    // Jar do cliente
    arquivos.push(ArquivoParaBaixar {
        endereco: detalhes.downloads.cliente.endereco.clone(),
        destino: pastas.arquivo_jar_da_versao(&detalhes.id),
        sha1_esperado: Some(detalhes.downloads.cliente.sha1.clone()),
        tamanho_esperado: Some(detalhes.downloads.cliente.tamanho),
        deve_ser_executavel: false,
    });

    // Bibliotecas (inclui as nativas, que o LWJGL extrai sozinho do classpath)
    for biblioteca in &detalhes.bibliotecas {
        if !regras_permitem_uso(biblioteca.regras.as_deref()) {
            continue;
        }
        if !biblioteca_nativa_e_desta_arquitetura(&biblioteca.nome) {
            continue;
        }
        let Some(artefato) = biblioteca
            .downloads
            .as_ref()
            .and_then(|downloads| downloads.artefato.as_ref())
        else {
            continue;
        };
        let destino = pastas.pasta_das_bibliotecas().join(&artefato.caminho);
        caminhos_das_bibliotecas.push(destino.clone());
        arquivos.push(ArquivoParaBaixar {
            endereco: artefato.endereco.clone(),
            destino,
            sha1_esperado: Some(artefato.sha1.clone()),
            tamanho_esperado: Some(artefato.tamanho),
            deve_ser_executavel: false,
        });
    }

    // Configuracao de log do jogo
    if let Some(log) = detalhes
        .configuracao_de_log
        .as_ref()
        .and_then(|log| log.cliente.as_ref())
    {
        arquivos.push(ArquivoParaBaixar {
            endereco: log.arquivo.endereco.clone(),
            destino: pastas.arquivo_de_configuracao_de_log(&log.arquivo.id),
            sha1_esperado: Some(log.arquivo.sha1.clone()),
            tamanho_esperado: Some(log.arquivo.tamanho),
            deve_ser_executavel: false,
        });
    }

    // Assets (sons, texturas, idiomas). O mesmo arquivo pode aparecer com nomes diferentes.
    let mut hashes_ja_listados = HashSet::new();
    for objeto in indice_de_assets.objetos.values() {
        if !hashes_ja_listados.insert(objeto.hash.clone()) {
            continue;
        }
        arquivos.push(ArquivoParaBaixar {
            endereco: format!(
                "{ENDERECO_DOS_ASSETS}/{}/{}",
                &objeto.hash[..2],
                objeto.hash
            ),
            destino: pastas.arquivo_do_asset(&objeto.hash),
            sha1_esperado: Some(objeto.hash.clone()),
            tamanho_esperado: Some(objeto.tamanho),
            deve_ser_executavel: false,
        });
    }

    (arquivos, caminhos_das_bibliotecas)
}
