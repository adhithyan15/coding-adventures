# scrypt (C)

**scrypt** — the sequential memory-hard password-based key derivation function
(RFC 7914) — in pure ISO C17. A faithful port of the Rust `scrypt` crate.

PBKDF2 and bcrypt can be parallelised cheaply on GPUs / FPGAs. scrypt adds
*memory hardness*: it deliberately allocates a large random-access working set
(`N * 128 * r` bytes) and reads it in a data-dependent order, so an attacker
cannot trade memory for speed. That working set is what makes brute-force
attacks expensive.

```
scrypt(P, S, N, r, p, dkLen):
  1. B    = PBKDF2-HMAC-SHA256(P, S, 1, p*128*r)   -- expand into p blocks
  2. B[i] = ROMix(B[i], N)   for each 128*r block  -- the memory-hard step
  3. DK   = PBKDF2-HMAC-SHA256(P, B, 1, dkLen)      -- extract the key
```

ROMix fills a table `V` of `N` snapshots (BlockMix run `N` times), then does `N`
more BlockMix steps each XORing in a data-chosen `V` entry. BlockMix mixes `2r`
64-byte blocks with the **Salsa20/8** core. Built on the sibling
[`pbkdf2`](../pbkdf2) package (which uses [`hmac`](../hmac) + the SHA family).

## Parameters

| Name     | Meaning                              | Typical |
|----------|--------------------------------------|---------|
| `n`      | CPU/memory cost — power of 2, ≥ 2     | 16384   |
| `r`      | Block-size multiplier                | 8       |
| `p`      | Parallelisation factor               | 1       |
| `dk_len` | Output key length in bytes           | 32 / 64 |

Memory: `N * 128 * r` bytes (N=16384, r=8 → 16 MiB).

## API

The derived key is written into a **caller-provided buffer** (no ownership
transfer); the function returns a `ScryptStatus`.

```c
#include "scrypt.h"

uint8_t dk[64];
ScryptStatus st = scrypt(
    (const uint8_t *)"pleaseletmein", 13,
    (const uint8_t *)"SodiumChloride", 14,
    16384, 8, 1,   /* N, r, p     */
    64, dk);       /* dk_len, out */
/* st == SCRYPT_OK; dk = 7023bdcb3afd7348... */
```

Status codes: `SCRYPT_OK`, `SCRYPT_INVALID_N` (< 2 or not a power of two),
`SCRYPT_N_TOO_LARGE` (> 2^20), `SCRYPT_INVALID_R` / `SCRYPT_INVALID_P` (< 1),
`SCRYPT_INVALID_KEY_LENGTH` (0), `SCRYPT_KEY_LENGTH_TOO_LARGE` (> 2^20),
`SCRYPT_PR_TOO_LARGE` (`p*r` ≥ 2^30 or `p*128*r` > 2^30), `SCRYPT_HMAC_ERROR`
(internal PBKDF2 failure), `SCRYPT_ALLOC_ERROR` (out of memory for the working
set). The `calloc(N, 128*r)` V-table allocation uses a checked multiply, so an
oversized request fails cleanly rather than overflowing.

## Portability

Pure ISO C17 — compiles clean under GCC, Clang, and MSVC with `-pedantic-errors`
/ `/permissive-` and warnings-as-errors, via the shared
[`iso-harness`](../iso-harness).

## Development

```bash
# Compile and run the RFC 7914 vector tests under every C compiler.
sh BUILD
```
