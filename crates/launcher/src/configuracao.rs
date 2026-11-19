//! Opcoes do launcher salvas em `%APPDATA%\HonkaiClient\configuracao.json`.

use std::fs;

use anyhow::Result;
use instalador_de_versoes::{PastasDoHonkai, VERSOES_VALIDADAS};
use serde::{Deserialize, Serialize};

/// "Application (client) ID" do app "Honkai Client" registrado no Azure.
/// Nao e segredo: vai dentro do launcher e identifica o programa para a Microsoft.
pub const ID_DO_APLICATIVO_HONKAI_NO_AZURE: &str = "37d65a5a-ba0d-423e-a2ee-fb1dd98540c9";

pub const MEMORIA_MINIMA_EM_MEGABYTES: u32 = 2048;
pub const MEMORIA_MAXIMA_EM_MEGABYTES: u32 = 16384;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ConfiguracaoDoLauncher {
    /// "Application (client) ID" do app registrado no portal Azure.
    pub id_do_cliente_azure: String,
    pub memoria_maxima_em_megabytes: u32,
    pub versao_escolhida: String,
    /// Ultimo nome usado na versao de teste offline.
    pub nome_do_jogador_offline: String,
}

impl Default for ConfiguracaoDoLauncher {
    fn default() -> Self {
        Self {
            id_do_cliente_azure: ID_DO_APLICATIVO_HONKAI_NO_AZURE.to_string(),
            memoria_maxima_em_megabytes: 4096,
            versao_escolhida: VERSOES_VALIDADAS[0].to_string(),
            nome_do_jogador_offline: "Jogador".to_string(),
        }
    }
}

pub fn carregar_configuracao(pastas: &PastasDoHonkai) -> ConfiguracaoDoLauncher {
    let mut configuracao: ConfiguracaoDoLauncher = fs::read(pastas.arquivo_de_configuracao())
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default();

    if configuracao.id_do_cliente_azure.trim().is_empty() {
        configuracao.id_do_cliente_azure = ID_DO_APLICATIVO_HONKAI_NO_AZURE.to_string();
    }
    // Uma versao removida da lista de validadas nunca pode ficar selecionada.
    if !VERSOES_VALIDADAS.contains(&configuracao.versao_escolhida.as_str()) {
        configuracao.versao_escolhida = VERSOES_VALIDADAS[0].to_string();
    }
    configuracao.memoria_maxima_em_megabytes = configuracao
        .memoria_maxima_em_megabytes
        .clamp(MEMORIA_MINIMA_EM_MEGABYTES, MEMORIA_MAXIMA_EM_MEGABYTES);
    configuracao
}

pub fn salvar_configuracao(
    pastas: &PastasDoHonkai,
    configuracao: &ConfiguracaoDoLauncher,
) -> Result<()> {
    fs::create_dir_all(&pastas.raiz)?;
    fs::write(
        pastas.arquivo_de_configuracao(),
        serde_json::to_vec_pretty(configuracao)?,
    )?;
    Ok(())
}
