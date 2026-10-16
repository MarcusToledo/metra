# Grava docs/assets/herdr.gif. Na raiz do repositório:
#   cargo build --release && vhs docs/demo/herdr.tape && herdr session stop metra-demo
# Roda o metra dentro de uma sessão herdr isolada (metra-demo) com docs/demo/herdr.toml.
# NO_COLOR e as variáveis HERDR_* herdadas do ambiente são removidas antes.
Output docs/assets/herdr.gif

Require git
Require herdr
Set Shell "bash"
Set FontSize 15
Set Width 1400
Set Height 800
Set Padding 16
Set Theme "Builtin Solarized Dark"
Set TypingSpeed 60ms
Set WindowBar Colorful

Hide
Type "unset NO_COLOR HERDR_ENV HERDR_PANE_ID HERDR_TAB_ID HERDR_WORKSPACE_ID HERDR_SOCKET_PATH HERDR_BIN_PATH && export PATH=$PWD/target/release:$PATH XDG_STATE_HOME=/tmp/metra-demo-state HERDR_CONFIG_PATH=$PWD/docs/demo/herdr.toml BASH_SILENCE_DEPRECATION_WARNING=1 GIT_PAGER=cat PS1='\[\e[1;36m\]\W \[\e[0m\]$ ' && bash docs/demo/workspace.sh >/dev/null && (herdr session stop metra-demo; herdr session delete metra-demo) >/dev/null 2>&1; cd /tmp/metra-demo/pull && clear && herdr --session metra-demo"
Enter
Sleep 15s
Show

Sleep 800ms
Type "metra pull"
Sleep 400ms
Enter
Sleep 5s

# Conflito em frontend: o detalhe sugere [p], que abre um pane do herdr no repositório.
Type "j"
Sleep 400ms
Enter
Sleep 2.5s
Type "p"
Sleep 2s

# O pane abre sem roubar o foco; prefix+j (Ctrl+B j) desce para ele, já na pasta do frontend.
Ctrl+B
Type "j"
Sleep 800ms
Type "git status --short"
Sleep 300ms
Enter
Sleep 1.5s
Type "git diff"
Sleep 300ms
Enter
Sleep 3.5s

# Volta ao metra (Ctrl+B k) e sai.
Ctrl+B
Type "k"
Sleep 800ms
Escape
Sleep 500ms
Type "q"
Sleep 3s
