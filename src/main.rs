//! Entrada da CLI e laço de eventos da TUI.

use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use clap::{Args, Parser, Subcommand};
use metra::acoes::{abrir_navegador, abrir_pane_herdr};
use metra::app::{AcaoEnum, App};
use metra::executor::OperacaoEnum;
use metra::fonte::{FonteEnum, descobrir};
use metra::pool::{Evento, Pool};
use metra::ui::desenhar;
use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, Event, KeyEventKind};

/// Executa pull ou push nos repositórios de um workspace.
#[derive(Parser)]
#[command(
    name = "metra",
    about = "Executa git pull/push em paralelo nos repositórios do workspace"
)]
struct Cli {
    #[command(subcommand)]
    comando: ComandoEnum,
}

/// Operação escolhida na linha de comando.
#[derive(Subcommand)]
enum ComandoEnum {
    /// Recebe alterações da branch atual de cada repositório.
    Pull(Opcoes),
    /// Envia alterações da branch atual de cada repositório.
    Push(Opcoes),
}

/// Opções comuns às operações de pull e push.
#[derive(Args)]
struct Opcoes {
    /// Quantidade de repositórios processados em paralelo.
    #[arg(short = 'j', long = "jobs", default_value_t = 8, value_parser = parse_jobs)]
    jobs: usize,
    /// Nome do remote usado em cada repositório.
    #[arg(long)]
    remote: Option<String>,
    /// Fonte dos repositórios do workspace.
    #[arg(long, value_enum, default_value_t = FonteEnum::Auto)]
    fonte: FonteEnum,
}

/// Rejeita pools sem trabalhadores.
fn parse_jobs(valor: &str) -> Result<usize, String> {
    match valor.parse::<usize>() {
        Ok(jobs) if jobs > 0 => Ok(jobs),
        _ => Err(format!(
            "número de jobs inválido: {valor} (use um inteiro maior que zero)"
        )),
    }
}

fn main() {
    let (operacao, opcoes, remote_padrao) = match Cli::parse().comando {
        ComandoEnum::Pull(opcoes) => (OperacaoEnum::Pull, opcoes, "upstream"),
        ComandoEnum::Push(opcoes) => (OperacaoEnum::Push, opcoes, "origin"),
    };
    let descoberta = std::env::current_dir()
        .map_err(anyhow::Error::from)
        .and_then(|cwd| descobrir(opcoes.fonte, &cwd));
    let (raiz, repos) = match descoberta {
        Ok(resultado) => resultado,
        Err(erro) => {
            eprintln!("metra: {erro:#}");
            std::process::exit(2);
        }
    };

    let mut app = App::new(
        operacao,
        opcoes.remote.unwrap_or_else(|| remote_padrao.into()),
        raiz,
        repos,
    );
    let (tx, rx) = mpsc::channel::<Evento>();
    let mut pool = Pool::iniciar(opcoes.jobs, tx);
    for tarefa in app.tarefas_iniciais() {
        pool.enviar(tarefa);
    }

    let mut terminal = ratatui::init();
    let resultado = rodar(&mut terminal, &mut app, &mut pool, &rx);
    ratatui::restore();
    if let Err(erro) = &resultado {
        eprintln!("metra: {erro:#}");
    }
    println!("{}", app.resumo());
    for linha in app.linhas_merge() {
        println!("{linha}");
    }
    std::process::exit(if resultado.is_err() {
        1
    } else {
        app.codigo_saida()
    });
}

/// Atualiza a tela até todos os trabalhadores terminarem após a saída.
fn rodar(
    terminal: &mut DefaultTerminal,
    app: &mut App,
    pool: &mut Pool,
    eventos: &Receiver<Evento>,
) -> anyhow::Result<()> {
    let mut quadro = 0_usize;
    loop {
        while let Ok(evento) = eventos.try_recv() {
            app.aplicar(evento);
        }
        if app.encerrado {
            break;
        }

        terminal.draw(|frame| desenhar(frame, app, quadro))?;
        quadro = quadro.wrapping_add(1);
        if event::poll(Duration::from_millis(100))?
            && let Event::Key(tecla) = event::read()?
            && tecla.kind == KeyEventKind::Press
        {
            for acao in app.tratar_tecla(tecla.code) {
                match acao {
                    AcaoEnum::Enviar(tarefa) => pool.enviar(tarefa),
                    AcaoEnum::AbrirPane(caminho) => {
                        if let Err(mensagem) = abrir_pane_herdr(&caminho) {
                            app.status = Some(mensagem);
                        }
                    }
                    AcaoEnum::AbrirNavegador(url) => {
                        app.status = Some(match abrir_navegador(&url) {
                            Ok(()) => "link de merge aberto no navegador".into(),
                            Err(mensagem) => mensagem,
                        });
                    }
                    AcaoEnum::Sair => pool.encerrar(),
                    AcaoEnum::Abortar => pool.abortar(),
                }
            }
        }
    }
    Ok(())
}
