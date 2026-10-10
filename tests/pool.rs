use std::collections::HashSet;
use std::path::Path;
use std::process::Command;
use std::sync::mpsc;
use std::time::Duration;

use metra::executor::{EstadoRepoEnum, OperacaoEnum, Tarefa};
use metra::pool::{Evento, Pool};
use metra::repo::{Repo, SituacaoRepoEnum};
use tempfile::TempDir;

fn git(argumentos: &[&str]) {
    let saida = Command::new("git")
        .args(argumentos)
        .output()
        .expect("git deve estar disponível");
    assert!(
        saida.status.success(),
        "git {argumentos:?} falhou: {}",
        String::from_utf8_lossy(&saida.stderr)
    );
}

fn tarefas(raiz: &Path, quantidade: usize) -> Vec<Tarefa> {
    (0..quantidade)
        .map(|indice| {
            let nome = format!("repo{indice}");
            let caminho = raiz.join(&nome);
            let bare = raiz.join(format!("up{indice}.git"));
            let bare = bare.to_str().expect("caminho temporário em UTF-8");
            let clone = caminho.to_str().expect("caminho temporário em UTF-8");

            git(&["init", "--bare", "-b", "main", bare]);
            git(&["clone", bare, clone]);
            git(&["-C", clone, "config", "user.name", "Teste Metra"]);
            git(&["-C", clone, "config", "user.email", "teste@metra.local"]);
            git(&["-C", clone, "commit", "--allow-empty", "-m", "base"]);
            git(&["-C", clone, "remote", "rename", "origin", "upstream"]);
            git(&["-C", clone, "push", "-u", "upstream", "main"]);

            Tarefa {
                indice,
                repo: Repo {
                    nome,
                    caminho,
                    situacao: SituacaoRepoEnum::Clonado,
                },
                operacao: OperacaoEnum::Pull,
                remote: "upstream".into(),
                sem_rebase: false,
            }
        })
        .collect()
}

#[test]
fn limita_execucoes_simultaneas_e_notifica_encerramento() {
    let temporario = TempDir::new().expect("criar diretório temporário");
    let tarefas = tarefas(temporario.path(), 4);
    let (sender, eventos) = mpsc::channel();
    let mut pool = Pool::iniciar(2, sender);
    for tarefa in tarefas {
        pool.enviar(tarefa);
    }

    let mut iniciados = HashSet::new();
    let mut terminados = HashSet::new();
    let mut em_execucao = 0;
    let mut maximo_em_execucao = 0;
    while terminados.len() < 4 {
        match eventos
            .recv_timeout(Duration::from_secs(30))
            .expect("receber evento do pool")
        {
            Evento::Iniciou(indice) => {
                assert!(
                    iniciados.insert(indice),
                    "índice iniciado duas vezes: {indice}"
                );
                em_execucao += 1;
                maximo_em_execucao = maximo_em_execucao.max(em_execucao);
            }
            Evento::Terminou(indice, resultado) => {
                assert!(
                    iniciados.contains(&indice),
                    "tarefa terminou sem iniciar: {indice}"
                );
                assert!(
                    terminados.insert(indice),
                    "índice terminado duas vezes: {indice}"
                );
                assert!(
                    matches!(resultado.estado, EstadoRepoEnum::SemMudanca),
                    "pull do repo {indice} falhou: {}",
                    resultado.log
                );
                em_execucao -= 1;
            }
            Evento::PoolEncerrado => panic!("pool encerrou antes das quatro tarefas"),
        }
    }
    assert_eq!(iniciados, HashSet::from([0, 1, 2, 3]));
    assert_eq!(terminados, HashSet::from([0, 1, 2, 3]));
    assert_eq!(em_execucao, 0);
    assert!(maximo_em_execucao <= 2);

    pool.encerrar();
    assert!(matches!(
        eventos
            .recv_timeout(Duration::from_secs(30))
            .expect("receber encerramento"),
        Evento::PoolEncerrado
    ));
}

#[test]
fn descarta_fila_ao_encerrar_sem_interromper_execucao_corrente() {
    let temporario = TempDir::new().expect("criar diretório temporário");
    let tarefas = tarefas(temporario.path(), 3);
    let (sender, eventos) = mpsc::channel();
    let mut pool = Pool::iniciar(1, sender);
    for tarefa in tarefas {
        pool.enviar(tarefa);
    }

    assert!(matches!(
        eventos
            .recv_timeout(Duration::from_secs(30))
            .expect("receber primeiro início"),
        Evento::Iniciou(_)
    ));
    pool.encerrar();

    let mut terminados = 0;
    loop {
        match eventos
            .recv_timeout(Duration::from_secs(30))
            .expect("receber encerramento")
        {
            Evento::Terminou(_, _) => terminados += 1,
            Evento::PoolEncerrado => break,
            Evento::Iniciou(_) => {}
        }
    }
    assert!(terminados < 3, "tarefas pendentes deveriam ser descartadas");
}
