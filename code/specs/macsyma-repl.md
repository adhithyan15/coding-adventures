# macsyma-repl — Interactive MACSYMA Session

> **Status**: New spec. Defines the program (not a library) that wires
> the MACSYMA pipeline together with the generic REPL framework.
> Parent: `symbolic-computation.md`. Depends on `PL00-repl.md`.

## Why this program exists

We have lexer, parser, compiler, VM, integrator (Phase 13), and now a
pretty-printer and a runtime. Until something wires them together you
cannot actually use the CAS. This program is that wiring — the user
runs `python -m macsyma_repl` and gets a Maxima-flavored interactive
prompt:

```
MACSYMA-on-symbolic-VM 0.1
(C) 2026 — derived from MACSYMA at MIT

(%i1) f(x) := x^2$
(%i2) diff(f(x), x);
(%o2)                                 2 x

(%i3) integrate(%, x);
(%o3)                                  2
                                      x

(%i4) kill(all)$
(%i5) quit;
```

## Reuse story

This program is the model that every future CAS REPL follows. To build
a Mathematica REPL, copy `macsyma-repl/` to `mathematica-repl/`, swap
the lexer/parser/compiler/runtime imports, and ship. The REPL framework
(`coding-adventures-repl`), the VM, and every substrate package stay
identical.

A future Matlab/Octave REPL likewise plugs in its own lexer/parser/
compiler/runtime and reuses everything else.

## Scope

In:

- A Python program at `code/programs/python/macsyma-repl/` with a
  `__main__.py` entry point invocable as `python -m macsyma_repl`.
- A `MacsymaLanguage` plugin for the generic REPL framework
  (`coding_adventures_repl.Language`).
- A `MacsymaPrompt` plugin (`(%iN) ` global, `        ` continuation).
- Multi-line input — incomplete expressions roll over until the
  parser is satisfied (or a syntax error happens).
- Statement-terminator handling — `;` echoes, `$` suppresses.
- `%`, `%iN`, `%oN` resolution via the runtime's `History`.
- Persistent environment — `Assign`/`Define` survive across turns.
- Pretty-printed output via `cas-pretty-printer`.
- Friendly errors — parse error shows the offending token; runtime
  error shows the operation that failed.
- A `:quit` / `quit;` / Ctrl-D exit path.

Out:

- Tab completion (future).
- Syntax highlighting (future).
- Persistent history file across sessions (future).
- 2D output (future — depends on `cas-pretty-printer` 2D support).
- Web UI (future).

## Architecture

```
┌──────────────────────────────────────────────────────────────────────┐
│ python -m macsyma_repl                                               │
│                                                                      │
│  coding_adventures_repl.Repl                                         │
│   ├─ language: MacsymaLanguage(pipeline, runtime)                    │
│   ├─ prompt:   MacsymaPrompt(history)                                │
│   └─ waiting:  SilentWaiting                                         │
│                                                                      │
│  MacsymaLanguage.eval(input):                                        │
│    1. tokens = macsyma_lexer.tokenize(input)                         │
│    2. ast    = macsyma_parser.parse(tokens)                          │
│       — if incomplete, return {needs_more, ...}                      │
│    3. ir_stmts = macsyma_compiler.compile_program(ast)               │
│    4. for stmt in ir_stmts:                                          │
│         is_display = isinstance(stmt, Display)                       │
│         result = vm.eval(stmt.unwrap())                              │
│         history.record_input(stmt); history.record_output(result)    │
│         if is_display:                                               │
│           output = pretty(result, dialect=MacsymaDialect())          │
│         else:                                                        │
│           output = nil                                               │
│    5. return {ok, output}                                            │
│                                                                      │
│  MacsymaPrompt.global_prompt():                                      │
│    return f"(%i{history.next_input_index()}) "                       │
│                                                                      │
│  MacsymaPrompt.line_prompt():                                        │
│    return "        "  # eight spaces — Maxima convention             │
└──────────────────────────────────────────────────────────────────────┘
```

## Public API

The program exposes only `python -m macsyma_repl`. As a library, it
also exposes `MacsymaLanguage` and `MacsymaPrompt` so that tests (and
future tooling — e.g., a Jupyter kernel) can run the same logic
without spawning a real terminal.

```python
from macsyma_repl import (
    MacsymaLanguage,    # Language plugin for the generic REPL
    MacsymaPrompt,      # Prompt plugin
    main,               # entry point — equivalent to running -m
)
```

## Multi-line input

Implements the `needs_more?(partial) → bool` callback documented in
PL00-repl.md "Future Extensions". The REPL framework will expose this
hook; if it does not yet, this program ships a tiny shim that handles
buffering inside `MacsymaLanguage.eval`:

