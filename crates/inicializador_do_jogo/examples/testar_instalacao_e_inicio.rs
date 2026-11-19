//! Teste manual de ponta a ponta, sem interface e sem login:
//! instala a versao numa pasta temporaria e inicia o jogo com um jogador de teste.
//!
//! cargo run -p inicializador_do_jogo --example testar_instalacao_e_inicio -- <pasta>

use std::time::{Duration, Instant};

use anyhow::Result;
use inicializador_do_jogo::{iniciar_jogo, DadosDoJogador, PedidoDeInicio};
use instalador_de_versoes::{
    instalar_versao, EtapaDaInstalacao, ModoDeVerificacao, PastasDoHonkai,
};

fn main() -> Result<()> {
    let pasta = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/tmp/honkai_teste".to_string());
    let pastas = PastasDoHonkai::usar_pasta(&pasta);
    let cliente = reqwest::blocking::Client::builder()
        .user_agent("HonkaiClient-teste")
        .build()?;

    let inicio = Instant::now();
    let versao = instalar_versao(
        &cliente,
        &pastas,
        "1.21.8",
        ModoDeVerificacao::Rapida,
        &|etapa| {
            if let EtapaDaInstalacao::BaixandoArquivosDoJogo(p)
            | EtapaDaInstalacao::BaixandoJava(p) = &etapa
            {
                if p.arquivos_concluidos % 500 == 0 || p.arquivos_concluidos == p.total_de_arquivos
                {
                    println!(
                        "{:?}: {}/{} ({} MB)",
                        std::mem::discriminant(&etapa),
                        p.arquivos_concluidos,
                        p.total_de_arquivos,
                        p.bytes_baixados / 1_000_000
                    );
                }
            }
        },
    )?;
    println!(
        "Instalado em {:?}. Java: {}",
        inicio.elapsed(),
        versao.caminho_do_executavel_java.display()
    );
    println!(
        "Bibliotecas no classpath: {}",
        versao.caminhos_das_bibliotecas.len()
    );

    let pedido = PedidoDeInicio {
        versao,
        pastas: pastas.clone(),
        jogador: DadosDoJogador {
            nome_do_jogador: "Teste".into(),
            uuid_do_jogador: "00000000000000000000000000000000".into(),
            token_de_acesso_minecraft: "0".into(),
            tipo_de_usuario: "legacy".into(),
        },
        memoria_maxima_em_megabytes: 2048,
        id_do_cliente_azure: String::new(),
    };
    let mut processo = iniciar_jogo(&pedido)?;
    std::thread::sleep(Duration::from_secs(25));
    let _ = processo.kill();
    let _ = processo.wait();
    let registro = std::fs::read_to_string(
        pastas
            .pasta_dos_registros()
            .join("ultimo_inicio_do_jogo.log"),
    )?;
    println!(
        "----- registro do jogo -----\n{}",
        registro.chars().take(4000).collect::<String>()
    );
    Ok(())
}
