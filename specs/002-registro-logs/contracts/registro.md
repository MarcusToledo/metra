# Contrato: arquivo de log de execução

## Diretório

| Ambiente | Diretório |
|---|---|
| `XDG_STATE_HOME` não vazio e absoluto | `$XDG_STATE_HOME/metra/` |
| `XDG_STATE_HOME` ausente, vazio ou relativo, `HOME` não vazio | `$HOME/.local/state/metra/` |
| nenhum dos dois utilizável | sem log; status `log indisponível: HOME não definido` |

O diretório é criado com os intermediários, se não existir.

## Nome do arquivo

`metra-AAAAMMDD-HHMMSS-<pid>.log`, com data e hora UTC do início da execução e o PID do processo.
Exemplo: `metra-20261013-132405-4242.log`.

## Retenção

Ao iniciar, antes de criar o arquivo novo, os arquivos regulares cujo nome começa com `metra-` e
termina com `.log` são ordenados pelo nome; os mais antigos são removidos até restarem 19. Com o
novo, ficam no máximo 20. Falha ao remover um arquivo é ignorada; outros arquivos e diretórios não
são tocados.

## Formato da entrada

```text
<AAAA-MM-DDTHH:MM:SS.mmmZ> <NIVEL> [<thread>] <origem>: <linha 1>
    <linha 2>
```

- `<NIVEL>`: `INFO`, `AVISO` ou `ERRO`, alinhado à esquerda em 5 colunas (`INFO `, `ERRO `).
- `<thread>`: nome da thread (`main`, `trabalhador-N`, `pool`) ou `sem-nome`.
- Linhas a partir da segunda: recuo de 4 espaços. `\n` finais descartados.
- Credenciais: em cada `://`, o trecho até um `@` (sem `/`, `'`, `"` ou espaço antes dele) vira
  `***`. Ex.: `https://usuario:token@host/x.git` → `https://***@host/x.git`;
  `git@host:grupo/repo.git` e `https://host/a@b` ficam iguais.
- Cada entrada é gravada no disco imediatamente.

Exemplo:

```text
2026-10-13T13:24:05.120Z INFO  [main] main: metra 0.1.0 Pull remote=upstream jobs=8 fonte=Auto
2026-10-13T13:24:05.131Z INFO  [trabalhador-0] executor: /w/bom $ git pull upstream main -> código 1 em 12 ms
    fatal: 'upstream' does not appear to be a git repository
```

## Origens e mensagens

| Origem | Nível | Mensagem |
|---|---|---|
| `main` | INFO | `metra <versão> <operação:?> remote=<remote> jobs=<n> fonte=<fonte:?>` |
| `main` | INFO | `git: <saída de git --version>` ou `git: indisponível` |
| `fonte` | INFO | `cwd=<cwd> raiz=<raiz> repos=<n>` |
| `fonte` | INFO | `<nome> <caminho> <situação:?>` (uma por repositório) |
| `fonte` | ERRO | `<erro de descoberta>` |
| `executor` | INFO | `início <nome> (<caminho>) operação=<op:?> remote=<remote> sem_rebase=<bool>` |
| `executor` | INFO | `<nome>: ssh não interativo=<bool>` |
| `executor` | INFO / AVISO | `<caminho> $ git <args> -> código <N> em <ms> ms` ou `-> encerrado por sinal em <ms> ms`, seguida da saída completa; AVISO se o código ≠ 0 |
| `executor` | AVISO | `<caminho> $ git <args>: não iniciado, abortado pelo usuário` |
| `executor` | ERRO | `<caminho> $ git <args>: não foi possível executar git: <erro>` |
| `executor` | INFO / AVISO | `fim <nome>: <estado:?> branch=<branch:?> link_merge=<link:?>`; AVISO se `Falhou(_)` |
| `executor` | AVISO | `aborto: matando <n> grupos de processos` |
| `pool` | INFO | `tarefa <indice> (<nome>) descartada: saída em andamento` |
| `pool` | ERRO | `trabalhador terminou com panic` |
| `pool` | INFO | `todos os trabalhadores terminaram` |
| `pool` | INFO | `saída iniciada: tarefas pendentes descartadas` |
| `pool` | AVISO | `aborto solicitado` |
| `app` | INFO | `ação <AcaoEnum:?>` |
| `acoes` | AVISO | mensagem de falha do herdr ou do navegador (a mesma da barra de status) |
| `panic` | ERRO | `<PanicHookInfo>` seguido do backtrace |
| `main` | ERRO | `<erro fatal do laço da TUI>` |
| `main` | INFO | `fim código=<c> duração=<s,1 casa>s: <linha-resumo>` |
| `main` | INFO | `merge <repo>: <url>` (uma por link) |

## Saída do processo

- stdout: inalterado.
- stderr, código 2 (erro de descoberta): `metra: <erro>` seguido de `metra: log em <caminho>`.
- stderr, código 1 (falhas ou erro fatal): depois das mensagens existentes,
  `metra: log em <caminho>`.
- Código 0, ou log indisponível: nenhuma linha nova no stderr.

## Barra de status

- Log não criado: `log indisponível: <motivo>` (ex.: `log indisponível: HOME não definido`,
  `log indisponível: Permission denied (os error 13)`), exibida ao abrir a TUI.
