//! Guarda o token de atualizacao da Microsoft no cofre do sistema operacional.
//! No Windows isso e o Gerenciador de Credenciais; o token nunca fica em arquivo de texto.

use anyhow::{Context, Result};

const NOME_DO_SERVICO_NO_COFRE: &str = "HonkaiClient";
const NOME_DA_ENTRADA_NO_COFRE: &str = "conta_microsoft";

fn abrir_entrada_do_cofre() -> Result<keyring::Entry> {
    keyring::Entry::new(NOME_DO_SERVICO_NO_COFRE, NOME_DA_ENTRADA_NO_COFRE)
        .context("Não foi possível abrir o cofre de credenciais do sistema")
}

pub fn salvar_token_de_atualizacao(token_de_atualizacao: &str) -> Result<()> {
    abrir_entrada_do_cofre()?
        .set_password(token_de_atualizacao)
        .context("Não foi possível salvar a conta no cofre de credenciais")
}

pub fn carregar_token_de_atualizacao() -> Option<String> {
    abrir_entrada_do_cofre().ok()?.get_password().ok()
}

pub fn apagar_token_de_atualizacao() {
    if let Ok(entrada) = abrir_entrada_do_cofre() {
        let _ = entrada.delete_credential();
    }
}
