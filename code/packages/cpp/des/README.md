# des (C++)

A pure ISO **C++17**, header-only implementation of the **DES** block cipher
(FIPS 46) and **Triple DES** (NIST SP 800-67), in namespace `ca::des`. A faithful
port of the Rust `des` crate.

It compiles clean under **GCC, Clang, and MSVC** with `-std=c++17
-pedantic-errors -Wall -Wextra -Werror` (and `/std:c++17 /permissive- /W4 /WX` on
MSVC), via the shared [`iso-harness`](../../c/iso-harness/). Standard library
only.

## ⚠️ Not for real use

DES (56-bit key) is brute-forceable and 3DES falls to SWEET32; NIST disallowed
3DES in 2023. This is a **faithful, educational** implementation for study and
legacy interop, not a recommendation — use a modern AEAD such as the sibling
[`chacha20-poly1305`](../chacha20-poly1305/) for real work.

## API

```cpp
#include "des.hpp"
using ca::des::block_t;   // std::array<std::uint8_t, 8>

block_t key   = {0x13,0x34,0x57,0x79,0x9B,0xBC,0xDF,0xF1};
block_t plain = {0x01,0x23,0x45,0x67,0x89,0xAB,0xCD,0xEF};
block_t ct = ca::des::encrypt_block(plain, key);          // → 85E813540F0AB405
block_t back = ca::des::decrypt_block(ct, key);

auto enc = ca::des::ecb_encrypt(std::vector<std::uint8_t>{...}, key);
std::optional<std::vector<std::uint8_t>> dec = ca::des::ecb_decrypt(enc, key);

block_t tct = ca::des::tdea_encrypt_block(plain, k1, k2, k3);   // Triple DES EDE
```

| Function | Purpose |
| --- | --- |
| `encrypt_block` / `decrypt_block` | the raw 8-byte block cipher |
| `ecb_encrypt` / `ecb_decrypt` | ECB with PKCS#7 padding (`ecb_decrypt` → `std::optional`) |
| `tdea_encrypt_block` / `tdea_decrypt_block` | Triple DES (EDE) |

Same bit-array algorithm and published tables as the C sibling; the tables are
`inline constexpr` and everything lives in `ca::des::detail`.

## Building & testing

```sh
sh BUILD          # POSIX: compiles + runs the tests under every compiler found
```

Tests are pinned to the FIPS 46 worked example and NIST SP 800-20 known-answer
vectors, plus round-trips, ECB padding, and Triple DES.
