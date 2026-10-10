//! Execução síncrona de pull/push num repositório.

use crate::classifica::TipoErroEnum;
use crate::repo::Repo;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperacaoEnum {
    Pull,
    Push,
}

/// Unidade de trabalho enviada ao pool.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tarefa {
    /// Posição do repo no vetor do `App`.
    pub indice: usize,
    pub repo: Repo,
    pub operacao: OperacaoEnum,
    pub remote: String,
    /// `git pull --no-rebase` (só pull, via tecla `n`).
    pub sem_rebase: bool,
}

/// Estado de um repositório na TUI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EstadoRepoEnum {
    Pendente,
    Executando,
    SemMudanca,
    /// Quantidade de commits que entraram na branch.
    Recebido(u32),
    Enviado,
    Falhou(TipoErroEnum),
}

/// Resultado de uma tentativa; `estado` nunca é `Pendente`/`Executando`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resultado {
    pub estado: EstadoRepoEnum,
    /// `None` se a branch não pôde ser lida.
    pub branch: Option<String>,
    /// stdout+stderr de todos os comandos da tentativa.
    pub log: String,
}
