---

description: "Task list for registro de logs de execução"
---

# Tasks: Registro de logs de execução do metra

**Input**: Design documents from `specs/002-registro-logs/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/registro.md, quickstart.md

**Tests**: obrigatórios pela constituição (princípio III e "Fluxo de Desenvolvimento"). Em cada
bloco, a tarefa de teste vem antes da implementação correspondente.

**Gate por tarefa**: a árvore compila e `cargo test` passa ao fim de cada tarefa de implementação
(as tarefas de teste podem deixar testes novos falhando até a implementação seguinte).

**Organization**: a fundação é o módulo `registro` completo (passo C1 do design). Cada história
percorre, na ordem, teste de integração (C2) → executor (C3) → pool (C4) → main (C5). O teste de
integração é uma única função (`test_registro_de_execucao`), porque o registro global só pode ser
iniciado uma vez por processo; US2 e US3 acrescentam passos a ela.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: pode rodar em paralelo (arquivo diferente de qualquer outra tarefa pendente da fase)
- **[Story]**: US1–US3 conforme spec.md

## Path Conventions

Projeto único: `src/`, `tests/` na raiz do repositório.

---

## Phase 1: Setup

- [X] T001 Criar src/registro.rs com doc de módulo, as constantes `MAXIMO_ARQUIVOS: usize = 20`, `PREFIXO_ARQUIVO = "metra-"`, `SUFIXO_ARQUIVO = ".log"`, `RECUO = "    "`, `#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum NivelEnum { Info, Aviso, Erro }` e as assinaturas exatas de plan.md/contracts/registro.md com corpos `todo!()`; declarar `pub mod registro;` em src/lib.rs entre `pool` e `repo`; usar só `std`

**Checkpoint**: `cargo build` compila.

---

## Phase 2: Foundational (Blocking Prerequisites) — passo C1

**⚠️ CRITICAL**: nenhuma história começa antes desta fase.

- [X] T002 Testes unitários em `#[cfg(test)] mod tests` de src/registro.rs (prefixo `test_`, `tempfile::TempDir`): `formatar_entrada(UNIX_EPOCH + 951782400s + 123ms, Aviso, "t1", "executor", "linha 1\nlinha 2\n")` == `"2000-02-29T00:00:00.123Z AVISO [t1] executor: linha 1\n    linha 2\n"`; mensagem vazia → cabeçalho terminando em `": \n"`; `formatar_utc(UNIX_EPOCH)` == `"1970-01-01T00:00:00.000Z"`; `formatar_utc(UNIX_EPOCH + 1798761599s)` == `"2026-12-31T23:59:59.000Z"`; `nome_arquivo(UNIX_EPOCH + 1791897845s, 4242)` == `"metra-20261013-132405-4242.log"`; tabela de `mascarar_credenciais` em laço (`https://user:tok@gitlab.com/g/r.git` → `https://***@gitlab.com/g/r.git`; `fatal: unable to access 'https://tok@host/x/': erro` → `fatal: unable to access 'https://***@host/x/': erro`; `ssh://git@127.0.0.1:2222/x.git` → `ssh://***@127.0.0.1:2222/x.git`; `git@github.com:org/r.git`, `https://gitlab.com/a@b` e `sem url` inalterados); `diretorio_padrao_com` (`/x/state`,`/home/u` → `/x/state/metra`; `relativo`,`/home/u` → `/home/u/.local/state/metra`; `""`,`/home/u` → `/home/u/.local/state/metra`; `None`,`None` → `None`; `None`,`""` → `None`); `podar(dir, 19)` com `metra-20260101-0000NN-1.log` (NN 00–24) e `outro.txt` deixa NN 06–24 e `outro.txt`; `Registro::criar` com 20 `metra-*.log` antigos deixa 20 `metra-*.log`, o novo entre eles e os 19 antigos mais recentes
- [X] T003 Implementar as funções puras em src/registro.rs: `data_civil` (civil_from_days, research.md R3), `formatar_utc` (instante antes da época → 0), `nome_arquivo`, `impl Display for NivelEnum` com `f.pad("INFO"|"AVISO"|"ERRO")`, `formatar_entrada` (mascara, `trim_end_matches('\n')`, `"{utc} {nivel:<5} [{thread}] {origem}: {linha1}\n"` + `"{RECUO}{linha}\n"`), `mascarar_credenciais` (para cada `://`, primeiro de `/`, `@`, `'`, `"` ou espaço; se `@` com texto antes → `***`), `diretorio_padrao_com` + `diretorio_padrao` (lê `XDG_STATE_HOME` e `HOME`), `podar` (arquivos com prefixo/sufixo, ordenados pelo nome; erro de remoção ignorado; só `read_dir` propaga)
- [X] T004 Implementar em src/registro.rs `Registro { arquivo: Mutex<File>, caminho: PathBuf }`: `criar` (`create_dir_all` → `let _ = podar(dir, MAXIMO_ARQUIVOS - 1)` → `OpenOptions::new().write(true).create_new(true)` em `nome_arquivo(instante, std::process::id())`, sem buffer), `caminho`, `escrever` (`SystemTime::now()`, nome da thread ou `sem-nome`, um `write_all` sob o lock, erros e mutex envenenado ignorados); `static GLOBAL: OnceLock<Registro>`, `iniciar` (`Err(io::Error::other("registro já iniciado"))` se repetido; devolve o caminho), `caminho_atual`, `registrar` (no-op sem global), `instalar_hook_panic` (`take_hook`; registra `Erro`, origem `panic`, `"{info}\n{Backtrace::force_capture()}"`; chama o anterior)

