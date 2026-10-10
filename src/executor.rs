//! Execução síncrona de pull/push num repositório.
use std::path::Path;
use std::process::{Command, Stdio};

use crate::classifica::{self, TipoErroEnum};
use crate::repo::{Repo, SituacaoRepoEnum};

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
}

fn git(caminho: &Path, args: &[&str], ssh_batch: bool, log: &mut String) -> (bool, String) {
    log.push_str("$ git");
    for arg in args {
        log.push(' ');
        log.push_str(arg);
    }
    log.push('\n');

    let mut comando = Command::new("git");
    comando
        .arg("-C")
        .arg(caminho)
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("LC_ALL", "C")
        .stdin(Stdio::null());
    if ssh_batch {
        comando.env("GIT_SSH_COMMAND", "ssh -o BatchMode=yes");
    }

    match comando.output() {
        Ok(saida) => {
            let mut texto = String::from_utf8_lossy(&saida.stdout).into_owned();
            texto.push_str(&String::from_utf8_lossy(&saida.stderr));
            log.push_str(&texto);
            (saida.status.success(), texto)
        }
        Err(erro) => {
            let texto = format!("não foi possível executar git: {erro}\n");
            log.push_str(&texto);
            (false, texto)
        }
    }
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
    }
}

pub fn executar(tarefa: &Tarefa) -> Resultado {
    let caminho = &tarefa.repo.caminho;
    // Não permitir que o git suba até um repositório pai se a pasta mudou após a descoberta.
    if tarefa.repo.situacao != SituacaoRepoEnum::Clonado || !caminho.join(".git").exists() {
        return Resultado {
            estado: EstadoRepoEnum::Falhou(TipoErroEnum::Desconhecido(
                "repositório não clonado".into(),
            )),
            branch: None,
            log: String::new(),
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

    let mut log = String::new();
    let (sucesso, saida) = git(
        caminho,
        &["rev-parse", "--abbrev-ref", "HEAD"],
        ssh_batch,
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
        };
    }

    let estado = match tarefa.operacao {
        OperacaoEnum::Pull => {
            let (sucesso, saida) = git(caminho, &["rev-parse", "HEAD"], ssh_batch, &mut log);
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
            let (sucesso, saida) = git(caminho, args, ssh_batch, &mut log);
            if !sucesso {
                EstadoRepoEnum::Falhou(classifica::classificar(&saida))
            } else {
                let (sucesso, depois) = git(caminho, &["rev-parse", "HEAD"], ssh_batch, &mut log);
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
                &mut log,
            );
            if !sucesso {
                EstadoRepoEnum::Falhou(classifica::classificar(&saida))
            } else if saida.contains("Everything up-to-date") {
                EstadoRepoEnum::SemMudanca
            } else {
                EstadoRepoEnum::Enviado
            }
        }
    };

    Resultado {
        estado,
        branch: Some(branch),
        log,
    }
}
