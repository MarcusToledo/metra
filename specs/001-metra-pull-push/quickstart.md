# Quickstart: validar metra pull/push

## Pré-requisitos

- rustc/cargo ≥ 1.88 (testado com 1.98.1), git ≥ 2.34 sem `pull.rebase`/`pull.ff` globais.
- herdr (opcional) para validar a tecla `p`.

## 1. Testes automatizados

```bash
cd ~/Projetos/metra
cargo test                     # unit (classifica, app, ui, acoes) + tests/{fonte,executor,pool}.rs
cargo clippy -- -D warnings
```

Esperado: todos passam; os cenários de `tests/executor.rs` cobrem sem mudança, recebido N,
divergente, divergente+`--no-rebase`, conflito, alterações locais, branch/remote inexistente,
HEAD destacado, push enviado/rejeitado e pasta sem `.git` dentro de repo pai.

## 2. Fixture manual

O script descartável `/tmp/metra-fixture.sh` (fora do repositório; tarefa T030) cria
`/tmp/metra-fixture/ws` (repo git pai com um commit) com repos `ok`, `recebe` (2 commits novos no
upstream), `diverge` (commits em arquivos distintos nos dois lados), `conflita` (mesma linha de
`a.txt` alterada nos dois lados e `pull.rebase=false` local, para o pull padrão fazer merge e
parar em CONFLICT em vez de Divergente) e `sujo` (upstream altera `a.txt`; clone com `a.txt` modificado
sem commit), todos clonados de `/tmp/metra-fixture/upstream/<r>.git`, com o remote renomeado para
`upstream` e na branch `build/teste`; a pasta `so-build/build/` sem `.git`; e `.meta` com
`projects` = `ok, recebe, diverge, conflita, sujo, so-build, ausente`.

```bash
bash /tmp/metra-fixture.sh
```

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
