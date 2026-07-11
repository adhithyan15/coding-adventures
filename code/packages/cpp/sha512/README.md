# sha512 (C++)

The **SHA-512** hash (FIPS 180-4), in pure ISO C++17 (header-only). A faithful
port of the Rust `sha512` crate. Output verified against the published FIPS test
vectors.

Compiles and runs under **GCC, Clang, and MSVC** with strict ISO-conformance
flags, via the shared [`iso-harness`](../../c/iso-harness/README.md).

## Usage

```cpp
#include "sha512.hpp"

std::string h = ca::sha512_hex(std::string("abc"));   // 128 hex chars
ca::sha512_digest d = ca::sha512(std::string("abc"));  // std::array<uint8_t,64>

ca::sha512_hasher hasher;
hasher.update(std::string("ab"));
hasher.update(std::string("c"));
std::string hex = hasher.hex_digest();
```

## Development

```bash
sh BUILD   # compile + run the tests under every C++ compiler present (strict ISO)
```

Ports `code/packages/rust/sha512`. See also the [C port](../../c/sha512/README.md).
