# chacha20-poly1305 (C++)

A pure ISO **C++17**, header-only implementation of the **ChaCha20** stream
cipher, the **Poly1305** one-time authenticator, and the **ChaCha20-Poly1305
AEAD** (RFC 8439). A faithful port of the Rust `chacha20-poly1305` crate.

Everything lives in namespace `ca`, in a single header. It compiles clean under
**GCC, Clang, and MSVC** with `-std=c++17 -pedantic-errors -Wall -Wextra
-Werror` (and `/std:c++17 /permissive- /W4 /WX` on MSVC), via the shared
[`iso-harness`](../../c/iso-harness/). No compiler extensions, no third-party
dependencies.

## API

```cpp
#include "chacha20_poly1305.hpp"

// Raw ChaCha20 (also decrypts — XOR again with the same key/nonce/counter):
std::vector<std::uint8_t> ca::chacha20_encrypt(input, key, nonce, counter);

// Poly1305 one-time authenticator:
std::array<std::uint8_t, 16> ca::poly1305_mac(msg, len, key);

// AEAD — encrypt returns {ciphertext, tag}:
ca::aead_result r = ca::aead_encrypt(plaintext, key, nonce, aad);

// AEAD — decrypt returns std::optional; empty means the tag failed:
std::optional<std::vector<std::uint8_t>> pt =
    ca::aead_decrypt(r.ciphertext, key, nonce, aad, r.tag);
```

Keys are 32 bytes, nonces 12 bytes, tags 16 bytes. `aead_decrypt` verifies the
tag in constant time and returns `std::nullopt` on any mismatch, so a caller
that unwraps the optional can never act on unauthenticated data.

## Why "pure ISO" is interesting here

Poly1305 accumulates modulo `2^130 - 5`, needing 130-bit arithmetic — but ISO
C++ has no 128-bit integer type. This port uses the **"poly1305-donna"**
representation: five 26-bit limbs, each partial product held in a
`std::uint64_t`. The output matches the RFC 8439 vectors exactly, with nothing
wider than `std::uint64_t`.

## Building & testing

```sh
sh BUILD          # POSIX: compiles + runs the tests under every compiler found
```

The tests are pinned to the RFC 8439 vectors (Poly1305 §2.5.2 and the full AEAD
§2.8.2), plus round-trip and ciphertext/AAD tamper-detection checks.

## Where it fits

Part of the `code/packages/cpp` pure-ISO C++ set — the AEAD companion to the
header-only hash ports (`sha256`, `sha512`, `blake2b`, `hmac`, `hkdf`, …).
