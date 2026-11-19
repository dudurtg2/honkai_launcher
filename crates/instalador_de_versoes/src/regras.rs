//! Regras da Mojang que dizem quais bibliotecas e argumentos valem em cada sistema.

use crate::modelos_da_mojang::Regra;

/// Nome do sistema como a Mojang escreve nas regras: "windows", "linux" ou "osx".
pub fn nome_do_sistema_para_a_mojang() -> &'static str {
    match std::env::consts::OS {
        "windows" => "windows",
        "macos" => "osx",
        _ => "linux",
    }
}

pub fn sistema_e_32_bits() -> bool {
    std::env::consts::ARCH == "x86"
}

/// Aplica as regras na ordem: a ultima regra que se aplica decide.
/// Sem regras, o item vale sempre. Com regras, comeca proibido.
pub fn regras_permitem_uso(regras: Option<&[Regra]>) -> bool {
    let Some(regras) = regras else {
        return true;
    };
    if regras.is_empty() {
        return true;
    }
    let mut permitido = false;
    for regra in regras {
        if regra_se_aplica_a_este_computador(regra) {
            permitido = regra.acao == "allow";
        }
    }
    permitido
}

fn regra_se_aplica_a_este_computador(regra: &Regra) -> bool {
    // Recursos opcionais (modo demo, resolucao personalizada, quick play) ficam todos desligados.
    if let Some(recursos) = &regra.recursos {
        if recursos.values().any(|exige_ligado| *exige_ligado) {
            return false;
        }
    }
    if let Some(sistema) = &regra.sistema {
        if let Some(nome) = &sistema.nome {
            if nome != nome_do_sistema_para_a_mojang() {
                return false;
            }
        }
        if let Some(arquitetura) = &sistema.arquitetura {
            if arquitetura == "x86" && !sistema_e_32_bits() {
                return false;
            }
        }
    }
    true
}

/// Bibliotecas nativas vem em varias arquiteturas com a mesma regra ("windows").
/// Ex.: `lwjgl:3.3.3:natives-windows`, `natives-windows-arm64`, `natives-windows-x86`.
/// Mantem so a da arquitetura deste computador.
pub fn biblioteca_nativa_e_desta_arquitetura(nome_da_biblioteca: &str) -> bool {
    let classificador = nome_da_biblioteca.rsplit(':').next().unwrap_or_default();
    if !classificador.starts_with("natives-") {
        return true;
    }
    let arquitetura_da_biblioteca = if classificador.ends_with("-arm64") {
        "aarch64"
    } else if classificador.ends_with("-x86") {
        "x86"
    } else {
        "x86_64"
    };
    arquitetura_da_biblioteca == std::env::consts::ARCH
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn biblioteca_comum_sempre_passa() {
        assert!(biblioteca_nativa_e_desta_arquitetura(
            "com.google.code.gson:gson:2.11.0"
        ));
    }

    #[test]
    fn natives_de_outra_arquitetura_sao_descartados() {
        if std::env::consts::ARCH == "x86_64" {
            assert!(biblioteca_nativa_e_desta_arquitetura(
                "org.lwjgl:lwjgl:3.3.3:natives-windows"
            ));
            assert!(!biblioteca_nativa_e_desta_arquitetura(
                "org.lwjgl:lwjgl:3.3.3:natives-windows-arm64"
            ));
            assert!(!biblioteca_nativa_e_desta_arquitetura(
                "org.lwjgl:lwjgl:3.3.3:natives-windows-x86"
            ));
        }
    }
}
