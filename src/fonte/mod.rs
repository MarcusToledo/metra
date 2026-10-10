//! Descoberta dos repositórios do workspace (`.meta` ou varredura de subpastas).

mod meta;
mod varredura;

use crate::repo::Repo;

/// Modo de descoberta escolhido em `--fonte`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum FonteEnum {
    Auto,
    Meta,
    Varredura,
}

/// Fonte de repositórios; novas fontes implementam esta trait.
pub trait FonteRepos {
    fn listar(&self) -> anyhow::Result<Vec<Repo>>;
}
