//! Fonte baseada na varredura das subpastas imediatas da raiz.

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};

use super::FonteRepos;
use crate::repo::{Repo, SituacaoRepoEnum};

pub(super) struct FonteVarredura {
    pub(super) raiz: PathBuf,
}

impl FonteRepos for FonteVarredura {
    fn listar(&self) -> Result<Vec<Repo>> {
        let entradas = fs::read_dir(&self.raiz)
            .with_context(|| format!("erro ao ler {}", self.raiz.display()))?;
        let mut repos = Vec::new();
        for entrada in entradas {
            let entrada =
                entrada.with_context(|| format!("erro ao ler {}", self.raiz.display()))?;
            let nome = entrada.file_name();
            if nome.to_string_lossy().starts_with('.') || !entrada.file_type()?.is_dir() {
                continue;
            }

            let caminho = entrada.path();
            if caminho.join(".git").exists() {
                repos.push(Repo {
                    nome: nome.to_string_lossy().into_owned(),
                    caminho,
                    situacao: SituacaoRepoEnum::Clonado,
                });
            }
        }
        repos.sort_by(|esquerdo, direito| esquerdo.nome.cmp(&direito.nome));
        Ok(repos)
    }
}
