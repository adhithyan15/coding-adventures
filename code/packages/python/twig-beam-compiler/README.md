# twig-beam-compiler

End-to-end Twig source → `.beam` → real `erl` execution.  This is
[BEAM01](../../../specs/BEAM01-twig-on-real-erl.md) Phase 4 — the
counterpart to [`twig-jvm-compiler`](../twig-jvm-compiler/) but
targeting the BEAM VM (Erlang's runtime) instead of the JVM.

## Pipeline

```
Twig source
    ↓ parse + extract       (twig package)
typed AST
    ↓ Compiler.compile()    (this module — Twig → compiler-IR)
IrProgram
    ↓ ir-optimizer          (constant folding, DCE)
IrProgram (optimised)
    ↓ lower_ir_to_beam      (ir-to-beam package)
BEAMModule
    ↓ encode_beam           (beam-bytecode-encoder)
.beam bytes
    ↓ erl -noshell ...      (real Erlang runtime)
program output
```

## V1 surface (this release)

The first iteration is intentionally narrow: arithmetic
expressions evaluated as the body of a synthesised `main/0`
function.

```
(+ 1 2)
(* 6 7)
(- 10 3)
(let ((x 5)) (* x x))
```

The value of the last top-level expression becomes `main/0`'s
return value, which `erl -eval 'io:format(...)'` prints.

## Out of scope for v1

- **Top-level `define`**: requires multi-function lowering.  v2.
- **Recursion**: requires the `CALL` IR-op path through
  `ir-to-beam`.  v2.
- **`if` / `let` outside of trivial cases**: needs branching
  support in `ir-to-beam`.  v2.
- **`SYSCALL` / output / I/O**: BEAM v1 uses the function's
  return value as the result; no `io:put_chars` round trip.

## Quick start

```python
from twig_beam_compiler import run_source

result = run_source("(+ 1 2)")
print(result.returncode)  # 0
print(result.stdout)      # b'3\n'  (erl printed the integer 3)
```

## Sister packages

- [`twig-jvm-compiler`](../twig-jvm-compiler/) — same pipeline
  shape, JVM target.
- [`twig`](../twig/) — language frontend (lexer/parser/AST + the
  in-process `vm-core` runtime).
- [`ir-to-beam`](../ir-to-beam/) — Phase 3, the actual IR → BEAM
  lowering this package wraps.
- [`beam-bytecode-encoder`](../beam-bytecode-encoder/) — Phase 2,
  the file-format writer.
