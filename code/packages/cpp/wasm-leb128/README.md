# wasm-leb128 (C++)

A pure ISO **C++17**, header-only implementation of **LEB128** variable-length
integer coding, in namespace `ca::leb128`. A faithful port of the Rust
`wasm-leb128` crate.

It compiles clean under **GCC, Clang, and MSVC** with `-std=c++17
-pedantic-errors -Wall -Wextra -Werror` (and `/std:c++17 /permissive- /W4 /WX`
on MSVC), via the shared [`iso-harness`](../../c/iso-harness/). Standard library
only.

## What it is

LEB128 ("Little-Endian Base 128") is the varint format used by **WebAssembly**,
**DWARF**, and Android DEX. Each byte carries 7 data bits; the high bit is a
continuation flag, and groups are emitted least-significant first. Unsigned
values are zero-extended (`624485 → E5 8E 26`); signed values use two's
complement with sign extension (`-2 → 7E`).

## API

```cpp
#include "wasm_leb128.hpp"
namespace leb = ca::leb128;

std::vector<std::uint8_t> u = leb::encode_unsigned(624485);   // {E5,8E,26}
std::vector<std::uint8_t> s = leb::encode_signed(-2);         // {7E}

auto [uv, un] = leb::decode_unsigned(u, 0);   // (624485, 3)
auto [sv, sn] = leb::decode_signed(s, 0);     // (-2, 1)
```

`encode_*` return a `std::vector<std::uint8_t>` and never fail. `decode_*` take a
`std::vector` (or `data, len`) plus an offset and return
`std::pair<value, bytes_consumed>`, throwing `ca::leb128::Error` (carrying the
`offset`) on a bad offset, an over-wide sequence, or an unterminated one.

## Building & testing

```sh
sh BUILD          # POSIX: compiles + runs the tests under every compiler found
```

Tests use the crate's own WASM/DWARF vectors — zero, multi-byte, u32/i32 min &
max, offset decoding, the error conditions, and encode↔decode round trips
including `u64::MAX` / `i64::MIN`.
