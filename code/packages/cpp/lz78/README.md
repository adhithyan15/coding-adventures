# lz78 (C++)

A pure ISO **C++17**, header-only implementation of the **LZ78** (1978) lossless
compression algorithm, in namespace `ca::lz78`. A faithful port of the Rust
`lz78` crate (CMP01).

It compiles clean under **GCC, Clang, and MSVC** with `-std=c++17
-pedantic-errors -Wall -Wextra -Werror` (and `/std:c++17 /permissive- /W4 /WX`
on MSVC), via the shared [`iso-harness`](../../c/iso-harness/). Standard library
only.

## What it is

LZ78 builds an explicit trie dictionary as it encodes; the encoder and decoder
build the same dictionary independently, so none is transmitted. Each token is a
`(dict_index, next_char)` pair. Wire format (CMP01, big-endian): u32 original
length, u32 token count, then `token_count × 4` bytes.

## API

```cpp
#include "lz78.hpp"
namespace lz78 = ca::lz78;
using Bytes = std::vector<std::uint8_t>;

Bytes comp = lz78::compress(data, 65536);
Bytes back = lz78::decompress(comp);            // == data

std::vector<lz78::Token> toks = lz78::encode(data, 65536);
Bytes out = lz78::decode(toks, data.size());    // std::optional<size_t> length
```

- `encode` / `decode` (`decode` takes `std::optional<std::size_t>`), plus the
  one-shot `compress` / `decompress`.
- `ca::lz78::TrieCursor` is the reusable byte-at-a-time trie walker
  (`step` / `insert` / `reset` / `dict_id` / `at_root`).

**Robustness:** `decode` / `decompress` bounds- and cycle-check the dictionary,
so malformed input can't read out of bounds or loop forever; well-formed streams
decode identically to the crate.

## Building & testing

```sh
sh BUILD          # POSIX: compiles + runs the tests under every compiler found
```

Tests use the crate's own token vectors, text/binary round trips, the max-dict
cap, the wire-size invariant, determinism, and a malformed-input safety check.
