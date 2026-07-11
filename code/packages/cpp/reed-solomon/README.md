# reed-solomon (C++)

A pure ISO **C++17**, header-only Reed-Solomon error-correcting codec over GF(2⁸)
— the code behind QR codes, CDs/DVDs, and deep-space communication. A faithful
port of the Rust `reed-solomon` crate, in namespace `ca::reed_solomon`.

It compiles clean under **GCC, Clang, and MSVC** with `-std=c++17
-pedantic-errors -Wall -Wextra -Werror` (and `/std:c++17 /permissive- /W4 /WX` on
MSVC), via the shared [`iso-harness`](../../c/iso-harness/). Its only dependency
is the sibling header-only [`gf256`](../gf256/).

## API

```cpp
#include "reed_solomon.hpp"
namespace rs = ca::reed_solomon;

std::vector<std::uint8_t> msg = {'H','E','L','L','O'};
auto code = rs::encode(msg, 4);              // 9 bytes: 5 message + 4 check
code[2] ^= 0x5A;                             // corrupt a byte...
std::optional<std::vector<std::uint8_t>> out = rs::decode(code, 4);  // ...recovered
```

| Function | Purpose |
| --- | --- |
| `encode` / `decode` | the code (`decode` → `std::optional`; nullopt if too many errors) |
| `build_generator` | the generator polynomial (little-endian) |
| `syndromes` / `error_locator` | decode internals |

Invalid arguments (odd `n_check`, oversize codeword, too-short input) throw
`std::invalid_argument`; too many errors to correct yield `std::nullopt`. The
decode pipeline is syndromes → Berlekamp-Massey → Chien → Forney, mirroring the C
sibling and the Rust crate, with `std::vector` polynomials.

## Building & testing

```sh
sh BUILD          # POSIX: compiles + runs the tests under every compiler found
```

Tests verify the generator polynomial, encode/decode round-trips, correction of
1/2/4 errors, and too-many-errors handling.
