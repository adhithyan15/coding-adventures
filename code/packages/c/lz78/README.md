# lz78 (C)

A pure ISO **C17** implementation of the **LZ78** (1978) lossless compression
algorithm. A faithful port of the Rust `lz78` crate (CMP01).

It compiles clean under **GCC, Clang, and MSVC** with `-std=c17
-pedantic-errors -Wall -Wextra -Werror` (and `/std:c17 /permissive- /W4 /WX` on
MSVC), via the shared [`iso-harness`](../iso-harness/). Standard library only.

## What it is

LZ78 (Lempel & Ziv, 1978) builds an explicit trie dictionary of byte sequences
as it encodes; the encoder and decoder build the same dictionary independently,
so none is transmitted. Each token is a `(dict_index, next_char)` pair —
`dict_index` is the id of the longest matching dictionary prefix (`0` for a
literal), `next_char` the byte that follows.

Wire format (CMP01, big-endian): a u32 original length, a u32 token count, then
`token_count × 4` bytes (`[dict_index u16][next_char u8][0x00]`).

## API

```c
#include "lz78.h"

unsigned char *comp; size_t comp_len;
lz78_compress(data, len, 65536, &comp, &comp_len);

unsigned char *back; size_t back_len;
lz78_decompress(comp, comp_len, &back, &back_len);   /* == data */
free(comp); free(back);
```

- `lz78_encode` / `lz78_decode` work with token arrays; `lz78_compress` /
  `lz78_decompress` are the one-shot wire-format helpers.
- `Lz78TrieCursor` is the reusable byte-at-a-time trie walker
  (`lz78_cursor_new`/`_step`/`_insert`/`_reset`/`_dict_id`/`_at_root`/`_free`).
- All return `LZ78_OK` or `LZ78_ERR_ALLOC`; output buffers are malloc'd (free
  them; they may be `NULL` when the length is 0). Every growable buffer is
  overflow-guarded.

**Robustness:** `lz78_decompress` bounds- and cycle-checks the dictionary, so a
malformed input can't cause an out-of-bounds read or an infinite loop; for
well-formed streams the output is identical to the crate.

## Building & testing

```sh
sh BUILD          # POSIX: compiles + runs the tests under every compiler found
```

Tests use the crate's own token vectors, text/binary round trips, the max-dict
cap, the wire-size invariant, determinism, and a malformed-input safety check.
