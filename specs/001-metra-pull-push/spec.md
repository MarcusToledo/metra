# Feature Specification: metra pull/push com TUI de progresso e triagem de falhas

**Feature Branch**: `001-metra-pull-push`

**Created**: 2026-10-09

**Status**: Draft

**Input**: User description: "metra pull/push com TUI de progresso e triagem de falhas — executar
pull/push em paralelo em todos os repositórios de um workspace multi-repo, mostrar progresso,
isolar falhas com motivo classificado e oferecer ações (detalhe/log, retry, retry `--no-rebase`,
abrir pane do herdr no repo)."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Atualizar o workspace inteiro e ver só o que falhou (Priority: P1)

O desenvolvedor, na raiz de um workspace com dezenas de repositórios, roda `metra pull`. Todos os
repositórios clonados são atualizados em paralelo, cada um na própria branch atual. A tela mostra o
progresso e, ao final, agrupa os resultados: falhas no topo com o motivo em poucas palavras,
repositórios que receberam commits logo abaixo e o restante resumido em linhas agregadas.

**Why this priority**: é o problema original — falhas enterradas no scroll de uma execução serial.
Sozinha, esta história já substitui o `mgpull` atual com ganho de tempo e de clareza.

**Independent Test**: workspace de teste com repositórios em situações conhecidas (sem mudança,
com commits novos, divergente, em conflito, com alterações locais, ausente, pasta sem repositório);
rodar `metra pull` e conferir os grupos, os textos de erro, a linha-resumo e o código de saída.

**Acceptance Scenarios**:

1. **Given** um workspace com `.meta` listando 45 repositórios, **When** o usuário roda
   `metra pull`, **Then** todos os clonados são atualizados com até 8 execuções simultâneas e a
   tela mostra `concluídos/total` e um spinner enquanto houver tarefa.
2. **Given** um repositório cujo upstream tem 2 commits novos, **When** o pull termina,
   **Then** a lista mostra `↓ <repo>  2 commits`.
3. **Given** um repositório com branches divergentes, **When** o pull termina, **Then** a lista
   mostra `✗ <repo>` com o texto curto de divergência no grupo de falhas, acima de todos os outros.
4. **Given** repositórios sem mudança, **When** o pull termina, **Then** eles aparecem apenas na
   linha agregada `✓ N sem mudança`.
5. **Given** uma pasta listada no `.meta` que não existe no disco, **When** o pull roda, **Then**
   nenhum comando é executado para ela e ela é contada em `○ N não clonados`.
6. **Given** uma pasta que existe mas não tem `.git` próprio, dentro de um diretório que é
   repositório git, **When** o pull roda, **Then** nenhum comando git é executado para ela, o
   repositório pai fica intacto e ela é contada em `⚠ N pastas sem repositório`.
7. **Given** a execução terminada com 2 falhas, **When** o usuário pressiona `q`, **Then** o
   terminal é restaurado, o stdout mostra
   `metra pull: X sem mudança, Y recebidos, 2 falhas (repo1, repo2), W não clonados, V sem repositório`
   e o código de saída é 1.

---

### User Story 2 - Investigar e resolver falhas sem sair da TUI (Priority: P2)

Com as falhas visíveis, o desenvolvedor seleciona um repositório, abre o detalhe (branch, log
completo e dica do tipo de erro) e resolve: reexecuta (`r`), reexecuta com merge `--no-rebase`
(`n`) quando divergente, reexecuta todas as falhas (`R`) ou abre um pane do herdr no repositório
(`p`) para resolver um conflito manualmente.

