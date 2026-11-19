//! Tela principal: menu lateral com as 7 abas, botao Jogar e configuracoes.

use eframe::egui::{self, Align, Layout, RichText};
use instalador_de_versoes::{ModoDeVerificacao, VERSOES_VALIDADAS};

use crate::aplicativo::{AbaPrincipal, AplicativoDoLauncher, SituacaoDoJogo};
use crate::configuracao::{MEMORIA_MAXIMA_EM_MEGABYTES, MEMORIA_MINIMA_EM_MEGABYTES};
use crate::tema_visual::{
    COR_DE_ERRO, COR_DE_SUCESSO, COR_DOURADA, COR_DO_MENU_LATERAL, COR_DO_TEXTO_SECUNDARIO,
};

pub fn desenhar(aplicativo: &mut AplicativoDoLauncher, contexto: &egui::Context) {
    desenhar_menu_lateral(aplicativo, contexto);
    egui::CentralPanel::default().show(contexto, |ui| {
        ui.add_space(16.0);
        match aplicativo.aba_selecionada {
            AbaPrincipal::Inicio => desenhar_aba_de_inicio(aplicativo, ui),
            AbaPrincipal::Configuracoes => desenhar_aba_de_configuracoes(aplicativo, ui),
            AbaPrincipal::Animacoes => desenhar_aba_em_breve(
                ui,
                "Animações",
                "Opções individuais de movimentação e combate.",
            ),
            AbaPrincipal::Personagem => {
                desenhar_aba_em_breve(ui, "Personagem", "Criação e personalização do modelo 3D.")
            }
            AbaPrincipal::Cosmeticos => {
                desenhar_aba_em_breve(ui, "Cosméticos", "Biblioteca de roupas e acessórios.")
            }
            AbaPrincipal::Loja => {
                desenhar_aba_em_breve(ui, "Loja", "Cosméticos exclusivos do servidor.")
            }
            AbaPrincipal::Perfil => {
                desenhar_aba_em_breve(ui, "Perfil", "Informações da conta e aparências salvas.")
            }
        }
    });
}

fn desenhar_menu_lateral(aplicativo: &mut AplicativoDoLauncher, contexto: &egui::Context) {
    egui::SidePanel::left("menu_lateral")
        .exact_width(200.0)
        .resizable(false)
        .frame(
            egui::Frame::default()
                .fill(COR_DO_MENU_LATERAL)
                .inner_margin(16.0),
        )
        .show(contexto, |ui| {
            ui.label(
                RichText::new("HONKAI")
                    .size(28.0)
                    .strong()
                    .color(COR_DOURADA),
            );
            ui.add_space(18.0);
            for aba in AbaPrincipal::TODAS {
                let selecionada = aplicativo.aba_selecionada == aba;
                let texto = RichText::new(aba.nome_na_tela()).size(16.0);
                if ui
                    .add_sized(
                        [168.0, 34.0],
                        egui::SelectableLabel::new(selecionada, texto),
                    )
                    .clicked()
                {
                    aplicativo.aba_selecionada = aba;
                }
            }

            ui.with_layout(Layout::bottom_up(Align::LEFT), |ui| {
                if ui.button("Sair da conta").clicked() {
                    aplicativo.sair_da_conta();
                }
                if let Some(conta) = &aplicativo.conta {
                    ui.label(RichText::new(&conta.nome_do_jogador).strong());
                }
            });
        });
}

