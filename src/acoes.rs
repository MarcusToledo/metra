//! Integrações externas opcionais (herdr).

use std::path::Path;
use std::process::{Command, Stdio};

/// Abre um pane do herdr no repositório quando a integração está ativa.
///
/// Retorna uma mensagem para a barra de status se o herdr estiver indisponível ou falhar.
pub fn abrir_pane_com(herdr_env: Option<&str>, caminho: &Path) -> Result<(), String> {
    if herdr_env != Some("1") {
        return Err("herdr indisponível".to_string());
    }

    let caminho_absoluto = std::path::absolute(caminho)
        .map_err(|erro| format!("não foi possível obter o caminho absoluto: {erro}"))?;
    let saida = Command::new("herdr")
        .args(["pane", "split", "--current", "--direction", "down", "--cwd"])
        .arg(caminho_absoluto)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .map_err(|_| "herdr indisponível".to_string())?;

    if saida.status.success() {
        return Ok(());
    }

    let erro = String::from_utf8_lossy(&saida.stderr);
    let erro = erro.trim();
    if !erro.is_empty() {
        return Err(erro.to_string());
    }

    Err(match saida.status.code() {
        Some(codigo) => format!("herdr falhou (código {codigo})"),
        None => "herdr falhou (sem código de saída)".to_string(),
    })
}

/// Abre um pane do herdr no repositório se `HERDR_ENV` estiver ativo.
pub fn abrir_pane_herdr(caminho: &Path) -> Result<(), String> {
    let herdr_env = std::env::var("HERDR_ENV").ok();
    abrir_pane_com(herdr_env.as_deref(), caminho)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::abrir_pane_com;

    #[test]
    fn sem_herdr_env_nao_abre_pane() {
        assert_eq!(
            abrir_pane_com(None, Path::new("/tmp")),
            Err("herdr indisponível".to_string())
        );
    }

    #[test]
    fn herdr_env_desativado_nao_abre_pane() {
        assert_eq!(
            abrir_pane_com(Some("0"), Path::new("/tmp")),
            Err("herdr indisponível".to_string())
        );
    }
}
