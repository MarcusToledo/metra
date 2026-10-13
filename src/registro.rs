//! Registro persistente de cada execução, independente das demais camadas do metra.

use std::ffi::OsStr;
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

const MAXIMO_ARQUIVOS: usize = 20;
const PREFIXO_ARQUIVO: &str = "metra-";
const SUFIXO_ARQUIVO: &str = ".log";
const RECUO: &str = "    ";

/// Nível de gravidade de uma entrada do registro.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NivelEnum {
    Info,
    Aviso,
    Erro,
}

impl fmt::Display for NivelEnum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad(match self {
            Self::Info => "INFO",
            Self::Aviso => "AVISO",
            Self::Erro => "ERRO",
        })
    }
}

/// Arquivo de log de uma única execução, compartilhável entre threads.
pub struct Registro {
    arquivo: Mutex<File>,
    caminho: PathBuf,
}

impl Registro {
    /// Cria o diretório, limita os arquivos antigos e abre um log novo sem buffer.
    pub fn criar(diretorio: &Path, instante: SystemTime) -> io::Result<Registro> {
        fs::create_dir_all(diretorio)?;
        let _ = podar(diretorio, MAXIMO_ARQUIVOS - 1);
        let caminho = diretorio.join(nome_arquivo(instante, std::process::id()));
        let arquivo = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&caminho)?;
        Ok(Registro {
            arquivo: Mutex::new(arquivo),
            caminho,
        })
    }

    /// Retorna o caminho do arquivo criado para esta execução.
    pub fn caminho(&self) -> &Path {
        &self.caminho
    }

    /// Grava uma entrada imediatamente, sem impedir a execução se houver falha no log.
    pub fn escrever(&self, nivel: NivelEnum, origem: &str, mensagem: &str) {
        let thread = std::thread::current();
        let nome = thread.name().unwrap_or("sem-nome");
        let entrada = formatar_entrada(SystemTime::now(), nivel, nome, origem, mensagem);
        if let Ok(mut arquivo) = self.arquivo.lock() {
            let _ = arquivo.write_all(entrada.as_bytes());
        }
    }
}

static GLOBAL: OnceLock<Registro> = OnceLock::new();

/// Inicia o registro global uma única vez e retorna o caminho do log.
pub fn iniciar(diretorio: &Path) -> io::Result<PathBuf> {
    let registro = Registro::criar(diretorio, SystemTime::now())?;
    let caminho = registro.caminho.clone();
    GLOBAL
        .set(registro)
        .map_err(|_| io::Error::other("registro já iniciado"))?;
    Ok(caminho)
}

/// Retorna o caminho do log global, caso tenha sido iniciado.
pub fn caminho_atual() -> Option<&'static Path> {
    GLOBAL.get().map(Registro::caminho)
}

/// Registra uma mensagem se o log global estiver disponível.
pub fn registrar(nivel: NivelEnum, origem: &str, mensagem: &str) {
    if let Some(registro) = GLOBAL.get() {
        registro.escrever(nivel, origem, mensagem);
    }
}

/// Registra panics com backtrace e preserva o hook anterior.
pub fn instalar_hook_panic() {
    let anterior = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        registrar(
            NivelEnum::Erro,
            "panic",
            &format!("{info}\n{}", std::backtrace::Backtrace::force_capture()),
        );
        anterior(info);
    }));
}

/// Obtém o diretório de estado a partir das variáveis de ambiente do processo.
pub fn diretorio_padrao() -> Option<PathBuf> {
    let xdg_state_home = std::env::var_os("XDG_STATE_HOME");
    let home = std::env::var_os("HOME");
    diretorio_padrao_com(xdg_state_home.as_deref(), home.as_deref())
}

/// Resolve o diretório de logs a partir de valores de ambiente injetados.
pub fn diretorio_padrao_com(
    xdg_state_home: Option<&OsStr>,
    home: Option<&OsStr>,
) -> Option<PathBuf> {
    if let Some(xdg) = xdg_state_home.filter(|valor| !valor.is_empty()) {
        let caminho = Path::new(xdg);
        if caminho.is_absolute() {
            return Some(caminho.join("metra"));
        }
    }
    home.filter(|valor| !valor.is_empty())
        .map(|valor| Path::new(valor).join(".local/state/metra"))
}

/// Formata a entrada em UTC, mascarando credenciais e recuando linhas adicionais.
pub fn formatar_entrada(
    instante: SystemTime,
    nivel: NivelEnum,
    thread: &str,
    origem: &str,
    mensagem: &str,
) -> String {
    let mensagem = mascarar_credenciais(mensagem);
    let mut linhas = mensagem.trim_end_matches('\n').split('\n');
    let primeira = linhas.next().unwrap_or("");
    let mut entrada = format!(
        "{} {nivel:<5} [{thread}] {origem}: {primeira}\n",
        formatar_utc(instante)
    );
    for linha in linhas {
        entrada.push_str(RECUO);
        entrada.push_str(linha);
        entrada.push('\n');
    }
    entrada
}