fn desenhar_aba_de_inicio(aplicativo: &mut AplicativoDoLauncher, ui: &mut egui::Ui) {
    let nome = aplicativo
        .conta
        .as_ref()
        .map(|conta| conta.nome_do_jogador.clone())
        .unwrap_or_default();
    ui.heading(format!("Bem-vindo, {nome}"));
    ui.label(RichText::new("Servidor Honkai · RPG").color(COR_DO_TEXTO_SECUNDARIO));
    ui.add_space(24.0);

    let jogo_parado = matches!(aplicativo.situacao_do_jogo, SituacaoDoJogo::Parado);

    ui.add_enabled_ui(jogo_parado, |ui| {
        ui.horizontal(|ui| {
            ui.label("Versão do Minecraft:");
            egui::ComboBox::from_id_salt("versao_do_minecraft")
                .selected_text(&aplicativo.configuracao.versao_escolhida)
                .show_ui(ui, |ui| {
                    for versao in VERSOES_VALIDADAS {
                        ui.selectable_value(
                            &mut aplicativo.configuracao.versao_escolhida,
                            versao.to_string(),
                            *versao,
                        );
                    }
                });
        });
    });
    ui.add_space(24.0);

    match &aplicativo.situacao_do_jogo {
        SituacaoDoJogo::Parado => {
            let botao_jogar = egui::Button::new(
                RichText::new("JOGAR")
                    .size(24.0)
                    .strong()
                    .color(egui::Color32::BLACK),
            )
            .fill(COR_DOURADA);
            if ui.add_sized([240.0, 60.0], botao_jogar).clicked() {
                aplicativo.jogar(ModoDeVerificacao::Rapida);
            }
            ui.label(
                RichText::new("Na primeira vez o launcher baixa cerca de 650 MB.")
                    .small()
                    .color(COR_DO_TEXTO_SECUNDARIO),
            );
        }
        SituacaoDoJogo::Preparando { descricao, fracao } => {
            ui.set_max_width(560.0);
            match fracao {
                Some(fracao) => {
                    ui.add(egui::ProgressBar::new(*fracao).show_percentage());
                }
                None => {
                    ui.spinner();
                }
            }
            ui.label(descricao);
        }
        SituacaoDoJogo::Rodando => {
            ui.label(
                RichText::new("O Minecraft está aberto.")
                    .size(18.0)
                    .color(COR_DE_SUCESSO),
            );
            ui.label(
                RichText::new("Feche o jogo para poder iniciar de novo.")
                    .color(COR_DO_TEXTO_SECUNDARIO),
            );
        }
    }

    if let Some(status) = &aplicativo.mensagem_de_status {
        ui.add_space(8.0);
        ui.label(RichText::new(status).color(COR_DE_SUCESSO));
    }
    if let Some(erro) = &aplicativo.mensagem_de_erro {
        ui.add_space(8.0);
        ui.set_max_width(620.0);
        ui.label(RichText::new(erro).color(COR_DE_ERRO));
    }
}

fn desenhar_aba_de_configuracoes(aplicativo: &mut AplicativoDoLauncher, ui: &mut egui::Ui) {
    ui.heading("Configurações");
    ui.add_space(16.0);

    ui.label(RichText::new("Desempenho").strong().color(COR_DOURADA));
    let resposta_da_memoria = ui.add(
        egui::Slider::new(
            &mut aplicativo.configuracao.memoria_maxima_em_megabytes,
            MEMORIA_MINIMA_EM_MEGABYTES..=MEMORIA_MAXIMA_EM_MEGABYTES,
        )
        .step_by(512.0)
        .suffix(" MB")
        .text("memória máxima do jogo"),
    );
    if resposta_da_memoria.drag_stopped() || resposta_da_memoria.lost_focus() {
        aplicativo.salvar_configuracao_atual();
    }
    ui.add_space(20.0);

    ui.label(RichText::new("Arquivos").strong().color(COR_DOURADA));
    ui.label(
        RichText::new(aplicativo.pastas.raiz.display().to_string())
            .small()
            .color(COR_DO_TEXTO_SECUNDARIO),
    );
    ui.horizontal(|ui| {
        if ui.button("Abrir pasta do jogo").clicked() {
            let pasta = aplicativo.pastas.pasta_do_jogo();
            let _ = std::fs::create_dir_all(&pasta);
            let _ = open::that(pasta);
        }
        if ui.button("Abrir registro do último início").clicked() {
            let _ = open::that(
                aplicativo
                    .pastas
                    .pasta_dos_registros()
                    .join("ultimo_inicio_do_jogo.log"),
            );
        }
    });
    let jogo_parado = matches!(aplicativo.situacao_do_jogo, SituacaoDoJogo::Parado);
    if ui
        .add_enabled(jogo_parado, egui::Button::new("Reparar arquivos e jogar"))
        .on_hover_text(
            "Confere o SHA-1 de todos os arquivos e baixa de novo os que estiverem danificados.",
        )
        .clicked()
    {
        aplicativo.aba_selecionada = AbaPrincipal::Inicio;
        aplicativo.jogar(ModoDeVerificacao::Completa);
    }
    ui.add_space(20.0);

    ui.label(RichText::new("Conta Microsoft").strong().color(COR_DOURADA));
    ui.label(
        RichText::new("ID do aplicativo Azure (avançado, só mude se souber o que está fazendo):")
            .small()
            .color(COR_DO_TEXTO_SECUNDARIO),
    );
    ui.horizontal(|ui| {
        ui.add(egui::TextEdit::singleline(&mut aplicativo.texto_do_id_azure).desired_width(340.0));
        if ui.button("Salvar").clicked() {
            aplicativo.salvar_id_do_cliente_azure();
        }
    });
}

fn desenhar_aba_em_breve(ui: &mut egui::Ui, titulo: &str, descricao: &str) {
    ui.heading(titulo);
    ui.add_space(8.0);
    ui.label(descricao);
    ui.add_space(4.0);
    ui.label(RichText::new("Em breve.").color(COR_DO_TEXTO_SECUNDARIO));
}
