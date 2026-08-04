# ibm704-encoder (C)

A **pure IBM 704 instruction encoder** in ISO C17. A faithful port of the Rust
[`ibm704-encoder`](../../rust/ibm704-encoder) crate — an encoder for the
IBM 704 (1954), the vacuum-tube mainframe on which John McCarthy and his MIT
students first ran **Lisp** in 1959.

## Why the 704?

`CAR` and `CDR` — Lisp's two universal accessors — were literally IBM 704
instruction-word field names:

- **CAR** = **C**ontents of the **A**ddress part of **R**egister
- **CDR** = **C**ontents of the **D**ecrement part of **R**egister

The 704's 36-bit word split into prefix / decrement / tag / address fields, and
a cons cell fit one per word; `(CAR x)` extracted the address half, `(CDR x)`
the decrement half. The names stuck. Encoding to real 704 words lets McCarthy
Lisp round-trip to the silicon it was designed for.

## Word format (idealised, v0.1.0)

A simplified layout sufficient for a minimal Lisp compile target:

| word bits | field | notes |
|-----------|-------|-------|
| 35..27 (9)  | opcode  | `HTR = 0o420`, `CLA = 0o500` |
| 26..15 (12) | zero    | tag + decrement + unused (not used yet) |
| 14..0 (15)  | address | 15-bit address (≤ 32 K words) |

36 bits don't divide evenly into 8, so each word packs into **5 bytes** (40
bits, 4 wasted), low byte first, the top byte's high nibble always zero.

## API

```c
#include "ibm704_encoder.h"

/* McCarthy's canonical "42" program: CLA 42 ; HTR 0 */
uint64_t cla_42 = ibm704_encode_cla(42);   /* 0xA_0000_002A (36-bit word) */
uint64_t htr_0  = ibm704_encode_htr(0);    /* 0x8_8000_0000 */

uint8_t bytes[5];
ibm704_pack_word(cla_42, bytes);           /* {0x2A,0x00,0x00,0x00,0x0A} */
/* IBM704_HTR_HALT_BYTES == pack_word(encode_htr(0)) == {0,0,0,0x80,0x08} */
```

- `ibm704_encode_instruction(opcode, address)` and the named helpers
  `ibm704_encode_htr` / `ibm704_encode_cla` → a 36-bit word (address out of the
  15-bit range is masked, never an error).
- `ibm704_pack_word(word, out5)` → the 5-byte little-endian wire form.
- Constants: `IBM704_HTR`, `IBM704_CLA`, `IBM704_WORD_BITS`, `IBM704_WORD_MASK`,
  `IBM704_BYTES_PER_WORD`, `IBM704_ADDR_BITS`, `IBM704_ADDR_MASK`,
  `IBM704_OPCODE_SHIFT`, and `IBM704_HTR_HALT_BYTES`.

## Building

Builds through the shared [`iso-harness`](../iso-harness) engine under every ISO
C compiler on `PATH` with `-std=c17 -pedantic-errors -Wall -Wextra -Werror`:

```sh
sh BUILD          # POSIX: gcc and/or clang
```

Each compiler prints `N checks, 0 failed`.
