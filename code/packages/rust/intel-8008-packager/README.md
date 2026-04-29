# intel-8008-packager

Intel HEX ROM image encoder/decoder for the Intel 8008.  The final stage
of the Oct → Intel 8008 compiler pipeline in `coding-adventures`.

---

## Pipeline position

```
Oct source (.oct)
  → oct-lexer, oct-parser, oct-type-checker
  → oct-ir-compiler
IrProgram
  → intel-8008-ir-validator
  → ir-to-intel-8008-compiler
8008 Assembly text (.asm)
  → intel-8008-assembler             ← feeds THIS crate
Binary bytes
  → intel-8008-packager              ← THIS crate
Intel HEX file (.hex)
  → intel8008-simulator
```

---

## What is Intel HEX?

Intel HEX is a text-based file format invented in the early 1970s for loading
programs into EPROM chips using a "programmer" device.  The format has
survived 50+ years unchanged and is understood by every EPROM programmer on
the market (hardware and software).

Each line is a "record":

```
:LLAAAATTDDDDDD...CC

  :     — start code
  LL    — byte count (2 hex digits)
  AAAA  — load address (4 hex digits, big-endian)
  TT    — record type: 00 = Data, 01 = End Of File
  DD    — data bytes (2 hex digits each)
  CC    — checksum: (0x100 - sum_of_all_bytes) % 256
```

### Checksum

`checksum = (0x100 - (LL + AAAA_hi + AAAA_lo + TT + all_DD) % 256) % 256`

The checksum is chosen so that summing **all** bytes in the record (including
the checksum itself) yields 0x00 mod 256.  ROM programmer firmware verifies
integrity by summing each record: non-zero means corruption.

### Intel 8008 address space

```
0x0000–0x1FFF   ROM: program code (8 KB)
0x2000–0x3FFF   RAM: static variable data (8 KB)
```

Maximum image size: 16 384 bytes (16 KB = full 8008 address space).

---

## Usage

```rust
use intel_8008_packager::{encode_hex, decode_hex, PackagerError};

// Binary → Intel HEX (e.g. write to .hex file for EPROM programmer)
let binary = vec![0x06u8, 0x00, 0xFF];  // MVI B, 0; HLT
let hex_text = encode_hex(&binary, 0).unwrap();
println!("{hex_text}");
// :030000000600FF...
// :00000001FF

// Intel HEX → binary (round-trip / loading into simulator)
let decoded = decode_hex(&hex_text).unwrap();
assert_eq!(decoded.origin, 0x0000);
assert_eq!(decoded.binary, binary);

// With a non-zero origin (e.g. code loaded at 0x0100)
let hex_at_100 = encode_hex(&binary, 0x0100).unwrap();
let decoded2 = decode_hex(&hex_at_100).unwrap();
assert_eq!(decoded2.origin, 0x0100);
```

---

## Error handling

Both functions return `Err(PackagerError)` on invalid input:

| Situation | Error |
|-----------|-------|
| Empty binary | "binary must be non-empty" |
| Origin > 0xFFFF | "origin must be 0–65535, got …" |
| origin + len > 65536 | "image overflows 16-bit address space" |
| Line longer than 1024 chars | "line N: line too long (M chars, maximum 1024)" |
| Missing `:` | "line N: expected ':'" |
| Non-hex characters | "line N: invalid hex data" |
| Record too short | "line N: record claims X data bytes but …" |
| Checksum mismatch | "line N: checksum mismatch (expected …, got …)" |
| Unsupported type (≥ 0x02) | "line N: unsupported record type 0xNN" |
| Image > 16 KB | "decoded image too large: N bytes (maximum …)" |
| Missing EOF record | "missing EOF record (type 0x01) — file may be truncated" |
| Overlapping records (in-order) | "line N: record at 0xAAAA overlaps previous record (ends at 0xBBBB)" |
| Overlapping records (out-of-order) | "line N: record at 0xAAAA (ends at 0xBBBB) overlaps next record at 0xCCCC" |

---

## Tests

```
cargo test -p intel-8008-packager
```

43 tests (38 unit + 5 doc-tests), all passing. Zero external dependencies.
