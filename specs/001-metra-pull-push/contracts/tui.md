# Contrato da TUI

## Layout

```text
metra pull · remote upstream · /caminho/da/raiz                 12/45 ⠙
────────────────────────────────────────────────────────────────────────
✗ diverge  branches divergentes
✗ conflita  conflito em a.txt
↓ recebe  2 commits
↑ publica  ⇗ merge
⠋ lento
· aguardando
✓ 30 sem mudança
○ 7 não clonados
⚠ 1 pastas sem repositório
────────────────────────────────────────────────────────────────────────
[detalhe, quando aberto: branch, link de merge, dica e log rolável]
────────────────────────────────────────────────────────────────────────
<barra de status>
↑↓/jk mover · Enter detalhe · PgUp/PgDn rolar log · r repetir · n sem rebase · R repetir falhas · p abrir pane · m abrir merge · q/Esc sair
```

- Ordem dos grupos: Falhou → Recebido/Enviado → Executando → Pendente → `✓` → `○` → `⚠`; ordem
  da fonte dentro de cada grupo; linhas agregadas omitidas quando a contagem é 0.
- Spinner (`⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏`) no cabeçalho e nas linhas Executando enquanto houver tarefa pendente ou
  em execução.
- Rodapé: tecla em destaque (ciano, negrito) seguida da ação; quebra em quantas linhas a largura
  exigir, sem cortar pares tecla/ação. `n` só aparece no pull.

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
| `m` | `xdg-open <url>` do link de merge | Repo com link; senão "nenhum link de merge para esta linha"; sem xdg-open, "xdg-open indisponível" |
| `q` | Inicia saída; durante a saída, aborta as em execução | Qualquer momento |
| `Esc` | Fecha detalhe; sem detalhe, igual a `q` | — |

`r`/`n` em linha agregada, repo não Clonado ou repo Pendente/Executando, e `p` em linha agregada ou
repo não Clonado: ignorados com mensagem na barra de status. Após `q`: `r`, `n`, `R`, `p`, `m` ignorados; status
"aguardando N tarefas… (q de novo aborta)" até o pool encerrar. Segundo `q`/Esc com tarefas em execução:
mata os comandos git (e ssh) em execução, status "abortando N tarefas…"; elas terminam como falha
"abortado pelo usuário".
