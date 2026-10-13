//! Estado da aplicação (sem I/O de terminal).

use std::path::PathBuf;

use ratatui::crossterm::event::KeyCode;

use crate::executor::{EstadoRepoEnum, OperacaoEnum, Tarefa};
use crate::pool::Evento;
use crate::repo::{Repo, SituacaoRepoEnum};

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
    /// Segundo `q`/`Esc` durante a saída: matar os comandos em execução.
    Abortar,
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
    /// Segundo `q`/`Esc` durante a saída: comandos em execução foram mortos.
    pub abortando: bool,
    /// `PoolEncerrado` recebido: o laço principal termina.
    pub encerrado: bool,
}

impl App {
    pub fn new(operacao: OperacaoEnum, remote: String, raiz: PathBuf, repos: Vec<Repo>) -> App {
        let quantidade = repos.len();
        App {
            operacao,
            remote,
            raiz,
            repos,
            estados: vec![EstadoRepoEnum::Pendente; quantidade],
            logs: vec![String::new(); quantidade],
            branches: vec![None; quantidade],
            selecao: 0,
            detalhe_aberto: false,
            rolagem: 0,
            status: None,
            saindo: false,
            abortando: false,
            encerrado: false,
        }
    }

    pub fn tarefas_iniciais(&self) -> Vec<Tarefa> {
        self.repos
            .iter()
            .enumerate()
            .filter(|(_, repo)| repo.situacao == SituacaoRepoEnum::Clonado)
            .map(|(indice, _)| self.tarefa(indice, false))
            .collect()
    }

    fn tarefa(&self, indice: usize, sem_rebase: bool) -> Tarefa {
        Tarefa {
            indice,
            repo: self.repos[indice].clone(),
            operacao: self.operacao,
            remote: self.remote.clone(),
            sem_rebase,
        }
    }

    pub fn linhas(&self) -> Vec<LinhaEnum> {
        let mut falhas = Vec::new();
        let mut sucessos = Vec::new();
        let mut executando = Vec::new();
        let mut pendentes = Vec::new();
        let mut sem_mudanca = Vec::new();
        let mut nao_clonados = Vec::new();
        let mut sem_repositorio = Vec::new();
        for (indice, repo) in self.repos.iter().enumerate() {
            match repo.situacao {
                SituacaoRepoEnum::NaoClonado => nao_clonados.push(indice),
                SituacaoRepoEnum::SemRepositorio => sem_repositorio.push(indice),
                SituacaoRepoEnum::Clonado => match self.estados[indice] {
                    EstadoRepoEnum::Falhou(_) => falhas.push(LinhaEnum::Repo(indice)),
                    EstadoRepoEnum::Recebido(_) | EstadoRepoEnum::Enviado => {
                        sucessos.push(LinhaEnum::Repo(indice))
                    }
                    EstadoRepoEnum::Executando => executando.push(LinhaEnum::Repo(indice)),
                    EstadoRepoEnum::Pendente => pendentes.push(LinhaEnum::Repo(indice)),
                    EstadoRepoEnum::SemMudanca => sem_mudanca.push(indice),
                },
            }
        }
        falhas.extend(sucessos);
        falhas.extend(executando);
        falhas.extend(pendentes);
        if !sem_mudanca.is_empty() {
            falhas.push(LinhaEnum::SemMudanca(sem_mudanca));
        }
        if !nao_clonados.is_empty() {
            falhas.push(LinhaEnum::NaoClonados(nao_clonados));
        }
        if !sem_repositorio.is_empty() {
            falhas.push(LinhaEnum::SemRepositorio(sem_repositorio));
        }
        falhas
    }

    pub fn aplicar(&mut self, evento: Evento) {
        match evento {
            Evento::Iniciou(indice) => self.estados[indice] = EstadoRepoEnum::Executando,
            Evento::Terminou(indice, resultado) => {
                self.estados[indice] = resultado.estado;
                self.branches[indice] = resultado.branch;
                self.logs[indice] = resultado.log;
            }
            Evento::PoolEncerrado => self.encerrado = true,
        }
        self.selecao = self.selecao.min(self.linhas().len().saturating_sub(1));
        if self.abortando {
            self.status = Some(format!("abortando {} tarefas…", self.executando()));
        } else if self.saindo {
            self.status = Some(format!(
                "aguardando {} tarefas… (q de novo aborta)",
                self.executando()
            ));
        }
    }

