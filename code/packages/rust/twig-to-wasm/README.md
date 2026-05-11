# twig-to-wasm

End-to-end Twig → WebAssembly pipeline crate.  Takes a Twig source string and
emits a valid WASM 1.0 binary that can be loaded by any WebAssembly runtime
(wasmtime, wasmer, V8, browser, etc.).

## What it does

The pipeline wires five stages together in a single function call:

```text
Twig source string
  │
  ▼  twig_ir_compiler::compile_source
IIRModule   (every instruction has type_hint = "any" — Twig is dynamic)
  │
  ▼  pre_lower_builtins   [pipeline-local, unconditional]
IIRModule   (call_builtin "+" → add, "=" → eq, "_move" → mov, etc.)
  │
  ▼  iir_type_checker::infer_and_check
IIRModule   (add/sub/eq now have concrete types: "i64", "bool", …)
  │
  ▼  fixup_control_flow_types   [pipeline-local]
IIRModule   (ret/call/jmp_if* "any" hints repaired; mov type propagated)
  │
  ▼  iir_to_wasm::validate_for_wasm
()          (validates; returns Err on unsupported ops or "any" types)
  │
  ▼  iir_to_wasm::lower_iir_to_wasm
WasmModule
  │
  ▼  iir_to_wasm::encode_module
Vec<u8>     ← WASM binary, starts with b"\x00asm"
```

## Public API

```rust
use twig_to_wasm::{compile_twig_to_wasm, TwigToWasmError};

let bytes = compile_twig_to_wasm(
    "(define (add a b) (+ a b)) (add 1 2)",
    "arith",
)?;
assert!(bytes.starts_with(b"\x00asm"));
```

### Error variants

| Variant | When |
|---------|------|
| `TwigToWasmError::CompileError(e)` | Twig syntax error or unbound name |
| `TwigToWasmError::WasmError(e)` | IIR → WASM validation or lowering failed |
| `TwigToWasmError::EncodeError(s)` | WASM binary encoding failed |

All variants implement `Display` and `std::error::Error` with source chaining.

## What programs compile successfully

Twig is dynamically typed, so the IR compiler emits `call_builtin "+"` rather
than a typed `add` instruction.  The pipeline pre-lowers the following builtins
to typed IIR ops that the WASM backend can handle:

| Twig builtin | IIR op | WASM opcode |
|---|---|---|
| `+` | `add` | `i64.add` |
| `-` | `sub` | `i64.sub` |
| `*` | `mul` | `i64.mul` |
| `/` | `div` | `i64.div_s` |
| `=` | `eq` | `i64.eq` |
| `<` | `lt` | `i64.lt_s` |
| `>` | `gt` | `i64.gt_s` |
| `<=` | `le` | `i64.le_s` |
| `>=` | `ge` | `i64.ge_s` |
| `not` | `lnot` | `i32.eqz` |
| `_move` | `mov` | `local.get + local.set` |

Programs that use only these operations — including recursive functions and
`if` expressions — compile to valid WASM binaries.

Programs that use `nil`, `cons`, closures, or global variables produce
`TwigToWasmError::WasmError` because those operations remain as
`call_builtin` instructions that the WASM validator rejects.

## Difference from `twig-to-beam`

The sister crate `twig-to-beam` targets the BEAM (Erlang) VM.  The two
pipelines share the same architecture but differ in:

| | `twig-to-beam` | `twig-to-wasm` |
|--|--|--|
| `_move` → | `load_reg` | `mov` |
| Comparison ops | `cmp_eq`, `cmp_lt`, … | `eq`, `lt`, … |
| Output magic | `b"FOR1"` | `b"\x00asm"` |
| Module name | embedded in AtU8 | NOT embedded (WASM 1.0 has no module-name section in the binary) |

## Running tests

```sh
cargo test -p twig-to-wasm
```

The test suite has 30 integration tests in five groups:

- **Group 1** — successful compilations (addition through mutual recursion)
- **Group 2** — compile errors (syntax errors, unbound names)
- **Group 3** — WASM backend errors (nil literal, empty program)
- **Group 4** — error type properties (Display, std::error::Error, source chain)
- **Group 5** — WASM binary structure (magic bytes, version field, determinism)

## Where it fits

```
twig-ir-compiler   →  twig-to-beam  →  BEAM VM (Erlang/OTP)
twig-ir-compiler   →  twig-to-wasm  →  WASM runtime (wasmtime, V8, …)
```
