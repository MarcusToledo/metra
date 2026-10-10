//! Pool fixo de threads que executa tarefas e emite eventos para a TUI.

use crate::executor::Resultado;

/// Evento emitido pelo pool.
#[derive(Debug)]
pub enum Evento {
    Iniciou(usize),
    Terminou(usize, Resultado),
    PoolEncerrado,
}
