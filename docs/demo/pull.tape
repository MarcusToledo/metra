# Grava docs/assets/pull.gif. Na raiz do repositório:
#   cargo build --release && vhs docs/demo/pull.tape
# NO_COLOR herdado do ambiente desligaria as cores da TUI; o tape o remove.
Output docs/assets/pull.gif

Require git
Set Shell "bash"
Set FontSize 16
Set Width 1200
Set Height 620
Set Padding 20
Set Theme "Catppuccin Mocha"
Set TypingSpeed 60ms
Set WindowBar Colorful

Hide
Type "unset NO_COLOR && export PATH=$PWD/target/release:$PATH XDG_STATE_HOME=/tmp/metra-demo-state PS1='\[\e[1;36m\]~/loja \[\e[0m\]$ ' && bash docs/demo/workspace.sh >/dev/null && cd /tmp/metra-demo/pull && clear"
Enter
Sleep 12s
Show

Sleep 500ms
Type "metra pull"
Sleep 400ms
Enter
Sleep 5s

# Branches divergentes: abre o detalhe e repete com --no-rebase.
Enter
Sleep 2s
Type "n"
Sleep 2.5s
Escape
Sleep 800ms

# Conflito: o rerun reordena a lista e a seleção cai em frontend.
Enter
Sleep 1.5s
Screenshot docs/assets/detalhe.png
Sleep 1.5s
Escape
Sleep 500ms

# Linhas agregadas listam os nomes.
Type "jjjjjj"
Sleep 400ms
Enter
Sleep 2s
Escape
Sleep 500ms

Type "q"
Sleep 3s
