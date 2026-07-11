# des (C)

A pure ISO **C17** implementation of the **DES** block cipher (FIPS 46) and
**Triple DES** (NIST SP 800-67). A faithful port of the Rust `des` crate.

It compiles clean under **GCC, Clang, and MSVC** with
`-std=c17 -pedantic-errors -Wall -Wextra -Werror` (and `/std:c17 /permissive-
/W4 /WX` on MSVC), via the shared [`iso-harness`](../iso-harness/). No compiler
extensions, no third-party dependencies.

## ⚠️ Not for real use

DES (56-bit key) is brute-forceable and 3DES (effective ~112-bit key, 64-bit
block) falls to the SWEET32 attack — NIST disallowed 3DES entirely in 2023. This
package is a **faithful, educational** implementation for studying the archetypal
block cipher and for legacy interop, not a recommendation. For real work use a
modern AEAD (see the sibling [`chacha20-poly1305`](../chacha20-poly1305/)).

## What's here

DES is a 16-round Feistel network: an initial permutation, then rounds that
expand the 32-bit half to 48 bits, mix in a round subkey, pass it through eight
S-boxes, and permute — followed by a final permutation.

| Function | Purpose |
| --- | --- |
| `des_expand_key(key, subkeys)` | derive the 16 round subkeys |
| `des_encrypt_block / des_decrypt_block` | the raw 8-byte block cipher |
| `des_ecb_encrypt / des_ecb_decrypt` | ECB mode with PKCS#7 padding |
| `des_tdea_encrypt_block / des_tdea_decrypt_block` | Triple DES (EDE) |

```c
#include "des.h"

uint8_t key[8]   = {0x13,0x34,0x57,0x79,0x9B,0xBC,0xDF,0xF1};
uint8_t plain[8] = {0x01,0x23,0x45,0x67,0x89,0xAB,0xCD,0xEF};
uint8_t ct[8];
des_encrypt_block(plain, key, ct);   /* → 85 E8 13 54 0F 0A B4 05 */
```

`des_ecb_encrypt` returns a malloc'd buffer (caller frees); `des_ecb_decrypt`
returns 1 and sets a malloc'd plaintext, or 0 on a bad length / bad padding.

## Implementation notes

- **Bit-array representation.** Like the crate, each bit is held in one byte so
  the permutation tables (IP/FP, PC-1/PC-2, E, P) read exactly as published in
  FIPS 46. All buffers are fixed-size stack arrays — the block cipher allocates
  nothing; only ECB mode mallocs its output.
- **Decryption is encryption with reversed subkeys** — the self-inverse property
  of the Feistel structure.

## Building & testing

```sh
sh BUILD          # POSIX: compiles + runs the tests under every compiler found
```

Tests are pinned to the **FIPS 46** worked example
(`E(0123456789ABCDEF, 133457799BBCDFF1) = 85E813540F0AB405`) and several **NIST
SP 800-20** known-answer vectors (which validate every table), plus round-trips
across all byte values, ECB padding, and Triple DES (including the
`K1=K2=K3` → single-DES reduction).
