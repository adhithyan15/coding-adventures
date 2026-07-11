# sha1 (C++)

The **SHA-1** hash (FIPS 180-4), in pure ISO C++17 (header-only). A faithful port
of the Rust `sha1` crate. Output verified against the published FIPS test
vectors.

> ⚠️ SHA-1 is broken for collision resistance — do not use it for security.

Compiles and runs under **GCC, Clang, and MSVC** with strict ISO-conformance
flags, via the shared [`iso-harness`](../../c/iso-harness/README.md).

## Usage

```cpp
#include "sha1.hpp"

std::string h = ca::sha1_hex(std::string("abc"));   // "a9993e36...9cd0d89d"
ca::sha1_digest d = ca::sha1(std::string("abc"));    // std::array<uint8_t,20>

ca::sha1_hasher hasher;
hasher.update(std::string("ab"));
hasher.update(std::string("c"));
std::string hex = hasher.hex_digest();
```

## Development

```bash
sh BUILD   # compile + run the tests under every C++ compiler present (strict ISO)
```

Ports `code/packages/rust/sha1`. See also the [C port](../../c/sha1/README.md).
