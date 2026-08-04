# aes (C++)

A pure ISO **C++17**, header-only implementation of the **AES** block cipher
(FIPS 197) — AES-128, AES-192, and AES-256, in namespace `ca::aes`. A faithful
port of the Rust `aes` crate.

It compiles clean under **GCC, Clang, and MSVC** with `-std=c++17
-pedantic-errors -Wall -Wextra -Werror` (and `/std:c++17 /permissive- /W4 /WX` on
MSVC), via the shared [`iso-harness`](../../c/iso-harness/). Its only dependency
is the sibling header-only [`gf256`](../gf256/) (used to build the S-box).

## API

```cpp
#include "aes.hpp"
using ca::aes::block_t;  // std::array<std::uint8_t, 16>

std::vector<std::uint8_t> key = { /* 16, 24, or 32 bytes */ };
block_t plain = { ... };
std::optional<block_t> ct = ca::aes::encrypt_block(plain, key);  // nullopt on bad key
std::optional<block_t> back = ca::aes::decrypt_block(*ct, key);
```

| Function | Purpose |
| --- | --- |
| `encrypt_block` / `decrypt_block` | the raw 16-byte block cipher (→ `std::optional<block_t>`) |
| `sbox` / `inv_sbox` | the 256-byte S-box tables |

Like the C sibling, the S-box is derived from GF(2⁸) inverses via
`ca::gf256::Field(0x11B)` plus the AES affine transform (built once through a
thread-safe function-local static). This is the raw block cipher — use a mode
(CBC/CTR/GCM) for variable-length data.

## Building & testing

```sh
sh BUILD          # POSIX: compiles + runs the tests under every compiler found
```

Tests are pinned to the FIPS 197 known-answer vectors for all three key sizes,
verify round-trips, and check the S-box bijection/inverse.
