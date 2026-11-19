//! Autenticacao do jogador.
//!
//! Fluxo completo de login: Microsoft (codigo de dispositivo) -> Xbox Live -> XSTS -> Minecraft.
//! O token de atualizacao da Microsoft fica guardado no cofre do sistema operacional
//! (Gerenciador de Credenciais no Windows) para o jogador nao precisar logar toda vez.

pub mod cofre_de_credenciais;
pub mod login_microsoft;

use std::time::{SystemTime, UNIX_EPOCH};

/// Como a conta foi obtida.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TipoDeConta {
    /// Conta Microsoft real, com Minecraft Java comprado.
    Microsoft,
    /// Conta local sem login, existe apenas em compilacoes de desenvolvimento (cargo run).
    TesteOffline,
}

/// Tudo que o launcher precisa saber sobre o jogador para iniciar o jogo.
#[derive(Debug, Clone)]
pub struct ContaMinecraft {
    pub nome_do_jogador: String,
    /// UUID do perfil, sem hifens, como a Mojang devolve.
    pub uuid_do_jogador: String,
    pub token_de_acesso_minecraft: String,
    pub token_de_atualizacao_microsoft: Option<String>,
    /// Momento (segundos desde 1970) em que o token do Minecraft deixa de valer.
    pub token_expira_em_segundos_unix: u64,
    pub tipo_de_conta: TipoDeConta,
}

impl ContaMinecraft {
    /// Considera o token vencido 10 minutos antes do prazo real, para nao expirar no meio do inicio.
    pub fn token_esta_vencido(&self) -> bool {
        if self.tipo_de_conta == TipoDeConta::TesteOffline {
            return false;
        }
        segundos_desde_1970() + 600 >= self.token_expira_em_segundos_unix
    }

    /// Valor que o Minecraft espera no argumento `--userType`.
    pub fn tipo_de_usuario_para_o_jogo(&self) -> &'static str {
        match self.tipo_de_conta {
            TipoDeConta::Microsoft => "msa",
            TipoDeConta::TesteOffline => "legacy",
        }
    }
}

pub(crate) fn segundos_desde_1970() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duracao| duracao.as_secs())
        .unwrap_or(0)
}

/// Cria uma conta local para testar o launcher antes da aprovacao do app Azure.
/// So existe em compilacoes de desenvolvimento; a versao distribuida (release) nao tem isso.
#[cfg(any(debug_assertions, feature = "modo_de_teste_offline"))]
pub fn criar_conta_de_teste_offline(nome_do_jogador: &str) -> ContaMinecraft {
    let semente = format!("OfflinePlayer:{nome_do_jogador}");
    let uuid = uuid::Uuid::new_v3(&uuid::Uuid::NAMESPACE_OID, semente.as_bytes());
    ContaMinecraft {
        nome_do_jogador: nome_do_jogador.to_string(),
        uuid_do_jogador: uuid.simple().to_string(),
        token_de_acesso_minecraft: "0".to_string(),
        token_de_atualizacao_microsoft: None,
        token_expira_em_segundos_unix: u64::MAX,
        tipo_de_conta: TipoDeConta::TesteOffline,
    }
}
