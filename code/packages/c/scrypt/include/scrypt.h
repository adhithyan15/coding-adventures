/*
 * scrypt.h — scrypt, the sequential memory-hard password-based key derivation
 * function (RFC 7914), in pure ISO C17. A faithful port of the Rust `scrypt`
 * crate.
 * ===========================================================================
 *
 * PBKDF2 and bcrypt can be parallelised cheaply on GPUs / FPGAs. scrypt adds
 * *memory hardness*: it deliberately allocates a large random-access working set
 * (N * 128 * r bytes) and reads it in a data-dependent order, so an attacker
 * cannot trade memory for speed. That working set is what makes brute-force
 * attacks expensive.
 *
 *   scrypt(P, S, N, r, p, dkLen):
 *     1. B  = PBKDF2-HMAC-SHA256(P, S, 1, p*128*r)   -- expand into p blocks
 *     2. B[i] = ROMix(B[i], N)   for each 128*r-byte block   -- memory-hard
 *     3. DK = PBKDF2-HMAC-SHA256(P, B, 1, dkLen)     -- extract the key
 *
 * ROMix fills a table V of N snapshots (BlockMix run N times), then does N more
 * BlockMix steps each XORing in a data-chosen V entry (integerify % N). BlockMix
 * mixes 2r 64-byte blocks with the Salsa20/8 core.
 *
 * Parameters: N (CPU/memory cost, a power of two >= 2), r (block-size
 * multiplier), p (parallelisation), dk_len (output length). Memory: N*128*r
 * bytes. Typical N=16384, r=8, p=1 -> 16 MiB.
 *
 * This port builds on the sibling `pbkdf2` package (which uses `hmac` + the SHA
 * family) and writes into a caller-provided buffer (no ownership transfer).
 *
 * Portability: pure ISO C17 — GCC, Clang, and MSVC with -pedantic-errors /
 * /permissive- and warnings-as-errors.
 */
#ifndef SCRYPT_H
#define SCRYPT_H

#include <stddef.h> /* size_t */
#include <stdint.h> /* uint8_t */

/* Status codes (mirroring the Rust crate's ScryptError, plus an allocation
 * failure code — the Rust Vec would abort, but a C library reports it). */
typedef enum {
    SCRYPT_OK = 0,
    SCRYPT_INVALID_N,             /* N < 2 or N not a power of two */
    SCRYPT_N_TOO_LARGE,           /* N > 2^20 */
    SCRYPT_INVALID_R,             /* r < 1 */
    SCRYPT_INVALID_P,             /* p < 1 */
    SCRYPT_INVALID_KEY_LENGTH,    /* dk_len < 1 */
    SCRYPT_KEY_LENGTH_TOO_LARGE,  /* dk_len > 2^20 */
    SCRYPT_PR_TOO_LARGE,          /* p*r >= 2^30, or p*128*r overflow / > 2^30 */
    SCRYPT_HMAC_ERROR,            /* an internal PBKDF2 computation failed */
    SCRYPT_ALLOC_ERROR            /* out of memory for the ROMix working set */
} ScryptStatus;

/* The upper bounds enforced on N and dk_len (matching the Rust crate). */
#define SCRYPT_MAX_N (1u << 20)
#define SCRYPT_MAX_DK_LEN (1u << 20)

/* scrypt — derive `dk_len` bytes into `out` (which must hold dk_len bytes) from
 * `password` and `salt`. `n` must be a power of two in [2, 2^20]; `r`, `p` >= 1.
 * Returns SCRYPT_OK on success, or a status describing the invalid parameter or
 * failure. An empty password/salt is permitted (RFC 7914 vector 1). */
ScryptStatus scrypt(const uint8_t *password, size_t password_len,
                    const uint8_t *salt, size_t salt_len, size_t n, size_t r,
                    size_t p, size_t dk_len, uint8_t *out);

#endif /* SCRYPT_H */
