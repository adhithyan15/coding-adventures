# aes-modes (C++)

AES **modes of operation** — ECB, CBC, CTR, GCM — with PKCS#7 padding, in pure
ISO C++17, header-only, in namespace `ca::aes_modes`. A faithful port of the
Rust `aes-modes` crate.

AES is a 128-bit block cipher; a *mode of operation* chains block calls to
encrypt arbitrary-length messages. Built on the raw block cipher of the sibling
header-only [`aes`](../aes) package (which uses [`gf256`](../../c/gf256)).

| Mode | Security | Notes |
|------|----------|-------|
| ECB  | **BROKEN** | Each block independent. Educational only. |
| CBC  | Legacy | `C[i] = E(P[i] XOR C[i-1])`; 16-byte IV. PKCS#7 padded. |
| CTR  | Modern | Stream cipher; 12-byte nonce + counter. No padding; enc == dec. |
| GCM  | Modern, authenticated | CTR + GHASH tag (AEAD). 12-byte IV; verifies the tag. |

GHASH multiplies in GF(2^128) with x^128+x^7+x^2+x+1, byte-wise (no 128-bit
integers).

## API

Functions take and return `std::vector<std::uint8_t>` (`ca::aes_modes::Bytes`);
GCM returns/accepts a 16-byte `ca::aes_modes::Block` tag. Validation errors throw
`std::invalid_argument`; a GCM tag mismatch throws
`ca::aes_modes::AuthenticationError`.

```cpp
#include "aes_modes.hpp"
namespace am = ca::aes_modes;

auto [ct, tag] = am::gcm_encrypt(pt, key, iv, aad);   // iv 12 bytes
am::Bytes back  = am::gcm_decrypt(ct, key, iv, aad, tag);  // throws on tamper
```

Functions: `pkcs7_pad`/`unpad`, `ecb_encrypt`/`decrypt`, `cbc_encrypt`/`decrypt`
(16-byte IV), `ctr_encrypt`/`decrypt` (12-byte nonce), `gcm_encrypt` (returns
`{ciphertext, tag}`) / `gcm_decrypt`.

## Portability

Pure ISO C++17 — compiles clean under GCC, Clang, and MSVC with
`-pedantic-errors` / `/permissive-` and warnings-as-errors, via the shared
[`iso-harness`](../../c/iso-harness). Standard library only.

## Development

```bash
# Compile and run the NIST SP 800-38A / GCM vector tests under every C++ compiler.
sh BUILD
```
