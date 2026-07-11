# chacha20-poly1305 (C)

A pure ISO **C17** implementation of the **ChaCha20** stream cipher, the
**Poly1305** one-time authenticator, and the **ChaCha20-Poly1305 AEAD**
(RFC 8439). A faithful port of the Rust `chacha20-poly1305` crate.

It compiles clean under **GCC, Clang, and MSVC** with
`-std=c17 -pedantic-errors -Wall -Wextra -Werror` (and `/std:c17 /permissive-
/W4 /WX` on MSVC), via the shared [`iso-harness`](../iso-harness/). No compiler
extensions, no third-party dependencies.

## What's in the box

| Function | Purpose |
| --- | --- |
| `chacha20_encrypt(in, len, key, nonce, counter, out)` | XOR data with the ChaCha20 keystream (also decrypts) |
| `poly1305_mac(msg, len, key, tag)` | 16-byte one-time authenticator |
| `aead_encrypt(pt, len, key, nonce, aad, aad_len, ct, tag)` | AEAD: encrypt + authenticate |
| `aead_decrypt(ct, len, key, nonce, aad, aad_len, tag, pt)` | AEAD: verify (constant-time) + decrypt |

Keys are 32 bytes, nonces 12 bytes, tags 16 bytes. `aead_decrypt` returns 0
(and you must discard the plaintext) if the tag does not verify.

## Why "pure ISO" is interesting here

Poly1305 accumulates modulo `2^130 - 5`, which needs 130-bit arithmetic — but
ISO C has no 128-bit integer type. This port uses the well-known
**"poly1305-donna"** representation: the accumulator is five 26-bit limbs, and
each partial product fits in a `uint64_t`. The result matches the RFC 8439 test
vectors exactly, with nothing wider than `uint64_t`.

## Usage

```c
#include "chacha20_poly1305.h"

uint8_t key[32]   = { /* 32-byte secret key */ };
uint8_t nonce[12] = { /* 96-bit nonce, unique per message */ };
uint8_t aad[]     = { /* associated data, authenticated but not encrypted */ };

const uint8_t *msg = (const uint8_t *)"attack at dawn";
size_t msg_len = 14;

uint8_t ciphertext[14];
uint8_t tag[16];
aead_encrypt(msg, msg_len, key, nonce, aad, sizeof aad, ciphertext, tag);

uint8_t recovered[14];
if (aead_decrypt(ciphertext, msg_len, key, nonce, aad, sizeof aad, tag,
                 recovered)) {
    /* tag verified — `recovered` holds the plaintext */
}
```

## Building & testing

```sh
sh BUILD          # POSIX: compiles + runs the tests under every compiler found
```

The tests are pinned to the RFC 8439 vectors (Poly1305 §2.5.2 and the full AEAD
§2.8.2), plus a round-trip and ciphertext/AAD tamper-detection checks.

## Where it fits

Part of the `code/packages/c` pure-ISO C set. Alongside the hash ports
(`sha256`, `sha512`, `blake2b`, `hmac`, `hkdf`, …), this is the AEAD building
block: confidentiality plus integrity in one primitive.
