# blake2b (C++)

The **BLAKE2b** hash (RFC 7693), in pure ISO C++17 (header-only). A faithful port
of the Rust `blake2b` crate. Output verified against the published RFC 7693 test
vectors.

Compiles and runs under **GCC, Clang, and MSVC** with strict ISO-conformance
flags, via the shared [`iso-harness`](../../c/iso-harness/README.md).

## Usage

```cpp
#include "blake2b.hpp"

std::string h = ca::blake2b_hex(std::string("abc"));            // BLAKE2b-512
std::vector<std::uint8_t> d = ca::blake2b(std::string("abc"), 32); // BLAKE2b-256

ca::blake2b_hasher hasher(64, key /*vector*/, salt /*16*/, personal /*16*/);
hasher.update(std::string("message"));
auto mac = hasher.digest();
```

Throws `std::invalid_argument` on an out-of-range parameter.

## Development

```bash
sh BUILD   # compile + run the tests under every C++ compiler present (strict ISO)
```

Ports `code/packages/rust/blake2b`. See also the [C port](../../c/blake2b/README.md).
