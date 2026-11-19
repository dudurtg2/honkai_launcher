//! Estado da janela do launcher e o que cada acao do jogador faz.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;

use autenticacao::login_microsoft::CodigoDeDispositivo;
use autenticacao::{cofre_de_credenciais, ContaMinecraft};
use eframe::egui;
use instalador_de_versoes::{EtapaDaInstalacao, ModoDeVerificacao, PastasDoHonkai};
use reqwest::blocking::Client;

use crate::configuracao::{carregar_configuracao, salvar_configuracao, ConfiguracaoDoLauncher};
use crate::mensagens::MensagemDaTarefa;
use crate::tarefas_em_segundo_plano::{
    criar_cliente_http, iniciar_entrada_com_login_salvo, iniciar_instalacao_e_jogo,
    iniciar_login_microsoft, AvisoParaAJanela, PedidoParaJogar,
};
use crate::{telas, tema_visual};

pub enum Tela {
    VerificandoLoginSalvo,
    Login,
    AguardandoCodigo(CodigoDeDispositivo),
    EntrandoNaConta,
    Principal,
}

/// As 7 abas da especificacao. Por enquanto so Inicio e Configuracoes funcionam.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AbaPrincipal {
    Inicio,
    Configuracoes,
    Animacoes,
    Personagem,
    Cosmeticos,
    Loja,
    Perfil,
}

impl AbaPrincipal {
    pub const TODAS: [AbaPrincipal; 7] = [
        AbaPrincipal::Inicio,
        AbaPrincipal::Configuracoes,
        AbaPrincipal::Animacoes,
        AbaPrincipal::Personagem,
        AbaPrincipal::Cosmeticos,
        AbaPrincipal::Loja,
        AbaPrincipal::Perfil,
    ];

    pub fn nome_na_tela(self) -> &'static str {
        match self {
            AbaPrincipal::Inicio => "Início",
            AbaPrincipal::Configuracoes => "Configurações",
            AbaPrincipal::Animacoes => "Animações",
            AbaPrincipal::Personagem => "Personagem",
            AbaPrincipal::Cosmeticos => "Cosméticos",
            AbaPrincipal::Loja => "Loja",
            AbaPrincipal::Perfil => "Perfil",
        }
    }
}

pub enum SituacaoDoJogo {
    Parado,
    Preparando {
        descricao: String,
        fracao: Option<f32>,
    },
    Rodando,
}

pub struct AplicativoDoLauncher {
    pub pastas: PastasDoHonkai,
    pub configuracao: ConfiguracaoDoLauncher,
    pub cliente_http: Client,
    pub tela: Tela,
    pub conta: Option<ContaMinecraft>,
    pub aba_selecionada: AbaPrincipal,
    pub situacao_do_jogo: SituacaoDoJogo,
    pub mensagem_de_erro: Option<String>,
    pub mensagem_de_status: Option<String>,
    /// Texto do campo onde o jogador cola o ID do app Azure.
    pub texto_do_id_azure: String,
    pub esta_editando_id_azure: bool,
    #[cfg(any(debug_assertions, feature = "modo_de_teste_offline"))]
    pub nome_para_teste_offline: String,
    aviso: AvisoParaAJanela,
    receptor: Receiver<MensagemDaTarefa>,
    cancelamento_do_login: Arc<AtomicBool>,
}

impl AplicativoDoLauncher {
    pub fn criar(contexto_de_criacao: &eframe::CreationContext<'_>) -> Self {
        tema_visual::aplicar_tema_do_honkai(&contexto_de_criacao.egui_ctx);

        let pastas = PastasDoHonkai::localizar_pasta_padrao()
            .unwrap_or_else(|_| PastasDoHonkai::usar_pasta("HonkaiClient"));
        let configuracao = carregar_configuracao(&pastas);
        let (remetente, receptor) = channel();
        let aviso = AvisoParaAJanela {
            remetente,
            contexto_da_janela: contexto_de_criacao.egui_ctx.clone(),
        };

        #[cfg(any(debug_assertions, feature = "modo_de_teste_offline"))]
        let nome_offline_inicial = configuracao.nome_do_jogador_offline.clone();

        let mut aplicativo = Self {
            texto_do_id_azure: configuracao.id_do_cliente_azure.clone(),
            esta_editando_id_azure: configuracao.id_do_cliente_azure.is_empty(),
            pastas,
            configuracao,
            cliente_http: criar_cliente_http(),
            tela: Tela::Login,
            conta: None,
            aba_selecionada: AbaPrincipal::Inicio,
            situacao_do_jogo: SituacaoDoJogo::Parado,
            mensagem_de_erro: None,
            mensagem_de_status: None,
            #[cfg(any(debug_assertions, feature = "modo_de_teste_offline"))]
            nome_para_teste_offline: nome_offline_inicial,
            aviso,
            receptor,
            cancelamento_do_login: Arc::new(AtomicBool::new(false)),
        };
        aplicativo.tentar_entrar_com_login_salvo();
        aplicativo
    }

