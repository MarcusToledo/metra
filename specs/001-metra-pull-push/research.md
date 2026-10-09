# Research: metra pull/push

Nenhum `NEEDS CLARIFICATION` no Technical Context. Itens abaixo registram as verificações feitas.

## R1. Versão e API do ratatui

- **Decision**: `ratatui` 0.30 (0.30.2 é a mais recente, 2026-06-19), feature padrão `crossterm`
  (reexporta crossterm 0.29 em `ratatui::crossterm`). Usar `ratatui::init()` /
  `ratatui::restore()` / `DefaultTerminal`; `init()` instala panic hook que restaura o terminal.
- **APIs usadas**: `Terminal::draw(|frame: &mut Frame| …)`, `Frame::area()` (`size()` está
  deprecated), `Layout::vertical([...]).areas(area)`, `List`/`ListItem`/`ListState` +
  `render_stateful_widget`, `Paragraph::scroll((y, x))`, `Line`/`Span` com `Style`,
  `Block::bordered()`; `ratatui::crossterm::event::{poll, read, Event, KeyEvent, KeyEventKind,
  KeyCode}` — filtrar `KeyEventKind::Press`.
- **MSRV**: 1.88.0 (< 1.98.1 local).
- **Rationale**: API atual; evita dependência direta de crossterm (princípio VI).
- **Alternatives considered**: `enable_raw_mode` + `EnterAlternateScreen` manuais com guard em
  `Drop` — só como contingência se `init/restore` não existissem.
- **Fonte**: docs.rs/ratatui/0.30.2 (`init`, `restore`, `Frame`, `Layout`, widgets),
  docs.rs/crossterm/0.29.0 (`event`).

## R2. Mensagens reais do git 2.34.1 (`LC_ALL=C`)

Capturadas em repositórios temporários (script descartável) com git 2.34.1, sem `pull.rebase`/
`pull.ff` globais. Todas casam com as regras da FR-013 na ordem definida:

| Cenário | Exit | Trecho decisivo |
|---|---|---|
| Divergente | 128 | `fatal: Need to specify how to reconcile divergent branches.` |
| Conflito (`--no-rebase`) | 1 | `CONFLICT (content): Merge conflict in a.txt` |
| Merge pendente | 128 | `error: Pulling is not possible because you have unmerged files.` |
| Alterações locais | 1 | `error: Your local changes to the following files would be overwritten by merge:` |
| Não rastreados | 1 | `error: The following untracked working tree files would be overwritten by merge:` |
| Branch remota inexistente | 1 | `fatal: couldn't find remote ref nao-existe` |
| Remote inexistente | 1 | `fatal: 'upstream' does not appear to be a git repository` + `fatal: Could not read from remote repository.` |
| Push rejeitado | 1 | ` ! [rejected]        main -> main (fetch first)` + `hint: Updates were rejected …` |
| Push sem mudança | 0 | `Everything up-to-date` (stderr) |
| Rede | 1 | `fatal: unable to access '…': Could not resolve host: host-inexistente.invalid` |
| HEAD destacado | 0 | `git rev-parse --abbrev-ref HEAD` imprime `HEAD` |

- **Decision**: os testes unitários de `classifica.rs` usam esses textos literalmente.
- **Observação**: "Remote inexistente" também contém "Could not read from remote repository";
  a regra RemoteInexistente vem antes de RedeOuAutenticacao, então a ordem da FR-013 é essencial.
- **Observação**: o cenário Divergente depende de `pull.rebase`/`pull.ff` não configurados. O
  executor não força configuração (mantém o comportamento do `mgpull`); os testes de integração
  assumem git sem essas chaves no escopo global/sistema.

## R3. Ordem das chaves do `.meta`

- **Decision**: `Deserialize` manual (`visit_map`) num `Vec<String>` de chaves de `projects`.
- **Rationale**: `serde_json::Map` padrão é `BTreeMap` (ordem alfabética); a FR-004 exige a ordem
  do arquivo. O visitor manual preserva a ordem sem adicionar features/dependências.
- **Alternatives considered**: feature `preserve_order` do serde_json (traz `indexmap`, fere o
  princípio VI); ordenar alfabeticamente (viola FR-004).

## R4. Paralelismo e cancelamento

- **Decision**: pool fixo de N `std::thread` lendo `Arc<Mutex<mpsc::Receiver<Tarefa>>>`; eventos
  por `mpsc::Sender<Evento>`; `Arc<AtomicBool>` `cancelado`; thread vigia dá `join` nos workers e
  emite `Evento::PoolEncerrado`.
- **Rationale**: sem runtime async (princípio VI); o `Sender<Tarefa>` vivo permite reexecuções;
  ao sair, dropar o sender + `cancelado` faz workers descartarem pendentes e terminarem após a
  tarefa corrente, sem matar processos git no meio.
- **Alternatives considered**: uma thread por repo (sem limite `-j`); `rayon` (dependência extra,
  não suporta reenvio dinâmico facilmente).

## R5. Contagem de commits recebidos

- **Decision**: `git rev-parse HEAD` antes e depois; se diferentes,
  `git rev-list --count <antes>..<depois>`.
- **Rationale**: conta também o commit de merge em pulls `--no-rebase` (ex.: divergente com 1
  commit remoto → 2), coerente com "commits que entraram na branch".

## R6. Herdr

- **Decision**: `herdr pane split --current --direction down --cwd <abs>` apenas se
  `HERDR_ENV=1`; stderr do herdr (ou "herdr indisponível") vira mensagem de status.
- **Rationale**: flags confirmadas no `herdr pane split --help` local; foco no novo pane é o
  padrão (sem `--no-focus`).
