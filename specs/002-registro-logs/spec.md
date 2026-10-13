# Feature Specification: Registro de logs de execução do metra

**Feature Branch**: `002-registro-logs`

**Created**: 2026-10-13

**Status**: Draft

**Input**: User description: "Registrar em arquivo tudo o que cada execução do metra faz, para
diagnosticar falhas depois que a TUI fecha e apoiar manutenções. Cada execução cria um arquivo
próprio no diretório de estado do usuário (`$XDG_STATE_HOME/metra/`, ou `~/.local/state/metra/` se
a variável estiver vazia ou for relativa). Só as 20 execuções mais recentes ficam guardadas. O
arquivo registra: início (versão do metra, versão do git, operação, remote, jobs, fonte, diretório
atual), resultado da descoberta (raiz e cada repositório com sua situação), cada comando git
(repositório, argumentos, código de saída, duração e saída completa), início e fim de cada tarefa
com o estado final, ações do usuário na TUI, falhas das integrações herdr/navegador, saída e
aborto, erros fatais, panics com backtrace e o resumo final com o código de saída. Credenciais
embutidas em URLs (`https://usuario:token@host`) aparecem como `***`. Se o log não puder ser
criado, o metra continua normalmente e avisa na barra de status. Quando a execução termina com
código diferente de 0, o stderr informa o caminho do arquivo de log."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Diagnosticar uma execução com falha depois que a TUI fechou (Priority: P1)

O desenvolvedor roda `metra pull`, algumas falhas aparecem, ele pressiona `q` e a TUI fecha.
Depois, quer entender o que aconteceu em um repositório específico (que comando rodou, quanto
tempo levou, o que o git respondeu). O stderr mostrou o caminho do arquivo de log da execução; ele
abre esse arquivo e encontra, em ordem cronológica, o início da execução com o contexto (versões,
operação, remote, jobs, fonte, diretório atual), a lista de repositórios descobertos, cada comando
git com código de saída, duração e saída completa, e o estado final de cada repositório, até o
resumo com o código de saída.

**Why this priority**: é o motivo da feature — hoje o log de cada repositório some quando a TUI
fecha, e relatos de falha chegam sem evidência. Sozinha, esta história já resolve o diagnóstico
pós-execução.

**Independent Test**: workspace de teste com um repositório que atualiza sem mudança e dois cujo
remote não existe; rodar `metra pull`, sair com `q` e conferir a linha `metra: log em <caminho>` no
stderr e o conteúdo do arquivo (contexto, descoberta, comandos com saída, estados finais, resumo).

**Acceptance Scenarios**:

1. **Given** uma execução que termina com falhas (código 1), **When** o usuário pressiona `q`,
   **Then** o stdout mostra a linha-resumo como antes e o stderr mostra
   `metra: log em <caminho do arquivo>`.
2. **Given** uma execução sem falhas (código 0), **When** ela termina, **Then** o stderr não
   mostra o caminho do log, mas o arquivo existe e contém a execução completa.
3. **Given** um erro de descoberta (código 2, ex.: nenhum repositório encontrado), **When** o
   metra encerra, **Then** o stderr mostra a mensagem de erro seguida de
   `metra: log em <caminho>`, e o arquivo contém o erro.
4. **Given** um repositório cujo pull falhou, **When** o usuário abre o arquivo de log, **Then**
   encontra o comando `git pull <remote> <branch>` daquele repositório com o código de saída, a
   duração em milissegundos, a saída completa do git e a linha de fim da tarefa com o estado
   final (ex.: `Falhou(RemoteInexistente)`).
5. **Given** o arquivo de log de uma execução, **When** o usuário o lê, **Then** a primeira
   entrada traz a versão do metra, a operação, o remote, o número de jobs e a fonte; a seguinte
   traz a versão do git; depois vem a descoberta com o diretório atual, a raiz e cada repositório
   com sua situação (Clonado, NaoClonado, SemRepositorio).

---

### User Story 2 - Investigar aborto, travamento e panic (Priority: P2)

Uma execução parece travar e o desenvolvedor pressiona `q` duas vezes para abortar, ou o metra
encerra com um panic. Depois, ele abre o log e vê a sequência de ações que digitou na TUI, quando a
saída foi pedida, quando o aborto matou os processos em andamento, quais tarefas pendentes foram
descartadas e, no caso de panic, a mensagem e o backtrace completo com a thread em que ocorreu.

**Why this priority**: travamentos e panics são os casos mais difíceis de reproduzir; sem registro
das ações e do backtrace, a manutenção depende de memória do usuário. Depende da infraestrutura da
história 1.

**Independent Test**: provocar um panic numa thread nomeada com o registro ativo e conferir no
arquivo a entrada de nível ERRO com a thread, a mensagem e o backtrace; numa execução real,
pressionar `q` duas vezes durante um pull lento e conferir as entradas de ação, saída e aborto.

**Acceptance Scenarios**:

