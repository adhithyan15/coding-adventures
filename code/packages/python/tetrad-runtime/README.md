# tetrad-runtime — Tetrad on the LANG pipeline

`tetrad-runtime` is the **end-to-end demonstration** that Tetrad — the small,
statically-typed u8 language defined by TET00–TET05 — runs on the same generic
LANG infrastructure (LANG01 InterpreterIR + LANG02 vm-core + LANG03 jit-core)
that future languages like Lisp, μScheme, and Python will share.

It exists because the original Tetrad pipeline lived in three bespoke
packages — `tetrad-compiler` emitted a Tetrad-specific `CodeObject`,
`tetrad-vm` interpreted it via `register-vm`, `tetrad-jit` compiled it to
Intel 4004 through Tetrad-specific intermediate forms.  None of that was
reusable by another language.

`tetrad-runtime` proves the new pipeline can host Tetrad without any of that
language-specific machinery downstream of the type checker:

```
Tetrad source
   ↓ tetrad_lexer + tetrad_parser + tetrad_type_checker  (unchanged)
   ↓ tetrad_compiler.compile_program → CodeObject       (unchanged)
   ↓ tetrad_runtime.compile_to_iir → IIRModule           ← THIS PACKAGE
   ↓ vm_core.VMCore.execute(module)                      (generic)
   ↓ result : int
```

The same `IIRModule` can be handed to `jit_core.JITCore.execute_with_jit` with
an `Intel4004Backend` for the JIT path — also exposed by this package.

## What this package contains

- `tetrad_runtime.compile_to_iir(source)` — the translator: walks a
  Tetrad `CodeObject` and emits a standard-opcode `IIRModule` that vm-core,
  jit-core, and aot-core can all consume without knowing it came from Tetrad.
- `tetrad_runtime.TetradRuntime` — a small façade that wires up vm-core
  with the Tetrad-specific builtins (`__io_in`, `__io_out`, `__get_global`,
  `__set_global`) and provides `run(source)` and `run_with_jit(source)`.
- `tetrad_runtime.Intel4004Backend` — adapts the existing
  `intel4004-simulator` package to the LANG `BackendProtocol`.  jit-core
  calls `compile()` with a `list[CIRInstr]`; the backend translates to
  4004 abstract assembly and assembles to bytes.

## Why a new package, not a tetrad-vm rewrite

`tetrad-vm` and `tetrad-jit` have ~3000 lines of tests asserting on
Tetrad-specific metric shapes (feedback vector state machines,
branch-stat dictionaries keyed by IP, loop-iteration counts).  Reproducing
those exact shapes through vm-core's profiler would take more code than the
underlying behaviour — so we kept those packages alive as the
"original Tetrad" and built the LANG-based path beside them.  Both paths
work; over time we expect new Tetrad work to flow through `tetrad-runtime`
and the legacy packages to be retired.

## Quick start

```python
from tetrad_runtime import TetradRuntime

source = """
fn add(a: u8, b: u8) -> u8 {
    return a + b;
}

fn main() -> u8 {
    return add(3, 4);
}
"""

runtime = TetradRuntime()
result = runtime.run(source)        # interpreted on vm-core
assert result == 7

result = runtime.run_with_jit(source)  # JIT path through jit-core (interp fallback for unsupported ops)
assert result == 7
```

## Translation table

The translator emits standard IIR opcodes wherever possible.  Tetrad's
accumulator-based execution model maps to a single SSA-named register
called `_acc`; the eight register slots map to `r0`–`r7`; locals are
named SSA variables; globals are accessed through builtins.

| Tetrad Op        | IIR emission                                                                                |
|------------------|----------------------------------------------------------------------------------------------|
| `LDA_IMM n`      | `const _acc, n`                                                                              |
| `LDA_ZERO`       | `const _acc, 0`                                                                              |
| `LDA_REG r`      | `load_reg _acc, r`                                                                           |
| `LDA_VAR i`      | `tetrad.move _acc, varname` (local) or `call_builtin _acc, "__get_global", varname`           |
| `STA_REG r`      | `store_reg r, _acc`                                                                          |
| `STA_VAR i`      | `tetrad.move varname, _acc` (local) or `call_builtin _, "__set_global", varname, _acc`        |
| `ADD r`          | `load_reg _t, r; add _acc, _acc, _t`                                                         |
| `ADD_IMM n`      | `add _acc, _acc, n`                                                                          |
| `EQ r`           | `load_reg _t, r; cmp_eq _b, _acc, _t; cast _acc, _b, u8`                                     |
| `LOGICAL_NOT`    | `cmp_eq _b, _acc, 0; cast _acc, _b, u8`                                                      |
| `LOGICAL_AND r`  | reduces to two `cmp_ne` plus `and`, then `cast` to u8                                         |
| `JMP off`        | `jmp L<target>`                                                                              |
| `JZ off`         | `cmp_eq _b, _acc, 0; jmp_if_true _b, L<target>`                                              |
| `JNZ off`        | `cmp_eq _b, _acc, 0; jmp_if_false _b, L<target>`                                             |
| `CALL idx,argc,_`| `load_reg _a0..; call _acc, fn_name, _a0, _a1, ...`                                          |
| `RET`            | `ret _acc`                                                                                   |
| `IO_IN`          | `call_builtin _acc, "__io_in"`                                                               |
| `IO_OUT`         | `call_builtin _, "__io_out", _acc`                                                           |
| `HALT`           | `ret_void`                                                                                   |

Two opcodes — `tetrad.move` (a typed-but-no-side-effect copy) — are
language extensions registered with vm-core via the `opcodes` constructor
parameter.  Everything else is standard LANG01 surface.

## Layer position

```
... → Tetrad source → lexer → parser → type-checker → tetrad-compiler (CodeObject)
                                                              │
                                                              ▼
                                              tetrad-runtime.compile_to_iir
                                                              │
                                                              ▼
                                                        IIRModule (LANG01)
                                                              │
                              ┌───────────────────────────────┼────────────────────────────────┐
                              ▼                               ▼                                ▼
                         vm-core (LANG02)              jit-core (LANG03)                aot-core (LANG04)
                              │                               │                                │
                              ▼                               ▼                                ▼
                          interpreted                 native binary +                    .aot snapshot
                                                      Intel4004Backend
```
