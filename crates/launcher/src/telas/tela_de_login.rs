//! Tela de login: ID do app Azure, botao "Entrar com Microsoft" e codigo de dispositivo.

use eframe::egui::{self, RichText};

use crate::aplicativo::{AplicativoDoLauncher, Tela};
use crate::tema_visual::{COR_DE_ERRO, COR_DOURADA, COR_DO_TEXTO_SECUNDARIO};

pub fn desenhar(aplicativo: &mut AplicativoDoLauncher, contexto: &egui::Context) {
    egui::CentralPanel::default().show(contexto, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(60.0);
            ui.label(
                RichText::new("HONKAI CLIENT")
                    .size(44.0)
                    .strong()
                    .color(COR_DOURADA),
            );
            ui.label(RichText::new("Launcher do servidor Honkai").color(COR_DO_TEXTO_SECUNDARIO));
            ui.add_space(40.0);

            match &aplicativo.tela {
                Tela::VerificandoLoginSalvo => desenhar_espera(ui, "Entrando com a conta salva…"),
                Tela::EntrandoNaConta => desenhar_espera(ui, "Falando com a Microsoft…"),
                Tela::AguardandoCodigo(codigo) => {
                    let codigo = codigo.clone();
                    desenhar_codigo_de_dispositivo(aplicativo, ui, &codigo);
                }
                Tela::Login | Tela::Principal => desenhar_opcoes_de_login(aplicativo, ui),
            }

            if let Some(erro) = &aplicativo.mensagem_de_erro {
                ui.add_space(20.0);
                ui.set_max_width(560.0);
                ui.label(RichText::new(erro).color(COR_DE_ERRO));
            }
        });
    });
}

fn desenhar_espera(ui: &mut egui::Ui, texto: &str) {
    ui.spinner();
    ui.label(texto);
}

fn desenhar_opcoes_de_login(aplicativo: &mut AplicativoDoLauncher, ui: &mut egui::Ui) {
    #[cfg(any(debug_assertions, feature = "modo_de_teste_offline"))]
    {
        desenhar_modo_de_teste_offline(aplicativo, ui);
        ui.add_space(30.0);
        ui.separator();
        ui.label(
            RichText::new("Ou entre com a sua conta Microsoft original")
                .small()
                .color(COR_DO_TEXTO_SECUNDARIO),
        );
        ui.add_space(6.0);
    }

    if aplicativo.esta_editando_id_azure {
        desenhar_campo_do_id_azure(aplicativo, ui);
    } else {
        let botao = egui::Button::new(RichText::new("Entrar com Microsoft").size(18.0).strong());
        if ui.add_sized([260.0, 48.0], botao).clicked() {
            aplicativo.comecar_login_microsoft();
        }
    }
}

fn desenhar_campo_do_id_azure(aplicativo: &mut AplicativoDoLauncher, ui: &mut egui::Ui) {
    ui.set_max_width(520.0);
    ui.label("Cole o \"Application (client) ID\" do seu aplicativo no portal Azure:");
    ui.add(
        egui::TextEdit::singleline(&mut aplicativo.texto_do_id_azure)
            .hint_text("00000000-0000-0000-0000-000000000000")
            .desired_width(380.0),
    );
    if ui.button("Salvar ID").clicked() {
        aplicativo.salvar_id_do_cliente_azure();
    }
    if !aplicativo.configuracao.id_do_cliente_azure.is_empty()
        && ui.small_button("Cancelar").clicked()
    {
        aplicativo.texto_do_id_azure = aplicativo.configuracao.id_do_cliente_azure.clone();
        aplicativo.esta_editando_id_azure = false;
    }
    ui.label(
        RichText::new("O passo a passo para criar o aplicativo está no LEIAME.md do projeto.")
            .small()
            .color(COR_DO_TEXTO_SECUNDARIO),
    );
}

fn desenhar_codigo_de_dispositivo(
    aplicativo: &mut AplicativoDoLauncher,
    ui: &mut egui::Ui,
    codigo: &autenticacao::login_microsoft::CodigoDeDispositivo,
) {
    ui.label(format!("1. Abra {}", codigo.endereco_de_verificacao));
    ui.label("2. Digite este código e entre com a conta que tem o Minecraft:");
    ui.add_space(8.0);
    ui.label(
        RichText::new(&codigo.codigo_para_o_jogador)
            .size(40.0)
            .monospace()
            .strong()
            .color(COR_DOURADA),
    );
    ui.add_space(12.0);
    if ui.button("Copiar código e abrir a página").clicked() {
        ui.ctx().copy_text(codigo.codigo_para_o_jogador.clone());
        let _ = open::that(&codigo.endereco_de_verificacao);
    }
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.add_space((ui.available_width() - 220.0).max(0.0) / 2.0);
        ui.spinner();
        ui.label("Aguardando confirmação…");
    });
    if ui.button("Cancelar").clicked() {
        aplicativo.cancelar_login();
    }
}

/// Existe so no build de desenvolvimento (`cargo run`) e no build de teste
/// (`--features modo_de_teste_offline`). O launcher oficial nao tem isso.
/// Conta offline: sem login, funciona em mundo local e em servidores em modo offline.
#[cfg(any(debug_assertions, feature = "modo_de_teste_offline"))]
fn desenhar_modo_de_teste_offline(aplicativo: &mut AplicativoDoLauncher, ui: &mut egui::Ui) {
    ui.label(
        RichText::new("Versão de teste offline")
            .strong()
            .color(COR_DOURADA),
    );
    ui.label(
        RichText::new(
            "Digite um nome de jogador. Sem conta Microsoft: só mundo local e servidores offline.",
        )
        .small()
        .color(COR_DO_TEXTO_SECUNDARIO),
    );
    ui.add_space(4.0);
    let campo = ui.add(
        egui::TextEdit::singleline(&mut aplicativo.nome_para_teste_offline)
            .hint_text("Nome do jogador")
            .char_limit(16)
            .desired_width(220.0),
    );
    let apertou_enter =
        campo.lost_focus() && ui.input(|entrada| entrada.key_pressed(egui::Key::Enter));
    let botao = egui::Button::new(RichText::new("Entrar offline").size(18.0).strong());
    if ui.add_sized([260.0, 44.0], botao).clicked() || apertou_enter {
        aplicativo.entrar_em_modo_de_teste_offline();
    }
}
