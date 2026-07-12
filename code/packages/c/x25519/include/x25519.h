/*
 * x25519.h — X25519 (Curve25519 ECDH), in pure ISO C17. A faithful port of the
 * Rust `x25519` crate.
 * ===========================================================================
 *
 * X25519 (RFC 7748) is the elliptic-curve Diffie-Hellman function on
 * Curve25519: a scalar multiplication `u' = k · u` on the Montgomery curve,
 * computed with a constant-time Montgomery ladder over the prime field
 * GF(2^255 - 19). It underpins TLS 1.3, Signal, WireGuard, and SSH key exchange.
 *
 * Field elements are represented in radix-2^51 (five 51-bit limbs), and the
 * schoolbook multiply accumulates 102-bit partial products — the Rust crate uses
 * `u128` for that; this port carries a small, contained 128-bit emulation
 * (64×64→128 multiply plus add/shift), so it needs no `__int128`.
 *
 * API. Keys and coordinates are 32-byte little-endian arrays; results are
 * written into a caller-provided 32-byte buffer. `x25519` returns -1 (rather
 * than the Rust `Err`) when the output is all zeros — the signal of a low-order
 * point input, which a Diffie-Hellman caller must reject.
 *
 * PORTABILITY. Pure ISO C17 — no `__int128`, no extensions. Builds clean under
 * GCC, Clang, and MSVC with -pedantic-errors / /permissive- and
 * warnings-as-errors. Verified against the RFC 7748 test vectors.
 */
#ifndef CA_X25519_H
#define CA_X25519_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* The standard Curve25519 base point (u = 9), 32 bytes little-endian. */
extern const uint8_t X25519_BASE_POINT[32];

/* Compute X25519: the u-coordinate of `scalar · u_coordinate`. Writes 32 bytes
 * to `out`. Returns 0 on success, or -1 if the result is all zeros (a low-order
 * point input, which is a security failure the caller must reject). */
int x25519(uint8_t out[32], const uint8_t scalar[32],
           const uint8_t u_coordinate[32]);

/* X25519 against the base point (u = 9): derive a public key from `scalar`. */
int x25519_base(uint8_t out[32], const uint8_t scalar[32]);

/* Generate a public key from a private key — an alias of `x25519_base`, for
 * API clarity. The private key should be 32 bytes of secure random data. */
int x25519_generate_keypair(uint8_t out[32], const uint8_t private_key[32]);

#ifdef __cplusplus
}
#endif

#endif /* CA_X25519_H */