/// Substitui por `***` o trecho de credenciais antes de `@` em URLs.
pub fn mascarar_credenciais(texto: &str) -> String {
    let mut saida = String::with_capacity(texto.len());
    let mut restante = texto;
    while let Some(posicao) = restante.find("://") {
        let apos_esquema = posicao + 3;
        let autoridade = &restante[apos_esquema..];
        let delimitador = autoridade.char_indices().find(|(_, caractere)| {
            matches!(caractere, '/' | '@' | '\'' | '"') || caractere.is_whitespace()
        });
        saida.push_str(&restante[..apos_esquema]);
        if let Some((posicao_arroba, '@')) = delimitador
            && posicao_arroba > 0
        {
            saida.push_str("***");
            restante = &autoridade[posicao_arroba..];
        } else {
            restante = autoridade;
        }
    }
    saida.push_str(restante);
    saida
}

/// Remove os arquivos de log mais antigos por nome até restarem `manter`.
pub fn podar(diretorio: &Path, manter: usize) -> io::Result<()> {
    let mut arquivos = fs::read_dir(diretorio)?
        .filter_map(Result::ok)
        .filter(|entrada| entrada.file_type().is_ok_and(|tipo| tipo.is_file()))
        .filter_map(|entrada| {
            let nome = entrada.file_name();
            let texto = nome.to_string_lossy();
            (texto.starts_with(PREFIXO_ARQUIVO) && texto.ends_with(SUFIXO_ARQUIVO))
                .then(|| (nome, entrada.path()))
        })
        .collect::<Vec<_>>();
    arquivos.sort_unstable_by(|(primeiro, _), (segundo, _)| primeiro.cmp(segundo));
    let excedentes = arquivos.len().saturating_sub(manter);
    for (_, caminho) in arquivos.into_iter().take(excedentes) {
        let _ = fs::remove_file(caminho);
    }
    Ok(())
}

fn formatar_utc(instante: SystemTime) -> String {
    let duracao = instante.duration_since(UNIX_EPOCH).unwrap_or_default();
    let segundos = i64::try_from(duracao.as_secs()).unwrap_or(i64::MAX);
    let (ano, mes, dia) = data_civil(segundos / 86400);
    let hora = (segundos % 86400) / 3600;
    let minuto = (segundos % 3600) / 60;
    let segundo = segundos % 60;
    format!(
        "{ano:04}-{mes:02}-{dia:02}T{hora:02}:{minuto:02}:{segundo:02}.{:03}Z",
        duracao.subsec_millis()
    )
}

fn nome_arquivo(instante: SystemTime, pid: u32) -> String {
    let duracao = instante.duration_since(UNIX_EPOCH).unwrap_or_default();
    let segundos = i64::try_from(duracao.as_secs()).unwrap_or(i64::MAX);
    let (ano, mes, dia) = data_civil(segundos / 86400);
    let hora = (segundos % 86400) / 3600;
    let minuto = (segundos % 3600) / 60;
    let segundo = segundos % 60;
    format!(
        "{PREFIXO_ARQUIVO}{ano:04}{mes:02}{dia:02}-{hora:02}{minuto:02}{segundo:02}-{pid}{SUFIXO_ARQUIVO}"
    )
}

