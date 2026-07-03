# `ibm704-encoder` spec

> **Status:** v0.1.0 — L4 of the McCarthy Lisp implementation,
> 2026-06-11.

## Purpose

Pure-Rust encoder for the IBM 704 (1954) — the vacuum-tube
mainframe John McCarthy and his MIT students first ran Lisp
on in 1959.  Has no IR knowledge — its job is to turn an
opcode + 15-bit address into a 36-bit IBM 704 instruction word,
plus pack that word into 5 bytes on disk.

Mirror of `aarch64-encoder` / `x86_64-encoder` / `ge225-encoder` /
`intel4004-encoder` / `armv7-encoder` / `intel8008-encoder` /
`riscv-encoder`.

## Public surface

### Opcode constants

| Constant | Value | Mnemonic |
|----------|-------|----------|
| `HTR` | `0o420` | **H**alt and **T**ransfe**R** — used as the canonical halt sentinel via `HTR 0` |
| `CLA` | `0o500` | **CL**ear accumulator and **A**dd — used to load a 15-bit immediate into AC |

### Word geometry

| Constant | Value | Meaning |
|----------|-------|---------|
| `WORD_BITS` | `36` | one IBM 704 word = 36 bits |
| `WORD_MASK` | `0xF_FFFF_FFFF` | covers exactly the 36 valid word bits |
| `BYTES_PER_WORD` | `5` | wire-format bytes per word (40 bits, top 4 wasted) |
| `ADDR_BITS` | `15` | 32 K word address space (~144 KB) |
| `ADDR_MASK` | `0x7FFF` | 15-bit address mask |
| `OPCODE_SHIFT` | `27` | opcode occupies bits 35..27; address bits 14..0 |

### Encoder functions

| Function | Returns | Purpose |
|----------|---------|---------|
| `encode_instruction(op, addr)` | `u64` | generic word builder |
| `encode_htr(addr)` | `u64` | `HTR addr` — opcode 0o420 + 15-bit address |
| `encode_cla(addr)` | `u64` | `CLA addr` — opcode 0o500 + 15-bit address |
| `pack_word(word)` | `[u8; 5]` | serialise a 36-bit word LSB-first, top byte's high nibble = 0 |

### Canonical byte constants

| Constant | Value | Meaning |
|----------|-------|---------|
| `HTR_HALT_BYTES` | `[0x00, 0x00, 0x00, 0x80, 0x08]` | pre-computed `HTR 0` — `ibm704-backend`'s halt sentinel |

## Word layout (v0.1.0 idealised form)

```text
bit 35              27 26              15 14               0
+-------------------+------------------+------------------+
|   opcode (9 bits) |  (zero, 12 bits) | address (15 bits)|
+-------------------+------------------+------------------+
```

The actual 1959-era IBM 704 ISA was richer (Type A / Type B
formats, prefix + decrement + tag fields).  v0.1.0 uses this
simplified layout — enough for the minimal-viable
`const_*`/`ret_*` backend scope.  A future increment can add
the Type A prefix + decrement fields for richer ISA coverage.

## Wire format

Each 36-bit word is packed as 5 bytes (40 bits — 4 wasted
padding bits zeroed in the top nibble of the high byte), low
byte first.  Matches the GE-225 precedent (20-bit words → 3
bytes) extended to 36 bits.

## Why this is L4 of McCarthy Lisp (not part of the historical-arch migration)

The historical-arch migration shipped GE-225, Intel 4004, Intel
8008, ARMv7, and RV32I — the "modern" historical lineup
(transistors and after).  The IBM 704 is older still — vacuum
tubes, 1954 — and earned its place by hosting the **first Lisp
implementation** at MIT in 1959.  `CAR` and `CDR` are literal
704 instruction-word field names; this encoder lets McCarthy
Lisp source compile back to that silicon.

## Tests (13 byte-pinned unit tests)

* Opcode constants pinned: `HTR = 0o420`, `CLA = 0o500`.
* Word geometry constants pinned.
* `encode_htr(0) == 0x8_8000_0000` and the bit math is verified.
* `encode_cla(42) == 0xA_0000_002A` (Twig 42 first instruction).
* Address mask drops oversize values silently.
* `pack_word` is LSB-first.
* Top byte's high nibble is always zero (5-byte-per-36-bit-word invariant).
* Stray high bits above word are masked.
* `HTR_HALT_BYTES` matches `pack_word(encode_htr(0))`.
* End-to-end byte sequence for `CLA 42; HTR 0` (Twig 42 program) pinned.

## Out of scope

* IBM 704 Type A instructions (decrement-field-using ops).
* Floating-point ops (the 704 had hardware FP).
* Index registers, address modification, indirect addressing.
* Disassembly / simulation — a future `ibm704-simulator`
  crate would handle both.
* The 9 distinct "decrement" / "tag" interpretation rules of
  the original 36-bit word format.
