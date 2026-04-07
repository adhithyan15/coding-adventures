# coding-adventures-repl

A pluggable, async-eval **REPL framework** — standard library only.

## What Is a REPL?

REPL stands for **Read–Eval–Print Loop**.  It is the interactive shell model
used by Python (`python -i`), Ruby (`irb`), Node.js (`node`), Elixir (`iex`),
and countless others:

```
1. Read   — print a prompt, read a line of input from the user
2. Eval   — pass that line to the language evaluator
3. Print  — display the result (or error message)
4. Loop   — go back to step 1
```

This package provides the *loop* infrastructure and leaves the language,
prompt, and waiting-animation behaviour entirely pluggable via three abstract
base classes.

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                    REPL loop                         │
│                                                      │
│  ┌──────────┐   ┌──────────┐   ┌──────────────────┐ │
│  │  Prompt  │   │ Language │   │    Waiting       │ │
│  │ (ABC)    │   │ (ABC)    │   │    (ABC)         │ │
│  └──────────┘   └──────────┘   └──────────────────┘ │
│       ↑               ↑                ↑             │
│  DefaultPrompt   EchoLanguage    SilentWaiting       │
│  (built-in)      (built-in)      (built-in)          │
│                                                      │
│  I/O injection: input_fn / output_fn callables       │
└─────────────────────────────────────────────────────┘
```

### The Three Interfaces

**`Language`** — maps user input to a result:

| Return value       | Meaning                                     |
|--------------------|---------------------------------------------|
| `("ok", str)`      | Success with a displayable result string    |
| `("ok", None)`     | Success, nothing to display                 |
| `("error", str)`   | Failure with a human-readable message       |
| `"quit"`           | End the session                             |

**`Prompt`** — supplies prompt strings:
- `global_prompt() → str` — shown before each new statement (e.g. `"> "`)
- `line_prompt() → str` — shown on continuation lines (e.g. `"... "`)

**`Waiting`** — animation while eval runs:
- `start() → state` — initialise animation state
- `tick(state) → state` — advance animation every `tick_ms()` ms
- `tick_ms() → int` — interval between ticks in milliseconds
- `stop(state) → None` — clean up

### Async Eval

The loop runs `language.eval()` in a background `threading.Thread`.  The
main thread polls the thread with `thread.join(timeout=tick_ms/1000)` in a
loop while running waiting ticks.  If the evaluator raises an uncaught
exception the thread converts it to an `("error", message)` result so the
REPL never crashes.

### I/O Injection

`run_with_io(input_fn, output_fn)` accepts explicit callables instead of
using `input()` / `print()`.  This makes the loop trivially testable without
patching built-ins and lets you embed the REPL in GUIs, sockets, Jupyter
notebooks, etc.

## Installation

```bash
uv add coding-adventures-repl
```

## Usage

### Interactive REPL (built-in echo language)

```python
from coding_adventures_repl import Repl
Repl.run()
```

```
> hello
hello
> world
world
> :quit
```

### Custom language

```python
from coding_adventures_repl import Language, Repl

class DoubleLanguage(Language):
    def eval(self, input: str) -> tuple[str, str | None] | str:
        if input == ":quit":
            return "quit"
        try:
            return ("ok", str(int(input) * 2))
        except ValueError:
            return ("error", f"not an integer: {input!r}")

Repl.run(language=DoubleLanguage())
```

```
> 7
14
> hello
Error: not an integer: 'hello'
> :quit
```

### Programmatic / testing use

```python
from coding_adventures_repl import Repl

inputs = iter(["hello", "world", ":quit"])
collected: list[str] = []

Repl.run_with_io(
    input_fn=lambda: next(inputs, None),
    output_fn=collected.append,
)
# collected contains prompts and values interleaved
```

### Custom spinner (waiting plugin)

```python
import sys
from coding_adventures_repl import Waiting, Repl

FRAMES = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]

class SpinnerWaiting(Waiting):
    def start(self) -> int:
        return 0

    def tick(self, state: int) -> int:
        frame = FRAMES[state % len(FRAMES)]
        sys.stdout.write(f"\r{frame} ")
        sys.stdout.flush()
        return state + 1

    def tick_ms(self) -> int:
        return 80

    def stop(self, state: int) -> None:
        sys.stdout.write("\r  \r")
        sys.stdout.flush()

Repl.run(waiting=SpinnerWaiting())
```

## Built-in Implementations

| Class | Description |
|-------|-------------|
| `EchoLanguage` | Mirrors input back unchanged; `":quit"` ends session |
| `DefaultPrompt` | Returns `"> "` and `"... "` |
| `SilentWaiting` | All no-ops; `tick_ms()` = 100 ms |

## Where It Fits

```
Logic Gates → Arithmetic → CPU → ARM/RISC-V → Assembler
   → Lexer → Parser → Compiler → VM → [REPL]
```

The REPL framework sits at the top of the computing stack, wrapping any
evaluator (bytecode VM, interpreter, expression evaluator) in a polished
interactive shell.

## No Dependencies

This package uses only Python standard library modules:

- `abc` — abstract base classes
- `threading` — background eval thread
- `collections.abc` — `Callable` type hint

## Spec

See the package source for full literate-programming-style documentation
in every module's docstring.
