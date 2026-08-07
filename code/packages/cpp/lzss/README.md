# lzss (C++)

A pure ISO **C++17**, header-only implementation of the **LZSS** lossless
compression algorithm, in namespace `ca::lzss`. A faithful port of the Rust
`lzss` crate (CMP02).

It compiles clean under **GCC, Clang, and MSVC** with `-std=c++17
-pedantic-errors -Wall -Wextra -Werror` (and `/std:c++17 /permissive- /W4 /WX`
on MSVC), via the shared [`iso-harness`](../../c/iso-harness/). Standard library
only.

## What it is

LZSS (Storer & Szymanski, 1982) is the sliding-window LZ77 variant behind
DEFLATE. Each position emits either a `Literal` byte or a `Match{offset, length}`
back-reference (matches may overlap the cursor, so runs compress well). Wire
format (CMP02, big-endian): u32 original length, u32 block count, then flagged
blocks of 8 tokens.

## API

```cpp
#include "lzss.hpp"
namespace lzss = ca::lzss;
using Bytes = std::vector<std::uint8_t>;

Bytes comp = lzss::compress(data);
Bytes back = lzss::decompress(comp);              // == data

std::vector<lzss::Token> toks = lzss::encode(data, 4096, 255, 3);
Bytes out = lzss::decode(toks, data.size());      // std::optional<size_t> length
```

- `encode` / `decode` (with `std::optional<std::size_t>` length), `serialise` /
  `deserialise`, and one-shot `compress` / `decompress`.
- `ca::lzss::Token` has `Token::lit(b)` and `Token::match(off, len)` factories
  plus `operator==`.

**Robustness:** decoding skips malformed matches, caps the block count to the
payload, and bounds the output by the declared length.

## Building & testing

```sh
sh BUILD          # POSIX: compiles + runs the tests under every compiler found
```

Tests use the crate's own token vectors, window/match-length limits, decode
overlap cases, text/binary round trips, and a malformed-input safety check.
