//! Execução síncrona de pull/push num repositório.
use std::collections::HashSet;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use crate::classifica::{self, TipoErroEnum};
use crate::registro::{NivelEnum, registrar};
use crate::repo::{Repo, SituacaoRepoEnum};

/// SSH não interativo e com limites de rede: sem `ConnectTimeout`/`ServerAlive*`, uma conexão
/// presa (ex.: queda da VPN) bloqueia o trabalhador indefinidamente e a saída nunca conclui.
const SSH_NAO_INTERATIVO: &str =
    "ssh -o BatchMode=yes -o ConnectTimeout=15 -o ServerAliveInterval=15 -o ServerAliveCountMax=3";
const MOTIVO_ABORTADO: &str = "abortado pelo usuário";
const SINAL_KILL: i32 = 9;

unsafe extern "C" {
    /// `kill(2)` da libc, já linkada pelo std; evita dependência nova só para sinalizar grupos.
    fn kill(pid: i32, sinal: i32) -> i32;
}

/// Permite abortar as tarefas em execução matando os grupos de processos dos comandos git.
#[derive(Debug, Default)]
pub struct Cancelamento {
    abortado: AtomicBool,
    /// PIDs dos líderes de grupo (git) em execução; cada git roda em grupo próprio.
    grupos: Mutex<HashSet<i32>>,
}

impl Cancelamento {
    pub fn is_abortado(&self) -> bool {
        self.abortado.load(Ordering::Acquire)
    }

    /// Mata git e filhos (ssh) em execução; comandos futuros não iniciam.
    pub fn abortar(&self) {
        let grupos = self.grupos.lock().expect("grupos de processos íntegros");
        self.abortado.store(true, Ordering::Release);
        registrar(
            NivelEnum::Aviso,
            "executor",
            &format!("aborto: matando {} grupos de processos", grupos.len()),
        );
        for &grupo in grupos.iter() {
            matar_grupo(grupo);
        }
    }

    /// Registra o grupo; se o aborto já ocorreu, mata-o na hora.
    fn registrar(&self, grupo: i32) {
        let mut grupos = self.grupos.lock().expect("grupos de processos íntegros");
        if self.is_abortado() {
            matar_grupo(grupo);
        } else {
            grupos.insert(grupo);
        }
    }

    fn remover(&self, grupo: i32) {
        self.grupos
            .lock()
            .expect("grupos de processos íntegros")
            .remove(&grupo);
    }
}

fn matar_grupo(grupo: i32) {
    // SAFETY: `kill` só lê os inteiros; PID negativo sinaliza o grupo inteiro.
    unsafe {
        kill(-grupo, SINAL_KILL);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperacaoEnum {
    Pull,
    Push,
}

/// Unidade de trabalho enviada ao pool.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tarefa {
    /// Posição do repo no vetor do `App`.
    pub indice: usize,
    pub repo: Repo,
    pub operacao: OperacaoEnum,
    pub remote: String,
    /// `git pull --no-rebase` (só pull, via tecla `n`).
    pub sem_rebase: bool,
}

/// Estado de um repositório na TUI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EstadoRepoEnum {
    Pendente,
    Executando,
    SemMudanca,
    /// Quantidade de commits que entraram na branch.
    Recebido(u32),
    Enviado,
    Falhou(TipoErroEnum),
}

/// Resultado de uma tentativa; `estado` nunca é `Pendente`/`Executando`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resultado {
    pub estado: EstadoRepoEnum,
    /// `None` se a branch não pôde ser lida.
    pub branch: Option<String>,
    /// stdout+stderr de todos os comandos da tentativa.
    pub log: String,
    /// URL de merge/pull request anunciada pelo remote no push (só em `Enviado`).
    pub link_merge: Option<String>,
}

fn git(
    caminho: &Path,
    args: &[&str],
    ssh_batch: bool,
    cancelamento: &Cancelamento,
    log: &mut String,
) -> (bool, String) {
    log.push_str("$ git");
    for arg in args {
        log.push(' ');
        log.push_str(arg);
    }
    log.push('\n');
    let argumentos = args.join(" ");

    if cancelamento.is_abortado() {
        registrar(
            NivelEnum::Aviso,
            "executor",
            &format!(
                "{} $ git {argumentos}: não iniciado, {MOTIVO_ABORTADO}",
                caminho.display()
            ),
        );
        let texto = format!("{MOTIVO_ABORTADO}\n");
        log.push_str(&texto);
        return (false, texto);
    }

    let mut comando = Command::new("git");
    comando
        .arg("-C")
        .arg(caminho)
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("LC_ALL", "C")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        // Grupo próprio: o aborto mata também o ssh filho, que segura os pipes de saída.
        .process_group(0);
    if ssh_batch {
        comando.env("GIT_SSH_COMMAND", SSH_NAO_INTERATIVO);
    }

    let inicio = Instant::now();
    let saida = comando.spawn().and_then(|filho| {
        let grupo = i32::try_from(filho.id()).expect("PID cabe em i32");
        cancelamento.registrar(grupo);
        let saida = filho.wait_with_output();
        cancelamento.remover(grupo);
        saida
    });
    match saida {
        Ok(saida) => {
            let mut texto = String::from_utf8_lossy(&saida.stdout).into_owned();
            texto.push_str(&String::from_utf8_lossy(&saida.stderr));
            log.push_str(&texto);
            let sucesso = saida.status.success();
            let codigo = match saida.status.code() {
                Some(codigo) => format!("código {codigo}"),
                None => "encerrado por sinal".to_string(),
            };
            registrar(
                if sucesso {
                    NivelEnum::Info
                } else {
                    NivelEnum::Aviso
                },
                "executor",
                &format!(
                    "{} $ git {argumentos} -> {codigo} em {} ms\n{texto}",
                    caminho.display(),
                    inicio.elapsed().as_millis()
                ),
            );
            (sucesso, texto)
        }
        Err(erro) => {
            let texto = format!("não foi possível executar git: {erro}\n");
            log.push_str(&texto);
            registrar(
                NivelEnum::Erro,
                "executor",
                &format!(
                    "{} $ git {argumentos}: não foi possível executar git: {erro}",
                    caminho.display()
                ),
            );
            (false, texto)
        }
    }
}

