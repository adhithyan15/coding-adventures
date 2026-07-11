# lzw (C++)

**LZW** compression with variable-width codes (9→16 bits), in pure ISO C++17
(header-only). A faithful port of the Rust `lzw` crate.

Compiles and runs under **GCC, Clang, and MSVC** with strict ISO-conformance
flags, via the shared [`iso-harness`](../../c/iso-harness/README.md).

## Usage

```cpp
#include "lzw.hpp"

std::vector<std::uint8_t> packed = ca::lzw::compress(data);
std::vector<std::uint8_t> restored = ca::lzw::decompress(packed);   // == data
```

`decompress` throws `std::invalid_argument` on a malformed stream.

## Development

```bash
sh BUILD   # compile + run the tests under every C++ compiler present (strict ISO)
```

Ports `code/packages/rust/lzw`. See also the [C port](../../c/lzw/README.md).