    pub fn executando(&self) -> usize {
        self.repos
            .iter()
            .enumerate()
            .filter(|(i, repo)| {
                repo.situacao == SituacaoRepoEnum::Clonado
                    && self.estados[*i] == EstadoRepoEnum::Executando
            })
            .count()
    }

    pub fn concluidos(&self) -> (usize, usize) {
        let mut concluidos = 0;
        let mut total = 0;
        for (indice, repo) in self.repos.iter().enumerate() {
            if repo.situacao != SituacaoRepoEnum::Clonado {
                continue;
            }
            total += 1;
            if matches!(
                self.estados[indice],
                EstadoRepoEnum::SemMudanca
                    | EstadoRepoEnum::Recebido(_)
                    | EstadoRepoEnum::Enviado
                    | EstadoRepoEnum::Falhou(_)
            ) {
                concluidos += 1;
            }
        }
        (concluidos, total)
    }

    pub fn ha_trabalho(&self) -> bool {
        self.repos.iter().enumerate().any(|(indice, repo)| {
            repo.situacao == SituacaoRepoEnum::Clonado
                && (self.estados[indice] == EstadoRepoEnum::Executando
                    || (!self.saindo && self.estados[indice] == EstadoRepoEnum::Pendente))
        })
    }

    fn repetir(&mut self, sem_rebase: bool) -> Vec<AcaoEnum> {
        if let Some(LinhaEnum::Repo(indice)) = self.linhas().get(self.selecao) {
            let indice = *indice;
            if matches!(
                self.estados[indice],
                EstadoRepoEnum::SemMudanca
                    | EstadoRepoEnum::Recebido(_)
                    | EstadoRepoEnum::Enviado
                    | EstadoRepoEnum::Falhou(_)
            ) {
                self.estados[indice] = EstadoRepoEnum::Pendente;
                self.logs[indice].clear();
                return vec![AcaoEnum::Enviar(self.tarefa(indice, sem_rebase))];
            }
        }
        self.status = Some("ação indisponível para esta linha".into());
        Vec::new()
    }

    pub fn tratar_tecla(&mut self, tecla: KeyCode) -> Vec<AcaoEnum> {
        match tecla {
            KeyCode::Up | KeyCode::Char('k') => {
                self.selecao = self.selecao.saturating_sub(1);
                self.rolagem = 0;
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.selecao = (self.selecao + 1).min(self.linhas().len().saturating_sub(1));
                self.rolagem = 0;
            }
            KeyCode::Enter => {
                self.detalhe_aberto = !self.detalhe_aberto;
                self.rolagem = 0;
            }
            KeyCode::PageDown if self.detalhe_aberto => {
                self.rolagem = self.rolagem.saturating_add(10)
            }
            KeyCode::PageUp if self.detalhe_aberto => {
                self.rolagem = self.rolagem.saturating_sub(10)
            }
            KeyCode::Esc if self.detalhe_aberto => self.detalhe_aberto = false,
            KeyCode::Esc | KeyCode::Char('q') => {
                if !self.saindo {
                    self.saindo = true;
                    self.status = Some(format!(
                        "aguardando {} tarefas… (q de novo aborta)",
                        self.executando()
                    ));
                    return vec![AcaoEnum::Sair];
                }
                if !self.abortando && self.executando() > 0 {
                    self.abortando = true;
                    self.status = Some(format!("abortando {} tarefas…", self.executando()));
                    return vec![AcaoEnum::Abortar];
                }
            }
            KeyCode::Char('r' | 'n' | 'R' | 'p') if self.saindo => {
                self.status = Some("saindo… ações desabilitadas".into());
            }
            KeyCode::Char('r') => return self.repetir(false),
            KeyCode::Char('n') => {
                if self.operacao == OperacaoEnum::Push {
                    self.status = Some("indisponível no push".into());
                } else {
                    return self.repetir(true);
                }
            }
            KeyCode::Char('R') => {
                let falhas: Vec<_> = self
                    .repos
                    .iter()
                    .enumerate()
                    .filter(|(indice, repo)| {
                        repo.situacao == SituacaoRepoEnum::Clonado
                            && matches!(self.estados[*indice], EstadoRepoEnum::Falhou(_))
                    })
                    .map(|(indice, _)| indice)
                    .collect();
                if falhas.is_empty() {
                    self.status = Some("nenhuma falha para repetir".into());
                } else {
                    let acoes = falhas
                        .iter()
                        .map(|&indice| AcaoEnum::Enviar(self.tarefa(indice, false)))
                        .collect();
                    for indice in falhas {
                        self.estados[indice] = EstadoRepoEnum::Pendente;
                        self.logs[indice].clear();
                    }
                    return acoes;
                }
            }
            KeyCode::Char('p') => {
                if let Some(LinhaEnum::Repo(indice)) = self.linhas().get(self.selecao) {
                    return vec![AcaoEnum::AbrirPane(self.repos[*indice].caminho.clone())];
                }
                self.status = Some("ação indisponível para esta linha".into());
            }
            _ => {}
        }
        Vec::new()
    }

