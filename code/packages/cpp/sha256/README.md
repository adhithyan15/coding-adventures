# sha256 (C++)

The **SHA-256** cryptographic hash (FIPS 180-4), in pure ISO C++17 (header-only).
A faithful port of the Rust `sha256` crate. Output verified against the published
FIPS test vectors.

Compiles and runs under **GCC, Clang, and MSVC** with strict ISO-conformance
flags, via the shared [`iso-harness`](../../c/iso-harness/README.md).

## Usage

```cpp
#include "sha256.hpp"

std::string h = ca::sha256_hex(std::string("abc"));  // "ba7816bf...20015ad"
ca::sha256_digest d = ca::sha256(std::string("abc")); // std::array<uint8_t,32>

ca::sha256_hasher hasher;         // streaming
hasher.update(std::string("ab"));
hasher.update(std::string("c"));
std::string hex = hasher.hex_digest();
```

`digest()` / `hex_digest()` finalise on a copy, so the hasher stays usable.

## Development

```bash
sh BUILD   # compile + run the tests under every C++ compiler present (strict ISO)
```

Ports `code/packages/rust/sha256`. See also the [C port](../../c/sha256/README.md).
