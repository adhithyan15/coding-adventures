# pbkdf2 (C++)

**PBKDF2** — Password-Based Key Derivation Function 2 (RFC 8018 § 5.2) — in pure
ISO C++17, header-only, in namespace `ca`. A faithful port of the Rust `pbkdf2`
crate.

PBKDF2 stretches a password into a cryptographic key by applying a pseudorandom
function (HMAC) `iterations` times per output block. The iteration count is the
tunable cost parameter: every brute-force guess pays the same price.

```
DK   = T_1 || T_2 || ... || T_n            (first key_length bytes)
T_i  = U_1 XOR U_2 XOR ... XOR U_c
U_1  = PRF(Password, Salt || INT_32_BE(i))
U_j  = PRF(Password, U_{j-1})              for j = 2..c
```

Real-world uses: WPA2 Wi-Fi (HMAC-SHA1, 4096 iterations), Django / macOS Keychain
(HMAC-SHA256), LUKS disk encryption.

The PRF is HMAC over SHA-1 / SHA-256 / SHA-512, built on the sibling header-only
[`hmac`](../hmac), [`sha1`](../sha1), [`sha256`](../sha256), and
[`sha512`](../sha512) packages.

## API

Functions take and return `std::vector<std::uint8_t>`; validation failures throw
`std::invalid_argument`.

```cpp
#include "pbkdf2.hpp"

std::vector<std::uint8_t> pw = {'p','a','s','s','w','o','r','d'};
std::vector<std::uint8_t> salt = {'s','a','l','t'};

auto dk  = ca::pbkdf2_hmac_sha1(pw, salt, 4096, 20);   // 20-byte key
auto hex = ca::pbkdf2_hmac_sha256_hex(pw, salt, 1, 32); // lowercase hex string
```

- `pbkdf2_hmac_sha1` / `_sha256` / `_sha512` and their `_hex` variants.
- `pbkdf2(hash, digest_size, block_size, ...)` — generic core over any hash
  callable `std::vector<uint8_t>(const std::vector<uint8_t>&)`.

An empty password throws unless the trailing `allow_empty_password` argument is
`true`. Key length is capped at `ca::pbkdf2_max_key_length` (2^20 bytes) to bound
memory and keep the block counter within 32 bits, as the Rust crate does.

## Portability

Pure ISO C++17 — compiles clean under GCC, Clang, and MSVC with
`-pedantic-errors` / `/permissive-` and warnings-as-errors, via the shared
[`iso-harness`](../../c/iso-harness). Standard library only.

## Development

```bash
# Compile and run the RFC 6070 / RFC 7914 vector tests under every C++ compiler.
sh BUILD
```
