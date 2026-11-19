//! Onde o Honkai Client guarda cada coisa no disco.
//!
//! Padrao no Windows: `%APPDATA%\HonkaiClient\`
//!
//! ```text
//! HonkaiClient/
//! ├─ configuracao.json      opcoes do launcher
//! ├─ versoes/<id>/          json e jar de cada versao
//! ├─ bibliotecas/           bibliotecas Java do jogo
//! ├─ assets/                indexes/, objects/, log_configs/ (formato da Mojang)
//! ├─ java/<componente>/     Java baixado da Mojang
//! ├─ natives/<id>/          pasta temporaria de bibliotecas nativas
//! ├─ jogo/                  mundos, prints, opcoes do Minecraft
//! └─ registros/             logs do launcher e do ultimo inicio do jogo
//! ```

use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};

const NOME_DA_PASTA_DO_HONKAI: &str = "HonkaiClient";

#[derive(Debug, Clone)]
pub struct PastasDoHonkai {
    pub raiz: PathBuf,
}

impl PastasDoHonkai {
    /// `%APPDATA%\HonkaiClient` no Windows, `~/.local/share/HonkaiClient` no Linux.
    pub fn localizar_pasta_padrao() -> Result<Self> {
        let pasta_de_dados = dirs::data_dir()
            .ok_or_else(|| anyhow!("Não foi possível encontrar a pasta de dados do usuário"))?;
        Ok(Self::usar_pasta(
            pasta_de_dados.join(NOME_DA_PASTA_DO_HONKAI),
        ))
    }

    pub fn usar_pasta(raiz: impl AsRef<Path>) -> Self {
        Self {
            raiz: raiz.as_ref().to_path_buf(),
        }
    }

    pub fn arquivo_de_configuracao(&self) -> PathBuf {
        self.raiz.join("configuracao.json")
    }

    pub fn pasta_da_versao(&self, id_da_versao: &str) -> PathBuf {
        self.raiz.join("versoes").join(id_da_versao)
    }

    pub fn arquivo_json_da_versao(&self, id_da_versao: &str) -> PathBuf {
        self.pasta_da_versao(id_da_versao)
            .join(format!("{id_da_versao}.json"))
    }

    pub fn arquivo_jar_da_versao(&self, id_da_versao: &str) -> PathBuf {
        self.pasta_da_versao(id_da_versao)
            .join(format!("{id_da_versao}.jar"))
    }

    pub fn pasta_das_bibliotecas(&self) -> PathBuf {
        self.raiz.join("bibliotecas")
    }

    pub fn pasta_dos_assets(&self) -> PathBuf {
        self.raiz.join("assets")
    }

    pub fn arquivo_do_indice_de_assets(&self, id_do_indice: &str) -> PathBuf {
        self.pasta_dos_assets()
            .join("indexes")
            .join(format!("{id_do_indice}.json"))
    }

    pub fn arquivo_do_asset(&self, hash: &str) -> PathBuf {
        self.pasta_dos_assets()
            .join("objects")
            .join(&hash[..2])
            .join(hash)
    }

    pub fn arquivo_de_configuracao_de_log(&self, nome_do_arquivo: &str) -> PathBuf {
        self.pasta_dos_assets()
            .join("log_configs")
            .join(nome_do_arquivo)
    }

    pub fn pasta_do_java(&self, componente: &str) -> PathBuf {
        self.raiz.join("java").join(componente)
    }

    pub fn pasta_dos_natives(&self, id_da_versao: &str) -> PathBuf {
        self.raiz.join("natives").join(id_da_versao)
    }

    pub fn pasta_do_jogo(&self) -> PathBuf {
        self.raiz.join("jogo")
    }

    pub fn pasta_dos_registros(&self) -> PathBuf {
        self.raiz.join("registros")
    }
}
