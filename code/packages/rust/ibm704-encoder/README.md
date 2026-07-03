# ibm704-encoder

Pure-Rust IBM 704 instruction encoder.  Mirror of
`ge225-encoder` / `intel4004-encoder` / `armv7-encoder` /
`intel8008-encoder` / `riscv-encoder`.

L4 of the McCarthy Lisp implementation — see
[`MCCARTHY-LISP-PLAN.md`](../../../specs/MCCARTHY-LISP-PLAN.md).

## Why the IBM 704?

The IBM 704 (1954) is the vacuum-tube mainframe John McCarthy
and his MIT students (Steve Russell, Tim Hart, Mike Levin) ran the
**first Lisp implementation** on, in 1959.  `CAR` and `CDR` — the
two universal Lisp accessors — were literally IBM 704 instruction
mnemonics:

* **C**ontents of **A**ddress part of **R**egister
* **C**ontents of **D**ecrement part of **R**egister

This crate lets us round-trip McCarthy Lisp source back to the
silicon it was born on — the symmetric counterpart of the
Dartmouth BASIC → GE-225 round-trip the migration already
established.

## Word format

The 704 has 36-bit words.  This encoder uses a clearly
documented idealised layout sufficient for the minimal-viable
McCarthy compile target:

| Word bits | Field | Notes |
|-----------|-------|-------|
| 35..27 (9) | Opcode | e.g. `HTR=0o420`, `CLA=0o500` |
| 26..15 (12) | (zero) | tag + decrement + unused; not used in v0.1.0 |
| 14..0 (15) | Address Y | 15-bit address (≤ 32 K word memory) |

## Wire format

5 bytes per 36-bit word, low byte first, high 4 bits of the
top byte always zero.  Matches the GE-225 precedent (20-bit
words → 3 bytes) extended to 36 bits.
