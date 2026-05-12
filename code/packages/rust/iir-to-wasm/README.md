# iir-to-wasm

Lowers an [`IIRModule`](../interpreter-ir/) directly to a
[`WasmModule`](../wasm-types/) **without going through the deprecated
`compiler-ir` layer**.

```text
IIRModule  (interpreter-ir)
    │
    ▼  validate_for_wasm()      — pre-flight check, returns Vec<String>
    │
    ▼  lower_iir_to_wasm()      — two-pass lowering, returns WasmModule
    │
    ▼  encode_module()          — binary encoding, returns Vec<u8>
                                  (via wasm-module-encoder)
```

## Why IIR → WASM directly?

The existing `ir-to-wasm-compiler` crate lowered `compiler_ir::IrProgram` — a
flat, single-function IR with no type information.  `IIRModule` is richer: it
has multiple functions, named variables, static type hints, and a full
arithmetic/comparison/bitwise operator set that maps cleanly to WASM's typed
numeric opcodes.  This crate exploits that richness without retrofitting it
through a deprecated intermediate.

## Features

- **Full numeric type support**: `i8/i16/i32/u8/u16/u32/bool → i32`,
  `i64/u64 → i64`, `f32 → f32`, `f64 → f64`.
- **Float constants supported** (unlike the BEAM backend): `f64.const` is
  emitted for `Operand::Float` and `f32.const` for narrower floats.
- **All arithmetic and bitwise ops**: add, sub, mul, div, rem, and, or, xor,
  shl, shr, for all numeric types.
- **All comparison ops**: eq, ne, lt, le, gt, ge (signed and unsigned variants
  where applicable).
- **Function calls**: `call` instructions are lowered to WASM `call`
  opcodes with the correct function index.
- **Control flow**: dispatch-loop pattern for functions with labels and jumps;
  plain linear emission for straight-line functions.
- **All functions exported**: every function in the IIR module is exported by
  name so host runtimes can invoke them.

## Quick start

```rust
use interpreter_ir::{IIRModule, IIRFunction, IIRInstr, Operand};
use iir_to_wasm::{validate_for_wasm, lower_iir_to_wasm, IIRWasmConfig};
use wasm_module_encoder::encode_module;

// Build a simple add(a: i32, b: i32) -> i32 function.
let fn_ = IIRFunction::new(
    "add",
    vec![("a".into(), "i32".into()), ("b".into(), "i32".into())],
    "i32",
    vec![
        IIRInstr::new("add", Some("v0".into()),
            vec![Operand::Var("a".into()), Operand::Var("b".into())], "i32"),
        IIRInstr::new("ret", None, vec![Operand::Var("v0".into())], "i32"),
    ],
);
let module = IIRModule {
    name: "calc".into(),
    functions: vec![fn_],
    entry_point: Some("add".into()),
    language: "test".into(),
};

// Validate first.
let errors = validate_for_wasm(&module);
assert!(errors.is_empty());

// Lower to WasmModule.
let config = IIRWasmConfig::new("calc");
let wasm_module = lower_iir_to_wasm(&module, &config).unwrap();

// Encode to bytes.
let bytes = encode_module(&wasm_module).unwrap();
assert!(bytes.starts_with(b"\x00asm"));
```

## File structure

```
iir-to-wasm/
├── Cargo.toml
├── BUILD                 # cargo test -p iir-to-wasm
├── CHANGELOG.md
├── README.md
└── src/
    ├── lib.rs            # Public API + module doc
    ├── validate.rs       # Pre-flight validation
    ├── lower.rs          # IIRWasmConfig, IIRWasmError, lower_iir_to_wasm
    └── codegen.rs        # WASM binary encoding helpers
tests/
└── test_backend.rs       # 40+ integration tests
```

## Validation errors

| Error | Condition |
|-------|-----------|
| `EmptyModule` | Module has no functions |
| `EmptyFunction` | A function has no instructions |
| `ClosureOpcode` | op is `alloc_closure` or `call_closure` — closures require the BEAM backend |
| `UntypedInstruction` | `type_hint` is `"any"` or `"polymorphic"` |
| `UnsupportedType` | `type_hint` is `"str"` or starts with `"ref<"` |
| `UnsupportedOp` | op is `call_builtin`, `io_in`, `io_out`, `cast`, `load_mem`, `store_mem`, `alloc`, `box`, `unbox`, `field_load`, `field_store`, `is_null`, or `safepoint` |

> **LANG35 note**: `alloc_closure` and `call_closure` (LANG34/LANG35 first-class
> closure opcodes) are BEAM-only.  Using them in a WASM module returns a clear
> `ClosureOpcode` error rather than the generic `UntypedInstruction` message.

## Relationship to other crates

| Crate | Role |
|-------|------|
| `interpreter-ir` | Provides `IIRModule`, `IIRFunction`, `IIRInstr`, `Operand` |
| `wasm-types` | Provides `WasmModule`, `FuncType`, `FunctionBody`, `ValueType`, `Export`, `ExternalKind` |
| `wasm-module-encoder` | Serialises `WasmModule` to raw `.wasm` bytes |
| `wasm-leb128` | Unsigned LEB128 encoding for WASM integer immediates |
| `codegen-core` | Shared code-generation infrastructure (optional in v1) |
