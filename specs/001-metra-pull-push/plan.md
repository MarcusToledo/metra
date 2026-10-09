# Implementation Plan: metra pull/push com TUI de progresso e triagem de falhas

**Branch**: `001-metra-pull-push` | **Date**: 2026-10-09 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/001-metra-pull-push/spec.md`

## Summary

Binário Rust `metra` com subcomandos `pull` e `push`. Descobre os repositórios do workspace
(`.meta` ou varredura de subpastas), classifica cada um (Clonado/NaoClonado/SemRepositorio),
executa git só nos clonados num pool fixo de threads e mostra uma TUI ratatui que agrupa falhas
no topo com o motivo classificado por uma função pura. Ações na TUI reenviam tarefas ao pool
(`r`, `n` com `--no-rebase`, `R`) ou abrem um pane do herdr no repositório (`p`). Ao sair, imprime
uma linha-resumo e devolve o código 0/1/2.

## Technical Context

**Language/Version**: Rust edition 2024, rustc/cargo 1.98.1 estável

**Primary Dependencies**: `ratatui` 0.30 (crossterm 0.29 via `ratatui::crossterm`, feature
padrão), `clap` (feature `derive`), `serde` (feature `derive`), `serde_json`, `anyhow`;
dev: `tempfile`. Todas adicionadas com `cargo add`.

**Storage**: N/A (lê `.meta` JSON; nada persistido)

**Testing**: `cargo test` — unitários em `src/classifica.rs`; integração em `tests/executor.rs` e
`tests/fonte.rs` com repositórios git reais em `tempfile::TempDir`

**Target Platform**: Linux (terminal com suporte a tela alternativa); git ≥ 2.34

**Project Type**: CLI/TUI (binário único)

**Performance Goals**: até N (`-j`, padrão 8) git simultâneos; TUI redesenha a cada ≤ 100 ms

**Constraints**: nenhum prompt interativo do git; nunca rodar git fora da raiz do próprio repo;
terminal restaurado em qualquer saída

**Scale/Scope**: workspaces de ~45 repositórios

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Princípio | Como o plano cumpre | Status |
|---|---|---|
| I. pt-BR e convenções Rust | Módulos, tipos e mensagens em português; `SituacaoRepoEnum`, `OperacaoEnum`, `EstadoRepoEnum`, `TipoErroEnum`, `FonteEnum`; sem prefixos húngaros | ✅ |
| II. Separação estrita | `fonte/` (descoberta), `executor.rs` (git), `classifica.rs` (pura), `pool.rs` (threads), `app.rs` (estado sem terminal), `ui.rs` (render), `acoes.rs` (herdr); UI só envia `Tarefa` e lê `Evento` | ✅ |
| III. Testes com git real | `classifica` com saídas reais capturadas do git 2.34.1 (ver research.md); `tests/executor.rs` e `tests/fonte.rs` com repos temporários reais, sem mocks | ✅ |
| IV. Git só na raiz própria | `situacao` calculada por `caminho.join(".git").exists()`; `executar` recusa repo não Clonado; teste prova que repo pai não muda | ✅ |
| V. Não-interativo | helper `git()` com `GIT_TERMINAL_PROMPT=0`, `LC_ALL=C`, `Stdio::null()` | ✅ |
| VI. Dependências mínimas | Exatamente a lista da constituição; threads `std` | ✅ |
| VII. Integrações degradam | `abrir_pane_herdr` devolve `Result<(), String>` exibido na barra de status | ✅ |

Re-check pós-design (Phase 1): sem violações; Complexity Tracking vazio.

## Project Structure

### Documentation (this feature)

```text
specs/001-metra-pull-push/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── cli.md
│   └── tui.md
└── tasks.md
```

### Source Code (repository root)

```text
Cargo.toml
src/
├── main.rs          # clap, resolução da fonte, loop da TUI, linha-resumo, código de saída
├── lib.rs           # declara e reexporta os módulos (permite testes de integração)
├── fonte/
│   ├── mod.rs       # trait FonteRepos, Repo, SituacaoRepoEnum, FonteEnum, resolver_fonte
│   ├── meta.rs      # FonteMeta (.meta)
│   └── varredura.rs # FonteVarredura (subpastas)
├── classifica.rs    # TipoErroEnum + classificar() puro + texto()/dica(); testes unitários
├── executor.rs      # Tarefa, OperacaoEnum, Resultado, EstadoRepoEnum, git(), executar()
├── pool.rs          # Pool de N threads, Evento, cancelamento
├── app.rs           # App: estado da tela, linhas agrupadas, teclas → ações, resumo
├── acoes.rs         # abrir_pane_herdr()
└── ui.rs            # desenhar(frame, &App)
tests/
├── executor.rs      # cenários git reais
└── fonte.rs         # descoberta meta/varredura/auto
```

**Structure Decision**: projeto único; `lib.rs` expõe os módulos para que `tests/` exercite
`executor` e `fonte` como API pública, mantendo `main.rs` fino.

### Design dos módulos

- **`fonte/mod.rs`**: `pub trait FonteRepos { fn listar(&self) -> anyhow::Result<Vec<Repo>>; }`;
  `pub struct Repo { pub nome: String, pub caminho: PathBuf, pub situacao: SituacaoRepoEnum }`;
  `pub enum SituacaoRepoEnum { Clonado, NaoClonado, SemRepositorio }`;
  `pub enum FonteEnum { Auto, Meta, Varredura }` (`clap::ValueEnum`);
  `pub fn resolver_fonte(modo: FonteEnum, cwd: &Path) -> anyhow::Result<(PathBuf, Box<dyn FonteRepos>)>`;
  `pub fn situacao(caminho: &Path) -> SituacaoRepoEnum`. Lista vazia é erro tratado em
  `resolver_fonte`/`main` com "nenhum repositório encontrado em <raiz>".
- **`fonte/meta.rs`**: desserializa `.meta` com `serde` em `struct ArquivoMeta { projects:
  ChavesOrdenadas }`, onde `ChavesOrdenadas(Vec<String>)` tem `Deserialize` manual via
  `visit_map` para preservar a ordem do arquivo (ver research.md R3); erro de parse ou `projects`
  ausente vira `anyhow` com a mensagem do parser.
- **`fonte/varredura.rs`**: `read_dir` da raiz, filtra diretórios não ocultos com `.git`, ordena.
- **`classifica.rs`**: `pub enum TipoErroEnum { MergePendente, Conflito(Vec<String>), Divergente,
  AlteracoesLocais, ArquivosNaoRastreados, BranchRemotaInexistente, RemoteInexistente,
  PushRejeitado, RedeOuAutenticacao, HeadDestacado, Desconhecido(String) }`;
  `pub fn classificar(texto: &str) -> TipoErroEnum`; `texto(&self) -> String`;
  `dica(&self) -> &'static str`.