    pub fn texto_detalhe(&self) -> Vec<String> {
        match self.linhas().get(self.selecao) {
            Some(LinhaEnum::Repo(indice)) => {
                let mut linhas = vec![format!(
                    "branch: {}",
                    self.branches[*indice].as_deref().unwrap_or("?")
                )];
                if let EstadoRepoEnum::Falhou(erro) = &self.estados[*indice] {
                    linhas.push(erro.dica().into());
                }
                linhas.push(String::new());
                linhas.extend(self.logs[*indice].lines().map(str::to_owned));
                linhas
            }
            Some(linha) => {
                let (titulo, indices) = match linha {
                    LinhaEnum::SemMudanca(indices) => ("sem mudança", indices),
                    LinhaEnum::NaoClonados(indices) => ("não clonados", indices),
                    LinhaEnum::SemRepositorio(indices) => ("pastas sem repositório", indices),
                    LinhaEnum::Repo(_) => unreachable!(),
                };
                let mut linhas = Vec::with_capacity(indices.len() + 1);
                linhas.push(titulo.into());
                linhas.extend(
                    indices
                        .iter()
                        .map(|&indice| self.repos[indice].nome.clone()),
                );
                linhas
            }
            None => Vec::new(),
        }
    }

    pub fn resumo(&self) -> String {
        let mut sem_mudanca = 0;
        let mut alterados = 0;
        let mut falhas = Vec::new();
        let mut nao_clonados = 0;
        let mut sem_repositorio = 0;
        for (indice, repo) in self.repos.iter().enumerate() {
            match repo.situacao {
                SituacaoRepoEnum::NaoClonado => nao_clonados += 1,
                SituacaoRepoEnum::SemRepositorio => sem_repositorio += 1,
                SituacaoRepoEnum::Clonado => match self.estados[indice] {
                    EstadoRepoEnum::SemMudanca => sem_mudanca += 1,
                    EstadoRepoEnum::Recebido(_) | EstadoRepoEnum::Enviado => alterados += 1,
                    EstadoRepoEnum::Falhou(_) => falhas.push(repo.nome.as_str()),
                    EstadoRepoEnum::Pendente | EstadoRepoEnum::Executando => {}
                },
            }
        }
        let (operacao, alteracao) = match self.operacao {
            OperacaoEnum::Pull => ("pull", "recebidos"),
            OperacaoEnum::Push => ("push", "enviados"),
        };
        let nomes = if falhas.is_empty() {
            String::new()
        } else {
            format!(" ({})", falhas.join(", "))
        };
        format!(
            "metra {operacao}: {sem_mudanca} sem mudança, {alterados} {alteracao}, {} falhas{nomes}, {nao_clonados} não clonados, {sem_repositorio} sem repositório",
            falhas.len()
        )
    }

