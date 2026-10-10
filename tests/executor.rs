use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use metra::classifica::TipoErroEnum;
use metra::executor::{EstadoRepoEnum, OperacaoEnum, Tarefa, executar};
use metra::repo::{Repo, SituacaoRepoEnum};
use tempfile::TempDir;

fn verificar_ambiente() {
    for escopo in ["--global", "--system"] {
        for chave in ["pull.rebase", "pull.ff"] {
            let saida = Command::new("git")
                .args(["config", escopo, "--get", chave])
                .output()
                .expect("não foi possível consultar a configuração do git");
            match saida.status.code() {
                Some(1) => {}
                Some(0) => panic!(
                    "remova git config {escopo} {chave} antes dos testes: o cenário divergente depende da configuração padrão"
                ),
                _ => panic!(
                    "não foi possível consultar git config {escopo} {chave}: {}",
                    String::from_utf8_lossy(&saida.stderr)
                ),
            }
        }
    }
}

fn git(caminho: &Path, argumentos: &[&str]) -> String {
    let saida = Command::new("git")
        .arg("-C")
        .arg(caminho)
        .args(argumentos)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("LC_ALL", "C")
        .output()
        .expect("git não iniciou no teste");
    assert!(
        saida.status.success(),
        "git {argumentos:?} falhou: {}{}",
        String::from_utf8_lossy(&saida.stdout),
        String::from_utf8_lossy(&saida.stderr)
    );
    String::from_utf8(saida.stdout).expect("saída git não é UTF-8")
}

struct Cenario {
    _temp: TempDir,
    raiz: PathBuf,
    trab: PathBuf,
    aux: PathBuf,
}

impl Cenario {
    fn novo() -> Self {
        let temp = TempDir::new().expect("não foi possível criar diretório temporário");
        let raiz = temp.path().to_path_buf();
        git(&raiz, &["init", "--bare", "-b", "main", "up.git"]);
        let bare = raiz.join("up.git");
        git(&bare, &["config", "user.name", "Teste Metra"]);
        git(&bare, &["config", "user.email", "teste@exemplo.invalid"]);
        git(&raiz, &["clone", "up.git", "aux"]);
        let aux = raiz.join("aux");
        Self::configurar_usuario(&aux);
        git(&aux, &["checkout", "-b", "main"]);
        fs::write(aux.join("a.txt"), "base a\n").expect("não foi possível criar a.txt");
        fs::write(aux.join("b.txt"), "base b\n").expect("não foi possível criar b.txt");
        git(&aux, &["add", "a.txt", "b.txt"]);
        git(&aux, &["commit", "-m", "commit base"]);
        git(&aux, &["push", "origin", "main"]);
        git(&raiz, &["clone", "up.git", "trab"]);
        let trab = raiz.join("trab");
        Self::configurar_usuario(&trab);
        git(&trab, &["remote", "rename", "origin", "upstream"]);
        Self {
            _temp: temp,
            raiz,
            trab,
            aux,
        }
    }

    fn configurar_usuario(caminho: &Path) {
        git(caminho, &["config", "user.name", "Teste Metra"]);
        git(caminho, &["config", "user.email", "teste@exemplo.invalid"]);
    }

    fn commit(caminho: &Path, arquivo: &str, conteudo: &str) {
        fs::write(caminho.join(arquivo), conteudo).expect("não foi possível escrever arquivo");
        git(caminho, &["add", arquivo]);
        git(caminho, &["commit", "-m", "alteração de teste"]);
    }

    fn tarefa(&self, operacao: OperacaoEnum, sem_rebase: bool) -> Tarefa {
        Tarefa {
            indice: 0,
            repo: Repo {
                nome: "trab".into(),
                caminho: self.trab.clone(),
                situacao: SituacaoRepoEnum::Clonado,
            },
            operacao,
            remote: "upstream".into(),
            sem_rebase,
        }
    }
}

