# Contrato da TUI

## Layout

```text
metra pull · remote upstream · /caminho/da/raiz                 12/45 ⠙
────────────────────────────────────────────────────────────────────────
✗ diverge  branches divergentes
✗ conflita  conflito em a.txt
↓ recebe  2 commits
⠋ lento
· aguardando
✓ 30 sem mudança
○ 7 não clonados
⚠ 1 pastas sem repositório
────────────────────────────────────────────────────────────────────────
[detalhe, quando aberto: branch, dica e log rolável]
────────────────────────────────────────────────────────────────────────
<barra de status>
↑↓/jk navegar · Enter detalhe · r repetir · n --no-rebase · R repetir falhas · p pane · q sair
```

- Ordem dos grupos: Falhou → Recebido/Enviado → Executando → Pendente → `✓` → `○` → `⚠`; ordem
  da fonte dentro de cada grupo; linhas agregadas omitidas quando a contagem é 0.
- Spinner (`⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏`) no cabeçalho e nas linhas Executando enquanto houver tarefa pendente ou
  em execução.

## Teclas

| Tecla | Efeito | Restrições |
|---|---|---|
| `↑`/`k`, `↓`/`j` | Move seleção | Limitada às linhas existentes |
| `Enter` | Alterna detalhe | Repo: branch, dica, log; agregada: nomes |
| `PgUp`/`PgDn` | Rola o log | Só com detalhe aberto |
| `r` | Reexecuta o repo selecionado | Repo Clonado em estado terminal |
| `n` | Reexecuta com `--no-rebase` | Só pull; no push: "indisponível no push" |
| `R` | Reexecuta todas as falhas | — |
| `p` | `herdr pane split --current --direction down --cwd <repo>` | `HERDR_ENV=1`; senão "herdr indisponível" |
| `q` | Inicia saída | Qualquer momento |
| `Esc` | Fecha detalhe; sem detalhe, igual a `q` | — |

Ações `r`/`n`/`p` em linha agregada, repo não Clonado ou repo Pendente/Executando: ignoradas com
mensagem na barra de status. Após `q`: `r`, `n`, `R`, `p` ignorados; status
"aguardando N tarefas…" até o pool encerrar.
