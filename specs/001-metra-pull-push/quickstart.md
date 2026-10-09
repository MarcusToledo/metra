# Quickstart: validar metra pull/push

## Pré-requisitos

- rustc/cargo ≥ 1.88 (testado com 1.98.1), git ≥ 2.34 sem `pull.rebase`/`pull.ff` globais.
- herdr (opcional) para validar a tecla `p`.

## 1. Testes automatizados

```bash
cd ~/Projetos/metra
cargo test                     # classifica (unit), tests/fonte.rs, tests/executor.rs
cargo clippy -- -D warnings
```

Esperado: todos passam; os cenários de `tests/executor.rs` cobrem sem mudança, recebido N,
divergente, divergente+`--no-rebase`, conflito, alterações locais, branch/remote inexistente,
HEAD destacado, push enviado/rejeitado e pasta sem `.git` dentro de repo pai.

## 2. Fixture manual

Script descartável cria `/tmp/metra-fixture/ws` (repo git pai) com repos `ok`, `recebe` (2
commits novos no upstream), `diverge`, `conflita`, `sujo` na branch `build/teste` com remote
`upstream`, a pasta `so-build/build/` sem `.git` e `.meta` listando também `ausente`.

```bash
cargo install --path ~/Projetos/metra
git -C /tmp/metra-fixture/ws rev-parse HEAD > /tmp/metra-fixture/head-antes
cd /tmp/metra-fixture/ws && metra pull
```

Esperado na TUI ([contrato](contracts/tui.md)):

- `✗ diverge  branches divergentes`, `✗ conflita  conflito em a.txt`,
  `✗ sujo  alterações locais seriam sobrescritas`
- `↓ recebe  2 commits`, `✓ 1 sem mudança`, `○ 1 não clonados`, `⚠ 1 pastas sem repositório`
- `git -C /tmp/metra-fixture/ws rev-parse HEAD` igual a `head-antes`.

Ações:

1. Selecionar `diverge`, `n` → `↓ diverge  2 commits`.
2. Dentro do herdr, selecionar `conflita`, `p` → `herdr pane list` mostra pane com
   `cwd=/tmp/metra-fixture/ws/conflita`.
3. `q` → stdout
   `metra pull: 1 sem mudança, 2 recebidos, 2 falhas (conflita, sujo), 1 não clonados, 1 sem repositório`;
   `echo $?` → `1`.

## 3. Erros de descoberta

```bash
cd /tmp && metra pull --fonte meta; echo $?
# nenhum .meta encontrado a partir de /tmp
# 2
```
