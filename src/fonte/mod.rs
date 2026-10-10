//! Descoberta dos repositórios do workspace (`.meta` ou varredura de subpastas).

mod meta;
mod varredura;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};

use self::meta::FonteMeta;
use self::varredura::FonteVarredura;

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

/// Escolhe a fonte e a raiz do workspace a partir do modo selecionado.
pub fn resolver_fonte(modo: FonteEnum, cwd: &Path) -> Result<(PathBuf, Box<dyn FonteRepos>)> {
    let raiz = std::path::absolute(cwd)
        .with_context(|| format!("erro ao resolver caminho {}", cwd.display()))?;
    if modo == FonteEnum::Varredura {
        return Ok((raiz.clone(), Box::new(FonteVarredura { raiz })));
    }

    for ancestral in raiz.ancestors() {
        let arquivo = ancestral.join(".meta");
        if arquivo.exists() {
            let raiz = ancestral.to_path_buf();
            return Ok((raiz.clone(), Box::new(FonteMeta { raiz, arquivo })));
        }
    }
    if modo == FonteEnum::Meta {
        Err(anyhow!(
            "nenhum .meta encontrado a partir de {}",
            cwd.display()
        ))
    } else {
        Ok((raiz.clone(), Box::new(FonteVarredura { raiz })))
    }
}

/// Descobre os repositórios e rejeita workspaces sem nenhum resultado.
pub fn descobrir(modo: FonteEnum, cwd: &Path) -> Result<(PathBuf, Vec<Repo>)> {
    let (raiz, fonte) = resolver_fonte(modo, cwd)?;
    let repos = fonte.listar()?;
    if repos.is_empty() {
        return Err(anyhow!(
            "nenhum repositório encontrado em {}",
            raiz.display()
        ));
    }
    Ok((raiz, repos))
}
