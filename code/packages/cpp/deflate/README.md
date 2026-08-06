# deflate (C++)

A pure ISO **C++17**, header-only implementation of **DEFLATE** (RFC 1951)
lossless compression, in namespace `ca::deflate`. A faithful port of the Rust
`deflate` crate (CMP05).

It compiles clean under **GCC, Clang, and MSVC** with `-std=c++17
-pedantic-errors -Wall -Wextra -Werror` (and `/std:c++17 /permissive- /W4
/WX` on MSVC), via the shared [`iso-harness`](../../c/iso-harness/). Standard
library only — no third-party dependencies.

## What it is

DEFLATE (Phil Katz, PKZIP 1989; specified as RFC 1951 by L. Peter Deutsch,
1996) is the compression layer inside ZIP, gzip, PNG, and zlib. It composes
two earlier algorithms in this series:

1. **LZSS tokenization** (CMP02, the sibling [`lzss`](../lzss/) package) —
   replaces repeated substrings with back-references into the full RFC 1951
   32768-byte sliding window.
2. **Huffman coding** (CMP04) — entropy-codes the resulting token stream
   using a combined literal/length (LL) alphabet (symbols 0–285) plus a
   separate distance alphabet (symbols 0–29), both with "extra bits" for the
   exact length/distance within a code's range.

`compress` builds **both** a fixed-table encoding (RFC 1951 §3.2.6 — no table
transmitted) and a **dynamic**, length-limited (package-merge,
Larmore–Hirschberg 1990) encoding, then emits whichever is smaller in exact
bits, as a single `BFINAL=1` block. `inflate` (aliased `decompress`) reads
all three RFC 1951 block types — stored, fixed, and dynamic Huffman — so it
decodes this library's own output as well as real `zlib`/`gzip`/Microsoft
Office DEFLATE streams.

## API

```cpp
#include "deflate.hpp"
namespace deflate = ca::deflate;
using Bytes = std::vector<std::uint8_t>;

Bytes comp = deflate::compress(data);        // never fails
Bytes back = deflate::decompress(comp);      // == data; throws DeflateException on malformed input
Bytes same = deflate::inflate(comp);         // decompress is an alias for inflate
```

- `compress(const Bytes&) -> Bytes` — always succeeds, returns a standard raw
  RFC 1951 DEFLATE stream (`compress({})` is the 2-byte fixed-Huffman block
  `03 00`).
- `inflate(const Bytes&) -> Bytes` / `decompress(const Bytes&) -> Bytes`
  (alias) — decode any RFC 1951 stream (stored / fixed / dynamic Huffman);
  **throw** `ca::deflate::DeflateException` (carrying a `DeflateError`) on
  malformed input, mirroring the sibling `canonical-cbor` package's
  exception-based convention for decoders of untrusted bytes.

```cpp
try {
    Bytes out = deflate::inflate(untrusted_bytes);
} catch (const deflate::DeflateException& e) {
    // e.error() is a ca::deflate::DeflateError
}
```

**Robustness (`inflate`):** caps output at 256 MB (`detail::MAX_INFLATE_OUTPUT`)
against decompression bombs; validates every back-reference distance against
the bytes decoded so far; validates length/distance Huffman symbols against
their tables; every size computation is checked for overflow before it
reaches an allocation or `push_back`.

## Dependency

The sibling [`lzss`](../lzss/) package (CMP02) supplies `ca::lzss::Token` and
`ca::lzss::encode` for the LZSS tokenization pass (window=32768,
max_match=255, min_match=3 — the full RFC 1951 window). See `BUILD`.

## Building & testing

```sh
sh tools/run.sh    # POSIX: compiles + runs the tests under every compiler found
```

Tests cover the CMP05 spec's exact byte-level vectors (empty input, "AAABBC"
fixed-Huffman encoding, matches with overlapping back-references), full
round-trip invariants (every byte value 0–255, repetitive and mixed data),
dynamic-vs-fixed block selection, and malformed-input handling (truncated
streams, reserved BTYPE, corrupted stored-block LEN/NLEN, out-of-range
back-references) — plus decoding a **real** raw-DEFLATE dynamic-Huffman
stream produced independently by CPython's `zlib` module, proving `inflate`
reads dynamic Huffman it never produced itself.
