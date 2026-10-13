# Implementation Plan: Registro de logs de execução do metra

**Branch**: `002-registro-logs` | **Date**: 2026-10-13 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/002-registro-logs/spec.md`

## Summary

Novo módulo folha `src/registro.rs`, só com a biblioteca padrão, que grava um arquivo por execução
em `$XDG_STATE_HOME/metra/` (ou `~/.local/state/metra/`), mantém os 20 mais recentes, formata cada
entrada com data/hora UTC, nível, thread e origem, mascara credenciais em URLs e grava sem buffer.
Um registro global (`OnceLock`) é iniciado no `main`; `executor`, `pool` e `main` chamam
`registrar(...)`, que não faz nada se o registro não foi iniciado (testes e uso como biblioteca).
Um panic hook instalado antes de `ratatui::init()` grava panics com backtrace. Em código de saída 1
ou 2, o stderr recebe `metra: log em <caminho>`; se o log não puder ser criado, a barra de status
mostra `log indisponível: <motivo>`.

## Technical Context

**Language/Version**: Rust edition 2024, rustc/cargo 1.98.1 estável

**Primary Dependencies**: as já existentes (`ratatui` 0.30, `clap`, `serde`/`serde_json`, `anyhow`;
dev: `tempfile`). Nenhuma nova: data UTC calculada à mão, `std::backtrace::Backtrace` para o panic.

**Storage**: arquivos texto em `$XDG_STATE_HOME/metra/metra-AAAAMMDD-HHMMSS-<pid>.log`, retenção de 20

**Testing**: `cargo test` — unitários em `src/registro.rs` (formato, data UTC, nome, mascaramento,
diretório, retenção); integração em `tests/registro.rs` com repositórios git reais

**Target Platform**: Linux; git ≥ 2.34

**Project Type**: CLI/TUI (binário único)

**Performance Goals**: uma escrita por entrada, sem buffer; custo desprezível frente a um processo git

**Constraints**: o log nunca derruba nem trava o app; stdout inalterado; hook de panic antes do
hook do ratatui; nenhuma dependência nova

**Scale/Scope**: ~45 repositórios por execução; 20 arquivos retidos

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Princípio | Como o plano cumpre | Status |
|---|---|---|
| I. pt-BR e convenções Rust | Módulo `registro`, funções e mensagens em português; enum `NivelEnum` | ✅ |
| II. Separação estrita | `registro` é módulo folha: não importa nenhum outro módulo do crate; é usado por `executor`, `pool` e `main`; não cria ciclo. `App` continua sem I/O (a mensagem de falha do log entra por `app.status` no `main`) | ✅ |
| III. Testes com git real | `tests/registro.rs` roda `executor::executar` contra repositórios temporários reais e confere o arquivo; unitários com `tempfile` | ✅ |
| IV. Git só na raiz própria | Nenhuma execução git nova em repositório; `git --version` não usa `-C` nem depende de diretório | ✅ |
| V. Não-interativo | `versao_git()` roda `git --version` com `GIT_TERMINAL_PROMPT=0`, `LC_ALL=C` e stdin nulo, fora de qualquer repositório | ✅ |
| VI. Dependências mínimas | Nenhuma crate nova; só `std` (`OnceLock`, `Mutex`, `File`, `Backtrace`, `SystemTime`) | ✅ |
| VII. Integrações degradam | Falha ao criar o log → `log indisponível: <motivo>` na barra de status; falha ao gravar é ignorada | ✅ |

Re-check pós-design (Phase 1): sem violações; Complexity Tracking vazio.

## Project Structure

### Documentation (this feature)

```text
specs/002-registro-logs/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   └── registro.md
├── checklists/
│   └── requirements.md
└── tasks.md
```

### Source Code (repository root)

```text
src/
├── lib.rs           # + pub mod registro (entre pool e repo)
├── registro.rs      # NOVO: Registro, NivelEnum, global, hook de panic, formato, retenção; testes unitários
├── executor.rs      # registra comandos git, início/fim de tarefa, ssh_batch, aborto; + versao_git()
├── pool.rs          # threads nomeadas; registra descarte, panic de trabalhador, término, saída e aborto
└── main.rs          # inicia registro e hook antes do ratatui; contexto, descoberta, ações, fim; stderr
tests/
└── registro.rs      # NOVO: integração com git real (comandos, fim de tarefa, máscara, panic)
README.md            # + seção "Logs de execução"
```

**Structure Decision**: projeto único existente; o registro é um módulo novo da biblioteca para
ser exercitado por `tests/registro.rs` e usado pelo binário.

### Design dos módulos

- **`registro.rs`** (detalhe em [contracts/registro.md](contracts/registro.md) e
  [data-model.md](data-model.md)): constantes `MAXIMO_ARQUIVOS = 20`, `PREFIXO_ARQUIVO = "metra-"`,
  `SUFIXO_ARQUIVO = ".log"`, `RECUO = "    "`.
  - `pub enum NivelEnum { Info, Aviso, Erro }` com `Display` (`INFO`, `AVISO`, `ERRO`, via
    `f.pad` para aceitar `:<5`).
  - `pub struct Registro { arquivo: Mutex<File>, caminho: PathBuf }` com
    `criar(diretorio, instante) -> io::Result<Registro>` (`create_dir_all`, `podar(dir, 19)`
    ignorando erro, `OpenOptions::write+create_new`, sem buffer), `caminho()` e
    `escrever(nivel, origem, mensagem)` (um `write_all` sob o lock; erros e mutex envenenado
    ignorados; thread sem nome → `sem-nome`).
  - Global `static GLOBAL: OnceLock<Registro>`; `iniciar(diretorio) -> io::Result<PathBuf>` (erro
    `"registro já iniciado"` se repetido); `caminho_atual() -> Option<&'static Path>`;
    `registrar(nivel, origem, mensagem)` (no-op sem global); `instalar_hook_panic()` (encadeia o
    hook anterior, registra `"{info}\n{Backtrace::force_capture()}"` em `Erro`, origem `panic`).
  - Puras: `diretorio_padrao_com(xdg, home)`, `formatar_entrada(...)`,
    `mascarar_credenciais(texto)`, `podar(diretorio, manter)`; privadas `formatar_utc`,
    `nome_arquivo`, `data_civil` (civil_from_days, Howard Hinnant).
  - Injeção de ambiente para teste no mesmo padrão de `acoes::abrir_pane_com`.
- **`executor.rs`** (origem `executor`): no `git()` privado, entrada por comando
  (`"{caminho} $ git {args} -> código N em X ms\n{saída}"`, Info/Aviso), erro de execução (Erro) e
  "não iniciado" por aborto (Aviso); em `executar()`, início e fim da tarefa; em
  `executar_tentativa`, `ssh não interativo=`; em `Cancelamento::abortar`, número de grupos
  mortos; `pub fn versao_git() -> Option<String>`.
- **`pool.rs`** (origem `pool`): `thread::Builder` com nomes `trabalhador-{i}` e `pool`; descarte
  de tarefa na saída, panic de trabalhador, término de todos, `encerrar()` e `abortar()`.
- **`main.rs`** (origens `main`, `fonte`, `app`, `acoes`): `Instant` no início; registro iniciado
  após o parse; hook instalado antes de `ratatui::init()`; `remote` calculado antes da descoberta;
  contexto, versão do git, descoberta, ações e falhas de integração; fim com código, duração,
  resumo e links; `metra: log em <caminho>` no stderr em código 1/2; falha do log em `app.status`.

## Complexity Tracking

Sem violações da constituição.
