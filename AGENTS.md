# AGENTS.md — metra

CLI/TUI em Rust que roda `git pull`/`git push` em paralelo nos repositórios de um workspace multi-repo e classifica as falhas. Uso e atalhos: [README.md](README.md); descoberta, tipos de falha, saída e logs: [docs/referencia.md](docs/referencia.md). Arquitetura, fluxo, glossário e status das features: [docs/CONTEXTO.md](docs/CONTEXTO.md).

## Precedência

1. [`.specify/memory/constitution.md`](.specify/memory/constitution.md): regras do projeto (MUST/MUST NOT). Prevalece sobre o resto.
2. Este arquivo.
3. Instruções globais do usuário. Este repositório é pessoal, não é CISSBOX: aqui **não se aplicam** as regras CISSBOX de C++/Qt/QML/SQL, prefixos de tipo (`ds`, `nr`, `fg`…), sufixos de classe, ID do Jira em commits, branches `build/{equipe}-…` e Uncrustify.

## Comandos

| Objetivo | Comando |
|---|---|
| Testes (unitários + integração com git real) | `cargo test` |
| Lint (zero avisos) | `cargo clippy --all-targets -- -D warnings` |
| Formatação | `cargo fmt` (conferir com `cargo fmt --check`) |
| Rodar sem instalar | `cargo run -- pull` ou `cargo run -- push`, de dentro de um workspace multi-repo |
| Instalar | `cargo install --path .` |

Requisitos: Rust estável ≥ 1.98 (edition 2024) e git ≥ 2.34. Os testes de integração de pull falham se `pull.rebase` ou `pull.ff` estiverem definidos no git global ou do sistema: `git config --get pull.rebase` e `git config --get pull.ff` devem sair vazios.

## Regras que mais quebram (resumo da constituição)

- Código, comentários, mensagens da TUI e docs em pt-BR. Identificadores no padrão Rust (`snake_case`/`CamelCase`), sem prefixos húngaros; enums de estado ou erro terminam em `Enum`.
- Dependências só no sentido UI → app → execução → classificação. `ui.rs` nunca executa git; `app.rs` não faz I/O de terminal.
- Testes usam git real com `tempfile`; mocks de git são proibidos. As saídas do git que a classificação reconhece são as do git 2.34 com `LC_ALL=C`.
- Git só roda em pasta com `.git` próprio. Todo processo git usa `GIT_TERMINAL_PROMPT=0`, `LC_ALL=C` e stdin nulo.
- Dependência nova ou runtime async exigem emenda da constituição.
- Integrações opcionais (herdr, `xdg-open`) degradam com mensagem na barra de status; nunca encerram nem travam o app.

## Fluxo de trabalho (Spec Kit)

- Feature nova ou mudança de comportamento: `/speckit.specify` → `/speckit.clarify` (opcional) → `/speckit.plan` → `/speckit.tasks` → `/speckit.analyze` → `/speckit.implement`. Pare para revisão do usuário depois de `spec.md` e depois de `tasks.md`.
- Artefatos ficam em `specs/NNN-nome/` (spec, plan, research, data-model, contracts, quickstart, tasks), com numeração sequencial.
- Ajuste pequeno sem spec nova (texto, bug pontual): atualize o contrato afetado em `specs/*/contracts/` no mesmo commit do código.
- `.specify/feature.json` aponta a feature ativa e é por máquina (fica fora do git). Para continuar uma feature existente em outra máquina: `printf '{"feature_directory":"specs/NNN-nome"}\n' > .specify/feature.json` ou `export SPECIFY_FEATURE_DIRECTORY=specs/NNN-nome`. `/speckit.specify` cria o arquivo ao abrir uma feature nova.
- Scripts, templates e comandos do Spec Kit estão versionados em `.specify/` e `.omp/commands/`; o CLI `specify` (versão 1.0.9, ver `.specify/init-options.json`) só é necessário para atualizar o Spec Kit. Não edite `.specify/scripts/` nem `.specify/templates/` à mão: a atualização sobrescreve. A constituição só muda via `/speckit.constitution`.

## Antes de commitar

1. `cargo test`, `cargo clippy --all-targets -- -D warnings` e `cargo fmt --check` passam.
2. Mudança visível ao usuário → README e `docs/referencia.md` atualizados (demos em `docs/demo/`, se a tela mudar). Mudança de contrato → `specs/*/contracts/` atualizado.
3. Feature concluída → `tasks.md` todo marcado e tabela "Features" de `docs/CONTEXTO.md` atualizada.
4. Commit `<tipo>: <descrição>` (`feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`), em português, sem ID do Jira e sem nome de agente. Um commit por mudança lógica.

## Não faça

- Rodar testes contra workspaces reais: use repositórios temporários.
- Classificar uma mensagem nova do git sem um teste com a saída real capturada.
- `git push --force` em `master`.
