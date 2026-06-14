# twig-to-beam

End-to-end Twig → BEAM pipeline crate.  Takes a Twig source string and emits
a valid BEAM binary (`.beam` file format) that can be loaded by the Erlang VM.

## What it does

The pipeline wires five stages together in a single function call:

```text
Twig source string
  │
  ▼  twig_ir_compiler::compile_source
IIRModule   (every instruction has type_hint = "any" — Twig is dynamic)
  │
  ▼  pre_lower_builtins   [pipeline-local, unconditional]
IIRModule   (call_builtin "+" → add, "=" → cmp_eq, "_move" → load_reg, etc.)
  │
  ▼  iir_type_checker::infer_and_check
IIRModule   (add/sub/cmp_eq now have concrete types: "i64", "bool", …)
  │
  ▼  fixup_control_flow_types   [pipeline-local]
IIRModule   (ret/call/jmp_if* "any" hints repaired; load_reg type propagated)
  │
  ▼  iir_to_beam::validate_for_beam
()          (validates; returns Err on unsupported ops or "any" types)
  │
  ▼  iir_to_beam::lower_iir_to_beam
BeamModule
  │
  ▼  ir_to_beam::encode_beam
Vec<u8>     ← BEAM binary, starts with b"FOR1"
```

## Public API

```rust
use twig_to_beam::{compile_twig_to_beam, TwigToBeamError};

let bytes = compile_twig_to_beam(
    "(define (add a b) (+ a b)) (add 1 2)",
    "my_module",
)?;
assert!(bytes.starts_with(b"FOR1"));
```

### Error variants

| Variant | When |
|---------|------|
| `TwigToBeamError::CompileError(e)` | Twig syntax error or unbound name |
| `TwigToBeamError::BeamError(e)` | IIR → BEAM validation or lowering failed |

Both variants implement `Display` and `std::error::Error` with source chaining.

## What programs compile successfully

Twig is dynamically typed, so the IR compiler emits `call_builtin "+"` rather
than a typed `add` instruction.  The pipeline pre-lowers the following builtins
to typed IIR ops that the BEAM backend can handle:

| Twig builtin | IIR op |
|---|---|
| `+` | `add` |
| `-` | `sub` |
| `*` | `mul` |
| `/` | `div` |
| `=` | `cmp_eq` |
| `<` | `cmp_lt` |
| `>` | `cmp_gt` |
| `<=` | `cmp_le` |
| `>=` | `cmp_ge` |
| `not` | `lnot` |
| `_move` | `load_reg` |

Programs that use only these operations — including recursive functions and
`if` expressions — compile to valid BEAM binaries.

Programs that use `nil`, `cons`, closures, or global variables produce
`TwigToBeamError::BeamError` because those operations remain as
`call_builtin` instructions that the BEAM validator rejects.

## Why the ordering matters

The type-checker's inference rules fire on `add`/`sub`/`cmp_eq` (which it
knows are arithmetic), NOT on `call_builtin "+"`.  So the pipeline must:

1. Pre-lower `call_builtin` → typed ops (before inference).
2. Run type inference.
3. Fix up control-flow ops (`ret`, `call`, `jmp_if_*`) which inference skips.

Skipping step 1 would leave all arithmetic at `type_hint = "any"`, which the
BEAM validator rejects.

## Running tests

```sh
cargo test -p twig-to-beam
```

The test suite has 30 integration tests in five groups:

- **Group 1** — successful compilations (addition through mutual recursion)
- **Group 2** — compile errors (syntax errors, unbound names)
- **Group 3** — BEAM backend errors (nil literal, empty program)
- **Group 4** — error type properties (Display, std::error::Error, source chain)
- **Group 5** — BEAM binary structure (magic bytes, BEAM tag, determinism)

## Where it fits

```
twig-ir-compiler   →  twig-to-beam  →  BEAM VM
twig-ir-compiler   →  twig-to-wasm  →  WASM runtime
```

`twig-to-beam` is the BEAM-specific pipeline crate.  The `twig-to-wasm` sister
crate provides the same interface but emits WebAssembly 1.0.
