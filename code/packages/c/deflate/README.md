# deflate (C)

A pure ISO **C17** implementation of **DEFLATE** (RFC 1951), the compression
algorithm behind ZIP, gzip, PNG, and zlib. A faithful port of the Rust
`deflate` crate (CMP05).

It compiles clean under **GCC, Clang, and MSVC** with `-std=c17
-pedantic-errors -Wall -Wextra -Werror` (and `/std:c17 /permissive- /W4 /WX`
on MSVC), via the shared [`iso-harness`](../iso-harness/). Standard library
only, plus the sibling [`c/lzss`](../lzss/) package for LZ tokenization.

## What it is

DEFLATE (Phil Katz, 1989; RFC 1951, L. Peter Deutsch, 1996) composes two
earlier algorithms in this series:

1. **LZSS tokenization** ([CMP02](../lzss/)) — replace repeated substrings
   with back-references into a 32768-byte sliding window (the full RFC 1951
   window; offset 1-32768, length 3-255).
2. **Huffman coding** (CMP04) — entropy-code the resulting
   literal/length/distance token stream. `deflate_compress` builds both a
   **fixed**-table encoding (RFC 1951 §3.2.6, no table transmitted) and a
   **dynamic** encoding (code lengths adapted to the data, length-limited to
   15 bits via the package-merge algorithm), then emits whichever is smaller
   in exact bits.

The two token types (literal byte / length-distance match) share one combined
Literal/Length alphabet (symbols 0-255 are literal bytes, 256 is end-of-block,
257-285 are length codes with extra bits), plus a separate distance alphabet
(0-29, also with extra bits) — see RFC 1951 §3.2.5 or `CMP05-deflate.md` for
the exact tables.

## Wire format

`deflate_compress` emits a **standard RFC 1951 raw DEFLATE stream** — the
exact bytes a ZIP entry or gzip body carries, no envelope. A single final
block (`BFINAL=1`), `BTYPE=01` (fixed) or `BTYPE=10` (dynamic), whichever is
smaller. `deflate_decompress` is the standard `inflate`: it reads **all
three** block types (stored `00`, fixed `01`, dynamic `10`) across as many
blocks as the stream contains, so it decodes real `zlib`/`gzip`/ZIP/PNG data,
not only this library's own output.

This is a deliberate asymmetry: **encode conservatively, decode liberally.**
The encoder's own tables (length symbols 257-284, our `max_match=255`) never
reach length symbol 285 or the largest distance codes, but the decoder
implements the *complete* RFC 1951 alphabet (symbol 285, distance codes
0-29 up to 32768) because it must read *anyone's* output.

## API

```c
#include "deflate.h"

unsigned char *comp; size_t comp_len;
deflate_compress(data, len, &comp, &comp_len);

unsigned char *back; size_t back_len;
deflate_decompress(comp, comp_len, &back, &back_len);   /* == data */
deflate_free(comp); deflate_free(back);
```

- Both functions return a `DeflateStatus`: `DEFLATE_OK`, `DEFLATE_ERR_ALLOC`
  (out of memory), or (decompress only) `DEFLATE_ERR_MALFORMED` (not a
  well-formed RFC 1951 stream). `deflate_compress` never rejects an input —
  every byte sequence has a valid DEFLATE encoding.
- On `DEFLATE_OK`, `*out` is a malloc'd buffer of `*out_len` bytes; free it
  with `deflate_free()` (or plain `free()` — they're equivalent, but the
  named function means callers linking across a library boundary never have
  to remember which allocator produced it). On any error `*out` is `NULL`.

**Robustness (untrusted decompression input):** output is capped at
`DEFLATE_MAX_OUTPUT` (256 MiB) to bound decompression-bomb blast radius, every
back-reference distance is checked against the bytes decoded so far (never an
out-of-bounds read), every decoded length/distance/CL symbol is range-checked
before table lookup, stored-block `LEN`/`NLEN` are cross-verified, and no
allocation is sized directly from an attacker-controlled declared length.
Malformed input yields `DEFLATE_ERR_MALFORMED`, never undefined behaviour.

## Building & testing

```sh
sh BUILD          # POSIX: compiles + runs the tests under every compiler found
```

Tests cover: the spec's byte-exact vectors (`compress(b"") == 03 00`,
`compress(b"AAABBC") == 73 74 74 74 72 72 06 00`), round trips across
literals/matches/overlapping runs/all 256 byte values/highly repetitive data,
every length-code boundary, a hand-built stored block and a hand-built
malformed back-reference (both cross-checked against Python's `zlib` while
authoring the vectors), a bit-flip robustness sweep, and — critically — a
**real `zlib`-produced dynamic-Huffman raw DEFLATE stream** (generated via
`python3 -c "import zlib; ..."`, `wbits=-15`) that this decoder must read
correctly, proving it handles real-world dynamic Huffman and not only its own
encoder's output.

## Dependency

Depends on [`c/lzss`](../lzss/) (CMP02) for `lzss_encode`/`LzssToken`, the LZ
pass. DEFLATE's own Huffman coder is self-contained: fixed codes need no tree
at all, and the dynamic-code alphabets (LL/distance/code-length) are specific
to this format, so there is no separate `huffman-tree` dependency.
