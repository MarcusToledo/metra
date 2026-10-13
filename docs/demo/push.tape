# Grava docs/assets/push.gif. Na raiz do repositório:
#   cargo build --release && vhs docs/demo/push.tape
Output docs/assets/push.gif

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
Type "export PATH=$PWD/target/release:$PATH XDG_STATE_HOME=/tmp/metra-demo-state PS1='\[\e[1;36m\]~/loja \[\e[0m\]$ ' && bash docs/demo/workspace.sh >/dev/null && cd /tmp/metra-demo/push && clear"
Enter
Sleep 12s
Show

Sleep 500ms
Type "metra push"
Sleep 400ms
Enter
Sleep 4s

# Push rejeitado.
Enter
Sleep 2.5s
Escape
Sleep 500ms

# Enviado com link de merge anunciado pelo remote.
Type "j"
Sleep 400ms
Enter
Sleep 3s
Escape
Sleep 500ms

Type "q"
Sleep 3.5s