#[test]
fn test_pull_sem_mudanca() {
    verificar_ambiente();
    let cenario = Cenario::novo();
    let resultado = executar(&cenario.tarefa(OperacaoEnum::Pull, false));
    assert_eq!(resultado.estado, EstadoRepoEnum::SemMudanca);
    assert_eq!(resultado.branch.as_deref(), Some("main"));
    assert!(resultado.log.contains("$ git pull upstream main"));
}

#[test]
fn test_pull_recebe_dois_commits() {
    verificar_ambiente();
    let cenario = Cenario::novo();
    Cenario::commit(&cenario.aux, "a.txt", "novo a\n");
    Cenario::commit(&cenario.aux, "b.txt", "novo b\n");
    git(&cenario.aux, &["push", "origin", "main"]);
    let resultado = executar(&cenario.tarefa(OperacaoEnum::Pull, false));
    assert_eq!(resultado.estado, EstadoRepoEnum::Recebido(2));
    assert_eq!(resultado.branch.as_deref(), Some("main"));
    assert!(resultado.log.contains("$ git pull upstream main"));
    assert!(resultado.log.contains("Fast-forward"));
    assert!(resultado.log.contains("$ git rev-list --count"));
}

#[test]
fn test_pull_divergente() {
    verificar_ambiente();
    let cenario = Cenario::novo();
    Cenario::commit(&cenario.trab, "a.txt", "local\n");
    Cenario::commit(&cenario.aux, "b.txt", "remoto\n");
    git(&cenario.aux, &["push", "origin", "main"]);
    let resultado = executar(&cenario.tarefa(OperacaoEnum::Pull, false));
    assert_eq!(
        resultado.estado,
        EstadoRepoEnum::Falhou(TipoErroEnum::Divergente)
    );
}

#[test]
fn test_pull_divergente_com_merge() {
    verificar_ambiente();
    let cenario = Cenario::novo();
    Cenario::commit(&cenario.trab, "a.txt", "local\n");
    Cenario::commit(&cenario.aux, "b.txt", "remoto\n");
    git(&cenario.aux, &["push", "origin", "main"]);
    let resultado = executar(&cenario.tarefa(OperacaoEnum::Pull, true));
    assert_eq!(resultado.estado, EstadoRepoEnum::Recebido(2));
    assert!(
        resultado
            .log
            .contains("$ git pull --no-rebase upstream main")
    );
}

#[test]
fn test_pull_conflito() {
    verificar_ambiente();
    let cenario = Cenario::novo();
    Cenario::commit(&cenario.trab, "a.txt", "local\n");
    Cenario::commit(&cenario.aux, "a.txt", "remoto\n");
    git(&cenario.aux, &["push", "origin", "main"]);
    let resultado = executar(&cenario.tarefa(OperacaoEnum::Pull, true));
    assert_eq!(
        resultado.estado,
        EstadoRepoEnum::Falhou(TipoErroEnum::Conflito(vec!["a.txt".into()]))
    );
}

#[test]
fn test_pull_alteracoes_locais() {
    verificar_ambiente();
    let cenario = Cenario::novo();
    Cenario::commit(&cenario.aux, "a.txt", "remoto\n");
    git(&cenario.aux, &["push", "origin", "main"]);
    fs::write(cenario.trab.join("a.txt"), "local sem commit\n")
        .expect("não foi possível alterar a.txt");
    let resultado = executar(&cenario.tarefa(OperacaoEnum::Pull, false));
    assert_eq!(
        resultado.estado,
        EstadoRepoEnum::Falhou(TipoErroEnum::AlteracoesLocais)
    );
}

#[test]
fn test_pull_branch_remota_inexistente() {
    verificar_ambiente();
    let cenario = Cenario::novo();
    git(&cenario.trab, &["checkout", "-b", "feature"]);
    let resultado = executar(&cenario.tarefa(OperacaoEnum::Pull, false));
    assert_eq!(resultado.branch.as_deref(), Some("feature"));
    assert_eq!(
        resultado.estado,
        EstadoRepoEnum::Falhou(TipoErroEnum::BranchRemotaInexistente)
    );
}