**Why this priority**: elimina o ciclo "entrar no repo, rodar `git pull upstream <branch>
--no-rebase`, voltar". Depende da história 1 para existir a lista.

**Independent Test**: no workspace de teste, selecionar o repositório divergente e pressionar `n`;
ele deve passar a `↓ <repo>  N commits`. Selecionar o repositório em conflito e pressionar `p`
dentro do herdr; deve surgir um novo pane com diretório de trabalho no repositório.

**Acceptance Scenarios**:

1. **Given** um repositório com falha Divergente selecionado, **When** o usuário pressiona `n`,
   **Then** ele volta a Pendente, é reexecutado com `--no-rebase` e termina como Recebido.
2. **Given** um repositório com falha selecionado, **When** o usuário pressiona Enter, **Then** o
   painel de detalhe mostra a branch, o log completo (rolável com PgUp/PgDn) e a dica do tipo de
   erro; Enter ou Esc fecha o painel.
3. **Given** a TUI rodando dentro do herdr (`HERDR_ENV=1`) com um repositório em conflito
   selecionado, **When** o usuário pressiona `p`, **Then** um pane é aberto abaixo com diretório
   de trabalho no repositório, o foco vai para ele e a TUI continua rodando no pane original.
4. **Given** a TUI rodando fora do herdr, **When** o usuário pressiona `p`, **Then** a barra de
   status mostra "herdr indisponível" e o app continua.
5. **Given** várias falhas, **When** o usuário pressiona `R`, **Then** todas as falhas voltam a
   Pendente e são reexecutadas.
6. **Given** `metra push`, **When** o usuário pressiona `n`, **Then** a barra de status mostra
   "indisponível no push" e nada é reexecutado.
7. **Given** a linha agregada `○ N não clonados` selecionada, **When** o usuário pressiona Enter,
   **Then** o painel de detalhe lista os nomes desses repositórios.

---

### User Story 3 - Publicar todas as branches com `metra push` (Priority: P3)

O desenvolvedor roda `metra push` para enviar a branch atual de cada repositório ao remote
`origin`, com a mesma tela, agrupamento e ações (exceto `n`).

**Why this priority**: substitui o `mgpush`; reaproveita a infraestrutura das histórias 1 e 2.

**Independent Test**: workspace de teste com um repositório com commit local não enviado, um
sincronizado e um cujo remote tem commits que o local não tem; rodar `metra push` e conferir
`↑ <repo>`, `✓ 1 sem mudança` e a falha de push rejeitado.

**Acceptance Scenarios**:

1. **Given** um repositório com commits locais não enviados, **When** o push termina, **Then** a
   lista mostra `↑ <repo>`.
2. **Given** um repositório já sincronizado, **When** o push termina, **Then** ele conta em
   `✓ N sem mudança`.
3. **Given** um repositório cujo remote avançou, **When** o push termina, **Then** ele aparece como
   falha de push rejeitado.
4. **Given** a execução terminada, **When** o usuário pressiona `q`, **Then** a linha-resumo usa
   `enviados` no lugar de `recebidos`.

---

### User Story 4 - Workspace sem `.meta` (Priority: P3)

Num diretório que contém repositórios como subpastas, mas nenhum `.meta`, `metra pull` descobre os
repositórios varrendo as subpastas imediatas.

**Why this priority**: torna a ferramenta útil fora dos workspaces gerenciados pelo meta.

**Independent Test**: diretório com subpastas `a` (repositório), `.oculta` (repositório), `b`
(sem `.git`) e nenhum `.meta`; `metra pull` deve tratar apenas `a`.

**Acceptance Scenarios**:

1. **Given** nenhum `.meta` no diretório atual nem nos ancestrais, **When** o usuário roda
   `metra pull`, **Then** os repositórios são as subpastas imediatas não ocultas com `.git`,
   ordenadas por nome.
2. **Given** um `.meta` num diretório ancestral, **When** o usuário roda `metra pull` num
   subdiretório, **Then** a raiz é o diretório do `.meta` e os repositórios são os listados nele.
3. **Given** nenhum `.meta` a partir de `/tmp`, **When** o usuário roda
   `metra pull --fonte meta`, **Then** a mensagem "nenhum .meta encontrado a partir de /tmp" é
   exibida e o código de saída é 2.

---

### Edge Cases

- `.meta` com JSON inválido ou sem `projects`: erro com a mensagem do parser, código 2.
- Nenhum repositório resultante da descoberta: "nenhum repositório encontrado em <raiz>", código 2.
- Repositório em HEAD destacado: falha HeadDestacado, nenhum pull/push executado.
- Merge pendente de uma execução anterior: falha MergePendente.
- Remote inexistente no repositório: falha RemoteInexistente.
- Branch local sem equivalente no remote: falha BranchRemotaInexistente.
- Remote que pediria credencial: o git não pode solicitar senha; falha RedeOuAutenticacao em vez de
  travar.
- Saída de erro não reconhecida: falha Desconhecido com a última linha não vazia da saída.
- `q` com tarefas em execução: nenhuma nova tarefa é iniciada; as em execução terminam (status
  "aguardando N tarefas…"); pendentes não executadas não entram em nenhum grupo da linha-resumo.
- Ações `r`, `n`, `R`, `p` em linhas agregadas ou repositórios não clonados/sem repositório:
  ignoradas com mensagem na barra de status.
- Ações após `q`: desabilitadas.
- Caminho do `.meta` com subdiretórios (ex.: `grupo/repo`): tratado como caminho relativo à raiz.

## Requirements *(mandatory)*

### Functional Requirements

**CLI**

- **FR-001**: O sistema MUST oferecer os comandos `metra pull` e `metra push`, ambos com as opções
  `-j N` (padrão 8), `--remote NOME` (padrão `upstream` no pull e `origin` no push) e
  `--fonte auto|meta|varredura` (padrão `auto`).

**Descoberta**

- **FR-002**: Com `--fonte auto`, o sistema MUST procurar `.meta` no diretório atual e nos
  ancestrais; se encontrar, usa a fonte Meta com raiz no diretório do `.meta`; senão, usa a fonte
  Varredura com raiz no diretório atual.
- **FR-003**: Com `--fonte meta` e nenhum `.meta` encontrado, o sistema MUST exibir
  "nenhum .meta encontrado a partir de <cwd>" e sair com código 2.
- **FR-004**: A fonte Meta MUST tratar cada chave de `projects` como caminho relativo à raiz,
  preservando a ordem do arquivo; JSON inválido ou ausência de `projects` MUST resultar em erro
  com a mensagem do parser e código 2.
- **FR-005**: A fonte Varredura MUST listar os subdiretórios imediatos da raiz, não ocultos, que
  contenham `.git` (diretório ou arquivo), ordenados por nome.
- **FR-006**: A raiz MUST NOT ser incluída como repositório; nenhum repositório resultante MUST
  produzir "nenhum repositório encontrado em <raiz>" e código 2.
- **FR-007**: Cada repositório MUST ser classificado antes da execução como Clonado (`.git`
  existe no próprio caminho), NaoClonado (caminho não existe) ou SemRepositorio (caminho existe sem
  `.git` próprio); apenas Clonado executa git.

**Execução**

- **FR-008**: O sistema MUST executar as tarefas num conjunto fixo de N execuções simultâneas,
  iniciando-as na ordem da fonte.
- **FR-009**: Pull MUST: (1) obter a branch atual; se for `HEAD`, falhar com HeadDestacado sem
  executar o pull; (2) registrar o commit atual; (3) executar `git pull [--no-rebase] <remote>
  <branch>`; (4) em sucesso, comparar o commit: igual → SemMudanca; diferente → Recebido com a
  quantidade de commits entre o antes e o depois.
- **FR-010**: Push MUST obter a branch como no pull e executar `git push <remote> <branch>`; em
  sucesso, saída contendo "Everything up-to-date" → SemMudanca, senão Enviado.
- **FR-011**: Todo processo git MUST rodar sem possibilidade de solicitar entrada (sem prompt de
  terminal, entrada nula) e com mensagens em inglês padrão (locale C).
- **FR-012**: O log guardado de cada repositório MUST conter stdout e stderr de todos os comandos
  da tentativa mais recente.

**Classificação de falhas**

- **FR-013**: Uma falha (código de saída diferente de zero) MUST ser classificada pelo texto
  stdout+stderr, aplicando a primeira regra que casar, nesta ordem:

  | Texto contido | Tipo |
  |---|---|
  | "You have not concluded your merge" ou "Pulling is not possible because you have unmerged files" | MergePendente |
  | "CONFLICT (" | Conflito (arquivos extraídos das linhas "Merge conflict in <arquivo>") |
  | "Need to specify how to reconcile divergent branches" | Divergente |
  | "Your local changes to the following files would be overwritten" | AlteracoesLocais |
  | "untracked working tree files would be overwritten" | ArquivosNaoRastreados |
  | "couldn't find remote ref" | BranchRemotaInexistente |
  | "does not appear to be a git repository" | RemoteInexistente |
  | "[rejected]" ou "Updates were rejected" | PushRejeitado |
  | "Permission denied", "Could not read from remote repository", "Could not resolve host", "Connection timed out", "Connection refused" ou "terminal prompts disabled" | RedeOuAutenticacao |
  | nenhuma das anteriores | Desconhecido (última linha não vazia) |

- **FR-014**: Cada tipo MUST ter um texto curto exibido na lista e uma dica exibida no detalhe:
  Divergente → "use [n] para merge com --no-rebase"; Conflito, MergePendente, AlteracoesLocais e
  ArquivosNaoRastreados → "resolva manualmente: [p] abre pane no repo"; demais → "veja o log".

**TUI**

- **FR-015**: A TUI MUST ocupar a tela alternativa do terminal e exibir o cabeçalho
  `metra <operação> · remote <remote> · <raiz>`, o contador `concluídos/total` e um spinner
  enquanto houver tarefa pendente ou em execução.
- **FR-016**: A lista MUST exibir, nesta ordem de grupos e na ordem da fonte dentro de cada grupo:
  Falhou (`✗ <repo>  <texto do erro>`); Recebido (`↓ <repo>  N commits`) ou Enviado
  (`↑ <repo>`); Executando (`⠋ <repo>`, com spinner); Pendente (`· <repo>`); a linha agregada
  `✓ N sem mudança`; a linha agregada `○ N não clonados`; a linha agregada
  `⚠ N pastas sem repositório`.
- **FR-017**: Enter MUST alternar o painel de detalhe; para um repositório, o painel mostra a
  branch, o log completo com rolagem (PgUp/PgDn) e a dica do tipo de erro; para uma linha
  agregada, mostra os nomes dos repositórios do grupo.
- **FR-018**: O rodapé MUST listar os atalhos: ↑/↓ e k/j navegam; Enter detalhe; `r` reexecuta o
  selecionado; `n` reexecuta com `--no-rebase`; `R` reexecuta todas as falhas; `p` abre pane do
  herdr; `q` sai; Esc fecha o detalhe se aberto, senão sai.
- **FR-019**: Reexecução MUST levar o repositório de volta a Pendente e reenviá-lo para execução.
  `n` no push MUST apenas mostrar "indisponível no push" na barra de status.
- **FR-020**: Ações sobre repositórios não Clonado ou linhas agregadas MUST ser ignoradas com
  mensagem na barra de status.

**Herdr**

- **FR-021**: `p` MUST funcionar apenas com `HERDR_ENV=1`, executando
  `herdr pane split --current --direction down --cwd <caminho absoluto do repositório>`; o foco vai
  para o novo pane e a TUI continua no pane original.
- **FR-022**: Fora do herdr, a barra de status MUST mostrar "herdr indisponível"; se o comando
  falhar, MUST mostrar o stderr do herdr; em ambos os casos o app continua.

**Saída**

- **FR-023**: `q` MUST funcionar a qualquer momento: nenhuma nova tarefa é iniciada, as em execução
  são aguardadas (status "aguardando N tarefas…"), o terminal é restaurado e o stdout recebe a
  linha-resumo `metra pull: X sem mudança, Y recebidos, Z falhas[ (repo1, repo2…)], W não
  clonados, V sem repositório` (no push, `enviados` no lugar de `recebidos`).
- **FR-024**: Tarefas ainda pendentes na saída MUST contar como não executadas e não entrar em
  nenhum grupo da linha-resumo.
- **FR-025**: O código de saída MUST ser 0 sem falhas, 1 com falhas e 2 para erro de configuração
  ou descoberta.
- **FR-026**: O terminal MUST ser restaurado em qualquer encerramento, inclusive erro inesperado.

### Key Entities

- **Repositório**: nome (chave do `.meta` ou nome da subpasta), caminho absoluto e situação
  (Clonado, NaoClonado, SemRepositorio).
- **Tarefa**: repositório, operação (Pull ou Push), remote e indicação de merge sem rebase.
- **Estado do repositório**: Pendente, Executando, SemMudanca, Recebido(N commits), Enviado ou
  Falhou(tipo de erro); acompanhado do log da última tentativa e da branch.
- **Tipo de erro**: um dos tipos da FR-013, com texto curto e dica.
- **Fonte de repositórios**: Meta (arquivo `.meta`) ou Varredura (subpastas); extensível a novas
  fontes.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Num workspace com 45 repositórios, todas as falhas ficam visíveis na primeira tela,
  sem rolagem, num terminal de 24 linhas com até 10 falhas.
- **SC-002**: Repositórios em situação SemRepositorio ou NaoClonado nunca executam git (o commit
  do repositório pai permanece igual após a execução).
- **SC-003**: Uma falha Divergente é resolvida com uma tecla (`n`), sem sair da TUI.
- **SC-004**: 100% das falhas de pull/push dos cenários de teste (divergente, conflito, alterações
  locais, branch remota inexistente, remote inexistente, HEAD destacado, push rejeitado) recebem o
  tipo correto.

## Assumptions

- Pull mantém o comportamento do `mgpull` atual (sem `--no-rebase` por padrão); `--no-rebase` só
  pela tecla `n`.
- `p` move o foco para o novo pane, pois o objetivo é resolver o conflito lá.
- O `.meta` segue o formato `{"projects": {"<caminho relativo>": "<url>"}}`; outros campos são
  ignorados.
- O git instalado é 2.34 ou mais recente; as mensagens classificadas são as do locale C.
- O herdr é opcional; sem ele, todas as demais funcionalidades permanecem disponíveis.
- Clonar repositórios ausentes está fora do escopo; eles apenas são reportados.