/// Consulta a versão instalada do git sem depender de um repositório.
pub fn versao_git() -> Option<String> {
    let saida = Command::new("git")
        .arg("--version")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("LC_ALL", "C")
        .stdin(Stdio::null())
        .output()
        .ok()?;
    saida
        .status
        .success()
        .then(|| String::from_utf8_lossy(&saida.stdout).trim().to_string())
}

fn ultimo_erro(saida: &str) -> TipoErroEnum {
    TipoErroEnum::Desconhecido(
        saida
            .lines()
            .rev()
            .find(|linha| !linha.trim().is_empty())
            .unwrap_or("")
            .to_owned(),
    )
}

fn falha_leitura(saida: &str, branch: Option<String>, log: String) -> Resultado {
    Resultado {
        estado: EstadoRepoEnum::Falhou(ultimo_erro(saida)),
        branch,
        log,
        link_merge: None,
    }
}

/// Primeira URL http(s) numa linha `remote:` logo após uma linha `remote:` que cita
/// "merge request" ou "pull request" (GitLab, GitHub, Bitbucket).
fn extrair_link_merge(saida: &str) -> Option<String> {
    let mut anuncio = false;
    for linha in saida.lines() {
        let Some(resto) = linha.strip_prefix("remote:") else {
            anuncio = false;
            continue;
        };
        let conteudo = resto.trim();
        if conteudo.is_empty() {
            continue;
        }
        if anuncio
            && let Some(url) = conteudo
                .split_whitespace()
                .find(|token| token.starts_with("https://") || token.starts_with("http://"))
        {
            return Some(url.to_owned());
        }
        let minusculo = conteudo.to_lowercase();
        anuncio = minusculo.contains("merge request") || minusculo.contains("pull request");
    }
    None
}

/// Executa a tarefa; se o usuário abortou, a falha resultante é reportada como aborto.
pub fn executar(tarefa: &Tarefa, cancelamento: &Cancelamento) -> Resultado {
    registrar(
        NivelEnum::Info,
        "executor",
        &format!(
            "início {} ({}) operação={:?} remote={} sem_rebase={}",
            tarefa.repo.nome,
            tarefa.repo.caminho.display(),
            tarefa.operacao,
            tarefa.remote,
            tarefa.sem_rebase
        ),
    );
    let mut resultado = executar_tentativa(tarefa, cancelamento);
    if cancelamento.is_abortado() && matches!(resultado.estado, EstadoRepoEnum::Falhou(_)) {
        resultado.estado =
            EstadoRepoEnum::Falhou(TipoErroEnum::Desconhecido(MOTIVO_ABORTADO.into()));
    }
    registrar(
        if matches!(resultado.estado, EstadoRepoEnum::Falhou(_)) {
            NivelEnum::Aviso
        } else {
            NivelEnum::Info
        },
        "executor",
        &format!(
            "fim {}: {:?} branch={:?} link_merge={:?}",
            tarefa.repo.nome, resultado.estado, resultado.branch, resultado.link_merge
        ),
    );
    resultado
}