    fn tentar_entrar_com_login_salvo(&mut self) {
        if self.configuracao.id_do_cliente_azure.is_empty() {
            return;
        }
        if let Some(token) = cofre_de_credenciais::carregar_token_de_atualizacao() {
            self.tela = Tela::VerificandoLoginSalvo;
            iniciar_entrada_com_login_salvo(
                self.aviso.clone(),
                self.cliente_http.clone(),
                self.configuracao.id_do_cliente_azure.clone(),
                token,
            );
        }
    }

    // ---------- Acoes do jogador ----------

    pub fn salvar_id_do_cliente_azure(&mut self) {
        self.configuracao.id_do_cliente_azure = self.texto_do_id_azure.trim().to_string();
        self.esta_editando_id_azure = self.configuracao.id_do_cliente_azure.is_empty();
        self.salvar_configuracao_atual();
    }

    pub fn comecar_login_microsoft(&mut self) {
        self.mensagem_de_erro = None;
        self.cancelamento_do_login = Arc::new(AtomicBool::new(false));
        self.tela = Tela::EntrandoNaConta;
        iniciar_login_microsoft(
            self.aviso.clone(),
            self.cliente_http.clone(),
            self.configuracao.id_do_cliente_azure.clone(),
            self.cancelamento_do_login.clone(),
        );
    }

    pub fn cancelar_login(&mut self) {
        self.cancelamento_do_login.store(true, Ordering::Relaxed);
        self.tela = Tela::Login;
    }

    #[cfg(any(debug_assertions, feature = "modo_de_teste_offline"))]
    pub fn entrar_em_modo_de_teste_offline(&mut self) {
        let nome = self.nome_para_teste_offline.trim().to_string();
        if !nome_de_jogador_e_valido(&nome) {
            self.mensagem_de_erro = Some(
                "Nome inválido: use de 3 a 16 caracteres, só letras, números e _.".to_string(),
            );
            return;
        }
        self.configuracao.nome_do_jogador_offline = nome.clone();
        self.salvar_configuracao_atual();
        self.conta = Some(autenticacao::criar_conta_de_teste_offline(&nome));
        self.mensagem_de_erro = None;
        self.tela = Tela::Principal;
    }

    pub fn sair_da_conta(&mut self) {
        cofre_de_credenciais::apagar_token_de_atualizacao();
        self.conta = None;
        self.tela = Tela::Login;
        self.mensagem_de_status = None;
        self.mensagem_de_erro = None;
    }

    pub fn jogar(&mut self, modo_de_verificacao: ModoDeVerificacao) {
        let Some(conta) = self.conta.clone() else {
            return;
        };
        self.salvar_configuracao_atual();
        self.mensagem_de_erro = None;
        self.mensagem_de_status = None;
        self.situacao_do_jogo = SituacaoDoJogo::Preparando {
            descricao: "Preparando…".to_string(),
            fracao: None,
        };
        iniciar_instalacao_e_jogo(
            self.aviso.clone(),
            self.cliente_http.clone(),
            PedidoParaJogar {
                conta,
                pastas: self.pastas.clone(),
                id_da_versao: self.configuracao.versao_escolhida.clone(),
                memoria_maxima_em_megabytes: self.configuracao.memoria_maxima_em_megabytes,
                id_do_cliente_azure: self.configuracao.id_do_cliente_azure.clone(),
                modo_de_verificacao,
            },
        );
    }

    pub fn salvar_configuracao_atual(&mut self) {
        if let Err(erro) = salvar_configuracao(&self.pastas, &self.configuracao) {
            self.mensagem_de_erro = Some(format!(
                "Não foi possível salvar as configurações: {erro:#}"
            ));
        }
    }

    // ---------- Mensagens das tarefas ----------

