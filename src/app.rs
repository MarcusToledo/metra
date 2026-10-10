//! Estado da aplicação (sem I/O de terminal).

use std::path::PathBuf;

use crate::executor::{EstadoRepoEnum, OperacaoEnum, Tarefa};
use crate::repo::Repo;

/// Linha exibida na lista da TUI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinhaEnum {
    /// Repo individual (Falhou, Recebido/Enviado, Executando ou Pendente).
    Repo(usize),
    SemMudanca(Vec<usize>),
    NaoClonados(Vec<usize>),
    SemRepositorio(Vec<usize>),
}

/// Efeito pedido pelo `App` ao laço principal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AcaoEnum {
    Enviar(Tarefa),
    AbrirPane(PathBuf),
    Sair,
}

/// Estado da tela.
#[derive(Debug)]
pub struct App {
    pub operacao: OperacaoEnum,
    pub remote: String,
    pub raiz: PathBuf,
    /// Ordem da fonte.
    pub repos: Vec<Repo>,
    /// Por índice de `repos`; repos não Clonado ficam em `Pendente` e nunca executam.
    pub estados: Vec<EstadoRepoEnum>,
    pub logs: Vec<String>,
    pub branches: Vec<Option<String>>,
    /// Índice em `linhas()`.
    pub selecao: usize,
    pub detalhe_aberto: bool,
    pub rolagem: u16,
    pub status: Option<String>,
    /// `q` pressionado: ações desabilitadas, aguardando o pool.
    pub saindo: bool,
    /// `PoolEncerrado` recebido: o laço principal termina.
    pub encerrado: bool,
}