1. **Given** a TUI em execução, **When** o usuário pressiona teclas que geram ações (navegar,
   detalhe, reexecutar, sair), **Then** cada ação aparece no log na ordem em que ocorreu.
2. **Given** tarefas em execução, **When** o usuário pressiona `q` e depois `q` de novo, **Then**
   o log registra a saída iniciada (pendentes descartadas), o aborto solicitado, quantos grupos de
   processos foram encerrados e o fim de cada tarefa abortada.
3. **Given** um panic em qualquer thread, **When** ele ocorre, **Then** o log registra, em nível
   ERRO, o nome da thread, a mensagem do panic e o backtrace, antes de o terminal ser restaurado.
4. **Given** `p` ou `m` pressionados e a integração herdr ou navegador falhando, **When** a
   mensagem aparece na barra de status, **Then** a mesma mensagem é registrada no log como aviso.

---

### User Story 3 - Logs limitados e seguros (Priority: P3)

O desenvolvedor usa o metra várias vezes por dia por meses e não quer que os logs acumulem no
disco nem que tokens de acesso fiquem gravados em texto claro. Se o diretório de logs não puder
ser usado (permissão, disco cheio, `HOME` ausente), o metra funciona como antes e avisa.

**Why this priority**: sem limite e mascaramento a feature criaria problemas novos (disco e
vazamento de credencial); é proteção, não valor direto.

**Independent Test**: rodar o metra 22 vezes num diretório de estado vazio e conferir que restam
20 arquivos; registrar uma URL com usuário e token e conferir que o token não aparece no arquivo;
rodar com o diretório de estado apontando para um local sem permissão e conferir a mensagem na
barra de status e o pull funcionando.

**Acceptance Scenarios**:

1. **Given** 20 arquivos de log de execuções anteriores, **When** uma nova execução começa,
   **Then** o mais antigo é removido e restam 20 arquivos, incluindo o da execução atual.
2. **Given** uma saída do git contendo `https://usuario:token@host/grupo/repo.git`, **When** ela é
   registrada, **Then** o arquivo contém `https://***@host/grupo/repo.git` e não contém o token.
3. **Given** o diretório de estado inacessível, **When** o metra inicia, **Then** a TUI abre, a
   barra de status mostra `log indisponível: <motivo>`, o pull/push roda normalmente e o stderr
   não mostra a linha `metra: log em`.
4. **Given** outros arquivos no diretório de logs que não seguem o padrão de nome do metra,
   **When** a retenção remove arquivos antigos, **Then** esses outros arquivos ficam intactos.

---

### Edge Cases

- `XDG_STATE_HOME` vazio ou relativo: o diretório usado é `~/.local/state/metra/`.
- `XDG_STATE_HOME` e `HOME` ausentes (ou `HOME` vazio): não há log; a barra de status mostra
  `log indisponível: HOME não definido`.
- Diretório de logs inexistente: é criado, incluindo os diretórios intermediários.
- Falha ao gravar uma entrada no meio da execução (ex.: disco cheio): a entrada é perdida em
  silêncio; o app continua.
- Duas execuções no mesmo segundo: os nomes diferem pelo identificador do processo; nenhuma
  sobrescreve a outra.
- Falha ao remover um arquivo antigo: ignorada; a execução continua e cria o próprio log.
- URL `scp-like` (`git@host:grupo/repo.git`) e `@` no caminho da URL (`https://host/a@b`): não são
  credenciais e ficam inalterados; `ssh://git@host` tem o usuário mascarado.
- Saída do git com várias linhas: cada linha após a primeira aparece recuada, para que o log
  continue legível e cada entrada comece com data e nível.
- Comando git que nem chega a iniciar (git ausente) ou que é encerrado por sinal (aborto): o log
  registra o erro de execução ou "encerrado por sinal".
- Comando que não chega a rodar porque o aborto já estava em andamento: registrado como "não
  iniciado".
- Execução encerrada por `exit` sem passar por destrutores: entradas já registradas estão no
  arquivo, porque cada entrada é gravada imediatamente.

## Requirements *(mandatory)*

### Functional Requirements

**Local e retenção**

- **FR-001**: Cada execução do metra MUST criar um arquivo de log próprio em
  `$XDG_STATE_HOME/metra/` quando `XDG_STATE_HOME` estiver definido, não vazio e for caminho
  absoluto; senão, em `$HOME/.local/state/metra/`; sem `HOME` utilizável, não há log.
- **FR-002**: O nome do arquivo MUST ser `metra-<AAAAMMDD>-<HHMMSS>-<pid>.log`, com data e hora em
  UTC do início da execução, de modo que a ordem alfabética seja a ordem cronológica.
- **FR-003**: Ao iniciar, o metra MUST remover os arquivos mais antigos que seguem o padrão de nome
  `metra-*.log` até que, somados ao novo, restem no máximo 20; arquivos fora do padrão MUST NOT ser
  removidos.
- **FR-004**: O registro MUST estar sempre ativo, sem opção de linha de comando.

**Formato**

