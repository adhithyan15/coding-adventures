# aes-modes (C)

AES **modes of operation** — ECB, CBC, CTR, GCM — with PKCS#7 padding, in pure
ISO C17. A faithful port of the Rust `aes-modes` crate.

AES is a 128-bit block cipher; a *mode of operation* chains block calls to
encrypt arbitrary-length messages. This package builds on the raw block cipher
from the sibling [`aes`](../aes) package (which in turn uses [`gf256`](../gf256)).

| Mode | Security | Notes |
|------|----------|-------|
| ECB  | **BROKEN** | Each block independent; identical blocks leak. Educational only. |
| CBC  | Legacy | `C[i] = E(P[i] XOR C[i-1])`; needs a 16-byte IV. PKCS#7 padded. |
| CTR  | Modern | Stream cipher; 12-byte nonce + 32-bit counter. No padding; enc == dec. |
| GCM  | Modern, authenticated | CTR + GHASH tag (AEAD). 12-byte IV; verifies the tag on decrypt. |

GHASH multiplies in GF(2^128) with the reducing polynomial x^128+x^7+x^2+x+1,
done byte-wise (no 128-bit integers).

## API

Variable-length outputs are returned in a `malloc`'d buffer via an out-pointer;
the caller frees it with `free()`. Every function returns an `AesmStatus`.

```c
#include "aes_modes.h"

uint8_t *ct; size_t ct_len; uint8_t tag[16];
AesmStatus st = aesm_gcm_encrypt(pt, pt_len, key, 16, iv, 12,
                                 aad, aad_len, &ct, &ct_len, tag);
/* ... use ct[0..ct_len) and the 16-byte tag ... */
free(ct);
```

Functions: `aesm_pkcs7_pad`/`_unpad`, `aesm_ecb_encrypt`/`_decrypt`,
`aesm_cbc_encrypt`/`_decrypt` (16-byte IV), `aesm_ctr_encrypt`/`_decrypt`
(12-byte nonce), `aesm_gcm_encrypt`/`_decrypt` (12-byte IV, 16-byte tag).
`aesm_gcm_decrypt` returns `AESM_AUTH_FAILED` (writing nothing) on a tag
mismatch. Status codes cover bad key/IV/nonce/ciphertext lengths, invalid
padding, authentication failure, NULL/overflow args, and allocation failure.

## Portability

Pure ISO C17 — compiles clean under GCC, Clang, and MSVC with `-pedantic-errors`
/ `/permissive-` and warnings-as-errors, via the shared
[`iso-harness`](../iso-harness).

## Development

```bash
# Compile and run the NIST SP 800-38A / GCM vector tests under every C compiler.
sh BUILD
```
