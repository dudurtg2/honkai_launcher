//! Avisos que as tarefas em segundo plano mandam para a janela.

use autenticacao::login_microsoft::CodigoDeDispositivo;
use autenticacao::ContaMinecraft;
use instalador_de_versoes::EtapaDaInstalacao;

pub enum MensagemDaTarefa {
    // Login
    CodigoDeLoginRecebido(CodigoDeDispositivo),
    EntrandoNaConta,
    LoginConcluido(ContaMinecraft),
    LoginFalhou(String),
    /// O login salvo nao serve mais; a janela volta para a tela de login sem mostrar erro.
    LoginSalvoInvalido,

    // Jogo
    ContaRenovada(ContaMinecraft),
    ProgressoDaInstalacao(EtapaDaInstalacao),
    JogoIniciado,
    JogoEncerrado {
        codigo_de_saida: Option<i32>,
    },
    FalhaAoIniciarJogo(String),
}
