# range-coder (C++)

A pure ISO **C++17**, header-only implementation of the **VP8 boolean range
coder** (RFC 6386 §7), in namespace `ca::range_coder`. A faithful port of the
Rust `range-coder` crate.

It compiles clean under **GCC, Clang, and MSVC** with `-std=c++17
-pedantic-errors -Wall -Wextra -Werror` (and `/std:c++17 /permissive- /W4 /WX`
on MSVC), via the shared [`iso-harness`](../../c/iso-harness/). Standard library
only.

## What it is

A boolean range coder is the binary arithmetic coder at the entropy stage of
VP8 / WebP. It compresses a sequence of bits, each with an 8-bit probability the
bit is 0 (`prob`; 128 ≈ 50/50). Encoding then decoding with the same
probabilities recovers the bits.

## API

```cpp
#include "range_coder.hpp"
namespace rc = ca::range_coder;

rc::BoolEncoder enc;
enc.write_bit(true, 128);
enc.write_bits(0xCAFEBABE, 32);
std::vector<std::uint8_t> bytes = enc.finish();

rc::BoolDecoder dec(bytes);
bool b = dec.read_bit(128);
std::uint32_t v = dec.read_bits(32);
```

- `BoolEncoder`: `write_bit` / `write_bits` / `finish` (returns the bytes).
- `BoolDecoder`: constructed from a `std::vector<std::uint8_t>` (or `data, len`),
  `read_bit` / `read_bits` / `is_exhausted`.

Note: unlike the Rust crate (which borrows `&[u8]`), the C++ `BoolDecoder` owns a
copy of the input — borrowing a temporary such as `BoolDecoder(enc.finish())`
would otherwise dangle in C++.

## Building & testing

```sh
sh BUILD          # POSIX: compiles + runs the tests under every compiler found
```

Tests round-trip single bits, mixed and skewed probability sequences, and
`write_bits`/`read_bits` for 8/16/32-bit fields, plus decoder seeding,
exhaustion, and determinism — from the crate's own tests.
