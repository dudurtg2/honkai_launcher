//! Login Microsoft por codigo de dispositivo e troca de tokens ate chegar no Minecraft.
//!
//! Etapas:
//! 1. `solicitar_codigo_de_dispositivo`  -> o jogador recebe um codigo e um link.
//! 2. `aguardar_confirmacao_do_jogador`  -> consulta a Microsoft ate o jogador confirmar.
//! 3. `concluir_login_com_tokens_microsoft` -> Xbox Live -> XSTS -> Minecraft -> perfil.
//!
//! Requisito: um app registrado no Azure com permissao aprovada pela Mojang para a API do
//! Minecraft. Sem essa aprovacao a ultima etapa responde 403.

use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Context, Result};
use reqwest::blocking::Client;
use reqwest::StatusCode;
use serde::Deserialize;
use serde_json::json;

use crate::{segundos_desde_1970, ContaMinecraft, TipoDeConta};

const ENDERECO_DO_CODIGO_DE_DISPOSITIVO: &str =
    "https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode";
const ENDERECO_DO_TOKEN_MICROSOFT: &str =
    "https://login.microsoftonline.com/consumers/oauth2/v2.0/token";
const ENDERECO_DO_XBOX_LIVE: &str = "https://user.auth.xboxlive.com/user/authenticate";
const ENDERECO_DO_XSTS: &str = "https://xsts.auth.xboxlive.com/xsts/authorize";
const ENDERECO_DO_LOGIN_MINECRAFT: &str =
    "https://api.minecraftservices.com/authentication/login_with_xbox";
const ENDERECO_DO_PERFIL_MINECRAFT: &str = "https://api.minecraftservices.com/minecraft/profile";
const PERMISSOES_SOLICITADAS: &str = "XboxLive.signin offline_access";
const TIPO_DE_CONCESSAO_POR_DISPOSITIVO: &str = "urn:ietf:params:oauth:grant-type:device_code";

