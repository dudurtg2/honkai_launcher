//! Inicia o Minecraft a partir de uma versao ja instalada.
//!
//! Monta a linha de comando do Java seguindo o JSON da versao:
//! `java <argumentos jvm> <classe principal> <argumentos do jogo>`
//! e troca cada `${variavel}` pelo valor certo.
//!
//! Aqui, no futuro, entra o `-javaagent:agente_honkai.jar` (Fase 2 do planejamento).

use std::collections::HashMap;
use std::fs::{self, File};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

use anyhow::{Context, Result};
use instalador_de_versoes::modelos_da_mojang::Argumento;
use instalador_de_versoes::regras::regras_permitem_uso;
use instalador_de_versoes::{PastasDoHonkai, VersaoInstalada};

const NOME_DO_LAUNCHER: &str = "honkai-client";
const VERSAO_DO_LAUNCHER: &str = env!("CARGO_PKG_VERSION");

/// Dados do jogador que o Minecraft recebe na inicializacao.
#[derive(Debug, Clone)]
pub struct DadosDoJogador {
    pub nome_do_jogador: String,
    pub uuid_do_jogador: String,
    pub token_de_acesso_minecraft: String,
    /// "msa" para conta Microsoft.
    pub tipo_de_usuario: String,
}

#[derive(Debug, Clone)]
pub struct PedidoDeInicio {
    pub versao: VersaoInstalada,
    pub pastas: PastasDoHonkai,
    pub jogador: DadosDoJogador,
    pub memoria_maxima_em_megabytes: u32,
    pub id_do_cliente_azure: String,
}

/// Inicia o jogo e devolve o processo, para o launcher saber quando ele fechar.
/// A saida do jogo vai para `registros/ultimo_inicio_do_jogo.log`.
pub fn iniciar_jogo(pedido: &PedidoDeInicio) -> Result<Child> {
    let mut comando = montar_comando_do_jogo(pedido)?;

    let pasta_dos_registros = pedido.pastas.pasta_dos_registros();
    fs::create_dir_all(&pasta_dos_registros)?;
    let arquivo_de_registro = File::create(pasta_dos_registros.join("ultimo_inicio_do_jogo.log"))
        .context("Não foi possível criar o arquivo de registro do jogo")?;
    let copia_para_erros = arquivo_de_registro.try_clone()?;

    comando
        .stdin(Stdio::null())
        .stdout(Stdio::from(arquivo_de_registro))
        .stderr(Stdio::from(copia_para_erros));

    comando.spawn().with_context(|| {
        format!(
            "Não foi possível iniciar o Java em {}",
            pedido.versao.caminho_do_executavel_java.display()
        )
    })
}

/// Monta o comando completo sem executar. Separado para poder ser testado.
pub fn montar_comando_do_jogo(pedido: &PedidoDeInicio) -> Result<Command> {
    let detalhes = &pedido.versao.detalhes;
    let pasta_do_jogo = pedido.pastas.pasta_do_jogo();
    let pasta_dos_natives = pedido.pastas.pasta_dos_natives(&detalhes.id);
    fs::create_dir_all(&pasta_do_jogo)?;
    fs::create_dir_all(&pasta_dos_natives)?;

    let valores = montar_tabela_de_variaveis(pedido, &pasta_do_jogo, &pasta_dos_natives);

    let mut argumentos = vec![format!("-Xmx{}M", pedido.memoria_maxima_em_megabytes)];
    argumentos.extend(resolver_argumentos(&detalhes.argumentos.jvm, &valores));

    if let (Some(caminho), Some(log)) = (
        &pedido.versao.caminho_da_configuracao_de_log,
        detalhes
            .configuracao_de_log
            .as_ref()
            .and_then(|log| log.cliente.as_ref()),
    ) {
        argumentos.push(log.argumento.replace("${path}", &caminho.to_string_lossy()));
    }

    argumentos.push(detalhes.classe_principal.clone());
    argumentos.extend(resolver_argumentos(&detalhes.argumentos.jogo, &valores));

    let mut comando = Command::new(&pedido.versao.caminho_do_executavel_java);
    comando.args(&argumentos).current_dir(&pasta_do_jogo);
    Ok(comando)
}

fn montar_tabela_de_variaveis(
    pedido: &PedidoDeInicio,
    pasta_do_jogo: &std::path::Path,
    pasta_dos_natives: &std::path::Path,
) -> HashMap<&'static str, String> {
    let detalhes = &pedido.versao.detalhes;
    let jogador = &pedido.jogador;
    let texto = |caminho: &std::path::Path| caminho.to_string_lossy().into_owned();

    HashMap::from([
        ("auth_player_name", jogador.nome_do_jogador.clone()),
        ("version_name", detalhes.id.clone()),
        ("game_directory", texto(pasta_do_jogo)),
        ("assets_root", texto(&pedido.pastas.pasta_dos_assets())),
        ("assets_index_name", detalhes.indice_de_assets.id.clone()),
        ("auth_uuid", jogador.uuid_do_jogador.clone()),
        (
            "auth_access_token",
            jogador.token_de_acesso_minecraft.clone(),
        ),
        ("clientid", pedido.id_do_cliente_azure.clone()),
        ("auth_xuid", "0".to_string()),
        ("user_type", jogador.tipo_de_usuario.clone()),
        ("version_type", detalhes.tipo.clone()),
        ("natives_directory", texto(pasta_dos_natives)),
        ("launcher_name", NOME_DO_LAUNCHER.to_string()),
        ("launcher_version", VERSAO_DO_LAUNCHER.to_string()),
        ("classpath", montar_classpath(&pedido.versao)),
    ])
}

/// Bibliotecas + jar do cliente, separados por `;` no Windows e `:` nos outros sistemas.
pub fn montar_classpath(versao: &VersaoInstalada) -> String {
    let separador = if cfg!(windows) { ";" } else { ":" };
    let mut caminhos: Vec<PathBuf> = Vec::new();
    for caminho in &versao.caminhos_das_bibliotecas {
        if !caminhos.contains(caminho) {
            caminhos.push(caminho.clone());
        }
    }
    caminhos.push(versao.caminho_do_jar_do_cliente.clone());
    caminhos
        .iter()
        .map(|caminho| caminho.to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join(separador)
}

fn resolver_argumentos(
    argumentos: &[Argumento],
    valores: &HashMap<&'static str, String>,
) -> Vec<String> {
    let mut resultado = Vec::new();
    for argumento in argumentos {
        match argumento {
            Argumento::Simples(texto) => resultado.push(substituir_variaveis(texto, valores)),
            Argumento::Condicional { regras, valor } => {
                if regras_permitem_uso(Some(regras)) {
                    for texto in valor.como_lista() {
                        resultado.push(substituir_variaveis(&texto, valores));
                    }
                }
            }
        }
    }
    resultado
}

/// Troca `${nome}` pelo valor da tabela. Variavel desconhecida fica como esta.
fn substituir_variaveis(texto: &str, valores: &HashMap<&'static str, String>) -> String {
    let mut resultado = texto.to_string();
    for (nome, valor) in valores {
        resultado = resultado.replace(&format!("${{{nome}}}"), valor);
    }
    resultado
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn troca_variaveis_conhecidas() {
        let valores = HashMap::from([("auth_player_name", "Dudu".to_string())]);
        assert_eq!(
            substituir_variaveis("${auth_player_name}", &valores),
            "Dudu"
        );
        assert_eq!(
            substituir_variaveis("-Dx=${desconhecida}", &valores),
            "-Dx=${desconhecida}"
        );
    }
}
