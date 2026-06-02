# iir-to-ge225

IIR → GE-225 machine code backend.

Lowers an `interpreter_ir::IIRModule` to a `Vec<u8>` of encoded
20-bit GE-225 instruction words (packed 3 bytes per word, big-endian,
with the top 4 bits of byte 0 zero).

## What's this for?

The **GE-225** (1959) was the General Electric mainframe at Dartmouth
College where **John Kemeny and Thomas Kurtz designed Dartmouth BASIC
in 1964**. BASIC ran on this very machine — and BASIC's defaults
(line numbers, 1-indexed arrays, single-letter variables) still bear
the imprint of this 20-bit, accumulator-based mainframe.

This crate is the **fifth architecture backend** in the LANG VM
pipeline:

| | Width | Year | Primary fit |
|---|---|---|---|
| iir-to-riscv (A1) | 32-bit | 2015 | generic |
| iir-to-intel8008 (A2) | 8-bit | 1972 | Oct |
| iir-to-armv7 (A3) | 32-bit | 2005 | phone-class targets |
| iir-to-intel4004 (A4) | 4-bit | 1971 | Brainfuck |
| **iir-to-ge225 (A5)** | **20-bit** | **1959** | **Dartmouth BASIC** |

## Status — v0.4.0 (A5+++ accumulator arithmetic)

| IIR op | GE-225 lowering |
|--------|-----------------|
| `const dest, Int(n)` | `(STA r_evict)?` + `LDA n` |
| `mov dest, src` | `(STA r_evict_src)?` + `LD r_src` + `STA r_dest` |
| `add dest, lhs, rhs` | (evict ACC pieces)? + `LD r_lhs` + `ADD r_rhs` |
| `sub dest, lhs, rhs` | (evict ACC pieces)? + `LD r_lhs` + `SUB r_rhs` |
| `ret <var>` | `(LD r_var)?` + `HLT` |
| `ret_void` | `HLT` |

17-slot pool (ACC + r0..r15).  Trivial-case 6-byte ROM for
`const v; ret v` preserved from v0.2.0.

### Cumulative opcode map

| Nibble | Mnemonic | Word | Effect |
|--------|----------|------|--------|
| `0x0` | `HLT`   | `[0x00, 0x00, 0x00]` | halt |
| `0x1` | `LDA n` | `[0x01, hi, lo]` | ACC ← n |
| `0x2` | `STA r` | `[0x02, 0x00, r]` | ACC ↔ r (XCH semantics) |
| `0x3` | `LD r`  | `[0x03, 0x00, r]` | ACC ← r |
| `0x4` | `ADD r` | `[0x04, 0x00, r]` | ACC ← ACC + r |
| `0x5` | `SUB r` | `[0x05, 0x00, r]` | ACC ← ACC - r |

Branches and call/return follow in A5++++.

## Quick start

```rust
use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use iir_to_ge225::{validate_for_ge225, lower_iir_to_ge225, IIRGe225Config};

// const v=5; ret v
let f = IIRFunction::new("five", vec![], "i16", vec![
    IIRInstr::new("const", Some("v".into()), vec![Operand::Int(5)], "i16"),
    IIRInstr::new("ret",   None,             vec![Operand::Var("v".into())], "i16"),
]);
let module = IIRModule {
    name: "demo".into(),
    functions: vec![f],
    entry_point: Some("five".into()),
    language: "demo".into(),
    exports: vec![],
    imports: vec![],
};

assert!(validate_for_ge225(&module).is_empty());

let bytes = lower_iir_to_ge225(&module, &IIRGe225Config::default())
    .expect("lowering should succeed");
// LDA 5 + HLT = 6 bytes.
assert_eq!(bytes, vec![0x01, 0x00, 0x05, 0x00, 0x00, 0x00]);
```

## Word packing

Each 20-bit GE-225 word is emitted as 3 bytes (24 bits), big-endian,
with the top 4 bits of byte 0 always zero:

```text
byte 0: 0000 BBBB   (top 4 bits zero + bits 19..16 of word)
byte 1: BBBB BBBB   (bits 15..8 of word)
byte 2: BBBB BBBB   (bits 7..0 of word)
```

A downstream simulator reads 3 bytes per instruction, masks off the
top 4 bits, and recovers the original 20-bit word.

## See also

- Spec: `code/specs/iir-to-ge225.md`
- Plan: `code/specs/MULTILANG-ARCHITECTURE-BACKENDS.md` §A5
- Sister crates: `iir-to-riscv`, `iir-to-intel8008`, `iir-to-armv7`,
  `iir-to-intel4004`
