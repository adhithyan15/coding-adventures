# aes (C)

A pure ISO **C17** implementation of the **AES** block cipher (FIPS 197) —
AES-128, AES-192, and AES-256. A faithful port of the Rust `aes` crate.

It compiles clean under **GCC, Clang, and MSVC** with
`-std=c17 -pedantic-errors -Wall -Wextra -Werror` (and `/std:c17 /permissive-
/W4 /WX` on MSVC), via the shared [`iso-harness`](../iso-harness/). Its only
dependency is the sibling [`gf256`](../gf256/) package (used to build the S-box);
no third-party libraries.

## What's here

AES transforms a 4×4 byte "state" through 10 / 12 / 14 rounds of:

| Step | What it does |
| --- | --- |
| **SubBytes** | replace each byte via the S-box (non-linearity) |
| **ShiftRows** | cyclically shift each row (diffusion) |
| **MixColumns** | mix each column by GF(2⁸) matrix multiply (diffusion) |
| **AddRoundKey** | XOR in the round key |

The S-box is derived — not hardcoded — from the multiplicative inverse in GF(2⁸)
(AES polynomial `0x11B`) plus an affine transform, using `gf256_field` exactly as
the Rust crate uses its `gf256::Field`.

```c
#include "aes.h"

uint8_t key[16] = { /* 16, 24, or 32 bytes */ };
uint8_t plain[16], ct[16], back[16];
aes_encrypt_block(plain, key, 16, ct);   /* returns 1, or 0 on bad key length */
aes_decrypt_block(ct, key, 16, back);
```

| Function | Purpose |
| --- | --- |
| `aes_encrypt_block` / `aes_decrypt_block` | the raw 16-byte block cipher |
| `aes_expand_key` | the key schedule (round keys) |
| `aes_sbox` / `aes_inv_sbox` | the 256-byte S-box tables |

This is the raw block cipher only — for variable-length data use a mode (CBC,
CTR, GCM) on top; ECB (as shown in the sibling `des`) leaks patterns.

## Implementation notes

- **S-box built from `gf256`.** `aes` declares `# build-tool: deps=c/gf256` and
  compiles the gf256 source into the test binary — the campaign's cross-package
  dependency pattern (as `hash-set` uses `hash-map`).
- All round keys and state live in fixed-size stack arrays (`round_keys[15][4][4]`
  covers AES-256's 15 keys) — the cipher allocates nothing.
- The lazy S-box build is single-threaded (the crate uses `OnceLock`; pure ISO C
  has no portable one-time-init primitive).

## Building & testing

```sh
sh BUILD          # POSIX: compiles + runs the tests under every compiler found
```

Tests are pinned to the **FIPS 197** known-answer vectors for all three key sizes
(Appendix B and Appendices C.1/C.2/C.3), verify encrypt/decrypt round-trips, and
check the S-box is a bijection with a correct inverse and the FIPS 197 Figure 7
spot values.