```
buffered = ""

eval(line):
    buffered += line
    try:
        ast = parse(tokenize(buffered))
    except IncompleteParseError:
        return {needs_more, line_prompt}
    except ParseError as e:
        buffered = ""
        return {error, format_parse_error(e)}
    buffered = ""
    return run(ast)
```

The simplest "incomplete" rule: input ends without a `;` or `$`
terminator at the top level. The parser already detects this when it
runs out of tokens before finishing a statement.

## Statement terminator handling

After `macsyma-runtime` introduces `Display`/`Suppress` wrappers, the
compiler emits one wrapper per top-level statement based on the
trailing `;` or `$` token. The REPL reads the wrapper to decide
whether to print:

| Statement form | Wrapper      | Behavior        |
|----------------|--------------|-----------------|
| `expr;`        | `Display`    | Evaluate, print |
| `expr$`        | `Suppress`   | Evaluate, no print |

`Display` and `Suppress` are pure containers — the VM unwraps them
before handler dispatch, so handlers do not need to know about them.

## %i / %o resolution

`MacsymaLanguage` registers a small "name lookup" hook on the backend:
when the VM walks `IRSymbol("%")`, it returns the most recent output;
`IRSymbol("%i3")` returns the third recorded input; `IRSymbol("%o3")`
returns the third recorded output. The `History` object owns this
state.

## Error handling

Three error families:

- **Parse errors** — caught from `macsyma-parser`, formatted as
  `parse error at line L col C: <msg>` with the offending token shown.
  The REPL prints the error and returns to the prompt.
- **Compile errors** — `CompileError` from `macsyma-compiler`, treated
  the same as parse errors.
- **Runtime errors** — anything raised inside the VM (NameError from
  `StrictBackend`, ZeroDivisionError, etc.). Formatted as
  `error: <message>`. The REPL prints and returns to the prompt; the
  session does not crash.

Unexpected exceptions (bugs in handlers) propagate to the framework's
generic handler, which prints `internal error: <type>: <message>` and
keeps the loop alive.

## Tests

`tests/test_session.py` runs the language plugin via the framework's
`Repl.run_with_io(input_fn, output_fn)` helper, which never touches a
real terminal:

- **Basic arithmetic**: `2 + 3;` → `5`.
- **Variable persistence**: `x: 5$ x + 1;` across turns → `6`.
- **Function definition**: `f(x) := x^2$ f(3);` → `9`.
- **Differentiation**: `diff(x^2 + 1, x);` → `2*x`.
- **Integration round-trip**: `integrate(diff(sin(x), x), x);` →
  `sin(x)` (after Simplify, deferred).
- **`%` reference**: `2 + 3; % * 2;` → `10`.
- **`%i` / `%o` reference**: `2 + 3; %o1 + 1;` → `6`.
- **Suppress**: `42$` returns nothing visible; the next line still works.
- **Display vs suppress**: `[a:1$, b:2;]` shows only `b`.
- **Quit**: `:quit` ends the session.
- **Parse error recovery**: `1 +;` prints an error and the next prompt
  still works.

Coverage target: ≥80%. The integration test exercises the full
pipeline end-to-end.

## Package layout

```
code/programs/python/macsyma-repl/
  pyproject.toml
  BUILD / BUILD_windows
  README.md
  CHANGELOG.md
  required_capabilities.json
  src/macsyma_repl/
    __init__.py        # exports MacsymaLanguage, MacsymaPrompt, main
    __main__.py        # `python -m macsyma_repl` entry point
    language.py        # MacsymaLanguage
    prompt.py          # MacsymaPrompt
    main.py            # main() — wires up the Repl and runs it
    py.typed
  tests/
    test_session.py
    test_language.py
    test_prompt.py
```

Dependencies (package, not OS):
- `coding-adventures-repl`
- `coding-adventures-symbolic-ir`
- `coding-adventures-symbolic-vm`
- `coding-adventures-macsyma-lexer`
- `coding-adventures-macsyma-parser`
- `coding-adventures-macsyma-compiler`
- `coding-adventures-macsyma-runtime`
- `coding-adventures-cas-pretty-printer`

## Future extensions

- **`batch("file.mac")`** — read and evaluate a file. Already partly
  designed in `macsyma-runtime`.
- **Tab completion** via `Language.complete(partial)`.
- **Syntax highlighting** via `Language.highlight(input)`.
- **2D output** when `cas-pretty-printer` supports it.
- **Notebook kernel** — wrap the same `MacsymaLanguage` in a Jupyter
  protocol shim (`LANG09-notebook-kernel.md` already specs this).
- **Web REPL** — wrap in a WebSocket handler for browser-based use.
