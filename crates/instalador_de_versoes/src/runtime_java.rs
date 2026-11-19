//! Instala o Java que a propria Mojang distribui para cada versao.
//! O 1.21.8 pede o componente "java-runtime-delta" (Java 21).
//! O jogador nao precisa ter Java instalado no computador.

use std::fs;
use std::path::PathBuf;

use anyhow::{anyhow, bail, Context, Result};
use reqwest::blocking::Client;

use crate::baixador::{
    baixar_arquivos_em_paralelo, baixar_bytes_verificados, ArquivoParaBaixar, ModoDeVerificacao,
    ProgressoDoDownload,
};
use crate::modelos_da_mojang::{CatalogoDeJava, ManifestoDoJava, ENDERECO_DO_CATALOGO_DE_JAVA};
use crate::pastas::PastasDoHonkai;

/// Nome da plataforma como aparece no catalogo de Java da Mojang.
fn identificar_plataforma_do_java() -> Result<&'static str> {
    let plataforma = match (std::env::consts::OS, std::env::consts::ARCH) {
        ("windows", "x86_64") => "windows-x64",
        ("windows", "x86") => "windows-x86",
        ("windows", "aarch64") => "windows-arm64",
        ("linux", "x86_64") => "linux",
        ("linux", "x86") => "linux-i386",
        ("macos", "x86_64") => "mac-os",
        ("macos", "aarch64") => "mac-os-arm64",
        (sistema, arquitetura) => {
            bail!("Sistema não suportado para o Java da Mojang: {sistema} {arquitetura}")
        }
    };
    Ok(plataforma)
}

pub fn caminho_do_executavel_java(pastas: &PastasDoHonkai, componente: &str) -> PathBuf {
    let pasta = pastas.pasta_do_java(componente);
    if cfg!(windows) {
        // javaw.exe abre o jogo sem janela de console preta.
        pasta.join("bin").join("javaw.exe")
    } else if cfg!(target_os = "macos") {
        pasta
            .join("jre.bundle")
            .join("Contents")
            .join("Home")
            .join("bin")
            .join("java")
    } else {
        pasta.join("bin").join("java")
    }
}

/// Garante que o Java do componente pedido esta instalado e inteiro.
/// Sem internet, usa o Java ja instalado se existir.
pub fn instalar_java(
    cliente: &Client,
    pastas: &PastasDoHonkai,
    componente: &str,
    modo: ModoDeVerificacao,
    ao_progredir: &(dyn Fn(ProgressoDoDownload) + Sync),
) -> Result<PathBuf> {
    let executavel = caminho_do_executavel_java(pastas, componente);
    match baixar_lista_de_arquivos_do_java(cliente, componente) {
        Ok(manifesto) => {
            preparar_pastas_e_baixar_java(
                cliente,
                pastas,
                componente,
                &manifesto,
                modo,
                ao_progredir,
            )?;
        }
        Err(erro) if executavel.exists() => {
            // Sem internet mas com Java ja instalado: segue com o que tem.
            eprintln!("Aviso: não foi possível verificar o Java ({erro:#}). Usando o instalado.");
        }
        Err(erro) => return Err(erro),
    }
    if !executavel.exists() {
        bail!(
            "O Java foi baixado mas o executável não foi encontrado em {}",
            executavel.display()
        );
    }
    Ok(executavel)
}

fn baixar_lista_de_arquivos_do_java(cliente: &Client, componente: &str) -> Result<ManifestoDoJava> {
    let plataforma = identificar_plataforma_do_java()?;
    let catalogo: CatalogoDeJava = cliente
        .get(ENDERECO_DO_CATALOGO_DE_JAVA)
        .send()
        .context("Não foi possível baixar o catálogo de Java da Mojang")?
        .error_for_status()?
        .json()
        .context("Catálogo de Java da Mojang em formato inesperado")?;

    let entrada = catalogo
        .get(plataforma)
        .and_then(|componentes| componentes.get(componente))
        .and_then(|entradas| entradas.first())
        .ok_or_else(|| anyhow!("A Mojang não publica o Java {componente} para {plataforma}"))?;

    let bytes = baixar_bytes_verificados(
        cliente,
        &entrada.manifesto.endereco,
        Some(&entrada.manifesto.sha1),
    )?;
    serde_json::from_slice(&bytes).context("Lista de arquivos do Java em formato inesperado")
}

fn preparar_pastas_e_baixar_java(
    cliente: &Client,
    pastas: &PastasDoHonkai,
    componente: &str,
    manifesto: &ManifestoDoJava,
    modo: ModoDeVerificacao,
    ao_progredir: &(dyn Fn(ProgressoDoDownload) + Sync),
) -> Result<()> {
    let pasta_do_java = pastas.pasta_do_java(componente);
    let mut arquivos_para_baixar = Vec::new();

    for (caminho_relativo, arquivo) in &manifesto.arquivos {
        let destino = pasta_do_java.join(caminho_relativo);
        match arquivo.tipo.as_str() {
            "directory" => fs::create_dir_all(&destino)?,
            "file" => {
                if let Some(downloads) = &arquivo.downloads {
                    arquivos_para_baixar.push(ArquivoParaBaixar {
                        endereco: downloads.bruto.endereco.clone(),
                        destino,
                        sha1_esperado: Some(downloads.bruto.sha1.clone()),
                        tamanho_esperado: Some(downloads.bruto.tamanho),
                        deve_ser_executavel: arquivo.executavel,
                    });
                }
            }
            "link" => criar_atalho_do_java(&destino, arquivo.alvo_do_atalho.as_deref())?,
            _ => {}
        }
    }

    baixar_arquivos_em_paralelo(cliente, &arquivos_para_baixar, modo, ao_progredir)
}

#[cfg(unix)]
fn criar_atalho_do_java(destino: &std::path::Path, alvo: Option<&str>) -> Result<()> {
    if let Some(alvo) = alvo {
        if let Some(pasta) = destino.parent() {
            fs::create_dir_all(pasta)?;
        }
        if fs::symlink_metadata(destino).is_err() {
            std::os::unix::fs::symlink(alvo, destino)?;
        }
    }
    Ok(())
}

#[cfg(not(unix))]
fn criar_atalho_do_java(_destino: &std::path::Path, _alvo: Option<&str>) -> Result<()> {
    // O Java da Mojang para Windows nao usa atalhos.
    Ok(())
}
