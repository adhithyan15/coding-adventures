# rans (C)

A pure ISO **C17** implementation of table-based **rANS** (range Asymmetric
Numeral Systems) entropy coding. A faithful port of the Rust `rans` crate.

It compiles clean under **GCC, Clang, and MSVC** with `-std=c17
-pedantic-errors -Wall -Wextra -Werror` (and `/std:c17 /permissive- /W4 /WX` on
MSVC), via the shared [`iso-harness`](../iso-harness/). Standard library only.

## What it is

rANS is the modern entropy coder used by Zstandard and JPEG XL: it codes a
stream of alphabet symbols against a fixed frequency table, approaching Shannon
entropy with integer-only arithmetic.

- **AnsTable** — built from raw symbol counts. The counts are normalised
  (largest-remainder method) so their frequencies sum to a power of two
  M = 2^k (M ≥ alphabet size, M ≤ 2^16); a flat M-entry decode table gives O(1)
  lookup.
- **RansEncoder** — `put` symbols in **reverse** order (rANS is LIFO), then
  `finish` for the byte stream (an 8-byte big-endian state + renorm bytes).
- **RansDecoder** — `get` symbols back in forward order.

## API

```c
#include "rans.h"

unsigned int counts[] = {3, 1};        /* symbol 0: 3/4, symbol 1: 1/4 */
AnsTable t;
ans_table_new(counts, 2, &t);          /* t.m == 4 */

RansEncoder enc; rans_encoder_init(&enc, &t);
/* encode [0,0,1,0] by pushing in reverse: */
rans_encoder_put(&enc, 0); rans_encoder_put(&enc, 1);
rans_encoder_put(&enc, 0); rans_encoder_put(&enc, 0);
unsigned char *bytes; size_t len;
rans_encoder_finish(&enc, &bytes, &len);

RansDecoder dec; rans_decoder_init(&dec, &t, bytes, len);
unsigned char s = rans_decoder_get(&dec);   /* 0, 0, 1, 0 */
free(bytes); ans_table_free(&t);
```

All calls return `RANS_OK` or a `RANS_ERR_*` status. Output buffers are malloc'd
(free them); allocations use `calloc`'s checked multiply. The decode table is
constructed so the decoder stays in bounds and never underflows for **any**
input state — a malformed byte stream cannot cause an out-of-bounds access.
Arithmetic is 64-bit (no 128-bit integers, no libm).

## Building & testing

```sh
sh BUILD          # POSIX: compiles + runs the tests under every compiler found
```

Tests cover the crate's table vectors (M, log2m, freq/cumfreq, power-of-two
sum), error cases, short-data rejection, symbol round trips (including skewed
distributions), and determinism.