fn executar_tentativa(tarefa: &Tarefa, cancelamento: &Cancelamento) -> Resultado {
    let caminho = &tarefa.repo.caminho;
    // Não permitir que o git suba até um repositório pai se a pasta mudou após a descoberta.
    if tarefa.repo.situacao != SituacaoRepoEnum::Clonado || !caminho.join(".git").exists() {
        return Resultado {
            estado: EstadoRepoEnum::Falhou(TipoErroEnum::Desconhecido(
                "repositório não clonado".into(),
            )),
            branch: None,
            log: String::new(),
            link_merge: None,
        };
    }

    let ssh_batch = std::env::var_os("GIT_SSH_COMMAND").is_none()
        && std::env::var_os("GIT_SSH").is_none()
        && !Command::new("git")
            .arg("-C")
            .arg(caminho)
            .args(["config", "--get", "core.sshCommand"])
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("LC_ALL", "C")
            .stdin(Stdio::null())
            .output()
            .is_ok_and(|saida| {
                saida.status.success() && !saida.stdout.iter().all(u8::is_ascii_whitespace)
            });
    registrar(
        NivelEnum::Info,
        "executor",
        &format!("{}: ssh não interativo={ssh_batch}", tarefa.repo.nome),
    );

    let mut log = String::new();
    let (sucesso, saida) = git(
        caminho,
        &["rev-parse", "--abbrev-ref", "HEAD"],
        ssh_batch,
        cancelamento,
        &mut log,
    );
    if !sucesso {
        return falha_leitura(&saida, None, log);
    }
    let branch = saida.trim().to_owned();
    if branch == "HEAD" {
        return Resultado {
            estado: EstadoRepoEnum::Falhou(TipoErroEnum::HeadDestacado),
            branch: Some(branch),
            log,
            link_merge: None,
        };
    }

    let mut link_merge = None;
    let estado = match tarefa.operacao {
        OperacaoEnum::Pull => {
            let (sucesso, saida) = git(
                caminho,
                &["rev-parse", "HEAD"],
                ssh_batch,
                cancelamento,
                &mut log,
            );
            if !sucesso {
                return falha_leitura(&saida, Some(branch), log);
            }
            let antes = saida.trim();
            let args_sem_rebase = [
                "pull",
                "--no-rebase",
                tarefa.remote.as_str(),
                branch.as_str(),
            ];
            let args_padrao = ["pull", tarefa.remote.as_str(), branch.as_str()];
            let args: &[&str] = if tarefa.sem_rebase {
                &args_sem_rebase
            } else {
                &args_padrao
            };
            let (sucesso, saida) = git(caminho, args, ssh_batch, cancelamento, &mut log);
            if !sucesso {
                EstadoRepoEnum::Falhou(classifica::classificar(&saida))
            } else {
                let (sucesso, depois) = git(
                    caminho,
                    &["rev-parse", "HEAD"],
                    ssh_batch,
                    cancelamento,
                    &mut log,
                );
                if !sucesso {
                    return falha_leitura(&depois, Some(branch), log);
                }
                let depois = depois.trim();
                if antes == depois {
                    EstadoRepoEnum::SemMudanca
                } else {
                    let intervalo = format!("{antes}..{depois}");
                    let (sucesso, quantidade) = git(
                        caminho,
                        &["rev-list", "--count", &intervalo],
                        ssh_batch,
                        cancelamento,
                        &mut log,
                    );
                    if !sucesso {
                        return falha_leitura(&quantidade, Some(branch), log);
                    }
                    match quantidade.trim().parse::<u32>() {
                        Ok(quantidade) => EstadoRepoEnum::Recebido(quantidade),
                        Err(_) => EstadoRepoEnum::Falhou(ultimo_erro(&quantidade)),
                    }
                }
            }
        }
        OperacaoEnum::Push => {
            let (sucesso, saida) = git(
                caminho,
                &["push", tarefa.remote.as_str(), &branch],
                ssh_batch,
                cancelamento,
                &mut log,
            );
            if !sucesso {
                EstadoRepoEnum::Falhou(classifica::classificar(&saida))
            } else if saida.contains("Everything up-to-date") {
                EstadoRepoEnum::SemMudanca
            } else {
                link_merge = extrair_link_merge(&saida);
                EstadoRepoEnum::Enviado
            }
        }
    };

    Resultado {
        estado,
        branch: Some(branch),
        log,
        link_merge,
    }
}

#[cfg(test)]
mod tests {
    use super::extrair_link_merge;

    #[test]
    fn test_extrai_link_merge() {
        let casos = [
            (
                "remote: \nremote: To create a merge request for feat, visit:\nremote:   https://gitlab.exemplo/g/p/-/merge_requests/new?merge_request%5Bsource_branch%5D=feat\nremote: \nTo gitlab.exemplo:g/p.git\n * [new branch]      feat -> feat",
                Some(
                    "https://gitlab.exemplo/g/p/-/merge_requests/new?merge_request%5Bsource_branch%5D=feat",
                ),
            ),
            (
                "remote: View merge request for feat:\nremote:   https://gitlab.exemplo/g/p/-/merge_requests/12\n",
                Some("https://gitlab.exemplo/g/p/-/merge_requests/12"),
            ),
            (
                "remote: Create a pull request for 'feat' on GitHub by visiting:\nremote:      https://github.com/o/r/pull/new/feat\n",
                Some("https://github.com/o/r/pull/new/feat"),
            ),
            ("remote: https://exemplo/qualquer\n", None),
            ("To /tmp/up.git\n   abc..def  main -> main\n", None),
        ];
        for (saida, esperado) in casos {
            assert_eq!(extrair_link_merge(saida).as_deref(), esperado, "{saida}");
        }
    }
}
