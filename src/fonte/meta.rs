//! Fonte baseada no arquivo `.meta`.

use std::fmt;
use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::de::{IgnoredAny, MapAccess, Visitor};
use serde::{Deserialize, Deserializer};

use super::FonteRepos;
use crate::repo::{Repo, situacao};

pub(super) struct FonteMeta {
    pub(super) raiz: PathBuf,
    pub(super) arquivo: PathBuf,
}

#[derive(Deserialize)]
struct ArquivoMeta {
    projects: ChavesOrdenadas,
}

struct ChavesOrdenadas(Vec<String>);

impl<'de> Deserialize<'de> for ChavesOrdenadas {
    fn deserialize<D>(deserializador: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct VisitanteChaves;

        impl<'de> Visitor<'de> for VisitanteChaves {
            type Value = ChavesOrdenadas;

            fn expecting(&self, formatador: &mut fmt::Formatter) -> fmt::Result {
                formatador.write_str("um objeto de projetos")
            }

            fn visit_map<M>(self, mut mapa: M) -> std::result::Result<Self::Value, M::Error>
            where
                M: MapAccess<'de>,
            {
                let mut chaves = Vec::new();
                while let Some((chave, _)) = mapa.next_entry::<String, IgnoredAny>()? {
                    chaves.push(chave);
                }
                Ok(ChavesOrdenadas(chaves))
            }
        }

        deserializador.deserialize_map(VisitanteChaves)
    }
}

impl FonteRepos for FonteMeta {
    fn listar(&self) -> Result<Vec<Repo>> {
        let conteudo = fs::read_to_string(&self.arquivo)
            .with_context(|| format!("erro ao ler {}", self.arquivo.display()))?;
        let arquivo: ArquivoMeta = serde_json::from_str(&conteudo).map_err(|erro| {
            let mensagem = format!("erro ao interpretar {}: {erro}", self.arquivo.display());
            anyhow::Error::new(erro).context(mensagem)
        })?;

        Ok(arquivo
            .projects
            .0
            .into_iter()
            .map(|nome| {
                let caminho = self.raiz.join(&nome);
                let situacao = situacao(&caminho);
                Repo {
                    nome,
                    caminho,
                    situacao,
                }
            })
            .collect())
    }
}
