# metra

CLI/TUI para executar `git pull` ou `git push` em paralelo nos repositórios de um workspace multi-repo. A tela acompanha o progresso e destaca falhas classificadas, com log e dicas para tratá-las.

## Instalação

Com Rust e Cargo instalados, na raiz do projeto:

```sh
cargo install --path .
```

## Uso

```text
metra pull [-j N] [--remote NOME] [--fonte auto|meta|varredura]
metra push [-j N] [--remote NOME] [--fonte auto|meta|varredura]
```

| Opção | Padrão | Descrição |
|---|---|---|
| `-j N`, `--jobs N` | `8` | Número de tarefas paralelas; deve ser pelo menos `1`. |
| `--remote NOME` | `upstream` no pull; `origin` no push | Remote usado na operação. |
| `--fonte` | `auto` | Fonte dos repositórios: `auto`, `meta` ou `varredura`. |

### Descoberta dos repositórios

| Fonte | Raiz | Repositórios |
|---|---|---|
| `auto`, com `.meta` no diretório atual ou em um ancestral | Diretório do `.meta` mais próximo | Chaves de `projects`, na ordem do arquivo. |
| `auto`, sem `.meta` | Diretório atual | Subpastas imediatas não ocultas que contenham `.git`, ordenadas por nome. |
| `meta` | Diretório do `.meta` mais próximo | Chaves de `projects`, na ordem do arquivo. |
| `varredura` | Diretório atual | Subpastas imediatas não ocultas que contenham `.git`, ordenadas por nome. |

A raiz não é incluída na varredura. Um repositório só é executado quando sua própria pasta contém `.git`.

### Saída e códigos de saída

Após o encerramento da TUI, o programa imprime no stdout um resumo, por exemplo:

```text
metra pull: X sem mudança, Y recebidos, Z falhas (repo1, repo2), W não clonados, V sem repositório
metra push: X sem mudança, Y enviados, Z falhas (repo1, repo2), W não clonados, V sem repositório
```

A lista de repositórios com falha aparece somente quando há falhas. No push, cada repositório cujo remote anunciou um link de merge/pull request gera em seguida uma linha `merge <repo>: <url>`. Erros de configuração e descoberta são exibidos no stderr, sem abrir a TUI: `.meta` não encontrado, erro ao interpretar o JSON ou nenhum repositório encontrado.

| Código | Significado |
|---|---|
| `0` | Operação concluída sem falhas. |
| `1` | Uma ou mais falhas na operação. |
| `2` | Erro de argumentos, configuração ou descoberta. |

## Tela e atalhos

A lista agrupa falhas primeiro, depois resultados recebidos/enviados, tarefas em execução e pendentes, e por fim as linhas agregadas de sem mudança, não clonados e pastas sem repositório. A ordem da fonte é mantida dentro de cada grupo. No push, um repositório enviado cujo remote anunciou um link de merge/pull request aparece como `↑ <repo>  ⇗ merge`. O detalhe mostra a branch, o link de merge (quando houver), a dica correspondente e o log rolável; nas linhas agregadas, mostra os nomes. A barra de status informa mensagens e, ao sair com tarefas em execução, indica que está aguardando.

| Tecla | Efeito | Restrições |
|---|---|---|
| `↑` / `k`, `↓` / `j` | Move a seleção. | Limitada às linhas existentes. |
| `Enter` | Abre ou fecha o detalhe. | Em uma linha agregada, lista os nomes. |
| `PgUp` / `PgDn` | Rola o log. | Só com detalhe aberto. |
| `r` | Reexecuta o repositório selecionado. | Repositório clonado em estado terminal. |
| `n` | Reexecuta com `--no-rebase`. | Somente no pull; no push, informa “indisponível no push”. |
| `R` | Reexecuta todos os repositórios com falha. | — |
| `p` | Abre um pane do herdr no repositório. | Requer `HERDR_ENV=1`. |
| `m` | Abre no navegador o link de merge/pull request anunciado pelo push. | Repositório com link; usa `xdg-open`. |
| `q` | Inicia a saída; pressionado de novo durante a saída, aborta. | Disponível a qualquer momento; a primeira vez aguarda as tarefas em execução, a segunda mata os comandos git (e o ssh) em execução, que contam como falha "abortado pelo usuário". |
| `Esc` | Fecha o detalhe ou inicia a saída. | Sem detalhe aberto, tem o efeito de `q`. |

Ações incompatíveis com a linha selecionada ou após iniciar a saída são ignoradas, com mensagem na barra de status.

### Tipos de falha

| Tipo | Texto curto na lista | Dica no detalhe |
|---|---|---|
| Merge pendente | `merge pendente` | resolva manualmente: `[p]` abre pane no repo |
| Conflito | `conflito em <arq1>, <arq2>, <arq3> (+N)` | resolva manualmente: `[p]` abre pane no repo |
| Branches divergentes | `branches divergentes` | use `[n]` para merge com `--no-rebase` |
| Alterações locais | `alterações locais seriam sobrescritas` | resolva manualmente: `[p]` abre pane no repo |
| Arquivos não rastreados | `arquivos não rastreados seriam sobrescritos` | resolva manualmente: `[p]` abre pane no repo |
| Branch remota inexistente | `branch não existe no remote` | veja o log |
| Remote inexistente | `remote inexistente` | veja o log |
| Push rejeitado | `push rejeitado` | veja o log |
| Rede ou autenticação | `falha de rede ou autenticação` | veja o log |
| HEAD destacado | `HEAD destacado` | veja o log |
| Desconhecida | Última linha não vazia do log | veja o log |

## Execução segura e herdr

O metra nunca executa Git em uma pasta sem `.git` próprio; pastas sem repositório próprio não fazem a operação subir para um repositório pai. Os comandos Git são não interativos, com `GIT_TERMINAL_PROMPT=0` e `LC_ALL=C`. Quando o usuário não configurou `GIT_SSH_COMMAND`, `GIT_SSH` nem `core.sshCommand`, o SSH é executado com `BatchMode=yes`, `ConnectTimeout=15`, `ServerAliveInterval=15` e `ServerAliveCountMax=3`: uma conexão que não responde (por exemplo, após queda da VPN) falha como rede/autenticação em vez de prender a tarefa.

A integração com herdr é opcional. A tecla `p` abre um pane no repositório selecionado somente quando `HERDR_ENV=1`; fora desse ambiente ou se o comando falhar, o metra informa o problema na barra de status e continua em execução.

## Desenvolvimento

Na raiz do projeto:

```sh
cargo test
cargo clippy --all-targets -- -D warnings
```

Os testes de integração que executam pull precisam que o Git não tenha `pull.rebase` nem `pull.ff` definidos na configuração global ou do sistema.

Mais detalhes de requisitos, arquitetura e contratos estão em [`specs/001-metra-pull-push/`](specs/001-metra-pull-push/).