/// Codigo que o jogador digita em microsoft.com/link para autorizar o launcher.
#[derive(Debug, Clone, Deserialize)]
pub struct CodigoDeDispositivo {
    #[serde(rename = "user_code")]
    pub codigo_para_o_jogador: String,
    #[serde(rename = "device_code")]
    pub codigo_interno_do_dispositivo: String,
    #[serde(rename = "verification_uri")]
    pub endereco_de_verificacao: String,
    #[serde(rename = "expires_in")]
    pub expira_em_segundos: u64,
    #[serde(rename = "interval")]
    pub intervalo_de_consulta_em_segundos: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TokensMicrosoft {
    #[serde(rename = "access_token")]
    pub token_de_acesso: String,
    #[serde(rename = "refresh_token")]
    pub token_de_atualizacao: String,
}

#[derive(Debug, Deserialize)]
struct RespostaDeErroMicrosoft {
    #[serde(rename = "error")]
    codigo_do_erro: String,
    #[serde(rename = "error_description", default)]
    descricao_do_erro: String,
}

#[derive(Debug, Deserialize)]
struct RespostaDoXbox {
    #[serde(rename = "Token")]
    token: String,
    #[serde(rename = "DisplayClaims")]
    declaracoes: DeclaracoesDoXbox,
}

#[derive(Debug, Deserialize)]
struct DeclaracoesDoXbox {
    #[serde(rename = "xui")]
    usuarios: Vec<UsuarioDoXbox>,
}

#[derive(Debug, Deserialize)]
struct UsuarioDoXbox {
    #[serde(rename = "uhs")]
    hash_do_usuario: String,
}

#[derive(Debug, Deserialize)]
struct RespostaDeErroDoXsts {
    #[serde(rename = "XErr", default)]
    codigo_do_erro: u64,
}

#[derive(Debug, Deserialize)]
struct RespostaDoLoginMinecraft {
    #[serde(rename = "access_token")]
    token_de_acesso: String,
    #[serde(rename = "expires_in")]
    expira_em_segundos: u64,
}

#[derive(Debug, Deserialize)]
struct PerfilDoMinecraft {
    #[serde(rename = "id")]
    uuid: String,
    #[serde(rename = "name")]
    nome: String,
}

/// Etapa 1: pede a Microsoft um codigo para o jogador digitar no navegador.
pub fn solicitar_codigo_de_dispositivo(
    cliente: &Client,
    id_do_cliente_azure: &str,
) -> Result<CodigoDeDispositivo> {
    if id_do_cliente_azure.trim().is_empty() {
        bail!("Configure o ID do aplicativo Azure antes de entrar.");
    }
    let resposta = cliente
        .post(ENDERECO_DO_CODIGO_DE_DISPOSITIVO)
        .form(&[
            ("client_id", id_do_cliente_azure.trim()),
            ("scope", PERMISSOES_SOLICITADAS),
        ])
        .send()
        .context("Não foi possível falar com a Microsoft. Verifique a internet.")?;

    if !resposta.status().is_success() {
        let texto = resposta.text().unwrap_or_default();
        bail!(
            "A Microsoft recusou o pedido de login. Confira o ID do aplicativo Azure.\n{}",
            descrever_erro_microsoft(&texto)
        );
    }
    resposta
        .json()
        .context("Resposta inesperada da Microsoft ao pedir o código")
}

/// Etapa 2: consulta a Microsoft ate o jogador confirmar o codigo no navegador.
/// Para quando `foi_cancelado` vira verdadeiro ou quando o codigo expira.
pub fn aguardar_confirmacao_do_jogador(
    cliente: &Client,
    id_do_cliente_azure: &str,
    codigo: &CodigoDeDispositivo,
    foi_cancelado: &AtomicBool,
) -> Result<TokensMicrosoft> {
    let inicio = Instant::now();
    let prazo = Duration::from_secs(codigo.expira_em_segundos);
    let mut intervalo = Duration::from_secs(codigo.intervalo_de_consulta_em_segundos.max(1));

    loop {
        esperar_permitindo_cancelamento(intervalo, foi_cancelado)?;
        if inicio.elapsed() >= prazo {
            bail!("O código expirou. Clique em entrar novamente.");
        }

        let resposta = cliente
            .post(ENDERECO_DO_TOKEN_MICROSOFT)
            .form(&[
                ("grant_type", TIPO_DE_CONCESSAO_POR_DISPOSITIVO),
                ("client_id", id_do_cliente_azure.trim()),
                ("device_code", codigo.codigo_interno_do_dispositivo.as_str()),
            ])
            .send()
            .context("Conexão com a Microsoft caiu durante o login")?;

        if resposta.status().is_success() {
            return resposta
                .json()
                .context("Resposta inesperada da Microsoft ao concluir o login");
        }

        let texto = resposta.text().unwrap_or_default();
        let erro: RespostaDeErroMicrosoft = serde_json::from_str(&texto)
            .map_err(|_| anyhow!("Erro desconhecido da Microsoft: {texto}"))?;
        match erro.codigo_do_erro.as_str() {
            "authorization_pending" => continue,
            "slow_down" => intervalo += Duration::from_secs(5),
            "authorization_declined" => bail!("O login foi recusado na página da Microsoft."),
            "expired_token" => bail!("O código expirou. Clique em entrar novamente."),
            _ => bail!("A Microsoft recusou o login: {}", erro.descricao_do_erro),
        }
    }
}

fn esperar_permitindo_cancelamento(tempo: Duration, foi_cancelado: &AtomicBool) -> Result<()> {
    let fim = Instant::now() + tempo;
    while Instant::now() < fim {
        if foi_cancelado.load(Ordering::Relaxed) {
            bail!("Login cancelado.");
        }
        thread::sleep(Duration::from_millis(200));
    }
    Ok(())
}

/// Troca um token de atualizacao salvo por tokens novos, sem o jogador digitar nada.
pub fn renovar_tokens_microsoft(
    cliente: &Client,
    id_do_cliente_azure: &str,
    token_de_atualizacao: &str,
) -> Result<TokensMicrosoft> {
    let resposta = cliente
        .post(ENDERECO_DO_TOKEN_MICROSOFT)
        .form(&[
            ("grant_type", "refresh_token"),
            ("client_id", id_do_cliente_azure.trim()),
            ("refresh_token", token_de_atualizacao),
            ("scope", PERMISSOES_SOLICITADAS),
        ])
        .send()
        .context("Não foi possível falar com a Microsoft para renovar o login")?;
    if !resposta.status().is_success() {
        let texto = resposta.text().unwrap_or_default();
        bail!(
            "O login salvo expirou. Entre novamente.\n{}",
            descrever_erro_microsoft(&texto)
        );
    }
    resposta
        .json()
        .context("Resposta inesperada da Microsoft ao renovar o login")
}

/// Etapa 3: transforma os tokens Microsoft em uma conta Minecraft pronta para jogar.
pub fn concluir_login_com_tokens_microsoft(
    cliente: &Client,
    tokens: &TokensMicrosoft,
) -> Result<ContaMinecraft> {
    let (token_do_xbox_live, hash_do_usuario) =
        autenticar_no_xbox_live(cliente, &tokens.token_de_acesso)?;
    let token_xsts = obter_token_xsts(cliente, &token_do_xbox_live)?;
    let login = entrar_no_minecraft(cliente, &hash_do_usuario, &token_xsts)?;
    let perfil = buscar_perfil_do_minecraft(cliente, &login.token_de_acesso)?;

    Ok(ContaMinecraft {
        nome_do_jogador: perfil.nome,
        uuid_do_jogador: perfil.uuid,
        token_de_acesso_minecraft: login.token_de_acesso,
        token_de_atualizacao_microsoft: Some(tokens.token_de_atualizacao.clone()),
        token_expira_em_segundos_unix: segundos_desde_1970() + login.expira_em_segundos,
        tipo_de_conta: TipoDeConta::Microsoft,
    })
}

/// Atalho usado ao abrir o launcher: token salvo -> conta pronta.
pub fn entrar_com_token_salvo(
    cliente: &Client,
    id_do_cliente_azure: &str,
    token_de_atualizacao: &str,
) -> Result<ContaMinecraft> {
    let tokens = renovar_tokens_microsoft(cliente, id_do_cliente_azure, token_de_atualizacao)?;
    concluir_login_com_tokens_microsoft(cliente, &tokens)
}

fn autenticar_no_xbox_live(cliente: &Client, token_microsoft: &str) -> Result<(String, String)> {
    let corpo = json!({
        "Properties": {
            "AuthMethod": "RPS",
            "SiteName": "user.auth.xboxlive.com",
            "RpsTicket": format!("d={token_microsoft}")
        },
        "RelyingParty": "http://auth.xboxlive.com",
        "TokenType": "JWT"
    });
    let resposta = cliente
        .post(ENDERECO_DO_XBOX_LIVE)
        .json(&corpo)
        .send()
        .context("Não foi possível falar com o Xbox Live")?;
    if !resposta.status().is_success() {
        bail!("O Xbox Live recusou o login (HTTP {}).", resposta.status());
    }
    let dados: RespostaDoXbox = resposta
        .json()
        .context("Resposta inesperada do Xbox Live")?;
    let hash_do_usuario = dados
        .declaracoes
        .usuarios
        .first()
        .map(|usuario| usuario.hash_do_usuario.clone())
        .ok_or_else(|| anyhow!("O Xbox Live não devolveu o usuário."))?;
    Ok((dados.token, hash_do_usuario))
}

fn obter_token_xsts(cliente: &Client, token_do_xbox_live: &str) -> Result<String> {
    let corpo = json!({
        "Properties": {
            "SandboxId": "RETAIL",
            "UserTokens": [token_do_xbox_live]
        },
        "RelyingParty": "rp://api.minecraftservices.com/",
        "TokenType": "JWT"
    });
    let resposta = cliente
        .post(ENDERECO_DO_XSTS)
        .json(&corpo)
        .send()
        .context("Não foi possível falar com o serviço XSTS do Xbox")?;

    if resposta.status() == StatusCode::UNAUTHORIZED {
        let erro: RespostaDeErroDoXsts = resposta
            .json()
            .unwrap_or(RespostaDeErroDoXsts { codigo_do_erro: 0 });
        bail!("{}", explicar_erro_do_xsts(erro.codigo_do_erro));
    }
    if !resposta.status().is_success() {
        bail!("O Xbox recusou o login (HTTP {}).", resposta.status());
    }
    let dados: RespostaDoXbox = resposta.json().context("Resposta inesperada do XSTS")?;
    Ok(dados.token)
}

fn explicar_erro_do_xsts(codigo_do_erro: u64) -> &'static str {
    match codigo_do_erro {
        2148916233 => {
            "Esta conta Microsoft não tem perfil Xbox. Entre uma vez em minecraft.net para criá-lo."
        }
        2148916235 => "O Xbox Live não está disponível no país desta conta.",
        2148916236 | 2148916237 => "Esta conta precisa de verificação de idade no site da Xbox.",
        2148916238 => {
            "Conta de menor de idade: um adulto precisa adicioná-la a uma Família Microsoft."
        }
        _ => "O Xbox recusou o login desta conta.",
    }
}

