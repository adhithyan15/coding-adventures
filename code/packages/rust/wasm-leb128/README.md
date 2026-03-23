# wasm-leb128

LEB128 (Little-Endian Base-128) variable-length integer encoding for the
WebAssembly binary format.

## What is LEB128?

LEB128 packs 7 bits of data into each byte and uses the high bit (bit 7) as a
"more bytes follow" flag. Small numbers fit in one byte; large numbers use more.
This keeps the WASM binary format compact — most integers in a WASM module are
small.

```
Byte layout:
  bit 7 (MSB): continuation flag  — 1 = more bytes follow
  bits 0–6   : 7 bits of payload data
```

### Encoding example: 624485 (unsigned)

```
624485 = 0b10011000011101100101
Split into 7-bit groups (LSB first):
  group 0: 1100101 = 0x65  → with flag: 0xE5
  group 1: 0001110 = 0x0E  → with flag: 0x8E
  group 2: 0100110 = 0x26  → last byte: 0x26
Result: [0xE5, 0x8E, 0x26]
```

## Where Does This Fit?

This crate is part of the `wasm-*` family in the coding-adventures monorepo:

```
wasm-leb128         ← this package (integer encoding primitives)
wasm-types          ← WASM type system (i32, i64, f32, f64, funcref …)
wasm-opcodes        ← opcode definitions
wasm-module-parser  ← full binary module parser (uses all of the above)
wasm-simulator      ← execution engine
```

## API

```rust
use wasm_leb128::{decode_unsigned, decode_signed, encode_unsigned, encode_signed, Leb128Error};

// Decode unsigned: returns (value, bytes_consumed)
let (value, n) = decode_unsigned(&[0xE5, 0x8E, 0x26], 0)?;
assert_eq!(value, 624485);
assert_eq!(n, 3);

// Decode signed: sign-extends the final byte
let (neg, n) = decode_signed(&[0x7E], 0)?;
assert_eq!(neg, -2);
assert_eq!(n, 1);

// Encode unsigned
assert_eq!(encode_unsigned(624485), vec![0xE5, 0x8E, 0x26]);

// Encode signed
assert_eq!(encode_signed(-2), vec![0x7E]);

// Non-zero offset
let buf = [0x00, 0x00, 0xE5, 0x8E, 0x26];
let (value, n) = decode_unsigned(&buf, 2)?;
assert_eq!(value, 624485);
```

## Error Handling

```rust
let result = decode_unsigned(&[0x80, 0x80], 0); // unterminated
assert!(result.is_err());
println!("{}", result.unwrap_err()); // "LEB128 error at offset 0: unexpected end of data …"
```

## Development

```bash
# Run tests
cargo test -p wasm-leb128

# Lint
cargo clippy -p wasm-leb128

# Via build script
bash BUILD
```

## Design Notes

- Uses `u64`/`i64` internally, which handles both 32-bit and 64-bit WASM values.
- Decoding works at an arbitrary byte offset — no need to slice the input first.
- All code is written in Knuth-style literate programming: algorithm walkthroughs,
  visual traces, and examples live inline with the source.
