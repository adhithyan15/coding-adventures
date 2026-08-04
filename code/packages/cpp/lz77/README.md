# lz77 (C++)

**LZ77** sliding-window compression, in pure ISO C++17 (header-only). A faithful
port of the Rust `lz77` crate.

Compiles and runs under **GCC, Clang, and MSVC** with strict ISO-conformance
flags, via the shared [`iso-harness`](../../c/iso-harness/README.md).

## Usage

```cpp
#include "lz77.hpp"
using namespace ca::lz77;

std::vector<std::uint8_t> packed =
    compress(data, default_window, default_max_match, default_min_match);
std::vector<std::uint8_t> restored = decompress(packed);   // == data
```

Lower-level `encode`/`decode`/`serialise`/`deserialise` return `std::vector`s of
`token` / bytes.

## Development

```bash
sh BUILD   # compile + run the tests under every C++ compiler present (strict ISO)
```

Ports `code/packages/rust/lz77`. See also the [C port](../../c/lz77/README.md).