    fn processar_mensagens_recebidas(&mut self) {
        while let Ok(mensagem) = self.receptor.try_recv() {
            match mensagem {
                MensagemDaTarefa::CodigoDeLoginRecebido(codigo) => {
                    self.tela = Tela::AguardandoCodigo(codigo)
                }
                MensagemDaTarefa::EntrandoNaConta => self.tela = Tela::EntrandoNaConta,
                MensagemDaTarefa::LoginConcluido(conta) => {
                    self.conta = Some(conta);
                    self.tela = Tela::Principal;
                    self.mensagem_de_erro = None;
                }
                MensagemDaTarefa::LoginFalhou(erro) => {
                    // Cancelamento pelo proprio jogador nao e erro.
                    if !self.cancelamento_do_login.load(Ordering::Relaxed) {
                        self.mensagem_de_erro = Some(erro);
                    }
                    self.tela = Tela::Login;
                }
                MensagemDaTarefa::LoginSalvoInvalido => self.tela = Tela::Login,
                MensagemDaTarefa::ContaRenovada(conta) => self.conta = Some(conta),
                MensagemDaTarefa::ProgressoDaInstalacao(etapa) => {
                    self.situacao_do_jogo = descrever_etapa_da_instalacao(&etapa)
                }
                MensagemDaTarefa::JogoIniciado => {
                    self.situacao_do_jogo = SituacaoDoJogo::Rodando;
                    self.mensagem_de_status = Some("Minecraft aberto. Bom jogo!".to_string());
                }
                MensagemDaTarefa::JogoEncerrado { codigo_de_saida } => {
                    self.situacao_do_jogo = SituacaoDoJogo::Parado;
                    self.mensagem_de_status = None;
                    if !matches!(codigo_de_saida, Some(0) | None) {
                        self.mensagem_de_erro = Some(format!(
                            "O jogo fechou com erro (código {}). Veja o registro em Configurações.",
                            codigo_de_saida.unwrap_or_default()
                        ));
                    }
                }
                MensagemDaTarefa::FalhaAoIniciarJogo(erro) => {
                    self.situacao_do_jogo = SituacaoDoJogo::Parado;
                    self.mensagem_de_erro = Some(erro);
                }
            }
        }
    }
}

/// Mesmas regras de nome do Minecraft: 3 a 16 caracteres, letras, numeros e _.
#[cfg(any(debug_assertions, feature = "modo_de_teste_offline"))]
fn nome_de_jogador_e_valido(nome: &str) -> bool {
    (3..=16).contains(&nome.len())
        && nome
            .chars()
            .all(|caractere| caractere.is_ascii_alphanumeric() || caractere == '_')
}

fn descrever_etapa_da_instalacao(etapa: &EtapaDaInstalacao) -> SituacaoDoJogo {
    let megabytes = |bytes: u64| bytes as f64 / 1_000_000.0;
    match etapa {
        EtapaDaInstalacao::BaixandoInformacoesDaVersao => SituacaoDoJogo::Preparando {
            descricao: "Buscando informações da versão…".to_string(),
            fracao: None,
        },
        EtapaDaInstalacao::BaixandoArquivosDoJogo(progresso) => SituacaoDoJogo::Preparando {
            descricao: format!(
                "Arquivos do jogo: {} de {} · {:.0} MB baixados",
                progresso.arquivos_concluidos,
                progresso.total_de_arquivos,
                megabytes(progresso.bytes_baixados)
            ),
            fracao: Some(progresso.fracao_concluida()),
        },
        EtapaDaInstalacao::BaixandoJava(progresso) => SituacaoDoJogo::Preparando {
            descricao: format!(
                "Java: {} de {} · {:.0} MB baixados",
                progresso.arquivos_concluidos,
                progresso.total_de_arquivos,
                megabytes(progresso.bytes_baixados)
            ),
            fracao: Some(progresso.fracao_concluida()),
        },
        EtapaDaInstalacao::Concluida => SituacaoDoJogo::Preparando {
            descricao: "Abrindo o Minecraft…".to_string(),
            fracao: Some(1.0),
        },
    }
}

impl eframe::App for AplicativoDoLauncher {
    fn update(&mut self, contexto: &egui::Context, _quadro: &mut eframe::Frame) {
        self.processar_mensagens_recebidas();
        match self.tela {
            Tela::Principal => telas::tela_principal::desenhar(self, contexto),
            _ => telas::tela_de_login::desenhar(self, contexto),
        }
    }
}
