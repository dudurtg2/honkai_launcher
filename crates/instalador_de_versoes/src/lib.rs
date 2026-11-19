//! Instalacao das versoes do Minecraft.
//!
//! Nada da Mojang e redistribuido: tudo e baixado dos servidores oficiais na maquina do jogador.
//!
//! Organizacao:
//! - `versoes_validadas`  lista das versoes que o Honkai Client aceita iniciar
//! - `pastas`             onde cada coisa fica no disco
//! - `modelos_da_mojang`  formato dos arquivos JSON da Mojang
//! - `regras`             regras de sistema operacional das bibliotecas e argumentos
//! - `baixador`           download paralelo com verificacao de SHA-1
//! - `runtime_java`       instalacao do Java certo para cada versao
//! - `instalacao`         junta tudo: `instalar_versao`

pub mod baixador;
pub mod instalacao;
pub mod modelos_da_mojang;
pub mod pastas;
pub mod regras;
pub mod runtime_java;
pub mod versoes_validadas;

pub use baixador::{ModoDeVerificacao, ProgressoDoDownload};
pub use instalacao::{instalar_versao, EtapaDaInstalacao, VersaoInstalada};
pub use pastas::PastasDoHonkai;
pub use versoes_validadas::VERSOES_VALIDADAS;
