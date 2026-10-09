# metra Constitution

metra é uma CLI/TUI em Rust que executa `git pull`/`git push` em todos os repositórios de um
workspace multi-repo, em paralelo, e destaca as falhas com o motivo classificado.

## Core Principles

### I. Idioma pt-BR e convenções Rust

- Código, comentários, mensagens da TUI e documentação MUST estar em português (pt-BR). Exceção:
  títulos e rótulos estruturais dos templates do Spec Kit (ex.: "User Story", "Phase",
  "Checkpoint", "Given/When/Then") permanecem em inglês para manter compatibilidade com o fluxo.
- Identificadores MUST seguir as convenções Rust: `snake_case` para funções, variáveis e
  módulos; `CamelCase` para tipos. Prefixos húngaros (`ds`, `nr`, `fg`…) MUST NOT ser usados.
- Enums de estado ou de erro MUST terminar em `Enum` (ex.: `TipoErroEnum`, `EstadoRepoEnum`).

Racional: o projeto é lido por falantes de português; o idioma das convenções da linguagem é
mantido para não brigar com o ecossistema (clippy, rustfmt, crates).

### II. Separação estrita de módulos

- Descoberta de repositórios, execução de git, classificação de erros e UI MUST ser módulos
  independentes, com dependências apenas no sentido UI → estado/app → execução → classificação.
- A UI MUST NOT executar git diretamente; ela só envia tarefas ao pool e consome eventos.
- O estado da aplicação (`App`) MUST ser testável sem terminal (sem I/O de terminal em `app.rs`).

Racional: cada camada pode ser testada e trocada isoladamente (ex.: novas fontes de repos).

### III. Testes com git real (NON-NEGOTIABLE)

- A classificação de erros MUST ser uma função pura coberta por testes unitários que usam
  saídas reais do git (2.34, `LC_ALL=C`).
- O comportamento de execução MUST ser coberto por testes de integração com repositórios git
  temporários reais (`tempfile`). Mocks de git MUST NOT ser usados.
- `cargo test` e `cargo clippy -- -D warnings` MUST passar antes de qualquer commit de código.

Racional: o valor do metra está em interpretar corretamente o git; só saídas reais provam isso.

### IV. Git apenas na raiz do próprio repositório

- O metra MUST NOT executar git em diretório que não tenha `.git` próprio (diretório ou
  arquivo) imediatamente dentro dele.
- Pastas sem `.git` MUST ser reportadas como "sem repositório" e caminhos ausentes como
  "não clonados", sem executar comando algum.

Racional: git sobe a árvore procurando um repositório; numa pasta sem `.git` ele operaria no
repositório pai e reportaria sucesso falso.

### V. Execução não-interativa

- Todo processo git MUST rodar com `GIT_TERMINAL_PROMPT=0`, `LC_ALL=C` e stdin nulo.
- Nenhuma operação pode aguardar entrada do usuário dentro de um worker.

Racional: prompts de credencial travariam threads invisíveis atrás da TUI; locale fixo garante
que a classificação por texto funcione.

### VI. Dependências mínimas

- Dependências de runtime MUST se limitar a `ratatui` (com `ratatui::crossterm` reexportado),
  `clap`, `serde`/`serde_json` e `anyhow`. Dependência de desenvolvimento: `tempfile`.
- Runtime async MUST NOT ser usado; paralelismo via threads da biblioteca padrão.
- Adicionar dependência exige emenda desta constituição.

Racional: binário pequeno, compilação rápida e superfície de manutenção reduzida.

### VII. Integrações opcionais degradam

- Integrações externas opcionais (ex.: herdr) MUST degradar com mensagem na barra de status
  quando indisponíveis ou quando falharem; elas MUST NOT encerrar nem travar o app.

Racional: o fluxo principal (pull/push) não pode depender de ferramentas de terceiros.

## Restrições Técnicas

- Linguagem: Rust edition 2024, toolchain estável (rustc 1.98+).
- Binário único `metra`; instalação via `cargo install --path .`.
- Códigos de saída: 0 sem falhas, 1 com falhas de repositório, 2 erro de configuração/descoberta.
- O terminal MUST ser restaurado em qualquer saída, inclusive panic.

## Fluxo de Desenvolvimento

- Desenvolvimento segue GitHub Spec Kit: constitution → specify → plan → tasks → analyze →
  implement, com revisão do usuário após `spec.md` e após `tasks.md`.
- Commits no padrão `<tipo>: <descrição>` (`feat`, `fix`, `docs`, `style`, `refactor`, `test`,
  `chore`), sem nome de agente na mensagem.
- Testes de classificação e de integração são escritos antes da implementação correspondente.

## Governance

- Esta constituição prevalece sobre outras práticas do projeto.
- Emendas exigem: alteração deste arquivo, justificativa no commit e atualização dos artefatos
  de spec/plan afetados.
- Versionamento semântico: MAJOR para remoção/redefinição de princípio, MINOR para princípio ou
  seção nova, PATCH para redação.
- Revisões de plano (`speckit.plan`, `speckit.analyze`) MUST verificar conformidade com cada
  princípio; violações MUST ser justificadas na seção de complexidade do plano.

**Version**: 1.0.1 | **Ratified**: 2026-10-09 | **Last Amended**: 2026-10-09
