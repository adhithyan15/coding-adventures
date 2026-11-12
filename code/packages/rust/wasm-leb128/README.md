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

### Bounded decoding (`decode_unsigned_bounded` / `decode_signed_bounded`)

`decode_unsigned`/`decode_signed` decode into a full 64-bit value — right
for a genuinely 64-bit WASM field, but the overwhelming majority of the
binary format's integers are `u32` (every section size, vector count,
index, and string length). Narrowing the result yourself (`value as u32`)
would silently accept a value too big to fit instead of rejecting it — use
the `_bounded` variants instead, which take a `max_bits` width and enforce
the WASM spec's own two malformed-encoding rules for that width:

```rust
use wasm_leb128::{decode_unsigned_bounded, decode_signed_bounded};

// Non-minimal but in-budget (5 bytes is the max for a 32-bit field): fine.
let (value, n) = decode_unsigned_bounded(&[0x82, 0x80, 0x80, 0x80, 0x00], 0, 32)?;
assert_eq!((value, n), (2, 5));

// Overlong: one byte past the 32-bit budget -- "integer representation too long".
assert!(decode_unsigned_bounded(&[0x82, 0x80, 0x80, 0x80, 0x80, 0x00], 0, 32).is_err());

// Out of range: in-budget byte count, but the value (2^32) doesn't fit in 32
// bits -- "integer too large".
assert!(decode_unsigned_bounded(&[0x80, 0x80, 0x80, 0x80, 0x10], 0, 32).is_err());

// Signed: padding bits above `max_bits` must repeat the sign bit, not just be zero.
let (neg, n) = decode_signed_bounded(&[0xFF, 0x7F], 0, 32)?; // i32.const -1
assert_eq!((neg, n), (-1, 2));
```

`decode_unsigned` and `decode_signed` are themselves just
`decode_{unsigned,signed}_bounded(.., 64)` — a `u64`/`i64` is a 64-bit-wide
`uN`/`sN`, so they get the exact same overlong/out-of-range protection at
the native width, not a separate code path.

## Error Handling

```rust
let result = decode_unsigned(&[0x80, 0x80], 0); // unterminated
assert!(result.is_err());
println!("{}", result.unwrap_err()); // "LEB128 error at offset 0: unexpected end of data …"
```

`max_bits` must be at most 64 (every WebAssembly width is 1..=64). A wider
width is rejected with an `Err` before any byte is read, because the decoder
accumulates into a `u64`:

```rust
assert!(wasm_leb128::decode_unsigned_bounded(&[0x01], 0, 65).is_err());
```

## Verification

The six public operations are specified by
[`code/specs/wasm-leb128.warrant`](../../../specs/wasm-leb128.warrant), a
Warrant v1 document, with the prose contract in
[LEB01](../../../specs/LEB01-wasm-leb128.md). `tests/generated/warrant_evidence.rs`
is *generated* from it and `evidence.bindings` (never edit it; `BUILD` fails if
it drifts) and runs against the real exports:

- **examples:** the 624485 vector, the `u32`/`u64`/`i32`/`i64` extremes, a
  padded zero, `2^64`/`2^32`/`2^31` too large, an eleven-byte overlong input,
  truncation, empty input, offset out of range, width 0;
- **exhaustive suites:** every input of length 0 to 3 for each 64-bit decoder
  (16,843,009 inputs each), every one- and two-byte input at every width
  0..=64 for each bounded decoder, the ten-byte boundary, round trips over the
  `U64`/`I64` edge sets (also after every prefix and with every suffix of the
  `Bytes` domain), and the encoders against independent canonical models;
- **property and fuzz suites** for every claim, and **sabotage controls** that
  inject wrong consumed counts, dropped counts, success reported as an error,
  a missing error, a flipped result bit and a dropped last byte, and must fail.

`evidence/test-manifest.json` is written by that run; `assurance_report.json`
is derived from it (verdict `report-only`). What is *not* claimed: the
step-bound proofs are **not live** (the WIR front end cannot parse
`src/lib.rs`; `evidence/proof-manifest.json` records the blocker), the error
kind and offset are not observed (`Leb128Error` has no kind, so every error is
one Warrant case), and CWE-208 is recorded as not applicable because every
operand is public.

## Development

```bash
# Run tests (unit and doc tests; the generated evidence target is separate)
cargo test -p wasm-leb128 --lib
cargo test -p wasm-leb128 --doc

# Regenerate the Warrant evidence harness after editing the .warrant document
cargo run -p coding_adventures_warrant_grammar --example gen_evidence -- wasm-leb128

# Lint
cargo clippy -p wasm-leb128

# Everything CI runs, including the evidence, sabotage and report checks
bash BUILD
```

## Design Notes

- Uses `u64`/`i64` internally, which handles both 32-bit and 64-bit WASM values.
- Decoding works at an arbitrary byte offset — no need to slice the input first.
- All code is written in Knuth-style literate programming: algorithm walkthroughs,
  visual traces, and examples live inline with the source.
