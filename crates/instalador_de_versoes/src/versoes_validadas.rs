//! Versoes do Minecraft que o Honkai Client aceita iniciar.
//!
//! Uma versao so entra aqui depois de passar no checklist do planejamento
//! (ganchos, animacoes, sincronizacao e desempenho testados).
//! No futuro esta lista vira o arquivo `versoes_validadas.json` servido pela API Honkai.

pub const VERSOES_VALIDADAS: &[&str] = &["1.21.8"];

pub fn versao_esta_validada(id_da_versao: &str) -> bool {
    VERSOES_VALIDADAS.contains(&id_da_versao)
}
