#!/usr/bin/env bash
# Cria workspaces descartáveis para gravar as demos do README (docs/demo/*.tape).
#
#   bash docs/demo/workspace.sh [DESTINO]   # padrão: /tmp/metra-demo
#
# DESTINO/pull: workspace com .meta cobrindo cada resultado do `metra pull`.
# DESTINO/push: workspace (varredura) com envios, link de merge e push rejeitado.
# Cada remote atrasa a resposta para a TUI mostrar tarefas em execução.
set -euo pipefail

destino="${1:-/tmp/metra-demo}"
rm -rf "$destino"
mkdir -p "$destino/remotes"

export GIT_AUTHOR_NAME=demo GIT_AUTHOR_EMAIL=demo@exemplo.dev
export GIT_COMMITTER_NAME=demo GIT_COMMITTER_EMAIL=demo@exemplo.dev
export GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_NOSYSTEM=1

branch=main

commit() { # commit <dir> <arquivo> <conteúdo> <mensagem>
	mkdir -p "$(dirname "$1/$2")"
	printf '%s\n' "$3" >"$1/$2"
	git -C "$1" add "$2"
	git -C "$1" commit -qm "$4"
}

# novo_remote <nome>: repositório bare com um commit inicial; imprime o caminho.
novo_remote() {
	local bare="$destino/remotes/$1.git" semente="$destino/remotes/$1.semente"
	git init -q --bare -b "$branch" "$bare"
	git init -q -b "$branch" "$semente"
	commit "$semente" README.md "# $1" "chore: inicial"
	commit "$semente" src/app.ts "export const versao = 1;" "feat: app"
	git -C "$semente" push -q "$bare" "$branch"
	echo "$bare"
}

# avancar_remote <nome> <n>: publica n commits novos no remote.
avancar_remote() {
	local semente="$destino/remotes/$1.semente" i
	for ((i = 1; i <= $2; i++)); do
		commit "$semente" "src/modulo$i.ts" "export const m$i = $i;" "feat: módulo $i"
	done
	git -C "$semente" push -q "$destino/remotes/$1.git" "$branch"
}

# clonar <workspace> <nome> <remote> <atraso>: clone com remote renomeado e resposta atrasada.
clonar() {
	local alvo="$1/$2"
	git clone -q -o "$3" "$destino/remotes/$2.git" "$alvo"
	git -C "$alvo" config "remote.$3.uploadpack" "sleep $4 && git-upload-pack"
	git -C "$alvo" config "remote.$3.receivepack" "sleep $4 && git-receive-pack"
	git -C "$alvo" config user.name demo
	git -C "$alvo" config user.email demo@exemplo.dev
}

# --- pull -----------------------------------------------------------------
ws="$destino/pull"
mkdir -p "$ws"
cat >"$ws/.meta" <<'JSON'
{
  "projects": {
    "api": "git@exemplo.dev:loja/api.git",
    "auth": "git@exemplo.dev:loja/auth.git",
    "billing": "git@exemplo.dev:loja/billing.git",
    "checkout": "git@exemplo.dev:loja/checkout.git",
    "frontend": "git@exemplo.dev:loja/frontend.git",
    "infra": "git@exemplo.dev:loja/infra.git",
    "mobile": "git@exemplo.dev:loja/mobile.git",
    "notifications": "git@exemplo.dev:loja/notifications.git",
    "search": "git@exemplo.dev:loja/search.git",
    "shared": "git@exemplo.dev:loja/shared.git"
  }
}
JSON

for nome in api auth billing checkout frontend infra mobile notifications; do
	novo_remote "$nome" >/dev/null
done
atraso=(0.8 1.4 2.0 1.1 2.6 1.7 2.3 0.9)
i=0
for nome in api auth billing checkout frontend infra mobile notifications; do
	clonar "$ws" "$nome" upstream "${atraso[$i]}"
	i=$((i + 1))
done

# Recebidos.
avancar_remote billing 3
avancar_remote mobile 1

# Branches divergentes: commits em arquivos distintos nos dois lados.
avancar_remote checkout 2
commit "$ws/checkout" src/carrinho.ts "export const carrinho = [];" "feat: carrinho local"

# Conflito: mesma linha alterada nos dois lados; merge explícito para parar em CONFLICT.
semente="$destino/remotes/frontend.semente"
commit "$semente" src/app.ts "export const versao = 2;" "feat: versão 2"
git -C "$semente" push -q "$destino/remotes/frontend.git" "$branch"
commit "$ws/frontend" src/app.ts "export const versao = 3;" "feat: versão 3"
git -C "$ws/frontend" config pull.rebase false

# Alterações locais seriam sobrescritas.
semente="$destino/remotes/infra.semente"
commit "$semente" README.md "# infra (remoto)" "docs: readme"
git -C "$semente" push -q "$destino/remotes/infra.git" "$branch"
printf '# infra (local)\n' >"$ws/infra/README.md"

# Pasta sem .git próprio; "search" fica sem clonar.
mkdir -p "$ws/shared/build"

# --- push -----------------------------------------------------------------
ws="$destino/push"
mkdir -p "$ws"
for nome in gateway ledger portal reports; do
	novo_remote "$nome" >/dev/null
done
atraso=(1.2 2.2 1.6 0.9)
i=0
for nome in gateway ledger portal reports; do
	clonar "$ws" "$nome" origin "${atraso[$i]}"
	i=$((i + 1))
done

# Remotes anunciam link de merge/pull request como GitLab e GitHub.
cat >"$destino/remotes/gateway.git/hooks/post-receive" <<'SH'
#!/bin/sh
echo
echo "To create a merge request for main, visit:"
echo "  https://gitlab.exemplo.dev/loja/gateway/-/merge_requests/new?merge_request%5Bsource_branch%5D=main"
SH
cat >"$destino/remotes/portal.git/hooks/post-receive" <<'SH'
#!/bin/sh
echo "Create a pull request for 'main' on GitHub by visiting:"
echo "     https://github.com/loja/portal/pull/new/main"
SH
chmod +x "$destino/remotes/"{gateway,portal}.git/hooks/post-receive

commit "$ws/gateway" src/rotas.ts "export const rotas = [];" "feat: rotas"
commit "$ws/portal" src/tema.ts "export const tema = 'escuro';" "feat: tema escuro"
commit "$ws/portal" src/menu.ts "export const menu = [];" "feat: menu"

# Push rejeitado: o remote avançou.
avancar_remote ledger 1
commit "$ws/ledger" src/saldo.ts "export const saldo = 0;" "feat: saldo"

echo "pull: $destino/pull"
echo "push: $destino/push"
