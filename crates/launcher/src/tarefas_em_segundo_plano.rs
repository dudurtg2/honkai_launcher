//! Trabalho demorado (rede, downloads, processo do jogo) roda em threads separadas.
//! Cada tarefa avisa a janela por `MensagemDaTarefa` e pede um redesenho.

use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::thread;

use anyhow::Result;
use autenticacao::login_microsoft::{
    aguardar_confirmacao_do_jogador, concluir_login_com_tokens_microsoft, entrar_com_token_salvo,
    solicitar_codigo_de_dispositivo,
};
use autenticacao::{cofre_de_credenciais, ContaMinecraft, TipoDeConta};
use eframe::egui;
use inicializador_do_jogo::{iniciar_jogo, DadosDoJogador, PedidoDeInicio};
use instalador_de_versoes::{instalar_versao, ModoDeVerificacao, PastasDoHonkai};
use reqwest::blocking::Client;

use crate::mensagens::MensagemDaTarefa;

/// Canal de volta para a janela: envia a mensagem e acorda a interface.
#[derive(Clone)]
pub struct AvisoParaAJanela {
    pub remetente: Sender<MensagemDaTarefa>,
    pub contexto_da_janela: egui::Context,
}

impl AvisoParaAJanela {
    pub fn enviar(&self, mensagem: MensagemDaTarefa) {
        let _ = self.remetente.send(mensagem);
        self.contexto_da_janela.request_repaint();
    }
}

pub fn criar_cliente_http() -> Client {
    Client::builder()
        .user_agent(concat!("HonkaiClient/", env!("CARGO_PKG_VERSION")))
        .build()
        .expect("cliente HTTP com configuracao fixa")
}

/// Ao abrir o launcher: tenta entrar com o login guardado no cofre.
pub fn iniciar_entrada_com_login_salvo(
    aviso: AvisoParaAJanela,
    cliente: Client,
    id_do_cliente_azure: String,
    token_de_atualizacao: String,
) {
    thread::spawn(move || {
        match entrar_com_token_salvo(&cliente, &id_do_cliente_azure, &token_de_atualizacao) {
            Ok(conta) => {
                guardar_login_no_cofre(&conta);
                aviso.enviar(MensagemDaTarefa::LoginConcluido(conta));
            }
            Err(_) => aviso.enviar(MensagemDaTarefa::LoginSalvoInvalido),
        }
    });
}

/// Botao "Entrar com Microsoft": codigo de dispositivo -> espera -> conta.
pub fn iniciar_login_microsoft(
    aviso: AvisoParaAJanela,
    cliente: Client,
    id_do_cliente_azure: String,
    foi_cancelado: Arc<AtomicBool>,
) {
    thread::spawn(move || {
        let resultado = (|| -> Result<ContaMinecraft> {
            let codigo = solicitar_codigo_de_dispositivo(&cliente, &id_do_cliente_azure)?;
            aviso.enviar(MensagemDaTarefa::CodigoDeLoginRecebido(codigo.clone()));
            let tokens = aguardar_confirmacao_do_jogador(
                &cliente,
                &id_do_cliente_azure,
                &codigo,
                &foi_cancelado,
            )?;
            aviso.enviar(MensagemDaTarefa::EntrandoNaConta);
            concluir_login_com_tokens_microsoft(&cliente, &tokens)
        })();

        match resultado {
            Ok(conta) => {
                guardar_login_no_cofre(&conta);
                aviso.enviar(MensagemDaTarefa::LoginConcluido(conta));
            }
            Err(erro) => aviso.enviar(MensagemDaTarefa::LoginFalhou(format!("{erro:#}"))),
        }
    });
}

fn guardar_login_no_cofre(conta: &ContaMinecraft) {
    if let Some(token) = &conta.token_de_atualizacao_microsoft {
        if let Err(erro) = cofre_de_credenciais::salvar_token_de_atualizacao(token) {
            // Nao impede de jogar; so vai pedir login de novo na proxima vez.
            eprintln!("Aviso: {erro:#}");
        }
    }
}

pub struct PedidoParaJogar {
    pub conta: ContaMinecraft,
    pub pastas: PastasDoHonkai,
    pub id_da_versao: String,
    pub memoria_maxima_em_megabytes: u32,
    pub id_do_cliente_azure: String,
    pub modo_de_verificacao: ModoDeVerificacao,
}

/// Botao "Jogar": renova o login se preciso -> instala/verifica -> inicia -> espera fechar.
pub fn iniciar_instalacao_e_jogo(
    aviso: AvisoParaAJanela,
    cliente: Client,
    pedido: PedidoParaJogar,
) {
    thread::spawn(move || {
        let resultado = (|| -> Result<std::process::Child> {
            let conta = renovar_conta_se_vencida(&aviso, &cliente, &pedido)?;

            let versao = instalar_versao(
                &cliente,
                &pedido.pastas,
                &pedido.id_da_versao,
                pedido.modo_de_verificacao,
                &|etapa| aviso.enviar(MensagemDaTarefa::ProgressoDaInstalacao(etapa)),
            )?;

            iniciar_jogo(&PedidoDeInicio {
                versao,
                pastas: pedido.pastas.clone(),
                jogador: DadosDoJogador {
                    nome_do_jogador: conta.nome_do_jogador.clone(),
                    uuid_do_jogador: conta.uuid_do_jogador.clone(),
                    token_de_acesso_minecraft: conta.token_de_acesso_minecraft.clone(),
                    tipo_de_usuario: conta.tipo_de_usuario_para_o_jogo().to_string(),
                },
                memoria_maxima_em_megabytes: pedido.memoria_maxima_em_megabytes,
                id_do_cliente_azure: pedido.id_do_cliente_azure.clone(),
            })
        })();

        match resultado {
            Ok(mut processo_do_jogo) => {
                aviso.enviar(MensagemDaTarefa::JogoIniciado);
                let codigo_de_saida = processo_do_jogo
                    .wait()
                    .ok()
                    .and_then(|status| status.code());
                aviso.enviar(MensagemDaTarefa::JogoEncerrado { codigo_de_saida });
            }
            Err(erro) => aviso.enviar(MensagemDaTarefa::FalhaAoIniciarJogo(format!("{erro:#}"))),
        }
    });
}

fn renovar_conta_se_vencida(
    aviso: &AvisoParaAJanela,
    cliente: &Client,
    pedido: &PedidoParaJogar,
) -> Result<ContaMinecraft> {
    let conta = &pedido.conta;
    if conta.tipo_de_conta != TipoDeConta::Microsoft || !conta.token_esta_vencido() {
        return Ok(conta.clone());
    }
    let Some(token) = &conta.token_de_atualizacao_microsoft else {
        anyhow::bail!("O login venceu. Saia da conta e entre novamente.");
    };
    let conta_renovada = entrar_com_token_salvo(cliente, &pedido.id_do_cliente_azure, token)?;
    guardar_login_no_cofre(&conta_renovada);
    aviso.enviar(MensagemDaTarefa::ContaRenovada(conta_renovada.clone()));
    Ok(conta_renovada)
}
