# reed-solomon (C)

A pure ISO **C17** Reed-Solomon error-correcting codec over GF(2⁸) — the code
behind QR codes, CDs/DVDs, and deep-space communication. A faithful port of the
Rust `reed-solomon` crate.

It compiles clean under **GCC, Clang, and MSVC** with
`-std=c17 -pedantic-errors -Wall -Wextra -Werror` (and `/std:c17 /permissive-
/W4 /WX` on MSVC), via the shared [`iso-harness`](../iso-harness/). Its only
dependency is the sibling [`gf256`](../gf256/) package (the field arithmetic).

## What it does

Encoding appends `n_check` parity bytes so that up to **`t = n_check/2`**
corrupted bytes can be *located and corrected*. A codeword is a GF(2⁸)
polynomial divisible by the generator `g(x) = (x+α¹)…(x+α^{n_check})`; encoding is
systematic (message bytes first, then check bytes).

Decoding runs the classic pipeline:

```text
received → syndromes → Berlekamp-Massey (error locator Λ)
        → Chien search (error positions) → Forney (error magnitudes) → correct
```

```c
#include "reed_solomon.h"

uint8_t msg[5] = {'H','E','L','L','O'};
uint8_t code[16]; size_t clen;
rs_encode(msg, 5, 4, code, &clen);          /* 9 bytes: 5 message + 4 check */

code[2] ^= 0x5A;                            /* a byte gets corrupted... */
uint8_t out[16]; size_t olen;
rs_decode(code, clen, 4, out, &olen);       /* ...and is corrected back to "HELLO" */
```

| Function | Purpose |
| --- | --- |
| `rs_encode` / `rs_decode` | the code (decode corrects up to `t` errors) |
| `rs_build_generator` | the generator polynomial (little-endian) |
| `rs_syndromes` / `rs_error_locator` | decode internals, for inspection |

`rs_decode` returns `RS_TOO_MANY_ERRORS` when more than `t` bytes were corrupted
(rather than silently mis-correcting).

## Implementation notes

- **No heap allocation.** Every polynomial is bounded by the GF(256) block size
  (255 bytes), so all working storage lives in fixed stack buffers; output goes
  into caller-provided buffers (`message_len + n_check` for encode,
  `received_len - n_check` for decode).
- **Field arithmetic from `gf256`.** `# build-tool: deps=c/gf256`; the gf256
  source is compiled into the test binary (the campaign's cross-package pattern).

## Building & testing

```sh
sh BUILD          # POSIX: compiles + runs the tests under every compiler found
```

Tests verify the generator polynomial (`n_check=2` → `[8,6,1]`), encode/decode
round-trips, correction of 1, 2, and 4 errors (at `n_check` = 4 and 8), and that
too many errors are reported.

## Where it fits

Builds on the `gf256` field to add the coding lane to `code/packages/c` (QR/RS is
the classic application of GF(2⁸)).
