# Quickstart: validar o registro de logs

Pré-requisitos: build em `target/debug/metra` (`cargo build`); git ≥ 2.34 sem `pull.rebase` nem
`pull.ff` no escopo global/sistema; terminal real (a TUI precisa de TTY).

## 1. Testes

```sh
cargo test
cargo clippy --all-targets -- -D warnings
```

Esperado: testes unitários de `registro` e `tests/registro.rs` passam; nenhum aviso.

## 2. Execução com falhas (US1, US2)

```sh
W=$(mktemp -d); S=$(mktemp -d)
git init -q -b main "$W/bom" && git -C "$W/bom" commit -q --allow-empty -m base
git init -q -b main "$W/ruim" && git -C "$W/ruim" commit -q --allow-empty -m base
git clone -q "$W/bom" "$W/clone" && git -C "$W/clone" remote rename origin upstream
cd "$W" && XDG_STATE_HOME="$S" /caminho/do/metra/target/debug/metra pull --fonte varredura
```

Na TUI, `clone` termina sem mudança; `bom` e `ruim` falham com remote inexistente. Pressione `q`.

Esperado:
- stdout: `metra pull: 1 sem mudança, 0 recebidos, 2 falhas (bom, ruim), ...`.
- stderr: `metra: log em $S/metra/metra-<data>-<pid>.log`.
- arquivo (formato em [contracts/registro.md](contracts/registro.md)) com:
  `INFO  [main] main: metra 0.1.0 Pull`, `git: git version ...`, `fonte: cwd=`,
  `[trabalhador-N] executor: ... $ git pull upstream main -> código ...` com a saída recuada,
  `fim bom: Falhou(RemoteInexistente)`, `app: ação Sair`, `pool: todos os trabalhadores terminaram`
  e `main: fim código=1`.

## 3. Retenção (US3)

```sh
V=$(mktemp -d); cd "$V"
for i in $(seq 22); do XDG_STATE_HOME="$S" /caminho/do/metra/target/debug/metra pull --fonte varredura; done
ls "$S/metra" | wc -l
```

Esperado: cada execução sai com código 2 e imprime `metra: log em ...`; a contagem final é `20`
(considerando só arquivos `metra-*.log` no diretório).

## 4. Degradação (US3)

No workspace do item 2:

```sh
XDG_STATE_HOME=/proc/metra-sem-permissao HOME=/proc/sem-permissao /caminho/do/metra/target/debug/metra pull --fonte varredura
```

Esperado: a TUI abre com `log indisponível: ...` na barra de status, o pull roda normalmente e,
após `q`, o stderr não mostra `metra: log em`.
