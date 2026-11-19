// Na versao distribuida (release) o Windows nao abre janela de console junto com o launcher.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! Honkai Launcher: login Microsoft, instalacao do Minecraft e botao Jogar.
//!
//! Organizacao deste crate:
//! - `aplicativo`                estado da janela e troca de telas
//! - `telas/`                    desenho de cada tela (login e principal)
//! - `tarefas_em_segundo_plano`  login e instalacao rodam fora da janela para ela nao travar
//! - `mensagens`                 o que as tarefas avisam para a janela
//! - `configuracao`              opcoes salvas em configuracao.json
//! - `tema_visual`               cores e fontes

mod aplicativo;
mod configuracao;
mod mensagens;
mod tarefas_em_segundo_plano;
mod telas;
mod tema_visual;

use aplicativo::AplicativoDoLauncher;

fn main() -> eframe::Result<()> {
    let opcoes_da_janela = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Honkai Client")
            .with_inner_size([960.0, 600.0])
            .with_min_inner_size([760.0, 480.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Honkai Client",
        opcoes_da_janela,
        Box::new(|contexto_de_criacao| {
            Ok(Box::new(AplicativoDoLauncher::criar(contexto_de_criacao)))
        }),
    )
}
