//! Formato dos arquivos JSON publicados pela Mojang.
//! Os nomes em ingles ficam so no `rename`; no codigo tudo e em portugues.

use std::collections::HashMap;

use serde::Deserialize;

pub const ENDERECO_DO_MANIFESTO_DE_VERSOES: &str =
    "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
pub const ENDERECO_DOS_ASSETS: &str = "https://resources.download.minecraft.net";
pub const ENDERECO_DO_CATALOGO_DE_JAVA: &str =
    "https://launchermeta.mojang.com/v1/products/java-runtime/2ec0cc96c44e5a76b9c8b7c39df7210883d12871/all.json";

// ---------- Manifesto com a lista de todas as versoes ----------

#[derive(Debug, Clone, Deserialize)]
pub struct ManifestoDeVersoes {
    #[serde(rename = "versions")]
    pub versoes: Vec<EntradaDoManifesto>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EntradaDoManifesto {
    pub id: String,
    #[serde(rename = "url")]
    pub endereco: String,
    pub sha1: String,
}

// ---------- Detalhes de uma versao (ex.: 1.21.8.json) ----------

#[derive(Debug, Clone, Deserialize)]
pub struct DetalhesDaVersao {
    pub id: String,
    #[serde(rename = "type")]
    pub tipo: String,
    #[serde(rename = "mainClass")]
    pub classe_principal: String,
    #[serde(rename = "arguments")]
    pub argumentos: ArgumentosDaVersao,
    #[serde(rename = "assetIndex")]
    pub indice_de_assets: ReferenciaDoIndiceDeAssets,
    pub downloads: DownloadsDaVersao,
    #[serde(rename = "javaVersion")]
    pub versao_do_java: VersaoDoJava,
    #[serde(rename = "libraries")]
    pub bibliotecas: Vec<Biblioteca>,
    #[serde(rename = "logging")]
    pub configuracao_de_log: Option<ConfiguracaoDeLog>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ArgumentosDaVersao {
    #[serde(rename = "game")]
    pub jogo: Vec<Argumento>,
    pub jvm: Vec<Argumento>,
}

/// Um argumento pode ser texto puro ou texto que so vale sob certas regras.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum Argumento {
    Simples(String),
    Condicional {
        #[serde(rename = "rules")]
        regras: Vec<Regra>,
        #[serde(rename = "value")]
        valor: ValorDoArgumento,
    },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum ValorDoArgumento {
    Unico(String),
    Varios(Vec<String>),
}

impl ValorDoArgumento {
    pub fn como_lista(&self) -> Vec<String> {
        match self {
            ValorDoArgumento::Unico(texto) => vec![texto.clone()],
            ValorDoArgumento::Varios(lista) => lista.clone(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Regra {
    #[serde(rename = "action")]
    pub acao: String,
    #[serde(rename = "os")]
    pub sistema: Option<RegraDeSistema>,
    #[serde(rename = "features")]
    pub recursos: Option<HashMap<String, bool>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RegraDeSistema {
    #[serde(rename = "name")]
    pub nome: Option<String>,
    #[serde(rename = "arch")]
    pub arquitetura: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReferenciaDoIndiceDeAssets {
    pub id: String,
    pub sha1: String,
    #[serde(rename = "url")]
    pub endereco: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DownloadsDaVersao {
    #[serde(rename = "client")]
    pub cliente: ArquivoRemoto,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ArquivoRemoto {
    pub sha1: String,
    #[serde(rename = "size")]
    pub tamanho: u64,
    #[serde(rename = "url")]
    pub endereco: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VersaoDoJava {
    #[serde(rename = "component")]
    pub componente: String,
    #[serde(rename = "majorVersion")]
    pub versao_principal: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Biblioteca {
    #[serde(rename = "name")]
    pub nome: String,
    pub downloads: Option<DownloadsDaBiblioteca>,
    #[serde(rename = "rules")]
    pub regras: Option<Vec<Regra>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DownloadsDaBiblioteca {
    #[serde(rename = "artifact")]
    pub artefato: Option<ArtefatoDaBiblioteca>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ArtefatoDaBiblioteca {
    #[serde(rename = "path")]
    pub caminho: String,
    pub sha1: String,
    #[serde(rename = "size")]
    pub tamanho: u64,
    #[serde(rename = "url")]
    pub endereco: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConfiguracaoDeLog {
    #[serde(rename = "client")]
    pub cliente: Option<LogDoCliente>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LogDoCliente {
    #[serde(rename = "argument")]
    pub argumento: String,
    #[serde(rename = "file")]
    pub arquivo: ArquivoDeLog,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ArquivoDeLog {
    pub id: String,
    pub sha1: String,
    #[serde(rename = "size")]
    pub tamanho: u64,
    #[serde(rename = "url")]
    pub endereco: String,
}

// ---------- Indice de assets (sons, texturas, idiomas) ----------

#[derive(Debug, Clone, Deserialize)]
pub struct IndiceDeAssets {
    #[serde(rename = "objects")]
    pub objetos: HashMap<String, ObjetoDeAsset>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ObjetoDeAsset {
    pub hash: String,
    #[serde(rename = "size")]
    pub tamanho: u64,
}

// ---------- Java distribuido pela Mojang ----------

/// plataforma (ex.: "windows-x64") -> componente (ex.: "java-runtime-delta") -> entradas
pub type CatalogoDeJava = HashMap<String, HashMap<String, Vec<EntradaDoCatalogoDeJava>>>;

#[derive(Debug, Clone, Deserialize)]
pub struct EntradaDoCatalogoDeJava {
    #[serde(rename = "manifest")]
    pub manifesto: ArquivoRemoto,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ManifestoDoJava {
    #[serde(rename = "files")]
    pub arquivos: HashMap<String, ArquivoDoJava>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ArquivoDoJava {
    #[serde(rename = "type")]
    pub tipo: String,
    pub downloads: Option<DownloadsDoArquivoDoJava>,
    #[serde(rename = "executable", default)]
    pub executavel: bool,
    #[serde(rename = "target")]
    pub alvo_do_atalho: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DownloadsDoArquivoDoJava {
    #[serde(rename = "raw")]
    pub bruto: ArquivoRemoto,
}
