/*
 * pbkdf2.h — PBKDF2, Password-Based Key Derivation Function 2 (RFC 8018 § 5.2),
 * in pure ISO C17. A faithful port of the Rust `pbkdf2` crate.
 * ===========================================================================
 *
 * PBKDF2 stretches a password into a cryptographic key by applying a
 * pseudorandom function (here HMAC) `iterations` times per output block. The
 * iteration count is the tunable cost: every brute-force guess pays the same
 * price, so a large count slows attackers down.
 *
 *   DK   = T_1 || T_2 || ... || T_n            (first key_length bytes)
 *   T_i  = U_1 XOR U_2 XOR ... XOR U_c
 *   U_1  = PRF(Password, Salt || INT_32_BE(i))
 *   U_j  = PRF(Password, U_{j-1})              for j = 2..c
 *
 * The block index `i` is appended to the salt as a 4-byte big-endian integer,
 * making each block's first U value distinct.
 *
 * Real-world uses: WPA2 Wi-Fi (HMAC-SHA1, 4096 iters), Django / macOS Keychain
 * (HMAC-SHA256), LUKS disk encryption.
 *
 * This port derives into a caller-provided buffer (no ownership transfer). The
 * PRF is HMAC over SHA-1 / SHA-256 / SHA-512 (the sibling packages); a generic
 * entry point lets you plug in any hash.
 *
 * Portability: pure ISO C17 — GCC, Clang, and MSVC with -pedantic-errors /
 * /permissive- and warnings-as-errors.
 */
#ifndef PBKDF2_H
#define PBKDF2_H

#include <stddef.h> /* size_t */
#include <stdint.h> /* uint8_t */

/* Status codes (mirroring the Rust crate's Pbkdf2Error). */
typedef enum {
    PBKDF2_OK = 0,
    PBKDF2_EMPTY_PASSWORD,       /* password empty and not explicitly allowed */
    PBKDF2_INVALID_ITERATIONS,   /* iterations == 0 */
    PBKDF2_INVALID_KEY_LENGTH,   /* key_length == 0 */
    PBKDF2_KEY_LENGTH_TOO_LARGE, /* key_length > 2^20 (1 MiB practical cap) */
    PBKDF2_PRF_ERROR,            /* the PRF (HMAC) failed, e.g. out of memory */
    PBKDF2_BAD_ARGS              /* NULL buffer or an implausible hash size */
} Pbkdf2Status;

/* A one-shot hash: hash `len` bytes of `data` into `out` (h_len bytes). This is
 * the same signature the sibling `hmac`/`sha*` packages use — pass `sha1`,
 * `sha256`, or `sha512` directly. */
typedef void (*pbkdf2_hash_fn)(const void *data, size_t len, uint8_t *out);

/* The practical maximum key length (1 MiB): bounds memory and keeps the block
 * counter within 32 bits, as the Rust crate does. */
#define PBKDF2_MAX_KEY_LENGTH (1u << 20)

/* pbkdf2 — the generic core. Derives `key_length` bytes into `dk_out` using
 * HMAC(hash, block_size) as the PRF, where the hash produces `h_len` bytes.
 * `dk_out` must have room for `key_length` bytes. If `allow_empty_password` is
 * 0 an empty password is rejected; if 1 it is permitted (RFC vectors, scrypt).
 * Returns PBKDF2_OK on success. */
Pbkdf2Status pbkdf2(pbkdf2_hash_fn hash, size_t h_len, size_t block_size,
                    const uint8_t *password, size_t password_len,
                    const uint8_t *salt, size_t salt_len, size_t iterations,
                    uint8_t *dk_out, size_t key_length,
                    int allow_empty_password);

/* Convenience wrappers over the standard PRFs (h_len / block_size fixed). */
Pbkdf2Status pbkdf2_hmac_sha1(const uint8_t *password, size_t password_len,
                              const uint8_t *salt, size_t salt_len,
                              size_t iterations, uint8_t *dk_out,
                              size_t key_length, int allow_empty_password);
Pbkdf2Status pbkdf2_hmac_sha256(const uint8_t *password, size_t password_len,
                                const uint8_t *salt, size_t salt_len,
                                size_t iterations, uint8_t *dk_out,
                                size_t key_length, int allow_empty_password);
Pbkdf2Status pbkdf2_hmac_sha512(const uint8_t *password, size_t password_len,
                                const uint8_t *salt, size_t salt_len,
                                size_t iterations, uint8_t *dk_out,
                                size_t key_length, int allow_empty_password);

#endif /* PBKDF2_H */