- **FR-005**: Cada entrada MUST começar com data/hora UTC com milissegundos
  (`AAAA-MM-DDTHH:MM:SS.mmmZ`), o nível (`INFO`, `AVISO`, `ERRO`), o nome da thread entre colchetes
  e a origem (parte do metra que registrou), seguidos da mensagem.
- **FR-006**: Mensagens com várias linhas MUST ter cada linha a partir da segunda recuada com
  quatro espaços; quebras de linha finais são descartadas.
- **FR-007**: Credenciais em URLs MUST ser mascaradas em toda entrada: o trecho entre `://` e `@`
  (quando não houver `/`, aspas ou espaço antes do `@`) é substituído por `***`.
- **FR-008**: Cada entrada MUST ser gravada no arquivo imediatamente, sem depender do término
  normal do processo.

**Conteúdo**

- **FR-009**: O início MUST registrar a versão do metra, a operação, o remote, o número de jobs, a
  fonte e a versão do git (ou "indisponível").
- **FR-010**: A descoberta MUST registrar o diretório atual, a raiz, a quantidade de repositórios e
  uma entrada por repositório com nome, caminho e situação; em erro de descoberta, a mensagem de
  erro em nível ERRO.
- **FR-011**: Cada comando git MUST registrar o repositório, os argumentos, o código de saída (ou
  "encerrado por sinal"), a duração em milissegundos e a saída completa (stdout+stderr), em nível
  INFO quando bem-sucedido e AVISO quando falhar; erro ao iniciar o processo em nível ERRO; comando
  não iniciado por aborto em nível AVISO.
- **FR-012**: Cada tarefa MUST registrar o início (repositório, caminho, operação, remote, uso de
  `--no-rebase`), se o ssh roda em modo não interativo, e o fim com estado final, branch e link de
  merge; o fim de tarefa com falha em nível AVISO.
- **FR-013**: Cada ação do usuário na TUI MUST ser registrada; falhas das integrações herdr e
  navegador MUST ser registradas em nível AVISO com a mesma mensagem da barra de status.
- **FR-014**: O pedido de saída, o descarte de tarefas pendentes, o aborto, o número de grupos de
  processos encerrados, um trabalhador encerrado por panic e o término de todos os trabalhadores
  MUST ser registrados.
- **FR-015**: Panics em qualquer thread MUST ser registrados em nível ERRO com a mensagem, o local e
  o backtrace, antes de o terminal ser restaurado.
- **FR-016**: O fim MUST registrar o código de saída, a duração total, a linha-resumo e as linhas de
  link de merge; erros fatais MUST ser registrados em nível ERRO.

**Avisos ao usuário**

- **FR-017**: Quando o código de saída for 1 ou 2 e o log existir, o stderr MUST receber
  `metra: log em <caminho>` depois das demais mensagens; com código 0, nada muda na saída.
- **FR-018**: O stdout MUST continuar igual ao de antes da feature.
- **FR-019**: Se o log não puder ser criado, o metra MUST continuar normalmente e mostrar
  `log indisponível: <motivo>` na barra de status; falhas ao gravar entradas MUST NOT interromper
  nem travar o app.

### Key Entities

- **Arquivo de log**: um por execução; nome com data/hora UTC e identificador do processo; vive no
  diretório de estado do usuário; no máximo 20 retidos.
- **Entrada de log**: data/hora UTC, nível, thread, origem e mensagem (possivelmente com várias
  linhas recuadas), já com credenciais mascaradas.
- **Nível**: Info, Aviso ou Erro.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Para qualquer repositório com falha, o arquivo de log contém o comando executado, o
  código de saída e a saída completa do git, sem que o usuário precise reproduzir a execução.
- **SC-002**: Após qualquer número de execuções, o diretório de logs contém no máximo 20 arquivos
  do metra.
- **SC-003**: Nenhum token presente em URLs `https://usuario:token@host` aparece em texto claro nos
  arquivos de log.
- **SC-004**: Com o diretório de logs inacessível, 100% das execuções de pull/push completam com o
  mesmo resultado que teriam com o log ativo.
- **SC-005**: Em execuções com código 1 ou 2, o usuário localiza o arquivo de log da execução sem
  procurar, a partir da mensagem no stderr.
- **SC-006**: Todo panic deixa no log a mensagem, a thread e o backtrace.

## Assumptions

- Horários em UTC (sufixo `Z`), sem conversão para o fuso local: o nome dos arquivos ordena
  cronologicamente e não há dependência nova.
- O caminho do log só aparece no stderr em execuções com código 1 ou 2; em código 0 o usuário pode
  achá-lo no diretório de estado.
- A saída completa do git é registrada (não só o resumo); o volume é limitado pela retenção de 20
  execuções.
- Mascaramento cobre credenciais em URLs; segredos em outros formatos (ex.: cabeçalhos impressos
  pelo git com `GIT_TRACE`) estão fora do escopo.
- Apenas Linux/ambientes XDG são considerados, como no restante do metra.
- Nenhuma dependência nova é adicionada (constituição, princípio VI).
