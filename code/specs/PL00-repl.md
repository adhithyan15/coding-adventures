# PL00 — Generic REPL Framework

## Overview

A **REPL** (Read-Eval-Print Loop) is the oldest and most natural programming
interface ever invented. You type something. The computer evaluates it. It prints
the result. You type again. This cycle — reading, evaluating, printing, looping —
is so fundamental that it predates the word "terminal."

The first REPL was the Lisp interpreter, developed at MIT around 1960. John
McCarthy's original Lisp manual described it as an "eval" function that could be
handed to itself. Within a few years, virtually every interactive language had
one. BASIC, introduced at Dartmouth in 1964, was arguably the first REPL designed
for ordinary people rather than computer scientists.

This package provides a **generic, pluggable REPL framework**. Any language
implemented in this codebase — or any program at all — can register itself with
the framework and immediately gain an interactive session. The framework owns the
loop. The language, the prompt, and the waiting experience are all pluggable
independently.

## Why a Shared Framework?

Without a shared framework, every interactive program reinvents the same
scaffolding: reading input, displaying a prompt, running something, printing
the result, handling errors without crashing. This package factors that out once.

Every language — BASIC, Brainfuck, a future Lisp — gets it for free by
implementing three small interfaces.

## Design

The framework is intentionally minimal. It owns exactly four things:

```
READ   → show prompt, read one line of input
EVAL   → call the language's evaluator (sync or async depending on mode)
WAIT   → in async mode: run the waiting plugin until eval completes
PRINT  → stop waiting, display the result
LOOP   → go back to READ
```

Three pieces are pluggable, each independently:

```
┌─────────────────────────────────────────────────────────────┐
│                      REPL Framework                         │
│                                                             │
│  ┌─────────────────┐  ┌────────────────┐  ┌─────────────┐ │
│  │ Language Plugin │  │ Prompt Plugin  │  │Waiting Plugin│ │
│  │                 │  │                │  │  (async only)│ │
│  │ eval(input)     │  │ global_prompt()│  │ start()     │ │
│  │   → result      │  │ line_prompt()  │  │ tick(state) │ │
│  │                 │  │                │  │ tick_ms()   │ │
│  │                 │  │                │  │ stop(state) │ │
│  └─────────────────┘  └────────────────┘  └─────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

## Execution Modes

The framework supports two execution modes, selected at startup:

### Sync Mode

Eval is called directly and blocks until it returns. No background thread is
spawned. The waiting plugin is not used — it is not even required as a parameter.

```
READ → EVAL (blocks) → PRINT → LOOP
```

Sync mode is simpler and universally portable. It is the only mode available in
languages without native threading in their standard library.

### Async Mode

Eval is fired in a background thread/task/goroutine. The waiting plugin runs on
the main thread and ticks until eval completes.

```
READ → fire EVAL async → WAIT (ticking) → PRINT → LOOP
```

Async mode enables rich waiting experiences: spinners, word lists, games.

### Mode Support by Language

Not every language can support async mode. The framework is honest about this:
requesting async mode in a language that cannot support it raises a configuration
error **at startup**, before any input is read. The session never silently
degrades.

| Language   | Sync | Async | Reason if async unsupported |
|------------|------|-------|-----------------------------|
| Elixir     | ✓    | ✓     | `Task.async` available      |
| Python     | ✓    | ✓     | `threading.Thread` available |
| TypeScript | ✓    | ✓     | `Promise` + `setInterval`   |
| Ruby       | ✓    | ✓     | `Thread` available          |
| Go         | ✓    | ✓     | goroutines + channels       |
| Rust       | ✓    | ✓     | `std::thread` + `mpsc`      |
| Lua        | ✓    | ✗     | No stdlib threads in Lua 5.4 |
| Perl       | ✓    | ✗     | `threads` module not in core |

If async is requested in Lua or Perl, the framework raises an error immediately:

```
Error: async mode is not supported in this implementation.
Use mode: :sync instead.
```

The default mode is **async** for languages that support it, **sync** for those
that do not. This means existing code that does not specify a mode gets the best
available behaviour for its platform.

## The Three Interfaces

### Language Plugin

The language plugin is what does the real work. The framework calls `eval` with
a string and expects one of three outcomes:

```
eval(input: String) →
    {ok, output: String | nil}   — success; output is printed (nil = print nothing)
    {error, message: String}     — recoverable error; loop continues
    quit                         — end the session