**Checkpoint**: `cargo test` passa com os testes de T002; `registro` não importa nenhum módulo do crate.

---

## Phase 3: User Story 1 - Diagnosticar uma execução com falha depois que a TUI fechou (Priority: P1) 🎯 MVP

**Goal**: cada execução deixa um arquivo com contexto, descoberta, comandos git com saída, fim de
cada tarefa e resumo; em código 1/2 o stderr aponta o arquivo.

**Independent Test**: quickstart.md, item 2 (stdout, stderr e conteúdo do arquivo).

- [X] T005 [US1] Criar tests/registro.rs (passo C2) com `fn test_registro_de_execucao` e a fixture copiada de tests/executor.rs (`git()`, bare `up.git` com `-b main`, clone `aux` com usuário e commit enviado em `main`, clone `trab` com `origin` renomeado para `upstream`): `let caminho = metra::registro::iniciar(&temp.path().join("logs"))` com nome iniciando em `metra-` e terminando em `.log`; `executor::executar` de pull (`upstream`, `sem_rebase: false`, nome `trab`, `Cancelamento::default()`) → log contém `executor: início trab (`, `{trab} $ git pull upstream main -> código 0 em ` e `fim trab: SemMudanca`; repetir com remote `inexistente` → log contém `{trab} $ git pull inexistente main -> código ` numa linha com `AVISO` e `fim trab: Falhou(RemoteInexistente)` (`{trab}` = `trab.display()`)
- [X] T006 [US1] Registro no executor em src/executor.rs (passo C3, origem `executor`, `use crate::registro::{NivelEnum, registrar}`, `std::time::Instant`): no `git()` privado, `Aviso` `"{caminho} $ git {args}: não iniciado, {MOTIVO_ABORTADO}"` quando já abortado; `Instant::now()` antes do `spawn`; `Ok` → `Info` se `success()` senão `Aviso`, `"{caminho} $ git {args} -> {codigo} em {ms} ms\n{texto}"` (`codigo` = `código N` ou `encerrado por sinal`); `Err` → `Erro` `"{caminho} $ git {args}: não foi possível executar git: {erro}"`; em `executar()`, `Info` `"início {nome} ({caminho}) operação={operacao:?} remote={remote} sem_rebase={sem_rebase}"` e, após o ajuste de aborto, `"fim {nome}: {estado:?} branch={branch:?} link_merge={link_merge:?}"` (`Aviso` se `Falhou(_)`); em `executar_tentativa`, após `ssh_batch`, `Info` `"{nome}: ssh não interativo={ssh_batch}"`; `pub fn versao_git() -> Option<String>` (`git --version`, `GIT_TERMINAL_PROMPT=0`, `LC_ALL=C`, `stdin(Stdio::null())`, `Some(stdout.trim())` só em sucesso)
- [X] T007 [P] [US1] Nomear as threads em src/pool.rs (passo C4): `thread::Builder::new().name(format!("trabalhador-{i}"))` (`.map(|i|`) e `.name("pool".into())` para a thread que dá join, com `.spawn(...).expect("não foi possível criar thread do pool")`
- [X] T008 [US1] Ligação em src/main.rs (passo C5, itens 1–4 e 6): `let inicio = Instant::now()` antes de `Cli::parse()`; após o parse, `falha_registro` (`diretorio_padrao()` `None` → `"log indisponível: HOME não definido"`; `iniciar` com erro → `"log indisponível: {erro}"`); `let remote` antes da descoberta; origem `main`: `Info` `"metra {CARGO_PKG_VERSION} {operacao:?} remote={remote} jobs={jobs} fonte={fonte:?}"` e `"git: {versao_git() ou "indisponível"}"`; `let cwd` antes de `descobrir`; origem `fonte`: erro → `Erro` `"{erro:#}"`, `eprintln!("metra: {erro:#}")`, `eprintln!("metra: log em {}")` se `caminho_atual()` for `Some`, `exit(2)`; sucesso → `Info` `"cwd={cwd} raiz={raiz} repos={n}"` e um `Info` `"{nome} {caminho} {situacao:?}"` por repo; depois de `App::new`, `if let Some(mensagem) = falha_registro { app.status = Some(mensagem); }` (FR-019); no fim, `let codigo` antes dos `println!`, `Info` `"fim código={codigo} duração={:.1}s: {resumo}"` e um `Info` por linha de `linhas_merge()`; após os `println!`, se `codigo != 0` e `caminho_atual()` for `Some`, `eprintln!("metra: log em {}", caminho.display())`; stdout inalterado

