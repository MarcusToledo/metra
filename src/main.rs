//! Entrada da CLI e laço de eventos da TUI.

use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use clap::{Args, Parser, Subcommand};
use metra::acoes::{abrir_navegador, abrir_pane_herdr};
use metra::app::{AcaoEnum, App};
use metra::executor::{OperacaoEnum, versao_git};
use metra::fonte::{FonteEnum, descobrir};
use metra::pool::{Evento, Pool};
use metra::registro::{self, NivelEnum, registrar};
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
    let inicio = Instant::now();
    let (operacao, opcoes, remote_padrao) = match Cli::parse().comando {
        ComandoEnum::Pull(opcoes) => (OperacaoEnum::Pull, opcoes, "upstream"),
        ComandoEnum::Push(opcoes) => (OperacaoEnum::Push, opcoes, "origin"),
    };
    let falha_registro = match registro::diretorio_padrao() {
        None => Some("log indisponível: HOME não definido".to_string()),
        Some(diretorio) => registro::iniciar(&diretorio)
            .err()
            .map(|erro| format!("log indisponível: {erro}")),
    };
    registro::instalar_hook_panic();

    let remote = opcoes.remote.unwrap_or_else(|| remote_padrao.into());
    registrar(
        NivelEnum::Info,
        "main",
        &format!(
            "metra {} {operacao:?} remote={remote} jobs={} fonte={:?}",
            env!("CARGO_PKG_VERSION"),
            opcoes.jobs,
            opcoes.fonte
        ),
    );
    registrar(
        NivelEnum::Info,
        "main",
        &format!(
            "git: {}",
            versao_git().unwrap_or_else(|| "indisponível".into())
        ),
    );

    let cwd = std::env::current_dir().map_err(anyhow::Error::from);
    let descoberta =
        cwd.and_then(|cwd| descobrir(opcoes.fonte, &cwd).map(|(raiz, repos)| (cwd, raiz, repos)));
    let (cwd, raiz, repos) = match descoberta {
        Ok(resultado) => resultado,
        Err(erro) => {
            registrar(NivelEnum::Erro, "fonte", &format!("{erro:#}"));
            eprintln!("metra: {erro:#}");
            if let Some(caminho) = registro::caminho_atual() {
                eprintln!("metra: log em {}", caminho.display());
            }
            std::process::exit(2);
        }
    };
    registrar(
        NivelEnum::Info,
        "fonte",
        &format!(
            "cwd={} raiz={} repos={}",
            cwd.display(),
            raiz.display(),
            repos.len()
        ),
    );
    for repo in &repos {
        registrar(
            NivelEnum::Info,
            "fonte",
            &format!(
                "{} {} {:?}",
                repo.nome,
                repo.caminho.display(),
                repo.situacao
            ),
        );
    }

    let mut app = App::new(operacao, remote, raiz, repos);
    if let Some(mensagem) = falha_registro {
        app.status = Some(mensagem);
    }
    let (tx, rx) = mpsc::channel::<Evento>();
    let mut pool = Pool::iniciar(opcoes.jobs, tx);
    for tarefa in app.tarefas_iniciais() {
        pool.enviar(tarefa);
    }

    let mut terminal = ratatui::init();
    let resultado = rodar(&mut terminal, &mut app, &mut pool, &rx);
    ratatui::restore();
    if let Err(erro) = &resultado {
        registrar(NivelEnum::Erro, "main", &format!("{erro:#}"));
        eprintln!("metra: {erro:#}");
    }
    let codigo = if resultado.is_err() {
        1
    } else {
        app.codigo_saida()
    };
    let resumo = app.resumo();
    registrar(
        NivelEnum::Info,
        "main",
        &format!(
            "fim código={codigo} duração={:.1}s: {resumo}",
            inicio.elapsed().as_secs_f64()
        ),
    );
    let linhas_merge = app.linhas_merge();
    for linha in &linhas_merge {
        registrar(NivelEnum::Info, "main", linha);
    }
    println!("{resumo}");
    for linha in linhas_merge {
        println!("{linha}");
    }
    if codigo != 0
        && let Some(caminho) = registro::caminho_atual()
    {
        eprintln!("metra: log em {}", caminho.display());
    }
    std::process::exit(codigo);
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
                registrar(NivelEnum::Info, "app", &format!("ação {acao:?}"));
                match acao {
                    AcaoEnum::Enviar(tarefa) => pool.enviar(tarefa),
                    AcaoEnum::AbrirPane(caminho) => {
                        if let Err(mensagem) = abrir_pane_herdr(&caminho) {
                            registrar(NivelEnum::Aviso, "acoes", &mensagem);
                            app.status = Some(mensagem);
                        }
                    }
                    AcaoEnum::AbrirNavegador(url) => {
                        app.status = Some(match abrir_navegador(&url) {
                            Ok(()) => "link de merge aberto no navegador".into(),
                            Err(mensagem) => {
                                registrar(NivelEnum::Aviso, "acoes", &mensagem);
                                mensagem
                            }
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
