# md5 (C++)

The **MD5** hash (RFC 1321), in pure ISO C++17 (header-only). A faithful port of
the Rust `md5` crate. Output verified against the RFC 1321 test suite.

> ⚠️ MD5 is broken for collision resistance — do not use it for security.

Compiles and runs under **GCC, Clang, and MSVC** with strict ISO-conformance
flags, via the shared [`iso-harness`](../../c/iso-harness/README.md).

## Usage

```cpp
#include "md5.hpp"

std::string h = ca::md5_hex(std::string("abc"));   // "900150983cd24fb0..."
ca::md5_digest d = ca::md5(std::string("abc"));      // std::array<uint8_t,16>

ca::md5_hasher hasher;
hasher.update(std::string("ab"));
hasher.update(std::string("c"));
std::string hex = hasher.hex_digest();
```

## Development

```bash
sh BUILD   # compile + run the tests under every C++ compiler present (strict ISO)
```

Ports `code/packages/rust/md5`. See also the [C port](../../c/md5/README.md).