// Conversão civil_from_days de Howard Hinnant: dias desde 1970-01-01 para data gregoriana.
fn data_civil(dias: i64) -> (i64, u32, u32) {
    let dias = dias + 719468;
    let era = if dias >= 0 { dias } else { dias - 146096 } / 146097;
    let dia_era = dias - era * 146097;
    let ano_era = (dia_era - dia_era / 1460 + dia_era / 36524 - dia_era / 146096) / 365;
    let mut ano = ano_era + era * 400;
    let dia_ano = dia_era - (365 * ano_era + ano_era / 4 - ano_era / 100);
    let mes_ajustado = (5 * dia_ano + 2) / 153;
    let dia = dia_ano - (153 * mes_ajustado + 2) / 5 + 1;
    let mes = mes_ajustado + if mes_ajustado < 10 { 3 } else { -9 };
    ano += if mes <= 2 { 1 } else { 0 };
    (ano, mes as u32, dia as u32)
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;
    use std::fs;
    use std::path::Path;
    use std::time::{Duration, UNIX_EPOCH};

    use tempfile::TempDir;

    use super::{
        NivelEnum, Registro, diretorio_padrao_com, formatar_entrada, formatar_utc,
        mascarar_credenciais, nome_arquivo, podar,
    };

    #[test]
    fn test_formatar_entrada_multilinha_e_vazia() {
        let instante = UNIX_EPOCH + Duration::from_secs(951782400) + Duration::from_millis(123);
        assert_eq!(
            formatar_entrada(
                instante,
                NivelEnum::Aviso,
                "t1",
                "executor",
                "linha 1\nlinha 2\n"
            ),
            "2000-02-29T00:00:00.123Z AVISO [t1] executor: linha 1\n    linha 2\n"
        );
        assert_eq!(
            formatar_entrada(instante, NivelEnum::Info, "t1", "executor", ""),
            "2000-02-29T00:00:00.123Z INFO  [t1] executor: \n"
        );
    }

    #[test]
    fn test_formatar_utc() {
        assert_eq!(formatar_utc(UNIX_EPOCH), "1970-01-01T00:00:00.000Z");
        assert_eq!(
            formatar_utc(UNIX_EPOCH + Duration::from_secs(1798761599)),
            "2026-12-31T23:59:59.000Z"
        );
    }

    #[test]
    fn test_nome_arquivo() {
        assert_eq!(
            nome_arquivo(UNIX_EPOCH + Duration::from_secs(1791897845), 4242),
            "metra-20261013-132405-4242.log"
        );
    }

    #[test]
    fn test_mascarar_credenciais() {
        for (entrada, esperado) in [
            (
                "https://user:tok@gitlab.com/g/r.git",
                "https://***@gitlab.com/g/r.git",
            ),
            (
                "fatal: unable to access 'https://tok@host/x/': erro",
                "fatal: unable to access 'https://***@host/x/': erro",
            ),
            (
                "ssh://git@127.0.0.1:2222/x.git",
                "ssh://***@127.0.0.1:2222/x.git",
            ),
            ("git@github.com:org/r.git", "git@github.com:org/r.git"),
            ("https://gitlab.com/a@b", "https://gitlab.com/a@b"),
            ("sem url", "sem url"),
        ] {
            assert_eq!(mascarar_credenciais(entrada), esperado, "{entrada}");
        }
    }

    #[test]
    fn test_diretorio_padrao_com() {
        assert_eq!(
            diretorio_padrao_com(Some(OsStr::new("/x/state")), Some(OsStr::new("/home/u"))),
            Some(Path::new("/x/state/metra").to_path_buf())
        );
        assert_eq!(
            diretorio_padrao_com(Some(OsStr::new("relativo")), Some(OsStr::new("/home/u"))),
            Some(Path::new("/home/u/.local/state/metra").to_path_buf())
        );
        assert_eq!(
            diretorio_padrao_com(Some(OsStr::new("")), Some(OsStr::new("/home/u"))),
            Some(Path::new("/home/u/.local/state/metra").to_path_buf())
        );
        assert_eq!(diretorio_padrao_com(None, None), None);
        assert_eq!(diretorio_padrao_com(None, Some(OsStr::new(""))), None);
    }

    #[test]
    fn test_podar_mantem_os_mais_recentes_e_outros_arquivos() {
        let temporario = TempDir::new().unwrap();
        let diretorio = temporario.path();
        for indice in 0..25 {
            fs::write(
                diretorio.join(format!("metra-20260101-0000{indice:02}-1.log")),
                "",
            )
            .unwrap();
        }
        fs::write(diretorio.join("outro.txt"), "").unwrap();

        podar(diretorio, 19).unwrap();

        for indice in 0..25 {
            assert_eq!(
                diretorio
                    .join(format!("metra-20260101-0000{indice:02}-1.log"))
                    .exists(),
                indice >= 6
            );
        }
        assert!(diretorio.join("outro.txt").exists());
        assert_eq!(fs::read_dir(diretorio).unwrap().count(), 20);
    }

    #[test]
    fn test_criar_poda_antes_de_adicionar_novo_arquivo() {
        let temporario = TempDir::new().unwrap();
        let diretorio = temporario.path();
        for indice in 0..20 {
            fs::write(
                diretorio.join(format!("metra-20200101-0000{indice:02}-1.log")),
                "",
            )
            .unwrap();
        }

        let registro =
            Registro::criar(diretorio, UNIX_EPOCH + Duration::from_secs(1791897845)).unwrap();
        assert!(registro.caminho().exists());
        assert_eq!(
            fs::read_dir(diretorio)
                .unwrap()
                .filter(|entrada| {
                    entrada
                        .as_ref()
                        .unwrap()
                        .file_name()
                        .to_string_lossy()
                        .starts_with("metra-")
                })
                .count(),
            20
        );
        for indice in 0..20 {
            assert_eq!(
                diretorio
                    .join(format!("metra-20200101-0000{indice:02}-1.log"))
                    .exists(),
                indice >= 1
            );
        }
    }
}
