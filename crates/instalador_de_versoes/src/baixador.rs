//! Download paralelo com verificacao de SHA-1.
//!
//! Cada arquivo e baixado para `<destino>.parcial` e so ganha o nome final depois
//! que o SHA-1 confere. Assim um download interrompido nunca deixa arquivo corrompido.

use std::fs::{self, File};
use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use rayon::prelude::*;
use reqwest::blocking::Client;
use sha1::{Digest, Sha1};

const QUANTIDADE_DE_DOWNLOADS_SIMULTANEOS: usize = 16;
const TENTATIVAS_POR_ARQUIVO: u32 = 3;
const TAMANHO_DO_BLOCO_DE_LEITURA: usize = 64 * 1024;

#[derive(Debug, Clone)]
pub struct ArquivoParaBaixar {
    pub endereco: String,
    pub destino: PathBuf,
    pub sha1_esperado: Option<String>,
    pub tamanho_esperado: Option<u64>,
    pub deve_ser_executavel: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModoDeVerificacao {
    /// Confere so existencia e tamanho. Usado em todo inicio do jogo.
    Rapida,
    /// Recalcula o SHA-1 de tudo. Usado no botao "Reparar arquivos".
    Completa,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ProgressoDoDownload {
    pub arquivos_concluidos: usize,
    pub total_de_arquivos: usize,
    pub bytes_baixados: u64,
}

impl ProgressoDoDownload {
    pub fn fracao_concluida(&self) -> f32 {
        if self.total_de_arquivos == 0 {
            1.0
        } else {
            self.arquivos_concluidos as f32 / self.total_de_arquivos as f32
        }
    }
}

pub fn calcular_sha1_do_arquivo(caminho: &Path) -> Result<String> {
    let mut leitor = BufReader::new(File::open(caminho)?);
    let mut calculadora = Sha1::new();
    let mut bloco = vec![0u8; TAMANHO_DO_BLOCO_DE_LEITURA];
    loop {
        let lidos = leitor.read(&mut bloco)?;
        if lidos == 0 {
            break;
        }
        calculadora.update(&bloco[..lidos]);
    }
    Ok(hex::encode(calculadora.finalize()))
}

pub fn calcular_sha1_dos_bytes(bytes: &[u8]) -> String {
    hex::encode(Sha1::digest(bytes))
}

pub fn arquivo_ja_esta_correto(arquivo: &ArquivoParaBaixar, modo: ModoDeVerificacao) -> bool {
    let Ok(metadados) = fs::metadata(&arquivo.destino) else {
        return false;
    };
    if let Some(tamanho) = arquivo.tamanho_esperado {
        if metadados.len() != tamanho {
            return false;
        }
    }
    if modo == ModoDeVerificacao::Completa {
        if let Some(sha1_esperado) = &arquivo.sha1_esperado {
            return calcular_sha1_do_arquivo(&arquivo.destino)
                .map(|sha1| sha1.eq_ignore_ascii_case(sha1_esperado))
                .unwrap_or(false);
        }
    }
    true
}

/// Baixa so o que falta ou esta errado. Chama `ao_progredir` a cada arquivo conferido.
pub fn baixar_arquivos_em_paralelo(
    cliente: &Client,
    arquivos: &[ArquivoParaBaixar],
    modo: ModoDeVerificacao,
    ao_progredir: &(dyn Fn(ProgressoDoDownload) + Sync),
) -> Result<()> {
    let total_de_arquivos = arquivos.len();
    let arquivos_concluidos = AtomicUsize::new(0);
    let bytes_baixados = AtomicU64::new(0);
    let erros_encontrados: Mutex<Vec<String>> = Mutex::new(Vec::new());

    let grupo_de_trabalho = rayon::ThreadPoolBuilder::new()
        .num_threads(QUANTIDADE_DE_DOWNLOADS_SIMULTANEOS)
        .build()
        .context("Não foi possível criar as tarefas de download")?;

    grupo_de_trabalho.install(|| {
        arquivos.par_iter().for_each(|arquivo| {
            if !arquivo_ja_esta_correto(arquivo, modo) {
                if let Err(erro) =
                    baixar_um_arquivo_com_tentativas(cliente, arquivo, &bytes_baixados)
                {
                    if let Ok(mut lista) = erros_encontrados.lock() {
                        lista.push(format!("{}: {erro:#}", arquivo.endereco));
                    }
                }
            }
            let concluidos_agora = arquivos_concluidos.fetch_add(1, Ordering::Relaxed) + 1;
            ao_progredir(ProgressoDoDownload {
                arquivos_concluidos: concluidos_agora,
                total_de_arquivos,
                bytes_baixados: bytes_baixados.load(Ordering::Relaxed),
            });
        });
    });

    let erros = erros_encontrados.into_inner().unwrap_or_default();
    if !erros.is_empty() {
        let primeiros: Vec<&str> = erros.iter().take(5).map(String::as_str).collect();
        bail!(
            "{} arquivo(s) não puderam ser baixados. Verifique a internet e tente de novo.\n{}",
            erros.len(),
            primeiros.join("\n")
        );
    }
    Ok(())
}

fn baixar_um_arquivo_com_tentativas(
    cliente: &Client,
    arquivo: &ArquivoParaBaixar,
    bytes_baixados: &AtomicU64,
) -> Result<()> {
    let mut ultimo_erro = None;
    for tentativa in 1..=TENTATIVAS_POR_ARQUIVO {
        match baixar_um_arquivo(cliente, arquivo, bytes_baixados) {
            Ok(()) => return Ok(()),
            Err(erro) => {
                ultimo_erro = Some(erro);
                thread::sleep(Duration::from_millis(500 * tentativa as u64));
            }
        }
    }
    Err(ultimo_erro.expect("houve pelo menos uma tentativa"))
}

fn baixar_um_arquivo(
    cliente: &Client,
    arquivo: &ArquivoParaBaixar,
    bytes_baixados: &AtomicU64,
) -> Result<()> {
    if let Some(pasta) = arquivo.destino.parent() {
        fs::create_dir_all(pasta)?;
    }
    let caminho_parcial = caminho_do_arquivo_parcial(&arquivo.destino);

    let mut resposta = cliente.get(&arquivo.endereco).send()?.error_for_status()?;
    let mut saida = File::create(&caminho_parcial)?;
    let mut calculadora = Sha1::new();
    let mut bloco = vec![0u8; TAMANHO_DO_BLOCO_DE_LEITURA];
    loop {
        let lidos = resposta.read(&mut bloco)?;
        if lidos == 0 {
            break;
        }
        saida.write_all(&bloco[..lidos])?;
        calculadora.update(&bloco[..lidos]);
        bytes_baixados.fetch_add(lidos as u64, Ordering::Relaxed);
    }
    saida.flush()?;
    drop(saida);

    if let Some(sha1_esperado) = &arquivo.sha1_esperado {
        let sha1_recebido = hex::encode(calculadora.finalize());
        if !sha1_recebido.eq_ignore_ascii_case(sha1_esperado) {
            let _ = fs::remove_file(&caminho_parcial);
            bail!("arquivo chegou corrompido (SHA-1 diferente)");
        }
    }

    marcar_como_executavel_se_preciso(&caminho_parcial, arquivo.deve_ser_executavel)?;
    fs::rename(&caminho_parcial, &arquivo.destino)?;
    Ok(())
}

fn caminho_do_arquivo_parcial(destino: &Path) -> PathBuf {
    let mut nome = destino.as_os_str().to_owned();
    nome.push(".parcial");
    PathBuf::from(nome)
}

#[cfg(unix)]
fn marcar_como_executavel_se_preciso(caminho: &Path, deve_ser_executavel: bool) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    if deve_ser_executavel {
        let mut permissoes = fs::metadata(caminho)?.permissions();
        permissoes.set_mode(0o755);
        fs::set_permissions(caminho, permissoes)?;
    }
    Ok(())
}

#[cfg(not(unix))]
fn marcar_como_executavel_se_preciso(_caminho: &Path, _deve_ser_executavel: bool) -> Result<()> {
    Ok(())
}

/// Baixa um arquivo pequeno (JSON) para a memoria e confere o SHA-1.
pub fn baixar_bytes_verificados(
    cliente: &Client,
    endereco: &str,
    sha1_esperado: Option<&str>,
) -> Result<Vec<u8>> {
    let bytes = cliente
        .get(endereco)
        .send()
        .with_context(|| format!("Falha ao baixar {endereco}"))?
        .error_for_status()?
        .bytes()?
        .to_vec();
    if let Some(sha1_esperado) = sha1_esperado {
        if !calcular_sha1_dos_bytes(&bytes).eq_ignore_ascii_case(sha1_esperado) {
            bail!("O arquivo {endereco} chegou corrompido (SHA-1 diferente)");
        }
    }
    Ok(bytes)
}
