# wasm-module-parser

Parse raw `.wasm` binary bytes into a structured `WasmModule`. No execution — pure decoding.

Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) monorepo,
a ground-up implementation of the computing stack from transistors to operating systems.

## What it does

A WebAssembly (WASM) binary file is a compact, structured sequence of bytes. This crate
decodes those bytes — header, sections, and payloads — into the typed `WasmModule` struct
from `wasm-types`, ready for validation, interpretation, or further analysis.

As of 0.2.0 the type section also decodes **WasmGC struct types** (`0x50 … 0x5F …`)
into `WasmModule.struct_types` — e.g. the `$LispyPair` cons cell emitted for
McCarthy Lisp — alongside the usual function types, including `anyref` / `i31ref`
/ concrete `structref` field types (LANG77 / McCarthy L3b-3a-3c).

## Where it fits in the stack

```
wasm-leb128      ←── variable-length integer decoding
wasm-types       ←── WasmModule struct and all sub-types
wasm-opcodes     ←── opcode constants (used transitively)
wasm-module-parser  ←── THIS CRATE: binary → WasmModule
wasm-simulator   ←── execution (depends on this crate)
```

## Usage

```rust
use wasm_module_parser::WasmModuleParser;

// Parse from raw bytes (e.g., std::fs::read("module.wasm"))
let bytes: &[u8] = &[0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00];
match WasmModuleParser::parse(bytes) {
    Ok(module) => {
        println!("types:    {}", module.types.len());
        println!("imports:  {}", module.imports.len());
        println!("exports:  {}", module.exports.len());
        println!("functions:{}", module.functions.len());
        println!("code:     {}", module.code.len());
    }
    Err(e) => eprintln!("parse error at byte {}: {}", e.offset, e.message),
}
```

## WASM Binary Format (overview)

```
┌──────────────────────────────────────────────────────────────────┐
│  Magic: 0x00 0x61 0x73 0x6D  ("asm")                            │
│  Version: 0x01 0x00 0x00 0x00                                    │
├──────┬───────────────────────────────────────────────────────────┤
│ §  0 │ Custom  — tool metadata (debug names, source maps, DWARF) │
│ §  1 │ Type    — function signature pool                         │
│ §  2 │ Import  — things needed from the host                     │
│ §  3 │ Function— type indices for local functions                │
│ §  4 │ Table   — function reference tables                       │
│ §  5 │ Memory  — linear memory declarations                      │
│ §  6 │ Global  — module-level globals with init expressions      │
│ §  7 │ Export  — names exposed to the host                       │
│ §  8 │ Start   — optional auto-called function index             │
│ §  9 │ Element — table initialisation data                       │
│ § 10 │ Code    — function bodies (locals + bytecode)             │
│ § 11 │ Data    — memory initialisation data                      │
└──────┴───────────────────────────────────────────────────────────┘
```

## Malformed-binary rejection

This crate rejects, not just accepts — a real robustness property, since a
malformed-but-parsed module could smuggle unintended behavior past parsing.
As of 0.2.10 it correctly rejects:

- **Overlong LEB128**: more bytes than a field's bit width allows (e.g. a
  6-byte encoding of a `u32` field — the max is 5).
- **Out-of-range LEB128**: byte count is fine, but the value's padding bits
  don't zero-extend (unsigned) or sign-extend (signed) correctly — e.g. a
  `u32` field encoding `2^32`.
- **Truncated streams**: a LEB128 sequence, string, or section that runs
  out of bytes mid-field.
- **Unrecognized section ids**, and numbered sections that repeat or
  appear out of the required canonical order.
- **Section-size mismatches**: bytes left over after a section's own
  parser runs, still inside its declared boundary.
- **Function/code section length mismatches**, **malformed `limits`
  flags**, and a **function body not ending in `end` (0x0B)**.
- An unreasonably large **total locals count** in a single function body
  (a DoS guard, not a spec rule — a few attacker-controlled bytes
  shouldn't be able to request billions of allocated locals).

Deliberately NOT yet caught: anything living inside a function body's
bytecode itself (a `memarg`'s `align`/`offset` LEB128 fields, an illegal
opcode, unbalanced block/loop/if nesting) — this crate reads a function
body as a raw byte blob after its locals, deferring instruction-level
decoding to a future validator. See `CHANGELOG.md`'s `0.2.10` entry for
the full "deliberately not fixed" list.

## Error handling

`WasmModuleParser::parse` returns `Result<WasmModule, WasmParseError>`. The error type
carries a `message` (human-readable description) and an `offset` (byte position in the
input where the error was detected).

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct WasmParseError {
    pub message: String,
    pub offset: usize,
}
```

## Dependencies

- `wasm-leb128` — LEB128 unsigned integer decoding
- `wasm-types` — `WasmModule` and all sub-types (`FuncType`, `Import`, `Export`, etc.)
- `wasm-opcodes` — opcode constants (transitive)

## Development

```bash
# Run tests
cargo test -p wasm-module-parser -- --nocapture

# Check for lints
cargo clippy -p wasm-module-parser

# Coverage (requires cargo-tarpaulin)
cargo tarpaulin -p wasm-module-parser --out stdout
```
