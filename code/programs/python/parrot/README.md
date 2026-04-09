# Parrot (Python)

🦜 The world's simplest REPL — it repeats back whatever you type.

Parrot is a demonstration program for the
[`coding-adventures-repl`](../../packages/python/repl/) framework. It shows how
to wire the three pluggable components — Language, Prompt, and Waiting — into a
working interactive program.

---

## What It Does

```
🦜 Parrot REPL
I repeat everything you say! Type :quit to exit.

hello
hello
🦜 Parrot REPL
I repeat everything you say! Type :quit to exit.

how are you?
how are you?
🦜 Parrot REPL
I repeat everything you say! Type :quit to exit.

:quit
```

Everything you type is echoed back verbatim. Type `:quit` to end the session.

---

## How It Works

Three pluggable components are combined:

| Component        | Class             | Role                                          |
|------------------|-------------------|-----------------------------------------------|
| **Language**     | `EchoLanguage`    | Mirrors input back; `:quit` ends the session  |
| **Prompt**       | `ParrotPrompt`    | Parrot-themed banner + `🦜 > ` line prompt    |
| **Waiting**      | `SilentWaiting`   | No spinner (EchoLanguage is instant)          |

The framework's `run_with_io` drives the read-eval-print cycle:

```
stdin ──► run_with_io ──► EchoLanguage.eval() ──► stdout
                │
          ParrotPrompt   (prints banner before each read)
          SilentWaiting  (does nothing — eval is instant)
```

---

## Running

### Interactive

```bash
cd code/programs/python/parrot
uv venv
uv pip install -e ../../../packages/python/repl
uv pip install -e .
uv run parrot
```

Or without installing the script:

```bash
uv run python -c "from parrot.main import main; main()"
```

### Tests

```bash
uv pip install -e ".[dev]"
uv run python -m pytest tests/ -v
```

---

## Project Structure

```
parrot/
├── src/
│   └── parrot/
│       ├── __init__.py   # Package docstring and usage examples
│       ├── main.py       # Entry point; wires components to stdin/stdout
│       └── prompt.py     # ParrotPrompt: banner and line_prompt strings
├── tests/
│   ├── __init__.py       # Makes tests/ a Python package
│   └── test_parrot.py    # 30+ pytest tests using I/O injection
├── pyproject.toml        # Build config; console script; dependencies
├── BUILD                 # Unix build script
├── BUILD_windows         # Windows build script
├── CHANGELOG.md
└── README.md             # This file
```

---

## Where It Fits in the Stack

```
code/programs/python/parrot/     ← you are here (demonstration program)
code/packages/python/repl/       ← REPL framework (run_with_io, EchoLanguage, etc.)
```

Parrot depends on the `repl` package but adds no new library functionality —
its sole purpose is to demonstrate how the framework is used.

---

## Design Notes

**Why `sys.stdin.readline()` instead of `input()`?**

`input()` raises `EOFError` on end-of-file and returns the line without a
trailing newline.  `readline()` returns `""` on EOF and `"\n"` for an empty
line — a cleaner sentinel model.  The `_read_line()` helper in `main.py`
wraps `readline()` to convert `""` → `None` (the loop's EOF signal).

**Why `sys.stdout.write` instead of `print`?**

`print()` always appends `\n`.  `sys.stdout.write()` writes exactly what it
receives.  The prompt strings and echo results already contain the newlines
they need; using `write` avoids double-newline artefacts.

**Why does `global_prompt` print a full banner every cycle?**

`ParrotPrompt.global_prompt()` is called once per REPL cycle.  The banner
is intentionally shown on every cycle for demo clarity.  A production REPL
would track state and only show the banner on the first iteration.

**Why `SilentWaiting`?**

`EchoLanguage.eval()` is a pure, instant function.  There is nothing to wait
for.  `SilentWaiting` implements the `Waiting` interface with no-ops —
the correct plugin when evaluation time is negligible.
