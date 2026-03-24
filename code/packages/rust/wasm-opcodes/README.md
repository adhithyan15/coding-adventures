# wasm-opcodes (Rust)

A complete compile-time lookup table for all 172 WebAssembly 1.0 MVP instructions,
with metadata for each opcode: name, byte value, category, immediates, and stack effects.

Part of the [coding-adventures](../../../../README.md) monorepo — a ground-up
implementation of the computing stack from transistors to operating systems.

## What it does

Every WASM instruction is identified by a single opcode byte. This crate provides:

- A `static` slice (`OPCODES`) of all 172 WASM 1.0 instructions, allocated in
  read-only memory with no heap allocation
- `get_opcode(byte)` — look up an instruction by its byte value
- `get_opcode_by_name(name)` — look up by its canonical text name (e.g. `"i32.add"`)

## Data structure

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct OpcodeInfo {
    pub name: &'static str,       // canonical text name, e.g. "i32.add"
    pub opcode: u8,               // byte value, e.g. 0x6A
    pub category: &'static str,   // "control", "memory", "numeric_i32", …
    pub immediates: &'static [&'static str], // e.g. &["memarg"] for loads
    pub stack_pop: u8,            // values consumed from operand stack
    pub stack_push: u8,           // values produced onto operand stack
}
```

## Usage

```rust
use wasm_opcodes::{get_opcode, get_opcode_by_name, OPCODES};

// Look up by byte
let info = get_opcode(0x6A).unwrap();
assert_eq!(info.name, "i32.add");
assert_eq!(info.stack_pop, 2);
assert_eq!(info.stack_push, 1);

// Look up by name
let info = get_opcode_by_name("i32.const").unwrap();
assert_eq!(info.opcode, 0x41);
assert_eq!(info.immediates, &["i32"]);

// Unknown byte → None
assert!(get_opcode(0xFF).is_none());

// Iterate all opcodes
for op in OPCODES {
    println!("{:#04x}  {}  (pop={}, push={})", op.opcode, op.name, op.stack_pop, op.stack_push);
}
```

## Categories

| Category      | Description                                 | Example instructions          |
|---------------|---------------------------------------------|-------------------------------|
| `control`     | Program flow, calls, branches               | `unreachable`, `call`, `br`   |
| `parametric`  | Type-agnostic stack operations              | `drop`, `select`              |
| `variable`    | Local and global variable access            | `local.get`, `global.set`     |
| `memory`      | Loads, stores, memory size/grow             | `i32.load`, `i64.store8`      |
| `numeric_i32` | 32-bit integer arithmetic and comparisons   | `i32.add`, `i32.lt_s`         |
| `numeric_i64` | 64-bit integer arithmetic and comparisons   | `i64.mul`, `i64.ge_u`         |
| `numeric_f32` | 32-bit float arithmetic and comparisons     | `f32.sqrt`, `f32.copysign`    |
| `numeric_f64` | 64-bit float arithmetic and comparisons     | `f64.add`, `f64.nearest`      |
| `conversion`  | Type conversions between numeric types      | `i32.wrap_i64`, `f64.promote_f32` |

## How it fits in the stack

```
wasm-types       ← type system (FuncType, ValueType, etc.)
wasm-leb128      ← integer encoding used in WASM binaries
wasm-opcodes     ← this crate: instruction metadata
wasm-module-parser ← parses .wasm binary files (uses all three above)
```

## Dependencies

- `wasm-types` — for shared type definitions

## Development

```bash
# Run tests (from the Rust workspace root)
cd code/packages/rust
cargo test -p wasm-opcodes -- --nocapture

# Lint
cargo clippy -p wasm-opcodes

# Or via the BUILD file
bash BUILD
```

## Opcode count note

WASM 1.0 MVP defines exactly **172** instructions. The byte range 0x00–0xBF
contains gaps (e.g. 0x06–0x0A, 0x12–0x1F, 0x25–0x27) that are
reserved/unassigned. The "~183" figure sometimes cited includes post-MVP
proposals (SIMD, bulk-memory, threads) that use a 0xFC prefix encoding.
