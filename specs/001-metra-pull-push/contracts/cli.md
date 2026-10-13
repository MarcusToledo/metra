# Contrato da CLI

```text
metra pull [-j N] [--remote NOME] [--fonte auto|meta|varredura]
metra push [-j N] [--remote NOME] [--fonte auto|meta|varredura]
```

| Opção | Padrão | Validação |
|---|---|---|
| `-j N` / `--jobs N` | 8 | inteiro ≥ 1 (clap rejeita 0 com exit 2) |
| `--remote NOME` | `upstream` (pull), `origin` (push) | texto livre |
| `--fonte` | `auto` | um de `auto`, `meta`, `varredura` |

## Descoberta

| `--fonte` | Raiz | Repos |
|---|---|---|
| `auto` com `.meta` no cwd ou ancestral | diretório do `.meta` mais próximo | chaves de `projects`, ordem do arquivo |
| `auto` sem `.meta` | cwd | subpastas imediatas não ocultas com `.git`, por nome |
| `meta` | diretório do `.meta` mais próximo | idem |
| `varredura` | cwd | idem varredura |

## Saídas

**stdout** (só após a TUI encerrar):

```text
metra pull: X sem mudança, Y recebidos, Z falhas (repo1, repo2), W não clonados, V sem repositório
metra push: X sem mudança, Y enviados, Z falhas, W não clonados, V sem repositório
merge <repo>: <url>
```

A lista entre parênteses aparece só quando Z > 0. As linhas `merge` (uma por repo com link, na
ordem da fonte, só no push) aparecem só quando algum push anunciou link de merge.

**stderr** (erros de configuração, sem abrir a TUI):

- `nenhum .meta encontrado a partir de <cwd>`
- `<mensagem do parser JSON>` (com o caminho do `.meta`)
- `nenhum repositório encontrado em <raiz>`

**Exit codes**: `0` sem falhas · `1` ao menos uma falha · `2` erro de configuração/descoberta
(inclui erro de argumentos do clap).
