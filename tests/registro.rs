use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;

use metra::executor::{self, Cancelamento, OperacaoEnum, Tarefa};
use metra::registro::{self, NivelEnum};
use metra::repo::{Repo, SituacaoRepoEnum};
use tempfile::TempDir;

fn verificar_ambiente() {
    for escopo in ["--global", "--system"] {
        for chave in ["pull.rebase", "pull.ff"] {
            let saida = Command::new("git")
                .args(["config", escopo, "--get", chave])
                .env("GIT_TERMINAL_PROMPT", "0")
                .env("LC_ALL", "C")
                .stdin(Stdio::null())
                .output()
                .expect("não foi possível consultar a configuração do git");
            match saida.status.code() {
                Some(1) => {}
                Some(0) => panic!(
                    "remova git config {escopo} {chave} antes dos testes: o cenário depende da configuração padrão"
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
        .stdin(Stdio::null())
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

#[test]
fn test_registro_de_execucao() {
    verificar_ambiente();
    let temp = TempDir::new().expect("não foi possível criar diretório temporário");
    let raiz = temp.path();
    git(raiz, &["init", "--bare", "-b", "main", "up.git"]);
    git(raiz, &["clone", "up.git", "aux"]);
    let aux = raiz.join("aux");
    git(&aux, &["config", "user.name", "Teste Metra"]);
    git(&aux, &["config", "user.email", "teste@exemplo.invalid"]);
    git(&aux, &["checkout", "-b", "main"]);
    fs::write(aux.join("a.txt"), "base\n").expect("não foi possível criar a.txt");
    git(&aux, &["add", "a.txt"]);
    git(&aux, &["commit", "-m", "commit base"]);
    git(&aux, &["push", "origin", "main"]);
    git(raiz, &["clone", "up.git", "trab"]);
    let trab = raiz.join("trab");
    git(&trab, &["config", "user.name", "Teste Metra"]);
    git(&trab, &["config", "user.email", "teste@exemplo.invalid"]);
    git(&trab, &["remote", "rename", "origin", "upstream"]);

    let caminho =
        registro::iniciar(&raiz.join("logs")).expect("não foi possível iniciar o registro");
    let nome = caminho
        .file_name()
        .expect("arquivo do registro sem nome")
        .to_string_lossy();
    assert!(
        nome.starts_with("metra-") && nome.ends_with(".log"),
        "nome inesperado do registro: {caminho:?}"
    );
    registro::instalar_hook_panic();

    let mut tarefa = Tarefa {
        indice: 0,
        repo: Repo {
            nome: "trab".into(),
            caminho: trab.clone(),
            situacao: SituacaoRepoEnum::Clonado,
        },
        operacao: OperacaoEnum::Pull,
        remote: "upstream".into(),
        sem_rebase: false,
    };
    executor::executar(&tarefa, &Cancelamento::default());
    let log = fs::read_to_string(&caminho).expect("não foi possível ler o registro após o pull");
    assert!(log.contains("executor: início trab ("), "log:\n{log}");
    assert!(
        log.contains(&format!(
            "{} $ git pull upstream main -> código 0 em ",
            trab.display()
        )),
        "log:\n{log}"
    );
    assert!(log.contains("fim trab: SemMudanca"), "log:\n{log}");

    tarefa.remote = "inexistente".into();
    executor::executar(&tarefa, &Cancelamento::default());
    let log = fs::read_to_string(&caminho).expect("não foi possível ler o registro após a falha");
    let comando_falho = format!("{} $ git pull inexistente main -> código ", trab.display());
    assert!(
        log.lines()
            .any(|linha| linha.contains("AVISO") && linha.contains(&comando_falho)),
        "log:\n{log}"
    );
    assert!(
        log.contains("fim trab: Falhou(RemoteInexistente)"),
        "log:\n{log}"
    );

    registro::registrar(
        NivelEnum::Info,
        "teste",
        "https://usuario:segredo@exemplo.invalid/x.git",
    );
    let log = fs::read_to_string(&caminho).expect("não foi possível ler o registro após a máscara");
    assert!(
        log.contains("https://***@exemplo.invalid/x.git"),
        "log:\n{log}"
    );
    assert!(!log.contains("segredo"), "log:\n{log}");

    let thread_panic = thread::Builder::new()
        .name("teste-panic".into())
        .spawn(|| panic!("falha proposital"))
        .expect("não foi possível criar a thread de panic");
    assert!(
        thread_panic.join().is_err(),
        "a thread deveria falhar por panic"
    );
    let log = fs::read_to_string(&caminho).expect("não foi possível ler o registro após o panic");
    assert!(log.contains("ERRO  [teste-panic] panic: "), "log:\n{log}");
    assert!(log.contains("falha proposital"), "log:\n{log}");
}
