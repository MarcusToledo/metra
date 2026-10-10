//! Repositório do workspace e sua situação no disco (módulo neutro entre fonte e executor).

use std::path::{Path, PathBuf};

/// Situação de um caminho listado pela fonte, calculada antes da execução.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SituacaoRepoEnum {
    /// `<caminho>/.git` existe (diretório ou arquivo).
    Clonado,
    /// O caminho não existe.
    NaoClonado,
    /// O caminho existe, mas sem `.git` próprio.
    SemRepositorio,
}

/// Repositório descoberto pela fonte.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Repo {
    /// Chave do `.meta` (caminho relativo, pode conter `/`) ou nome da subpasta.
    pub nome: String,
    /// Caminho absoluto: `raiz.join(nome)`.
    pub caminho: PathBuf,
    pub situacao: SituacaoRepoEnum,
}

/// Classifica o caminho sem consultar repositórios ancestrais.
pub fn situacao(caminho: &Path) -> SituacaoRepoEnum {
    if caminho.join(".git").exists() {
        SituacaoRepoEnum::Clonado
    } else if !caminho.exists() {
        SituacaoRepoEnum::NaoClonado
    } else {
        SituacaoRepoEnum::SemRepositorio
    }
}
