# Research: Registro de logs de execução do metra

## R1. Local dos arquivos

- **Decision**: `$XDG_STATE_HOME/metra/` quando a variável é não vazia e absoluta; senão
  `$HOME/.local/state/metra/`; sem `HOME` não vazio, sem log.
- **Rationale**: a XDG Base Directory Specification define `XDG_STATE_HOME` para dados que
  persistem entre execuções mas não são importantes o bastante para `XDG_DATA_HOME`, citando logs
  e histórico; valores relativos MUST ser ignorados pela especificação.
- **Alternatives considered**: `/tmp` (apagado no boot, compartilhado entre usuários);
  `XDG_CACHE_HOME` (semântica de dado regenerável); diretório do workspace (polui repositórios).

## R2. Um arquivo por execução, retenção de 20

- **Decision**: nome `metra-AAAAMMDD-HHMMSS-<pid>.log`; ao iniciar, remove os mais antigos pelo
  nome até restarem 19 e cria o novo com `create_new`.
- **Rationale**: ordenar pelo nome dispensa ler metadados; o pid evita colisão entre execuções no
  mesmo segundo; `create_new` nunca sobrescreve outro log.
- **Alternatives considered**: arquivo único com rotação por tamanho (mistura execuções e exige
  lock entre processos); ordenar por `mtime` (sensível a cópias e relógio).

## R3. Horário UTC sem dependência

- **Decision**: segundos desde a época via `SystemTime::duration_since(UNIX_EPOCH)`; data por
  `civil_from_days` (Howard Hinnant, "chrono-Compatible Low-Level Date Algorithms"); formato
  `AAAA-MM-DDTHH:MM:SS.mmmZ`.
- **Rationale**: a std não converte datas; fuso local exigiria `localtime_r` da libc (FFI) ou
  `chrono`/`time` (vetado pelo princípio VI). UTC também mantém a ordem do nome monotônica em
  mudanças de horário de verão.
- **Alternatives considered**: `chrono`/`time` (dependência nova); `localtime_r` via FFI (mais
  `unsafe` e dependência de `TZ`); segundos da época crus (ilegível).

## R4. Escrita sem buffer e saída por `exit`

- **Decision**: `File` sem `BufWriter`, uma chamada `write_all` por entrada sob `Mutex`.
- **Rationale**: `main` termina com `std::process::exit`, que não roda destrutores; um buffer
  perderia o fim do log e o panic. Uma escrita por entrada sob o lock mantém entradas inteiras com
  várias threads.
- **Alternatives considered**: `BufWriter` com `flush` explícito em cada ponto de saída (frágil,
  perde o panic); canal para uma thread escritora (complexidade sem ganho no volume do metra).

## R5. Registro global

- **Decision**: `static GLOBAL: OnceLock<Registro>`; `registrar` é no-op se não iniciado.
- **Rationale**: evita passar um handle por `Pool`, `executor::executar` e `App`, cuja API pública
  é usada pelos testes existentes; testes unitários e de integração que não iniciam o registro
  continuam sem efeito colateral. Em `tests/registro.rs`, por ser um binário de teste próprio, o
  global é iniciado uma vez por processo.
- **Alternatives considered**: parâmetro `&Registro` em todas as APIs (quebra assinaturas e
  testes); crate `log` + implementação própria (dependência nova).

## R6. Panic com backtrace e ordem dos hooks

- **Decision**: `instalar_hook_panic()` guarda `take_hook()` e instala um hook que registra
  `"{info}\n{Backtrace::force_capture()}"` em nível Erro e depois chama o anterior; instalado
  antes de `ratatui::init()`.
- **Rationale**: a documentação do ratatui 0.30 (`src/init.rs`) orienta chamar `init` depois de
  instalar outros hooks, para que o hook do ratatui restaure o terminal antes deles;
  `force_capture` ignora `RUST_BACKTRACE`, garantindo o backtrace no arquivo. O panic é gravado de
  forma síncrona, antes de o processo continuar.
- **Alternatives considered**: `Backtrace::capture()` (depende de `RUST_BACKTRACE`); instalar
  depois do `init` (ordem invertida em relação à documentação).

## R7. Mascaramento de credenciais

- **Decision**: para cada `://`, procurar o primeiro de `/`, `@`, `'`, `"` ou espaço; se for `@`
  com texto antes, trocar esse texto por `***`.
- **Rationale**: cobre `https://usuario:token@host`, `https://token@host` (como o git imprime em
  `fatal: unable to access '...'`) e `ssh://git@host`, sem tocar em `git@host:grupo/repo` (sem
  `://`) nem em `@` no caminho (o `/` vem antes).
- **Alternatives considered**: regex (dependência nova); mascarar só no `remote -v` (as URLs
  também aparecem em mensagens de erro do git).

## R8. Pontos de registro

- **Decision**: cada comando no `git()` privado do executor (repositório, argumentos, código,
  duração, saída); início/fim de tarefa em `executar()`; nomes de threads no pool para agrupar as
  linhas por trabalhador; ações vindas de `App::tratar_tecla` no laço do `main`; contexto e
  resumo no `main`.
- **Rationale**: o `git()` já concentra todo processo git das tarefas; o `App` continua sem I/O
  (princípio II) porque o registro das ações acontece no `main`.
- **Alternatives considered**: registrar dentro do `App` (viola o princípio II); registrar só o
  log consolidado no fim da tarefa (perde duração por comando e o que estava rodando num
  travamento).
