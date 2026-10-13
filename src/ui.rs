//! Renderização da TUI a partir de `&App`.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, List, ListItem, ListState, Paragraph, Wrap};

use crate::app::{App, LinhaEnum};
use crate::executor::{EstadoRepoEnum, OperacaoEnum};

const QUADROS: [char; 10] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
const RODAPE: &str = "↑↓/jk navegar · Enter detalhe · PgUp/PgDn log · r repetir · n --no-rebase · R repetir falhas · p pane · m merge · q/Esc sair";
const RODAPE_COMPACTO: &str =
    "↑↓/jk · Enter · PgUp/PgDn · r · n --no-rebase · R · p · m · q/Esc sair";

/// Desenha o estado atual; `quadro` avança a cada desenho no laço principal.
pub fn desenhar(frame: &mut Frame, app: &App, quadro: usize) {
    let area = frame.area();
    let [cabecalho, lista, detalhe, status, rodape] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(if app.detalhe_aberto {
            area.height.saturating_sub(3) / 2
        } else {
            0
        }),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(area);

    let operacao = match app.operacao {
        OperacaoEnum::Pull => "pull",
        OperacaoEnum::Push => "push",
    };
    let (concluidos, total) = app.concluidos();
    let progresso = if app.ha_trabalho() {
        format!("{concluidos}/{total} {}", QUADROS[quadro % QUADROS.len()])
    } else {
        format!("{concluidos}/{total}")
    };
    let largura_progresso = u16::try_from(progresso.chars().count()).unwrap_or(u16::MAX);
    let [titulo, contador] =
        Layout::horizontal([Constraint::Min(0), Constraint::Length(largura_progresso)])
            .areas(cabecalho);
    frame.render_widget(
        Paragraph::new(format!(
            "metra {operacao} · remote {} · {}",
            app.remote,
            app.raiz.display()
        )),
        titulo,
    );
    frame.render_widget(Paragraph::new(progresso), contador);

    let itens: Vec<ListItem<'_>> = app
        .linhas()
        .into_iter()
        .map(|linha| {
            let (texto, cor) = match linha {
                LinhaEnum::Repo(indice) => {
                    let nome = &app.repos[indice].nome;
                    match &app.estados[indice] {
                        EstadoRepoEnum::Falhou(erro) => {
                            (format!("✗ {nome}  {}", erro.texto()), Color::Red)
                        }
                        EstadoRepoEnum::Recebido(commits) => (
                            format!(
                                "↓ {nome}  {commits} {}",
                                if *commits == 1 { "commit" } else { "commits" }
                            ),
                            Color::Green,
                        ),
                        EstadoRepoEnum::Enviado => {
                            let texto = if app.links_merge[indice].is_some() {
                                format!("↑ {nome}  ⇗ merge")
                            } else {
                                format!("↑ {nome}")
                            };
                            (texto, Color::Green)
                        }
                        EstadoRepoEnum::Executando => (
                            format!("{} {nome}", QUADROS[quadro % QUADROS.len()]),
                            Color::Yellow,
                        ),
                        EstadoRepoEnum::Pendente => (format!("· {nome}"), Color::Gray),
                        EstadoRepoEnum::SemMudanca => {
                            (format!("✓ 1 sem mudança ({nome})"), Color::Green)
                        }
                    }
                }
                LinhaEnum::SemMudanca(indices) => {
                    (format!("✓ {} sem mudança", indices.len()), Color::Green)
                }
                LinhaEnum::NaoClonados(indices) => {
                    (format!("○ {} não clonados", indices.len()), Color::Gray)
                }
                LinhaEnum::SemRepositorio(indices) => (
                    format!("⚠ {} pastas sem repositório", indices.len()),
                    Color::Yellow,
                ),
            };
            ListItem::new(Line::from(Span::styled(texto, Style::default().fg(cor))))
        })
        .collect();
    let mut selecao = ListState::default().with_selected(Some(app.selecao));
    frame.render_stateful_widget(
        List::new(itens).highlight_style(Style::default().add_modifier(Modifier::REVERSED)),
        lista,
        &mut selecao,
    );

    if app.detalhe_aberto && detalhe.height > 0 {
        frame.render_widget(
            Paragraph::new(app.texto_detalhe().join("\n"))
                .block(Block::bordered().title("Detalhe"))
                .wrap(Wrap { trim: false })
                .scroll((app.rolagem, 0)),
            detalhe,
        );
    }

    frame.render_widget(Paragraph::new(app.status.as_deref().unwrap_or("")), status);
    if usize::from(rodape.width) >= RODAPE.chars().count() {
        frame.render_widget(Paragraph::new(RODAPE), rodape);
    } else if usize::from(rodape.width) >= RODAPE_COMPACTO.chars().count() {
        frame.render_widget(Paragraph::new(RODAPE_COMPACTO), rodape);
    } else {
        let [atalhos, sair] =
            Layout::horizontal([Constraint::Min(0), Constraint::Length(10)]).areas(rodape);
        frame.render_widget(Paragraph::new(RODAPE_COMPACTO), atalhos);
        frame.render_widget(Paragraph::new("q/Esc sair"), sair);
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::crossterm::event::KeyCode;

    use super::desenhar;
    use crate::app::App;
    use crate::classifica::TipoErroEnum;
    use crate::executor::{EstadoRepoEnum, OperacaoEnum, Resultado};
    use crate::pool::Evento;
    use crate::repo::{Repo, SituacaoRepoEnum};

    fn aplicativo() -> App {
        let raiz = PathBuf::from("/workspace");
        let repos = (0..45)
            .map(|indice| Repo {
                nome: format!("repo{indice:02}"),
                caminho: raiz.join(format!("repo{indice:02}")),
                situacao: SituacaoRepoEnum::Clonado,
            })
            .collect();
        let mut app = App::new(OperacaoEnum::Pull, "upstream".into(), raiz, repos);
        for indice in 0..45 {
            let estado = if indice < 10 {
                EstadoRepoEnum::Falhou(TipoErroEnum::Divergente)
            } else if indice < 15 {
                EstadoRepoEnum::Recebido(2)
            } else {
                EstadoRepoEnum::SemMudanca
            };
            app.aplicar(Evento::Terminou(
                indice,
                Resultado {
                    estado,
                    branch: Some("build/teste".into()),
                    log: "fatal: Need to specify how to reconcile divergent branches.".into(),
                    link_merge: None,
                },
            ));
        }
        app
    }

    fn tela(app: &App) -> String {
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).expect("terminal de teste");
        terminal
            .draw(|frame| desenhar(frame, app, 0))
            .expect("desenho da tela");
        let buffer = terminal.backend().buffer();
        buffer
            .content()
            .chunks(80)
            .map(|linha| {
                linha
                    .iter()
                    .map(|celula| celula.symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn exibe_falhas_progresso_e_atalhos_no_terminal() {
        let texto = tela(&aplicativo());
        for indice in 0..10 {
            assert!(texto.contains(&format!("✗ repo{indice:02}")));
        }
        assert!(texto.contains("metra pull · remote upstream"));
        assert!(texto.contains("45/45"));
        assert!(texto.contains("q/Esc sair"));
    }

    #[test]
    fn exibe_branch_dica_e_log_ao_abrir_detalhe_da_falha() {
        let mut app = aplicativo();
        app.tratar_tecla(KeyCode::Enter);
        let texto = tela(&app);
        assert!(texto.contains("branch:"));
        assert!(texto.contains("use [n] para merge com --no-rebase"));
        assert!(texto.contains("Need to specify how to reconcile divergent branches"));
    }

    #[test]
    fn test_exibe_marca_e_link_de_merge_no_push() {
        let raiz = PathBuf::from("/workspace");
        let repos = vec![Repo {
            nome: "publica".into(),
            caminho: raiz.join("publica"),
            situacao: SituacaoRepoEnum::Clonado,
        }];
        let mut app = App::new(OperacaoEnum::Push, "origin".into(), raiz, repos);
        app.aplicar(Evento::Terminou(
            0,
            Resultado {
                estado: EstadoRepoEnum::Enviado,
                branch: Some("feat".into()),
                log: String::new(),
                link_merge: Some("https://gitlab.exemplo/g/p/-/merge_requests/12".into()),
            },
        ));
        assert!(tela(&app).contains("↑ publica  ⇗ merge"));
        app.tratar_tecla(KeyCode::Enter);
        assert!(tela(&app).contains("merge: https://"));
    }
}
