# wasm-wast-parser

Parses the WebAssembly **text** format (`.wat` modules and the official spec
testsuite's `.wast` script dialect) into `wasm-types::WasmModule` and a
sequence of test directives. Hand-built, zero third-party dependencies —
matching this repo's `wasm-module-parser`, which hand-rolls the *binary*
format parser instead of using the `wasmparser` crate.

Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) monorepo,
a ground-up implementation of the computing stack from transistors to operating systems.

## Why this exists

Every other WASM crate here speaks the **binary** format only. The official
WebAssembly spec testsuite ships almost entirely as `.wast` text files — this
repo could not run it without a text-format front end, and none existed. See
[`code/specs/W05-wasm-conformance-harness.md`](../../../specs/W05-wasm-conformance-harness.md)
for the full design this crate is Phase A of.

## Where it fits in the stack

```
wasm-leb128       ←── variable-length integer encoding (encode_unsigned/encode_signed)
wasm-types        ←── WasmModule struct and all sub-types (the OUTPUT shape)
wasm-opcodes      ←── mnemonic -> opcode/immediate-shape lookup
wasm-wast-parser  ←── THIS CRATE: .wat/.wast text -> WasmModule + script directives
wasm-conformance  ←── consumes this crate to run the official testsuite
```

## What it does

- **`.wat` module text** (`(module ...)`) parses to exactly the same
  `WasmModule` shape `wasm-module-parser` would produce from binary bytes —
  including **encoding** every function body straight to raw WASM bytecode.
  Nothing downstream (the interpreter, the validator) needs to know whether a
  module came from text or binary.
- **`.wast` script directives** — `register`, `invoke`, `assert_return`,
  `assert_trap`, `assert_exhaustion`, `assert_invalid`, `assert_malformed`,
  `assert_unlinkable` — parse to a typed [`script::Directive`] per top-level
  form. A plain `(module ...)` is built eagerly; `assert_invalid`/
  `assert_malformed`'s module is kept as a **raw, unparsed S-expression**,
  since failing to build it is exactly what those two directives test for.

## Grammar coverage

Driven by what the real testsuite's `.wast` files actually use (see the
crate's own module-level doc comments for the full list): folded instruction
syntax (`(i32.add (i32.const 1) (local.get 0))`), symbolic identifiers
(`$name`) in every index space with correct per-scope resolution, implicit
function-type deduplication, hex float literals (`0x1.8p3`, bit-exact) and
`nan:0x<payload>`, nestable block comments, and both flat and folded
instruction forms for the same program — WAT allows either, and this crate
must produce byte-identical output regardless of which one a file uses.

## Usage

```rust
// A plain .wat module.
let module = wasm_wast_parser::parse_module(
    "(module (func (export \"add\") (param i32 i32) (result i32) \
       local.get 0 local.get 1 i32.add))"
).unwrap();

// A .wast conformance-testsuite script.
let directives = wasm_wast_parser::parse_script(r#"
    (module (func (export "add") (param i32 i32) (result i32)
      local.get 0 local.get 1 i32.add))
    (assert_return (invoke "add" (i32.const 1) (i32.const 2)) (i32.const 3))
"#).unwrap();
```

## Testing

`cargo test -p wasm-wast-parser` — unit tests per module (`tokenizer`,
`sexpr`, `numeric`, `module`, `script`), each targeting the specific grammar
corner it owns. Several tests exist specifically to pin down bugs found
during development (e.g. folded `call`/`br` with a value argument, where the
instruction's own index/label is the *first* arg and any operand
sub-expressions trail it — the opposite order from what a naive
"immediates always come last" assumption would produce).
