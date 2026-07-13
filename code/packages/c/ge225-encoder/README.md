# ge225-encoder (C)

A **pure GE-225 instruction encoder** in ISO C17. A faithful port of the Rust
[`ge225-encoder`](../../rust/ge225-encoder) crate — the encoding tables for the
GE-225 (1959), the General Electric mainframe at Dartmouth College where
**Dartmouth BASIC** was designed in 1964. (A nice companion to the
[`ibm704-encoder`](../ibm704-encoder), the machine Lisp was born on.)

It owns opcode constants and `encode_*` helpers and nothing else — no IR
knowledge — so a simulator, decoder, or fuzzer can reuse it.

## Word packing

Each 20-bit instruction word is emitted as **3 bytes**, big-endian, the top 4
bits of byte 0 always zero:

```
byte 0: 0000 OOOO   (4-bit opcode nibble)
byte 1: high 8 bits of immediate / address
byte 2: low 8 bits (for STA/LD/ADD/SUB the low nibble holds the register index)
```

| nibble | mnemonic | word |
|--------|----------|------|
| `0x0` | `HLT`   | `[0x00,0x00,0x00]` |
| `0x1` | `LDA n` | `[0x01, hi, lo]` |
| `0x2` | `STA r` | `[0x02, 0x00, r]` (exchange) |
| `0x3` | `LD r`  | `[0x03, 0x00, r]` |
| `0x4`..`0x5` | `ADD r` / `SUB r` | register ops |
| `0x6`..`0x9`, `0xB` | `BR`/`BNZ`/`BZ`/`JSR`/`BMI a` | branches |
| `0xA` | `RTS` | `[0x0A,0x00,0x00]` |

## API

```c
#include "ge225_encoder.h"

uint8_t w[3];
ge225_encode_lda(5, w);        /* {0x01,0x00,0x05} */
ge225_encode_sta(3, w);        /* {0x02,0x00,0x03} */
ge225_encode_br(0xABCD, w);    /* {0x06,0xAB,0xCD} */

uint8_t op; uint16_t payload;
ge225_decode_word(w, &op, &payload);   /* op=0x06, payload=0xABCD */
```

- `ge225_encode_lda`, `_sta`, `_ld`, `_add`, `_sub`, `_br`, `_bnz`, `_bz`,
  `_bmi`, `_jsr` — each writes 3 bytes into `out[3]`.
- `ge225_decode_word` — the inverse (strips byte 0's high nibble).
- Constants: the `GE225_*_OPCODE_NIBBLE`s, `GE225_HALT_WORD` / `GE225_RTS_WORD`,
  and the `GE225_LDA_*` / `GE225_GP_REGISTER_COUNT` capacity values.

## Building

Builds through the shared [`iso-harness`](../iso-harness) engine under every ISO
C compiler on `PATH` with `-std=c17 -pedantic-errors -Wall -Wextra -Werror`:

```sh
sh BUILD          # POSIX: gcc and/or clang
```

Each compiler prints `N checks, 0 failed`.
