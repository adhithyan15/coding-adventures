# lzss (C)

A pure ISO **C17** implementation of the **LZSS** lossless compression algorithm.
A faithful port of the Rust `lzss` crate (CMP02).

It compiles clean under **GCC, Clang, and MSVC** with `-std=c17
-pedantic-errors -Wall -Wextra -Werror` (and `/std:c17 /permissive- /W4 /WX` on
MSVC), via the shared [`iso-harness`](../iso-harness/). Standard library only.

## What it is

LZSS (Storer & Szymanski, 1982) is the sliding-window LZ77 variant behind
DEFLATE and LZ4. At each position it searches the last `window_size` bytes for
the longest match; a match of at least `min_match` bytes becomes a
back-reference `Match{offset, length}`, otherwise a `Literal` byte is emitted.
Matches may overlap the cursor, so runs encode as one short back-reference.

Wire format (CMP02, big-endian): a u32 original length, a u32 block count, then
blocks of a 1-byte flag (bit *b* set ⇒ token *b* is a match) followed by each
token's data (match = 2-byte offset + 1-byte length; literal = 1 byte).

## API

```c
#include "lzss.h"

unsigned char *comp; size_t comp_len;
lzss_compress(data, len, &comp, &comp_len);

unsigned char *back; size_t back_len;
lzss_decompress(comp, comp_len, &back, &back_len);   /* == data */
free(comp); free(back);
```

- `lzss_encode` / `lzss_decode` operate on `LzssToken` arrays; `lzss_serialise` /
  `lzss_deserialise` are the wire-format helpers; `lzss_compress` /
  `lzss_decompress` are the one-shot API with the default parameters
  (`LZSS_DEFAULT_WINDOW_SIZE` 4096, `_MAX_MATCH` 255, `_MIN_MATCH` 3).
- All return `LZSS_OK` or `LZSS_ERR_ALLOC`; output buffers are malloc'd (free
  them; may be `NULL` when the length is 0). Every growable buffer is
  overflow-guarded.

**Robustness:** decoding skips malformed matches (offset 0 or beyond the
output), caps the block count to the payload size, and bounds the output by the
declared length — no out-of-bounds access, no unbounded allocation.

## Building & testing

```sh
sh BUILD          # POSIX: compiles + runs the tests under every compiler found
```

Tests use the crate's own token vectors, window/match-length limits, decode
overlap cases, text/binary round trips, and a malformed-input safety check.
