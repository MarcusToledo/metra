# Data Model: metra pull/push

## Repo (`fonte::Repo`)

| Campo | Tipo | Regra |
|---|---|---|
| `nome` | `String` | Chave do `.meta` (caminho relativo, pode conter `/`) ou nome da subpasta |
| `caminho` | `PathBuf` | `raiz.join(nome)`, absoluto |
| `situacao` | `SituacaoRepoEnum` | Calculada na descoberta (abaixo) |

`SituacaoRepoEnum`:

- `Clonado` — `caminho/.git` existe (diretório ou arquivo).
- `NaoClonado` — `caminho` não existe.
- `SemRepositorio` — `caminho` existe sem `.git` próprio.

Só `Clonado` gera `Tarefa`.

## FonteEnum / FonteRepos

- `FonteEnum { Auto, Meta, Varredura }` — valor de `--fonte`.
- `FonteRepos::listar() -> Result<Vec<Repo>>` — implementações `FonteMeta { raiz, arquivo }`
  (ordem do arquivo) e `FonteVarredura { raiz }` (subpastas não ocultas com `.git`, ordenadas).

## Tarefa (`executor::Tarefa`)

| Campo | Tipo | Regra |
|---|---|---|
| `indice` | `usize` | Posição do repo no vetor do `App` |
| `repo` | `Repo` | Sempre `Clonado` |
| `operacao` | `OperacaoEnum { Pull, Push }` | Do subcomando |
| `remote` | `String` | `--remote` ou padrão (`upstream`/`origin`) |
| `sem_rebase` | `bool` | `true` só via tecla `n` no pull |

## Resultado (`executor::Resultado`)

| Campo | Tipo |
|---|---|
| `estado` | `EstadoRepoEnum` (nunca `Pendente`/`Executando`) |
| `branch` | `Option<String>` (`None` se o `rev-parse` falhou) |
| `log` | `String` — stdout+stderr de todos os comandos da tentativa, com o comando como cabeçalho |

## EstadoRepoEnum

`Pendente`, `Executando`, `SemMudanca`, `Recebido(u32)`, `Enviado`, `Falhou(TipoErroEnum)`.

```mermaid
stateDiagram-v2
    [*] --> Pendente: descoberta (Clonado)
    Pendente --> Executando: Evento::Iniciou
    Executando --> SemMudanca: Terminou
    Executando --> Recebido: Terminou (pull)
    Executando --> Enviado: Terminou (push)
    Executando --> Falhou: Terminou
    Falhou --> Pendente: r / n / R
    SemMudanca --> Pendente: r
    Recebido --> Pendente: r
    Enviado --> Pendente: r
```

Reexecução só é aceita para estados terminais e antes de `q`.

## TipoErroEnum (`classifica`)

| Variante | `texto()` (lista) | `dica()` (detalhe) |
|---|---|---|
| `MergePendente` | merge pendente | resolva manualmente: [p] abre pane no repo |
| `Conflito(Vec<String>)` | conflito em a.txt, b.txt | resolva manualmente: [p] abre pane no repo |
| `Divergente` | branches divergentes | use [n] para merge com --no-rebase |
| `AlteracoesLocais` | alterações locais seriam sobrescritas | resolva manualmente: [p] abre pane no repo |
| `ArquivosNaoRastreados` | arquivos não rastreados seriam sobrescritos | resolva manualmente: [p] abre pane no repo |
| `BranchRemotaInexistente` | branch não existe no remote | veja o log |
| `RemoteInexistente` | remote inexistente | veja o log |
| `PushRejeitado` | push rejeitado | veja o log |
| `RedeOuAutenticacao` | falha de rede ou autenticação | veja o log |
| `HeadDestacado` | HEAD destacado | veja o log |
| `Desconhecido(String)` | a própria linha | veja o log |

Regras de `classificar(texto)` e sua ordem: spec FR-013.

## Evento (`pool::Evento`)

`Iniciou(usize)`, `Terminou(usize, Resultado)`, `PoolEncerrado`.

## App (`app::App`)

| Campo | Significado |
|---|---|
| `operacao`, `remote`, `raiz` | Cabeçalho e criação de tarefas |
| `repos: Vec<Repo>` | Ordem da fonte |
| `estados: Vec<EstadoRepoEnum>`, `logs: Vec<String>`, `branches: Vec<Option<String>>` | Por índice; não Clonado não tem estado de execução |
| `selecao: usize` | Índice em `linhas()` |
| `detalhe_aberto: bool`, `rolagem: u16` | Painel de detalhe |
| `status: Option<String>` | Barra de status |
| `saindo: bool` | `q` pressionado; ações desabilitadas |
| `encerrado: bool` | `PoolEncerrado` recebido; loop termina |

`LinhaEnum`: `Repo(usize)` para Falhou/Recebido/Enviado/Executando/Pendente;
`SemMudanca(Vec<usize>)`, `NaoClonados(Vec<usize>)`, `SemRepositorio(Vec<usize>)` agregadas
(omitidas quando vazias).

Resumo: contam só estados terminais; `Pendente`/`Executando` na saída não entram em grupo.
`codigo_saida()`: 1 se existir `Falhou`, senão 0.