- **`executor.rs`**: `pub enum OperacaoEnum { Pull, Push }`; `pub struct Tarefa { pub indice: usize,
  pub repo: Repo, pub operacao: OperacaoEnum, pub remote: String, pub sem_rebase: bool }`;
  `pub enum EstadoRepoEnum { Pendente, Executando, SemMudanca, Recebido(u32), Enviado,
  Falhou(TipoErroEnum) }`; `pub struct Resultado { pub estado: EstadoRepoEnum, pub branch:
  Option<String>, pub log: String }`; `pub fn executar(tarefa: &Tarefa) -> Resultado`;
  helper privado `git(caminho, args) -> (bool, String)` com `Command::new("git").arg("-C")`.
  Repo não Clonado → `Falhou(Desconhecido("repositório não clonado"))` sem processo (defesa em
  profundidade; o app nunca envia).
- **`pool.rs`**: `pub enum Evento { Iniciou(usize), Terminou(usize, Resultado), PoolEncerrado }`;
  `pub struct Pool { sender: Option<mpsc::Sender<Tarefa>>, cancelado: Arc<AtomicBool> }` com
  `iniciar(n, eventos: Sender<Evento>) -> Pool`, `enviar(Tarefa)`, `encerrar()` (seta
  `cancelado`, dropa o sender). Workers leem `Arc<Mutex<Receiver<Tarefa>>>`; tarefa recebida com
  `cancelado` verdadeiro é descartada sem evento. Uma thread "vigia" dá join nos workers e emite
  `Evento::PoolEncerrado`.
- **`app.rs`**: `App { operacao, remote, raiz, repos, estados, logs, branches, selecao,
  detalhe_aberto, rolagem, status, saindo, encerrado }`; `linhas() -> Vec<LinhaEnum>` (grupos da
  FR-016); `tratar_tecla(KeyCode) -> Vec<AcaoEnum>` onde `AcaoEnum { Enviar(Tarefa), AbrirPane(PathBuf),
  Sair }`; `aplicar(Evento)`; `resumo() -> String`; `codigo_saida() -> i32`. Sem I/O.
- **`acoes.rs`**: `pub fn abrir_pane_herdr(caminho: &Path) -> Result<(), String>`.
- **`ui.rs`**: `pub fn desenhar(frame: &mut Frame, app: &App)` — cabeçalho, lista (`List` +
  `ListState`), detalhe (`Paragraph::scroll`), barra de status e rodapé.
- **`main.rs`**: clap → `resolver_fonte` (erro → stderr, exit 2) → `ratatui::init()` → loop
  (drena `try_recv`, `event::poll(100ms)`, `desenhar`) → `ratatui::restore()` → imprime
  `app.resumo()` → `std::process::exit(app.codigo_saida())`.

## Complexity Tracking

Sem violações da constituição.
