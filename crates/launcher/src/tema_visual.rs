//! Cores e fontes do launcher: fundo escuro, roxo e dourado (tema RPG).

use eframe::egui::{self, Color32, FontId, TextStyle};

pub const COR_DOURADA: Color32 = Color32::from_rgb(232, 190, 92);
pub const COR_ROXA: Color32 = Color32::from_rgb(122, 76, 196);
pub const COR_DO_FUNDO: Color32 = Color32::from_rgb(17, 13, 27);
pub const COR_DO_MENU_LATERAL: Color32 = Color32::from_rgb(24, 18, 38);
pub const COR_DOS_CAMPOS: Color32 = Color32::from_rgb(38, 30, 58);
pub const COR_DE_ERRO: Color32 = Color32::from_rgb(240, 110, 110);
pub const COR_DE_SUCESSO: Color32 = Color32::from_rgb(120, 210, 140);
pub const COR_DO_TEXTO_SECUNDARIO: Color32 = Color32::from_rgb(170, 160, 190);

pub fn aplicar_tema_do_honkai(contexto: &egui::Context) {
    let mut visuais = egui::Visuals::dark();
    visuais.panel_fill = COR_DO_FUNDO;
    visuais.window_fill = COR_DO_FUNDO;
    visuais.extreme_bg_color = COR_DOS_CAMPOS;
    visuais.selection.bg_fill = COR_ROXA;
    visuais.hyperlink_color = COR_DOURADA;
    visuais.widgets.inactive.weak_bg_fill = COR_DOS_CAMPOS;
    visuais.widgets.inactive.bg_fill = COR_DOS_CAMPOS;
    visuais.widgets.hovered.weak_bg_fill = Color32::from_rgb(58, 44, 90);
    visuais.widgets.active.weak_bg_fill = COR_ROXA;

    // Sempre escuro, mesmo com o Windows em tema claro.
    contexto.set_theme(egui::Theme::Dark);
    contexto.set_visuals_of(egui::Theme::Dark, visuais);

    contexto.style_mut_of(egui::Theme::Dark, |estilo| {
        estilo
            .text_styles
            .insert(TextStyle::Heading, FontId::proportional(26.0));
        estilo
            .text_styles
            .insert(TextStyle::Body, FontId::proportional(15.0));
        estilo
            .text_styles
            .insert(TextStyle::Button, FontId::proportional(15.0));
        estilo
            .text_styles
            .insert(TextStyle::Small, FontId::proportional(12.5));
        estilo.spacing.button_padding = egui::vec2(14.0, 8.0);
        estilo.spacing.item_spacing = egui::vec2(10.0, 10.0);
    });
}