#[test]
fn test_pull_remote_inexistente() {
    verificar_ambiente();
    let cenario = Cenario::novo();
    let mut tarefa = cenario.tarefa(OperacaoEnum::Pull, false);
    tarefa.remote = "nao-existe".into();
    let resultado = executar(&tarefa);
    assert_eq!(
        resultado.estado,
        EstadoRepoEnum::Falhou(TipoErroEnum::RemoteInexistente)
    );
}

#[test]
fn test_pull_head_destacado_sem_alterar_commit() {
    verificar_ambiente();
    let cenario = Cenario::novo();
    git(&cenario.trab, &["checkout", "--detach"]);
    let antes = git(&cenario.trab, &["rev-parse", "HEAD"]);
    let resultado = executar(&cenario.tarefa(OperacaoEnum::Pull, false));
    assert_eq!(
        resultado.estado,
        EstadoRepoEnum::Falhou(TipoErroEnum::HeadDestacado)
    );
    assert_eq!(resultado.branch.as_deref(), Some("HEAD"));
    assert!(!resultado.log.contains("$ git pull"));
    assert_eq!(git(&cenario.trab, &["rev-parse", "HEAD"]), antes);
}

#[test]
fn test_pasta_sem_repositorio_nao_altera_pai() {
    verificar_ambiente();
    let cenario = Cenario::novo();
    let pai = cenario.raiz.join("pai");
    git(&cenario.raiz, &["init", "-b", "main", "pai"]);
    Cenario::configurar_usuario(&pai);
    Cenario::commit(&pai, "a.txt", "pai\n");
    let antes = git(&pai, &["rev-parse", "HEAD"]);
    let filho = pai.join("filho");
    fs::create_dir_all(filho.join("build")).expect("não foi possível criar subpasta");
    let mut tarefa = cenario.tarefa(OperacaoEnum::Pull, false);
    tarefa.repo.caminho = filho;
    tarefa.repo.situacao = SituacaoRepoEnum::SemRepositorio;
    let resultado = executar(&tarefa);
    assert!(matches!(resultado.estado, EstadoRepoEnum::Falhou(_)));
    assert!(resultado.log.is_empty());
    assert_eq!(git(&pai, &["rev-parse", "HEAD"]), antes);
}

#[test]
fn test_push_envia_commit() {
    verificar_ambiente();
    let cenario = Cenario::novo();
    Cenario::commit(&cenario.trab, "a.txt", "local\n");
    let esperado = git(&cenario.trab, &["rev-parse", "HEAD"]);
    let resultado = executar(&cenario.tarefa(OperacaoEnum::Push, false));
    assert_eq!(resultado.estado, EstadoRepoEnum::Enviado);
    assert_eq!(resultado.branch.as_deref(), Some("main"));
    assert!(resultado.log.contains("$ git push upstream main"));
    assert_eq!(
        git(&cenario.raiz.join("up.git"), &["rev-parse", "main"]),
        esperado
    );
}

#[test]
fn test_push_sem_mudanca() {
    verificar_ambiente();
    let cenario = Cenario::novo();
    let resultado = executar(&cenario.tarefa(OperacaoEnum::Push, false));
    assert_eq!(resultado.estado, EstadoRepoEnum::SemMudanca);
    assert!(resultado.log.contains("Everything up-to-date"));
}

#[test]
fn test_push_rejeitado() {
    verificar_ambiente();
    let cenario = Cenario::novo();
    Cenario::commit(&cenario.trab, "a.txt", "local\n");
    Cenario::commit(&cenario.aux, "b.txt", "remoto\n");
    git(&cenario.aux, &["push", "origin", "main"]);
    let resultado = executar(&cenario.tarefa(OperacaoEnum::Push, false));
    assert_eq!(
        resultado.estado,
        EstadoRepoEnum::Falhou(TipoErroEnum::PushRejeitado)
    );
}