**Checkpoint**: `cargo test` passa; quickstart.md item 2 (sem as linhas de ação e de pool, que vêm na US2).

---

## Phase 4: User Story 2 - Investigar aborto, travamento e panic (Priority: P2)

**Goal**: o log registra ações, saída, aborto, descarte de tarefas e panics com backtrace.

**Independent Test**: passo de panic de `test_registro_de_execucao`; numa execução real, `q` duas
vezes durante um pull lento e conferir as entradas de ação, saída e aborto.

- [X] T009 [US2] Em tests/registro.rs, chamar `metra::registro::instalar_hook_panic()` logo após `iniciar` e acrescentar ao fim: thread `thread::Builder::new().name("teste-panic".into())` com `panic!("falha proposital")`; `join()` é `Err`; log contém `ERRO  [teste-panic] panic: ` e `falha proposital`
- [X] T010 [US2] Em src/executor.rs, `Cancelamento::abortar`, com o lock obtido: `Aviso` origem `executor` `"aborto: matando {n} grupos de processos"`
- [X] T011 [P] [US2] Em src/pool.rs (origem `pool`): tarefa descartada por `cancelado` → `Info` `"tarefa {indice} ({nome}) descartada: saída em andamento"`; `join()` com `Err` → `Erro` `"trabalhador terminou com panic"`; antes de `PoolEncerrado` → `Info` `"todos os trabalhadores terminaram"`; `encerrar()` → `Info` `"saída iniciada: tarefas pendentes descartadas"`; `abortar()` → `Aviso` `"aborto solicitado"` antes de `encerrar()`
- [X] T012 [US2] Em src/main.rs: `registro::instalar_hook_panic()` logo após iniciar o registro e antes de `ratatui::init()` (research.md R6); em `rodar`, para cada `acao` de `tratar_tecla`, `Info` origem `app` `"ação {acao:?}"` antes do `match`; `AbrirPane`/`AbrirNavegador` com `Err(mensagem)` → `Aviso` origem `acoes` com a mensagem; `resultado` `Err` → `Erro` origem `main` `"{erro:#}"`

**Checkpoint**: `cargo test` passa; log real mostra `app: ação Sair` e `pool: todos os trabalhadores terminaram`.

---

## Phase 5: User Story 3 - Logs limitados e seguros (Priority: P3)

**Goal**: retenção de 20 arquivos, credenciais mascaradas, falha do log sem derrubar o app.

**Independent Test**: quickstart.md itens 3 e 4; passo de máscara de `test_registro_de_execucao`.

- [X] T013 [US3] Em tests/registro.rs, acrescentar `registrar(NivelEnum::Info, "teste", "https://usuario:segredo@exemplo.invalid/x.git")`; log contém `https://***@exemplo.invalid/x.git` e não contém `segredo`
- [X] T014 [US3] Validar retenção e degradação com o binário real: quickstart.md item 3 (22 execuções num diretório vazio → 20 arquivos `metra-*.log`, cada execução com `metra: log em ...` no stderr) e item 4 (`log indisponível: ...` na barra de status, pull normal, sem `metra: log em` no stderr)

**Checkpoint**: `cargo test` passa; quickstart.md itens 3 e 4.

---

## Phase 6: Polish & Cross-Cutting Concerns

- [X] T015 [P] Seção "Logs de execução" em README.md: local (`$XDG_STATE_HOME/metra/` ou `~/.local/state/metra/`), nome do arquivo, retenção de 20, conteúdo (contexto, descoberta, comandos com saída, tarefas, ações, aborto, panic, resumo), mascaramento de credenciais em URLs, linha `metra: log em <caminho>` no stderr em código 1/2 e `log indisponível` na barra de status
- [X] T016 `cargo fmt`, `cargo test` e `cargo clippy --all-targets -- -D warnings` limpos
- [X] T017 Executar quickstart.md item 2 com o binário real (PTY) e conferir stdout, stderr e as linhas esperadas no arquivo
- [X] T018 Marcar as tarefas concluídas neste arquivo (passo C6)

---

## Dependencies & Execution Order

- Setup (T001) → Foundational (T002 → T003 → T004) → US1 → US2 → US3 → Polish.
- US2 e US3 dependem da US1 por compartilharem tests/registro.rs, src/main.rs, src/executor.rs e
  src/pool.rs; dentro de cada história a ordem é teste → executor → pool → main.
- T015 (README.md) não conflita com nenhum outro arquivo e pode rodar a qualquer momento após T004.

## Parallel Example

```text
Após T004: T015 (README.md) em paralelo com T005 (tests/registro.rs).
US1: T007 (src/pool.rs) em paralelo com T006 (src/executor.rs).
US2: T010 (src/executor.rs) e T011 (src/pool.rs) em paralelo.
```

## Implementation Strategy

1. MVP: Setup + Foundational + US1 → já há um arquivo por execução com comandos, saídas e resumo,
   e o stderr aponta o arquivo.
2. US2 acrescenta ações, aborto e panic; US3 verifica máscara, retenção e degradação (implementadas
   na fundação e, a mensagem na barra de status, na US1).
3. Polish: README, gates e smoke real.
