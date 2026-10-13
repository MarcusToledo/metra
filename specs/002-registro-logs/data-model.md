# Data Model: Registro de logs de execução do metra

## NivelEnum

| Variante | Rótulo | Uso |
|---|---|---|
| `Info` | `INFO` | fluxo normal: contexto, comandos bem-sucedidos, ações, fim |
| `Aviso` | `AVISO` | comando git com código ≠ 0, tarefa com falha, comando não iniciado, aborto, falha de integração |
| `Erro` | `ERRO` | erro de descoberta, erro ao iniciar git, panic, trabalhador com panic, erro fatal do laço |

`Display` usa `f.pad`, para que `{:<5}` alinhe os rótulos.

## Registro

| Campo | Tipo | Regra |
|---|---|---|
| `arquivo` | `Mutex<File>` | aberto com `create_new`; sem buffer; uma escrita por entrada |
| `caminho` | `PathBuf` | `<diretorio>/metra-AAAAMMDD-HHMMSS-<pid>.log` |

- Criação: `create_dir_all(diretorio)` → `podar(diretorio, 19)` (erro ignorado) → abrir arquivo.
- Escrita: erros de escrita e mutex envenenado são ignorados.
- Instância global única (`OnceLock`): `iniciar` só tem sucesso uma vez por processo.

## Entrada

```text
<AAAA-MM-DDTHH:MM:SS.mmmZ> <NIVEL:<5> [<thread>] <origem>: <linha 1>
    <linha 2>
    ...
```

- Mensagem mascarada (`mascarar_credenciais`) e sem `\n` finais antes de dividir.
- Mensagem vazia: só o cabeçalho, terminando em `": \n"`.
- Thread sem nome: `sem-nome`.

## Arquivo de log (no disco)

- Diretório: ver [contracts/registro.md](contracts/registro.md#diretório).
- Ciclo de vida: criado no início da execução; nunca reaberto; removido por uma execução futura
  quando houver mais de 19 arquivos `metra-*.log` mais recentes.
- Arquivos que não começam com `metra-` ou não terminam em `.log`, e diretórios, nunca são
  removidos.

## Relações

- `executor`, `pool` e `main` → `registro::registrar` (dependência de mão única; `registro` não
  importa nada do crate).
- `main` → `registro::iniciar`, `instalar_hook_panic`, `caminho_atual`, `diretorio_padrao`;
  `executor::versao_git`.
