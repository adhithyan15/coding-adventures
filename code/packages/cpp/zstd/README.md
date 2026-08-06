# zstd (C++)

A pure ISO **C++17**, header-only implementation of an educational **RFC 8878
subset** of **Zstandard (ZStd)**, in namespace `ca::zstd`. A port of the
(corrected) Rust `zstd` crate (CMP07), reusing `cpp/lzss` (CMP02) as the LZ77
match-finder.

It compiles clean under **GCC, Clang, and MSVC** with `-std=c++17
-pedantic-errors -Wall -Wextra -Werror` (and `/std:c++17 /permissive- /W4
/WX` on MSVC), via the shared [`iso-harness`](../../c/iso-harness/). Standard
library only.

## What it is

Zstandard (Yann Collet, Meta, 2015; RFC 8878, 2021) combines LZ77-style
back-references with **FSE (Finite State Entropy)** coding — a table-based
Asymmetric Numeral System (tANS) that approaches the Shannon entropy limit
more closely than Huffman coding. This package implements the *educational
subset* documented in `code/specs/CMP07-zstd.md`:

| Feature       | Full ZStd               | This implementation     |
|---------------|--------------------------|--------------------------|
| Literals      | Huffman or raw           | Raw only                 |
| Sequences FSE | Custom + predefined      | Predefined tables only   |
| Block types   | Raw / RLE / Compressed   | All three                |
| Dictionary    | Yes                      | No                       |
| Checksums     | Optional                 | Omitted (flag = 0)       |
| Window size   | Up to 8 MB               | Fixed (Single_Segment)   |

Despite the simplifications, the wire format is **real**: output from this
package's `compress()` decompresses correctly with the reference `zstd` CLI,
and this package's `decompress()` correctly reads real `zstd`-compressed
frames (see Testing, below).

## A note on a real conformance bug this package avoids

A repo-wide audit found that **every** pre-existing language port of this
package (Rust, Go, Python, TypeScript, Swift, Dart, Elixir, Lua, Perl, C#,
F#, Haskell, Java, Kotlin) had independently invented the same three wrong
conventions for the sequences-section FSE codec — each internally
self-consistent (an encoder/decoder pair that agree with each other on the
wrong convention pass every round-trip test against *themselves*), and each
silently non-conformant with real zstd. The bug was only visible by testing
against an **independent**, spec-conformant implementation (the real `zstd`
CLI). This package was implemented *after* that fix landed, directly against
the corrected reference, and its own `tc9_cli_interop`-equivalent test
exercises real CLI interop in both directions as part of the normal test
run — see `include/zstd.hpp`'s "THE FSE BUG CLASS" banner comment for the
exact three rules (table-spread algorithm, per-sequence field order, and the
last-sequence state-update skip) and `tests/zstd_test.cpp` for the
regression coverage.

## API

```cpp
#include "zstd.hpp"
namespace zstd = ca::zstd;
using Bytes = std::vector<std::uint8_t>;

Bytes comp = zstd::compress(data);        // never throws
Bytes back = zstd::decompress(comp);      // == data; throws zstd::ZstdError on
                                           // malformed/untrusted input
```

- `compress` / `decompress` operate on `std::vector<std::uint8_t>` (matching
  `cpp/lzss`'s convention — this repo pins `-std=c++17`, so `std::span`, a
  C++20 addition, isn't available).
- `decompress` throws `zstd::ZstdError` (a `std::runtime_error`) rather than
  silently degrading, matching `cpp/wasm-leb128`'s convention: a ZStd frame
  carries untrusted, security-relevant framing (declared sizes, offsets,
  sequence counts) that must be validated, unlike `lzss::decode`'s
  deliberately lenient token-level API.

**Robustness / security** (see `code/specs/CMP07-zstd.md`'s Security
Considerations): the declared `Frame_Content_Size` is never used to
pre-allocate output; total decompressed size is checked incrementally
against a 256 MB cap at every point output can grow, including inside the
per-sequence loop of Compressed-block decoding; block sizes are capped at
128 KB; match offsets are bounds-checked before copying; sequence counts are
validated against the 24-bit wire field's limit; only `Predefined_Mode` FSE
tables are accepted; and trailing bytes after the frame's end (including
past an optional content checksum) are rejected rather than ignored.

## Building & testing

```sh
sh BUILD          # POSIX: compiles + runs the tests under every compiler found
```

Tests cover the spec's 10 mandatory cases (`code/specs/CMP07-zstd.md`
TC-1..TC-10) — empty/single-byte/all-256-byte-value round trips, RLE
compression, English-prose and repeat-offset compression-ratio assertions, a
deterministic-PRNG binary blob, a 200 KB multi-block frame, **real `zstd`
CLI interoperability in both directions** (TC-9 — the test that actually
proves RFC 8878 conformance; shells out via `std::system` and skips
gracefully if `zstd` isn't on `PATH`), and a hand-built minimal wire-format
frame independent of this package's own encoder — plus malformed-input
safety checks and internal FSE-codec unit tests that pin the exact
peek/extras/update field order.
