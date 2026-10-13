//! Pool fixo de threads que executa tarefas e emite eventos para a TUI.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;

use crate::executor::{self, Cancelamento, Resultado, Tarefa};
use crate::registro::{NivelEnum, registrar};

/// Evento emitido pelo pool.
#[derive(Debug)]
pub enum Evento {
    Iniciou(usize),
    Terminou(usize, Resultado),
    PoolEncerrado,
}

/// Pool de tamanho fixo que aceita tarefas até ser encerrado.
pub struct Pool {
    sender: Option<mpsc::Sender<Tarefa>>,
    cancelado: Arc<AtomicBool>,
    cancelamento: Arc<Cancelamento>,
}

impl Pool {
    /// Inicia pelo menos um trabalhador e envia `PoolEncerrado` quando todos terminam.
    pub fn iniciar(n: usize, eventos: mpsc::Sender<Evento>) -> Pool {
        let (sender, receptor) = mpsc::channel::<Tarefa>();
        let receptor = Arc::new(Mutex::new(receptor));
        let cancelado = Arc::new(AtomicBool::new(false));
        let cancelamento = Arc::new(Cancelamento::default());
        let trabalhadores: Vec<_> = (0..n.max(1))
            .map(|i| {
                let receptor = Arc::clone(&receptor);
                let cancelado = Arc::clone(&cancelado);
                let cancelamento = Arc::clone(&cancelamento);
                let eventos = eventos.clone();
                thread::Builder::new()
                    .name(format!("trabalhador-{i}"))
                    .spawn(move || {
                        loop {
                            let tarefa = {
                                let receptor =
                                    receptor.lock().expect("receptor de tarefas íntegro");
                                receptor.recv()
                            };
                            let Ok(tarefa) = tarefa else {
                                break;
                            };
                            if cancelado.load(Ordering::Acquire) {
                                registrar(
                                    NivelEnum::Info,
                                    "pool",
                                    &format!(
                                        "tarefa {} ({}) descartada: saída em andamento",
                                        tarefa.indice, tarefa.repo.nome
                                    ),
                                );
                                continue;
                            }
                            let _ = eventos.send(Evento::Iniciou(tarefa.indice));
                            let resultado = executor::executar(&tarefa, &cancelamento);
                            let _ = eventos.send(Evento::Terminou(tarefa.indice, resultado));
                        }
                    })
                    .expect("não foi possível criar thread do pool")
            })
            .collect();

        thread::Builder::new()
            .name("pool".into())
            .spawn(move || {
                for trabalhador in trabalhadores {
                    if trabalhador.join().is_err() {
                        registrar(NivelEnum::Erro, "pool", "trabalhador terminou com panic");
                    }
                }
                registrar(NivelEnum::Info, "pool", "todos os trabalhadores terminaram");
                let _ = eventos.send(Evento::PoolEncerrado);
            })
            .expect("não foi possível criar thread do pool");

        Pool {
            sender: Some(sender),
            cancelado,
            cancelamento,
        }
    }

    /// Enfileira uma tarefa enquanto o pool estiver ativo.
    pub fn enviar(&self, tarefa: Tarefa) {
        if !self.cancelado.load(Ordering::Acquire)
            && let Some(sender) = &self.sender
        {
            let _ = sender.send(tarefa);
        }
    }

    /// Descarta pendentes; tarefas iniciadas terminam antes de `PoolEncerrado`.
    pub fn encerrar(&mut self) {
        registrar(
            NivelEnum::Info,
            "pool",
            "saída iniciada: tarefas pendentes descartadas",
        );
        self.cancelado.store(true, Ordering::Release);
        self.sender.take();
    }

    /// Encerra e mata os comandos git em execução; as tarefas terminam como abortadas.
    pub fn abortar(&mut self) {
        registrar(NivelEnum::Aviso, "pool", "aborto solicitado");
        self.encerrar();
        self.cancelamento.abortar();
    }
}