```

`eval` is called **asynchronously** — the framework fires it in a background
thread/task/goroutine and hands control to the waiting plugin while it runs.
From the language plugin's perspective, `eval` is just a normal synchronous
function. The async wrapping is the framework's responsibility.

### Prompt Plugin

The prompt plugin controls what the user sees before they type. Two prompts:

```
global_prompt() → String    shown at the start of each new input
line_prompt()   → String    shown for continuation lines (incomplete input)
```

Examples:
- BASIC: `global_prompt = "READY\n> "`, `line_prompt = "    > "`
- Generic: `global_prompt = "> "`, `line_prompt = "... "`
- Branded: `global_prompt = "myapp v1.0\n→ "`, `line_prompt = "→ "`

### Waiting Plugin

The waiting plugin runs while eval is in flight. It owns the terminal during
that time. Four functions:

```
start()               → waiting_state        called once when eval fires
tick(waiting_state)   → waiting_state        called every tick_ms milliseconds
tick_ms()             → Integer              how often to tick (e.g., 100)
stop(waiting_state)   → void                 called when eval completes
```

`start` and `stop` let the plugin set up and tear down any terminal state
(e.g., hide the cursor, restore it). `tick` is the animation frame. The waiting
plugin can be as simple or as rich as desired:

| Plugin | What it does |
|--------|-------------|
| `Silent` | Does nothing. tick_ms = 1000. The terminal just waits. |
| `Spinner` | Rotates `\|` `/` `—` `\` on each tick. |
| `Words` | Prints a random word each tick. Like Claude Code. |
| `Dots` | Appends `.` up to three, then resets. `   ` → `.  ` → `.. ` → `...` |
| `SpaceInvaders` | Full playable game. Eval finishing is your escape condition. |

The waiting plugin and the language plugin know nothing about each other.
Any waiting plugin works with any language.

## The Loop

**Sync mode:**
```
loop:
    print global_prompt()
    input ← read_line()
    if input is empty: continue

    result ← language.eval(input)   # blocks

    match result:
        {ok, nil}      → continue
        {ok, output}   → print(output)
        {error, msg}   → print("ERROR: " + msg)
        quit           → exit loop
```

**Async mode:**
```
loop:
    print global_prompt()
    input ← read_line()
    if input is empty: continue

    future ← async { language.eval(input) }
    waiting_state ← waiting.start()

    loop:
        if future is done: break
        sleep(waiting.tick_ms())
        waiting_state ← waiting.tick(waiting_state)

    waiting.stop(waiting_state)
    result ← future.result()

    match result:
        {ok, nil}      → continue
        {ok, output}   → print(output)
        {error, msg}   → print("ERROR: " + msg)
        quit           → exit loop
```

In both modes, if `eval` raises an unexpected exception (a bug in the plugin),
the framework catches it, prints a generic error, and continues. The session
never crashes due to a language error.

## Built-in Implementations

Each language package ships three ready-to-use implementations:

### EchoLanguage

The simplest possible language plugin: returns whatever it receives. Used as the
canonical test for the framework itself.

```
eval("hello")        → {ok, "hello"}
eval(":quit")        → quit
eval(anything else)  → {ok, that thing}
```

If the echo REPL works, the loop, prompt, async firing, waiting, print, and
quit paths are all verified.

### DefaultPrompt

```
global_prompt() → "> "
line_prompt()   → "... "
```

### SilentWaiting

Does nothing. `start` and `stop` are no-ops. `tick` is a no-op. `tick_ms` = 100.
Used when you want zero visual noise while waiting.

## I/O Injection

The framework never calls `stdin`/`stdout` directly. `read_line` and `print` are
injected at construction time:

```
Repl.new(
    language:  my_language,
    prompt:    my_prompt,
    waiting:   my_waiting,          # omit or nil in sync mode
    mode:      :async | :sync,      # default: :async if supported, else :sync
    input_fn:  fn → String,         # default: read from stdin
    output_fn: String → void        # default: print to stdout
)
```

In tests, `input_fn` returns values from a pre-loaded list. `output_fn` appends
to a capture buffer. Execution is fully deterministic — no real terminal needed.

This is the same pattern used by the `GenericVM`'s output list and the Brainfuck
VM's `:input_buffer`.

## Layer Position

```
User (keyboard / test harness)
          ↓
  [REPL Framework]   ← this package
          ↓
  [Language Plugin]  ← e.g. BASIC, Brainfuck, Echo
          ↓
  [Lexer → Parser → Compiler → VM]  (or direct interpretation)
          ↓
       output
```

## Package Details

| Language   | Package/Module                         | No dependencies |
|------------|----------------------------------------|-----------------|
| Elixir     | `CodingAdventures.Repl`                | none            |
| Python     | `coding_adventures_repl`               | none            |
| TypeScript | `@coding-adventures/repl`              | none            |
| Ruby       | `CodingAdventures::Repl`               | none            |
| Go         | `github.com/.../go/repl`               | none            |
| Rust       | `repl`                                 | none            |
| Lua        | `coding_adventures.repl`               | none            |
| Perl       | `CodingAdventures::Repl`               | none            |

The REPL framework has **no dependencies** on any other package in this codebase.
It is pure language, pure stdlib. This keeps it universally composable.

## Test Strategy

Every language implementation is tested with the EchoLanguage, DefaultPrompt,
and SilentWaiting built-ins. The tests cover:

1. **Basic echo** — input is returned as output
2. **Quit** — `:quit` ends the session
3. **Error handling** — `{error, message}` prints `ERROR: ...` and continues
4. **Nil output** — `{ok, nil}` prints nothing and continues
5. **Multiple turns** — a sequence of inputs produces the expected outputs in order
6. **Exception safety** — if eval raises, the loop continues with an error message
7. **Sync mode** — all of the above with `mode: :sync`
8. **Async mode rejection** — Lua and Perl: requesting async raises at startup

## Future Extensions

- **Tab completion** — an optional `complete(partial) → [String]` callback on
  the language plugin for context-aware completions.
- **Syntax highlighting** — an optional `highlight(input) → String` callback for
  colorizing input as it is typed.
- **Persistent history** — save/restore history to a file between sessions.
- **Remote REPL** — run the framework over a TCP socket.
- **Web REPL** — expose as a WebSocket endpoint for browser-based tutorials.
- **Multi-line input** — an optional `needs_more?(partial) → bool` callback on
  the language plugin; the framework shows `line_prompt()` and accumulates input
  until the language signals complete.
