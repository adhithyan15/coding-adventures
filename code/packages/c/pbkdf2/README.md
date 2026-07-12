# pbkdf2 (C)

**PBKDF2** — Password-Based Key Derivation Function 2 (RFC 8018 § 5.2) — in pure
ISO C17. A faithful port of the Rust `pbkdf2` crate.

PBKDF2 stretches a password into a cryptographic key by applying a pseudorandom
function (HMAC) `iterations` times per output block. The iteration count is the
tunable cost parameter: every brute-force guess pays the same price, so a large
count slows attackers down.

```
DK   = T_1 || T_2 || ... || T_n            (first key_length bytes)
T_i  = U_1 XOR U_2 XOR ... XOR U_c
U_1  = PRF(Password, Salt || INT_32_BE(i))
U_j  = PRF(Password, U_{j-1})              for j = 2..c
```

Real-world uses: WPA2 Wi-Fi (HMAC-SHA1, 4096 iterations), Django / macOS Keychain
(HMAC-SHA256), LUKS disk encryption.

The PRF is HMAC over SHA-1 / SHA-256 / SHA-512, built on the sibling
[`hmac`](../hmac), [`sha1`](../sha1), [`sha256`](../sha256), and
[`sha512`](../sha512) packages. A generic entry point lets you plug in any hash.

## API

The derived key is written into a **caller-provided buffer** (no ownership
transfer); every function returns a `Pbkdf2Status`.

```c
#include "pbkdf2.h"

uint8_t dk[20];
Pbkdf2Status st = pbkdf2_hmac_sha1(
    (const uint8_t *)"password", 8,
    (const uint8_t *)"salt", 4,
    4096,        /* iterations           */
    dk, 20,      /* output buffer + len  */
    0);          /* allow empty password */
/* st == PBKDF2_OK; dk = 4b007901b765489abead49d926f721d065a429c1 */
```

- `pbkdf2_hmac_sha1` / `pbkdf2_hmac_sha256` / `pbkdf2_hmac_sha512` — fixed PRFs.
- `pbkdf2(hash, h_len, block_size, ...)` — generic core over any one-shot hash
  (same `void(const void*, size_t, uint8_t*)` signature as the `sha*` packages).

Status codes: `PBKDF2_OK`, `PBKDF2_EMPTY_PASSWORD` (empty password and not
allowed), `PBKDF2_INVALID_ITERATIONS` (0), `PBKDF2_INVALID_KEY_LENGTH` (0),
`PBKDF2_KEY_LENGTH_TOO_LARGE` (> `PBKDF2_MAX_KEY_LENGTH`, 1 MiB),
`PBKDF2_PRF_ERROR` (HMAC failure / OOM), `PBKDF2_BAD_ARGS` (NULL buffer / bad
size). Key length is capped at 2^20 bytes to bound memory and keep the block
counter within 32 bits, as the Rust crate does.

## Portability

Pure ISO C17 — compiles clean under GCC, Clang, and MSVC with `-pedantic-errors`
/ `/permissive-` and warnings-as-errors, via the shared
[`iso-harness`](../iso-harness).

## Development

```bash
# Compile and run the RFC 6070 / RFC 7914 vector tests under every C compiler.
sh BUILD
```