fn entrar_no_minecraft(
    cliente: &Client,
    hash_do_usuario: &str,
    token_xsts: &str,
) -> Result<RespostaDoLoginMinecraft> {
    let corpo = json!({ "identityToken": format!("XBL3.0 x={hash_do_usuario};{token_xsts}") });
    let resposta = cliente
        .post(ENDERECO_DO_LOGIN_MINECRAFT)
        .json(&corpo)
        .send()
        .context("Não foi possível falar com os servidores do Minecraft")?;

    match resposta.status() {
        status if status.is_success() => resposta
            .json()
            .context("Resposta inesperada do Minecraft ao entrar"),
        StatusCode::FORBIDDEN => bail!(
            "A Mojang ainda não liberou este aplicativo Azure para a API do Minecraft (HTTP 403). \
             Envie o formulário de permissão da Mojang e aguarde a aprovação."
        ),
        StatusCode::TOO_MANY_REQUESTS => {
            bail!("Muitas tentativas de login seguidas. Aguarde alguns minutos.")
        }
        status => bail!("O Minecraft recusou o login (HTTP {status})."),
    }
}

fn buscar_perfil_do_minecraft(
    cliente: &Client,
    token_minecraft: &str,
) -> Result<PerfilDoMinecraft> {
    let resposta = cliente
        .get(ENDERECO_DO_PERFIL_MINECRAFT)
        .bearer_auth(token_minecraft)
        .send()
        .context("Não foi possível buscar o perfil do Minecraft")?;
    match resposta.status() {
        status if status.is_success() => resposta.json().context("Perfil do Minecraft inválido"),
        StatusCode::NOT_FOUND => {
            bail!("Esta conta não possui Minecraft: Java Edition (ou ainda não criou um nome de jogador).")
        }
        status => bail!("Não foi possível ler o perfil do Minecraft (HTTP {status})."),
    }
}

/// Mostra so o codigo e a primeira frase do erro da Microsoft (o resto sao IDs de rastreio).
fn descrever_erro_microsoft(texto: &str) -> String {
    serde_json::from_str::<RespostaDeErroMicrosoft>(texto)
        .map(|erro| {
            let primeira_frase = erro
                .descricao_do_erro
                .split(". ")
                .next()
                .unwrap_or_default()
                .trim_end_matches('.');
            format!("({}) {primeira_frase}.", erro.codigo_do_erro)
        })
        .unwrap_or_else(|_| texto.chars().take(300).collect())
}
