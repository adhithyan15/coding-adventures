# macsyma-repl

Interactive MACSYMA-flavored REPL built on top of the symbolic VM.

## Run it

```
uv run python main.py
```

or:

```
.venv/bin/python -m main
```

You should see something like:

```
MACSYMA-on-symbolic-VM 0.1
(%i1) f(x) := x^2$
(%i2) diff(f(x), x);
(%o2) 2*x
(%i3) integrate(%, x);
(%o3) x^3/3
(%i4) quit;
```

## Wiring

```
                ┌─────────────────────┐
                │  coding_adventures  │
input  ─────▶  │   _repl.Repl        │  ───▶ output
                │   .run_with_io()    │
                └──────────┬──────────┘
                           │
                ┌──────────┴──────────┐
                ▼                     ▼
        MacsymaLanguage         MacsymaPrompt
        (this program)          (this program)
                │                     │
   ┌────────────┼─────────────────────┘
   │            │
   ▼            ▼
macsyma_lexer  macsyma_runtime.History  ── (%i / %o table)
macsyma_parser
macsyma_compiler (wrap_terminators=True)
   │
   ▼
macsyma_runtime.MacsymaBackend
   │
   ▼
symbolic_vm.VM
   │
   ▼
cas_pretty_printer.pretty(.., MacsymaDialect())
```

## Phase A scope

- Single-line input — every input must end with ``;`` or ``$``. Multi-line
  buffering is a future extension (the REPL framework is missing the
  ``needs_more?`` hook today).
- ``(%iN) `` global prompt; result printed when the statement ends with
  ``;``, suppressed when it ends with ``$``.
- ``%``, ``%iN``, ``%oN`` resolve via :class:`History`.
- ``quit;`` or ``:quit`` exits.
- Parse, compile, and evaluation errors are caught and shown; the loop
  never crashes on user mistakes.

## Limitations (deferred)

- No multi-line input.
- No tab completion.
- No syntax highlighting.
- No ``batch("file.mac")`` file loading.
- No 2D output.

## Dependencies

- `coding-adventures-repl` (generic REPL framework)
- `coding-adventures-symbolic-ir`
- `coding-adventures-symbolic-vm`
- `coding-adventures-macsyma-lexer`
- `coding-adventures-macsyma-parser`
- `coding-adventures-macsyma-compiler` (>= 0.6 with `wrap_terminators`)
- `coding-adventures-macsyma-runtime`
- `coding-adventures-cas-pretty-printer`
