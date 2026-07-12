# rans (C++)

A pure ISO **C++17**, header-only implementation of table-based **rANS** (range
Asymmetric Numeral Systems) entropy coding, in namespace `ca::rans`. A faithful
port of the Rust `rans` crate.

It compiles clean under **GCC, Clang, and MSVC** with `-std=c++17
-pedantic-errors -Wall -Wextra -Werror` (and `/std:c++17 /permissive- /W4 /WX`
on MSVC), via the shared [`iso-harness`](../../c/iso-harness/). Standard library
only.

## What it is

rANS is the modern entropy coder used by Zstandard and JPEG XL. An `AnsTable`
is built from raw symbol counts (normalised so frequencies sum to a power of two
M ≤ 2^16); `RansEncoder` codes symbols in **reverse** order and `finish`es to
bytes; `RansDecoder` reads them back in forward order.

## API

```cpp
#include "rans.hpp"
namespace rans = ca::rans;

rans::AnsTable t = rans::AnsTable::build({3, 1});   // t.m() == 4

rans::RansEncoder enc(t);
for (std::uint8_t s : {0, 1, 0, 0}) enc.put(s);     // reverse of [0,0,1,0]
std::vector<std::uint8_t> bytes = enc.finish();

rans::RansDecoder dec(t, bytes);
std::uint8_t s = dec.get();                          // 0, 0, 1, 0
```

- `AnsTable::build` throws `std::invalid_argument` on bad counts; `m` / `log2m` /
  `alphabet_size` / `freq` / `cumfreq` (the last two return `std::optional`).
- `RansEncoder::put` / `finish`; `RansDecoder::get` / `is_exhausted`.
- The `RansDecoder` **owns** a copy of the input bytes (the Rust borrows `&[u8]`)
  so `RansDecoder(t, enc.finish())` is lifetime-safe. Arithmetic is 64-bit.

## Building & testing

```sh
sh BUILD          # POSIX: compiles + runs the tests under every compiler found
```

Tests cover the crate's table vectors, error cases, short-data rejection, symbol
round trips (including skewed distributions), and determinism.