    pub fn codigo_saida(&self) -> i32 {
        i32::from(self.repos.iter().enumerate().any(|(indice, repo)| {
            repo.situacao == SituacaoRepoEnum::Clonado
                && matches!(self.estados[indice], EstadoRepoEnum::Falhou(_))
        }))
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use ratatui::crossterm::event::KeyCode;

    use super::{AcaoEnum, App, LinhaEnum};
    use crate::classifica::TipoErroEnum;
    use crate::executor::{EstadoRepoEnum, OperacaoEnum, Resultado};
    use crate::pool::Evento;
    use crate::repo::{Repo, SituacaoRepoEnum};

    fn repo(nome: &str, situacao: SituacaoRepoEnum) -> Repo {
        Repo {
            nome: nome.into(),
            caminho: PathBuf::from("/tmp/x").join(nome),
            situacao,
        }
    }

    fn novo_app(operacao: OperacaoEnum, repos: Vec<Repo>) -> App {
        App::new(operacao, "upstream".into(), PathBuf::from("/tmp/x"), repos)
    }

    fn terminar(app: &mut App, indice: usize, estado: EstadoRepoEnum, log: &str) {
        app.aplicar(Evento::Terminou(
            indice,
            Resultado {
                estado,
                branch: Some("principal".into()),
                log: log.into(),
            },
        ));
    }

    #[test]
    fn inicia_estados_e_tarefas_apenas_para_clonados_na_ordem_da_fonte() {
        let app = novo_app(
            OperacaoEnum::Pull,
            vec![
                repo("primeiro", SituacaoRepoEnum::Clonado),
                repo("ausente", SituacaoRepoEnum::NaoClonado),
                repo("segundo", SituacaoRepoEnum::Clonado),
                repo("pasta", SituacaoRepoEnum::SemRepositorio),
            ],
        );
        assert_eq!(app.estados, vec![EstadoRepoEnum::Pendente; 4]);
        assert_eq!(app.logs, vec![String::new(); 4]);
        assert_eq!(app.branches, vec![None; 4]);
        assert_eq!(app.selecao, 0);
        assert_eq!(app.status, None);
        assert!(!app.saindo && !app.encerrado);
        let tarefas = app.tarefas_iniciais();
        assert_eq!(
            tarefas.iter().map(|t| t.indice).collect::<Vec<_>>(),
            vec![0, 2]
        );
        assert!(
            tarefas.iter().all(|t| t.operacao == OperacaoEnum::Pull
                && t.remote == "upstream"
                && !t.sem_rebase)
        );
    }

    #[test]
    fn ordena_grupos_e_preserva_ordem_da_fonte_em_cada_grupo() {
        let mut app = novo_app(
            OperacaoEnum::Pull,
            (0..12)
                .map(|i| {
                    repo(
                        &format!("r{i}"),
                        match i {
                            4 | 10 => SituacaoRepoEnum::NaoClonado,
                            5 => SituacaoRepoEnum::SemRepositorio,
                            _ => SituacaoRepoEnum::Clonado,
                        },
                    )
                })
                .collect(),
        );
        terminar(&mut app, 0, EstadoRepoEnum::Recebido(2), "");
        terminar(
            &mut app,
            1,
            EstadoRepoEnum::Falhou(TipoErroEnum::Divergente),
            "",
        );
        terminar(&mut app, 2, EstadoRepoEnum::Enviado, "");
        terminar(&mut app, 3, EstadoRepoEnum::SemMudanca, "");
        app.aplicar(Evento::Iniciou(6));
        terminar(
            &mut app,
            7,
            EstadoRepoEnum::Falhou(TipoErroEnum::MergePendente),
            "",
        );
        terminar(&mut app, 8, EstadoRepoEnum::SemMudanca, "");
        app.aplicar(Evento::Iniciou(9));
        assert_eq!(
            app.linhas(),
            vec![
                LinhaEnum::Repo(1),
                LinhaEnum::Repo(7),
                LinhaEnum::Repo(0),
                LinhaEnum::Repo(2),
                LinhaEnum::Repo(6),
                LinhaEnum::Repo(9),
                LinhaEnum::Repo(11),
                LinhaEnum::SemMudanca(vec![3, 8]),
                LinhaEnum::NaoClonados(vec![4, 10]),
                LinhaEnum::SemRepositorio(vec![5]),
            ]
        );
        assert_eq!(app.concluidos(), (6, 9));
        assert_eq!(app.executando(), 2);
        assert!(app.ha_trabalho());
    }

    #[test]
    fn test_omite_grupos_agregados_vazios() {
        let mut app = novo_app(
            OperacaoEnum::Push,
            vec![repo("um", SituacaoRepoEnum::Clonado)],
        );
        assert_eq!(app.linhas(), vec![LinhaEnum::Repo(0)]);
        assert_eq!(app.concluidos(), (0, 1));
        terminar(&mut app, 0, EstadoRepoEnum::Enviado, "");
        assert_eq!(app.linhas(), vec![LinhaEnum::Repo(0)]);
        assert!(!app.ha_trabalho());
    }

    #[test]
    fn test_nao_executa_repos_nao_clonados() {
        let app = novo_app(
            OperacaoEnum::Pull,
            vec![repo("ausente", SituacaoRepoEnum::NaoClonado)],
        );
        assert_eq!(app.linhas(), vec![LinhaEnum::NaoClonados(vec![0])]);
        assert_eq!(app.concluidos(), (0, 0));
        assert!(!app.ha_trabalho());
    }

    #[test]
    fn aplica_eventos_substitui_log_e_mantem_selecao_valida() {
        let mut app = novo_app(
            OperacaoEnum::Pull,
            vec![
                repo("a", SituacaoRepoEnum::Clonado),
                repo("b", SituacaoRepoEnum::Clonado),
            ],
        );
        app.selecao = 12;
        app.aplicar(Evento::Iniciou(0));
        assert_eq!(app.estados[0], EstadoRepoEnum::Executando);
        assert_eq!(app.selecao, 1);
        terminar(
            &mut app,
            0,
            EstadoRepoEnum::Falhou(TipoErroEnum::Divergente),
            "log antigo",
        );
        assert_eq!(app.branches[0].as_deref(), Some("principal"));
        terminar(&mut app, 0, EstadoRepoEnum::Recebido(1), "log novo");
        assert_eq!(app.logs[0], "log novo");
        app.aplicar(Evento::PoolEncerrado);
        assert!(app.encerrado);
    }

    #[test]
    fn navega_sem_wrap_e_rola_somente_detalhe_aberto() {
        let mut app = novo_app(
            OperacaoEnum::Pull,
            vec![
                repo("a", SituacaoRepoEnum::Clonado),
                repo("b", SituacaoRepoEnum::Clonado),
            ],
        );
        assert!(app.tratar_tecla(KeyCode::Up).is_empty());
        assert_eq!(app.selecao, 0);
        app.tratar_tecla(KeyCode::Down);
        app.tratar_tecla(KeyCode::Char('j'));
        assert_eq!(app.selecao, 1);
        app.tratar_tecla(KeyCode::PageDown);
        assert_eq!(app.rolagem, 0);
        app.tratar_tecla(KeyCode::Enter);
        assert!(app.detalhe_aberto);
        app.tratar_tecla(KeyCode::PageDown);
        app.tratar_tecla(KeyCode::PageDown);
        assert_eq!(app.rolagem, 20);
        app.tratar_tecla(KeyCode::PageUp);
        assert_eq!(app.rolagem, 10);
        app.tratar_tecla(KeyCode::Char('k'));
        assert_eq!((app.selecao, app.rolagem), (0, 0));
        app.tratar_tecla(KeyCode::PageUp);
        assert_eq!(app.rolagem, 0);
        app.tratar_tecla(KeyCode::Esc);
        assert!(!app.detalhe_aberto && !app.saindo);
        app.tratar_tecla(KeyCode::Enter);
        app.tratar_tecla(KeyCode::Enter);
        assert!(!app.detalhe_aberto && app.rolagem == 0);
        assert_eq!(app.tratar_tecla(KeyCode::Esc), vec![AcaoEnum::Sair]);
    }

    #[test]
    fn sair_aguarda_somente_tarefas_em_execucao_e_desabilita_acoes() {
        let mut app = novo_app(
            OperacaoEnum::Pull,
            vec![
                repo("a", SituacaoRepoEnum::Clonado),
                repo("b", SituacaoRepoEnum::Clonado),
            ],
        );
        app.aplicar(Evento::Iniciou(0));
        assert_eq!(app.tratar_tecla(KeyCode::Char('q')), vec![AcaoEnum::Sair]);
        assert_eq!(
            app.status.as_deref(),
            Some("aguardando 1 tarefas… (q de novo aborta)")
        );
        assert!(app.ha_trabalho());
        terminar(&mut app, 0, EstadoRepoEnum::SemMudanca, "");
        assert_eq!(
            app.status.as_deref(),
            Some("aguardando 0 tarefas… (q de novo aborta)")
        );
        assert!(app.tratar_tecla(KeyCode::Char('q')).is_empty());
        assert!(!app.ha_trabalho());
        app.aplicar(Evento::PoolEncerrado);
        assert!(app.encerrado);
        assert_eq!(
            app.status.as_deref(),
            Some("aguardando 0 tarefas… (q de novo aborta)")
        );
        for tecla in ['r', 'n', 'R', 'p'] {
            assert!(app.tratar_tecla(KeyCode::Char(tecla)).is_empty());
            assert_eq!(app.status.as_deref(), Some("saindo… ações desabilitadas"));
        }
    }

    #[test]
    fn segundo_sair_aborta_uma_vez_enquanto_ha_execucao() {
        let mut app = novo_app(
            OperacaoEnum::Pull,
            vec![
                repo("a", SituacaoRepoEnum::Clonado),
                repo("b", SituacaoRepoEnum::Clonado),
            ],
        );
        app.aplicar(Evento::Iniciou(0));
        app.aplicar(Evento::Iniciou(1));
        assert_eq!(app.tratar_tecla(KeyCode::Char('q')), vec![AcaoEnum::Sair]);
        assert_eq!(app.tratar_tecla(KeyCode::Esc), vec![AcaoEnum::Abortar]);
        assert!(app.abortando);
        assert_eq!(app.status.as_deref(), Some("abortando 2 tarefas…"));
        assert!(app.tratar_tecla(KeyCode::Char('q')).is_empty());
        terminar(&mut app, 0, EstadoRepoEnum::SemMudanca, "");
        assert_eq!(app.status.as_deref(), Some("abortando 1 tarefas…"));
    }

    #[test]
    fn detalhe_mostra_branch_dica_log_ou_nomes_agregados() {
        let mut app = novo_app(
            OperacaoEnum::Pull,
            vec![
                repo("falha", SituacaoRepoEnum::Clonado),
                repo("ok", SituacaoRepoEnum::Clonado),
                repo("ausente", SituacaoRepoEnum::NaoClonado),
                repo("pasta", SituacaoRepoEnum::SemRepositorio),
            ],
        );
        terminar(
            &mut app,
            0,
            EstadoRepoEnum::Falhou(TipoErroEnum::Divergente),
            "linha um\nlinha dois\n",
        );
        terminar(&mut app, 1, EstadoRepoEnum::SemMudanca, "");
        assert_eq!(
            app.texto_detalhe(),
            vec![
                "branch: principal",
                "use [n] para merge com --no-rebase",
                "",
                "linha um",
                "linha dois"
            ]
        );
        app.selecao = 1;
        assert_eq!(app.texto_detalhe(), vec!["sem mudança", "ok"]);
        app.selecao = 2;
        assert_eq!(app.texto_detalhe(), vec!["não clonados", "ausente"]);
        app.selecao = 3;
        assert_eq!(app.texto_detalhe(), vec!["pastas sem repositório", "pasta"]);
    }

    #[test]
    fn resumo_pull_conta_terminais_ignora_pendentes_e_lista_falhas_na_ordem() {
        let mut app = novo_app(
            OperacaoEnum::Pull,
            vec![
                repo("erro1", SituacaoRepoEnum::Clonado),
                repo("pendente", SituacaoRepoEnum::Clonado),
                repo("ok", SituacaoRepoEnum::Clonado),
                repo("erro2", SituacaoRepoEnum::Clonado),
                repo("recebido", SituacaoRepoEnum::Clonado),
                repo("ausente", SituacaoRepoEnum::NaoClonado),
                repo("pasta", SituacaoRepoEnum::SemRepositorio),
            ],
        );
        terminar(
            &mut app,
            0,
            EstadoRepoEnum::Falhou(TipoErroEnum::Divergente),
            "",
        );
        terminar(&mut app, 2, EstadoRepoEnum::SemMudanca, "");
        terminar(
            &mut app,
            3,
            EstadoRepoEnum::Falhou(TipoErroEnum::MergePendente),
            "",
        );
        terminar(&mut app, 4, EstadoRepoEnum::Recebido(3), "");
        assert_eq!(
            app.resumo(),
            "metra pull: 1 sem mudança, 1 recebidos, 2 falhas (erro1, erro2), 1 não clonados, 1 sem repositório"
        );
        assert_eq!(app.codigo_saida(), 1);
    }

    #[test]
    fn test_resumo_pull_sem_falhas_ignora_pendentes() {
        let app = novo_app(
            OperacaoEnum::Pull,
            vec![repo("pendente", SituacaoRepoEnum::Clonado)],
        );
        assert_eq!(
            app.resumo(),
            "metra pull: 0 sem mudança, 0 recebidos, 0 falhas, 0 não clonados, 0 sem repositório"
        );
        assert_eq!(app.codigo_saida(), 0);
    }

    #[test]
    fn test_resumo_push_sem_falhas_usa_rotulo_enviados() {
        let mut app = novo_app(
            OperacaoEnum::Push,
            vec![
                repo("enviado", SituacaoRepoEnum::Clonado),
                repo("quieto", SituacaoRepoEnum::Clonado),
            ],
        );
        terminar(&mut app, 0, EstadoRepoEnum::Enviado, "");
        terminar(&mut app, 1, EstadoRepoEnum::SemMudanca, "");
        assert_eq!(
            app.resumo(),
            "metra push: 1 sem mudança, 1 enviados, 0 falhas, 0 não clonados, 0 sem repositório"
        );
        assert_eq!(app.codigo_saida(), 0);
    }

    #[test]
    fn repetir_terminal_limpa_log_e_reenvia_com_ou_sem_rebase() {
        let mut app = novo_app(
            OperacaoEnum::Pull,
            vec![repo("falha", SituacaoRepoEnum::Clonado)],
        );
        terminar(
            &mut app,
            0,
            EstadoRepoEnum::Falhou(TipoErroEnum::Divergente),
            "log antigo",
        );
        let acoes = app.tratar_tecla(KeyCode::Char('n'));
        assert_eq!(acoes.len(), 1);
        let AcaoEnum::Enviar(tarefa) = &acoes[0] else {
            panic!("ação esperada: Enviar")
        };
        assert_eq!(
            (
                tarefa.indice,
                tarefa.repo.nome.as_str(),
                tarefa.operacao,
                tarefa.remote.as_str(),
                tarefa.sem_rebase
            ),
            (0, "falha", OperacaoEnum::Pull, "upstream", true)
        );
        assert_eq!(app.estados[0], EstadoRepoEnum::Pendente);
        assert!(app.logs[0].is_empty());
        terminar(&mut app, 0, EstadoRepoEnum::Recebido(2), "novo log");
        let acoes = app.tratar_tecla(KeyCode::Char('r'));
        let AcaoEnum::Enviar(tarefa) = &acoes[0] else {
            panic!("ação esperada: Enviar")
        };
        assert!(!tarefa.sem_rebase);
        assert!(app.logs[0].is_empty());
    }

    #[test]
    fn repetir_todas_as_falhas_preserva_ordem_e_ignora_outros_estados() {
        let mut app = novo_app(
            OperacaoEnum::Pull,
            vec![
                repo("a", SituacaoRepoEnum::Clonado),
                repo("b", SituacaoRepoEnum::Clonado),
                repo("c", SituacaoRepoEnum::Clonado),
                repo("d", SituacaoRepoEnum::NaoClonado),
            ],
        );
        terminar(
            &mut app,
            0,
            EstadoRepoEnum::Falhou(TipoErroEnum::Divergente),
            "erro a",
        );
        terminar(&mut app, 1, EstadoRepoEnum::SemMudanca, "sucesso");
        terminar(
            &mut app,
            2,
            EstadoRepoEnum::Falhou(TipoErroEnum::MergePendente),
            "erro c",
        );
        let acoes = app.tratar_tecla(KeyCode::Char('R'));
        assert_eq!(
            acoes
                .iter()
                .map(|acao| match acao {
                    AcaoEnum::Enviar(tarefa) => tarefa.indice,
                    _ => panic!("esperado Enviar"),
                })
                .collect::<Vec<_>>(),
            vec![0, 2]
        );
        assert!(
            acoes
                .iter()
                .all(|acao| matches!(acao, AcaoEnum::Enviar(tarefa) if !tarefa.sem_rebase))
        );
        assert_eq!(app.estados[0], EstadoRepoEnum::Pendente);
        assert_eq!(app.estados[1], EstadoRepoEnum::SemMudanca);
        assert_eq!(app.estados[2], EstadoRepoEnum::Pendente);
        assert!(app.logs[0].is_empty() && app.logs[2].is_empty());
        assert!(app.tratar_tecla(KeyCode::Char('R')).is_empty());
        assert_eq!(app.status.as_deref(), Some("nenhuma falha para repetir"));
    }

    #[test]
    fn test_ignora_repetir_em_agregadas_pendentes_e_executando() {
        let mut app = novo_app(
            OperacaoEnum::Pull,
            vec![
                repo("um", SituacaoRepoEnum::Clonado),
                repo("ausente", SituacaoRepoEnum::NaoClonado),
            ],
        );
        for tecla in ['r', 'n'] {
            assert!(app.tratar_tecla(KeyCode::Char(tecla)).is_empty());
            assert_eq!(
                app.status.as_deref(),
                Some("ação indisponível para esta linha")
            );
        }
        app.aplicar(Evento::Iniciou(0));
        assert!(app.tratar_tecla(KeyCode::Char('r')).is_empty());
        app.selecao = 1;
        for tecla in ['r', 'n'] {
            assert!(app.tratar_tecla(KeyCode::Char(tecla)).is_empty());
            assert_eq!(
                app.status.as_deref(),
                Some("ação indisponível para esta linha")
            );
        }
        terminar(&mut app, 0, EstadoRepoEnum::SemMudanca, "");
        app.selecao = 0;
        for tecla in ['r', 'n'] {
            assert!(app.tratar_tecla(KeyCode::Char(tecla)).is_empty());
            assert_eq!(
                app.status.as_deref(),
                Some("ação indisponível para esta linha")
            );
        }
    }

    #[test]
    fn test_ignora_repetir_com_sem_rebase_no_push() {
        let mut app = novo_app(
            OperacaoEnum::Push,
            vec![repo("um", SituacaoRepoEnum::Clonado)],
        );
        terminar(&mut app, 0, EstadoRepoEnum::Enviado, "");
        assert!(app.tratar_tecla(KeyCode::Char('n')).is_empty());
        assert_eq!(app.status.as_deref(), Some("indisponível no push"));
    }

    #[test]
    fn abre_pane_somente_para_linha_de_repo() {
        let mut app = novo_app(
            OperacaoEnum::Pull,
            vec![
                repo("um", SituacaoRepoEnum::Clonado),
                repo("ausente", SituacaoRepoEnum::NaoClonado),
            ],
        );
        assert_eq!(
            app.tratar_tecla(KeyCode::Char('p')),
            vec![AcaoEnum::AbrirPane(PathBuf::from("/tmp/x/um"))]
        );
        app.selecao = 1;
        assert!(app.tratar_tecla(KeyCode::Char('p')).is_empty());
        assert_eq!(
            app.status.as_deref(),
            Some("ação indisponível para esta linha")
        );
    }
}
