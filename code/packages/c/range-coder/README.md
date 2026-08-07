# range-coder (C)

A pure ISO **C17** implementation of the **VP8 boolean range coder** (RFC 6386
§7). A faithful port of the Rust `range-coder` crate.

It compiles clean under **GCC, Clang, and MSVC** with `-std=c17
-pedantic-errors -Wall -Wextra -Werror` (and `/std:c17 /permissive- /W4 /WX` on
MSVC), via the shared [`iso-harness`](../iso-harness/). Standard library only.

## What it is

A boolean range coder is the binary arithmetic coder at the entropy stage of
VP8 / WebP. It compresses a sequence of bits, each carrying an 8-bit probability
that the bit is 0 (`prob`; 128 ≈ 50/50). Encoding a sequence and then decoding it
with the same probabilities recovers the original bits.

```text
split = 1 + (((range - 1) * prob) >> 8)   // the +1 keeps both sub-intervals non-empty
bit 0 -> lower sub-interval, bit 1 -> upper
```

## API

```c
#include "range_coder.h"

RcBoolEncoder enc;
rc_encoder_init(&enc);
rc_encoder_write_bit(&enc, 1, 128);
rc_encoder_write_bits(&enc, 0xCAFEBABE, 32);
unsigned char *bytes; size_t len;
rc_encoder_finish(&enc, &bytes, &len);

RcBoolDecoder dec;
rc_decoder_init(&dec, bytes, len);
int b = rc_decoder_read_bit(&dec, 128);
unsigned int v = rc_decoder_read_bits(&dec, 32);
free(bytes);
```

- Encoder: `rc_encoder_init` / `_write_bit` / `_write_bits` / `_finish` (hands
  off the malloc'd output; `RC_ERR_ALLOC` on OOM) / `_free` (abandon path).
- Decoder: `rc_decoder_init` (borrows `data`, which must outlive it) / `_read_bit`
  / `_read_bits` / `_is_exhausted`. Bits are MSB-first; an exhausted stream reads
  as zeros.

Arithmetic is done in `uint64_t`/`uint32_t` — no 128-bit integers, no libm.

## Building & testing

```sh
sh BUILD          # POSIX: compiles + runs the tests under every compiler found
```

Tests round-trip single bits, mixed and skewed probability sequences, and
`write_bits`/`read_bits` for 8/16/32-bit fields, plus decoder seeding,
exhaustion, and determinism — from the crate's own tests.
