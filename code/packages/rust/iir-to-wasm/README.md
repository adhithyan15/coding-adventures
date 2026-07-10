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

- **Full numeric type support**: `u4/i8/i16/i32/u8/u16/u32/bool → i32`,
  `i64/u64 → i64`, `f32 → f32`, `f64 → f64`.
- **Float constants supported** (unlike the BEAM backend): `f64.const` is
  emitted for `Operand::Float` and `f32.const` for narrower floats.
- **All arithmetic and bitwise ops**: add, sub, mul, div, rem, and, or, xor,
  shl, shr, for all numeric types. **Narrow-width results wrap mod-2ⁿ**
  (LANG-FULL E2): narrow **unsigned** integers (`u4`/`u8`/`u16`/`u32`) ride the
  **i64 register model** — i64 locals and `i64.*` ops over their i64-slot
  operands — and the result is masked with `i64.const <mask>; i64.and`, so
  `200u8+100u8=44` and `~0u8=255`. Computing wide and masking the *value* (not
  typing the op narrow) is operand-width-agnostic: it works whatever width the
  operands arrive at — crucial because real frontends (Nib, …) carry every
  `const`/`let` as `i64` and put the narrow width only on the op. This matches
  the vm-core/jit-core/LLVM/native backends. *(v0.15.0 replaced the earlier
  i32-op-plus-`i32.and` approach, which trapped over i64 operands.)*
- **All comparison ops**: eq, ne, lt, le, gt, ge (signed and unsigned variants
  where applicable).
- **Function calls**: `call` instructions are lowered to WASM `call`
  opcodes with the correct function index.
- **Control flow**: dispatch-loop pattern for functions with labels and jumps;
  plain linear emission for straight-line functions. Branch conditions are
  width-correct: an `i32` guard tests directly / via `i32.eqz`, an `i64` guard
  (a widened Brainfuck cell) via `i64.eqz` (v0.13.0).
- **Byte-tape memory (v0.13.0 — Brainfuck)**: `alloc_bytes` (tape base in linear
  memory), `load_byte` (`i32.load8_u` + `i64.extend_i32_u`), `store_byte`
  (`i32.wrap_i64` + `i32.store8`), plus the `env.putchar`/`env.getchar` host
  imports for `.`/`,`. Byte width lives only at the tape boundary; registers in
  between are uniform `i64`. This is the wasm sibling of the LLVM byte-tape
  lowering, so **Brainfuck runs on wasm** (LANG-MATRIX LM-W Brainfuck).
- **Bounds-checked arrays (v0.16.0 — LANG-FULL E5)**: the *static* array model in
  linear memory. A synthetic mutable `i64` global `__array_bump` hands each
  `alloc_array` a fresh `[i64 length][elem 0][elem 1]…` region (the handle is the
  block's byte offset); `array_get`/`array_set` emit an **explicit** unsigned
  bounds check `idx >=u len` (`i64.ge_u`) → `if … unreachable` (the wasm trap, since
  there is no managed runtime to bounds-check for it) then an `i64.load`/`i64.store`
  (or `f64`) at `wrap(handle)+idx*elemsize` offset 8; `array_len` reads the header.
  The wasm sibling of the LLVM `@calloc` + `icmp uge` + `llvm.trap` lowering. `i64`
  and `f64` elements (the ALGOL `integer`/`real` arrays), plus **`str` elements**
  (v0.36.0 — E4d-BA-arr, BASIC `DIM A$(n)`): a 4-byte `i32` handle per element
  (`i32.load`/`i32.store`, 4-byte stride), each an E4-dyn runtime string block
  offset. A folded str literal stored via `array_set` is promoted to a runtime
  block handle so the element holds a real offset rather than an uninitialised `0`.
- **String literal foothold (v0.23.0 — LANG-FULL E4 / BA4)**: `str_const` writes
  printable ASCII literal bytes into a linear-memory data segment and stores the
  byte pointer in an `i32` local; `print_str` calls the host import
  `env.__print_str(ptr,len)`. `str_len` over a direct literal materialises that
  literal's byte count as an integer constant, `str_eq` over two direct literals
  materialises byte equality as `1`/`0`, `str_cmp` materialises `-1`/`0`/`1`
  from byte ordering, and literal `str_concat` creates another
  data entry for the combined bytes. Literal `str_slice` derives another data
  entry for a constant byte range. `str_index` over a direct literal emits a
  guarded `i32.load8_u` from the same data segment. This covers BASIC
  `PRINT "HELLO"` plus Twig `(string-length "HELLO")` and
  `(string-ref "ABC" 1)`, `(string=? "HELLO" "HELLO")`,
  `(string<? "ALPHA" "BETA")`, and
  `(string-length (string-append "AB" "CDE"))`, plus
  `(string-ref (substring "ABCDE" 1 4) 1)`; non-literal string values still fail
  closed until the full byte-string runtime lands.
- **Runtime (branch-selected) strings (v0.29.0 — LANG-FULL E4-dyn E4d-3)**: a
  string variable assigned by `str_const` in **more than one basic block** is
  chosen by control flow, so it cannot fold to one literal. Such a variable is
  promoted to carry an i32 **handle** = the byte offset of a length-prefixed
  block `[i32 len (little-endian)][bytes]` in linear memory. `str_const` of a
  promoted var stores its block offset; `print_str` reads the length back with
  `i32.load` at the handle and passes `handle + 4` + that length to
  `env.__print_str(ptr, len)`. This is the wasm sibling of the LLVM `inttoptr` +
  `load` + `getelementptr … i64 8` runtime path (E4d-2), and lets
  `10 INPUT N … 30 LET A$="LO" … 50 LET A$="HI" … 60 PRINT A$` print the branch's
  string at run time. Single-assignment (and straight-line-reassigned) strings
  keep the folded literal fast path unchanged. Runtime `str_len`/`str_concat`/
  `str_slice`/`str_index`/`str_cmp` over promoted operands are still deferred
  (E4d-3b).
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
| `UnsupportedOp` | op is `io_in`, `cast`, or `safepoint` (and `call_builtin` with a non-whitelisted name). Note: `alloc`/`field_load`/`field_store`/`is_null` are accepted for `ref<LispyPair>`; `box`/`unbox` are accepted and lower to WasmGC `ref.i31`/`i31.get_s` (LANG77 L3b-3a); `io_out`/`global_*`/`load_mem`/`store_mem` and the byte-tape ops `alloc_bytes`/`load_byte`/`store_byte` (v0.13.0) are supported |

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
