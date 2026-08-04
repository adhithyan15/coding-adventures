/*
 * reed_solomon.h — Reed-Solomon error correction over GF(2^8), in pure ISO C17.
 * A faithful port of the Rust `reed-solomon` crate.
 * ===========================================================================
 *
 * Reed-Solomon codes add `n_check` parity bytes to a message so that up to
 * `t = n_check/2` corrupted bytes can be located AND corrected — the technique
 * behind QR codes, CDs/DVDs, and deep-space communication.
 *
 * A codeword is a polynomial over GF(2^8) divisible by a generator
 * g(x) = (x+a^1)(x+a^2)...(x+a^{n_check}) (a = 2). Encoding is systematic (the
 * message bytes appear first, then the check bytes). Decoding runs the classic
 * pipeline: syndromes -> Berlekamp-Massey (error locator) -> Chien search
 * (positions) -> Forney (magnitudes) -> correct.
 *
 *   rs_encode / rs_decode        — the code
 *   rs_build_generator           — the generator polynomial (little-endian)
 *   rs_syndromes / rs_error_locator — decode internals, exposed for inspection
 *
 * The field arithmetic comes from the sibling `gf256` package (default
 * Reed-Solomon polynomial 0x11D).
 *
 * Constraints: `n_check` must be even and >= 2, and the total codeword length
 * (message + check) must not exceed 255 bytes (the GF(256) block limit).
 *
 * Buffers are caller-provided; nothing is heap-allocated. An output buffer must
 * hold `message_len + n_check` bytes for encode, `received_len - n_check` for
 * decode, and `n_check + 1` for the generator.
 *
 * Portability: pure ISO C17 — GCC, Clang, and MSVC with -pedantic-errors /
 * /permissive- and warnings-as-errors. No extensions.
 */
#ifndef REED_SOLOMON_H
#define REED_SOLOMON_H

#include <stddef.h> /* size_t */
#include <stdint.h> /* uint8_t */

typedef enum {
    RS_OK = 0,
    RS_TOO_MANY_ERRORS, /* more than t = n_check/2 errors — unrecoverable */
    RS_INVALID_INPUT    /* bad n_check, or codeword length out of range */
} rs_status;

/* rs_build_generator — the RS generator polynomial for `n_check` check bytes, in
 * little-endian form (index = degree), length `n_check + 1`. `out` must hold at
 * least `n_check + 1` bytes; *out_len is set. */
rs_status rs_build_generator(size_t n_check, uint8_t *out, size_t *out_len);

/* rs_encode — systematic RS encoding: writes `message_len + n_check` bytes
 * (message followed by check bytes) into `out` and sets *out_len. */
rs_status rs_encode(const uint8_t *message, size_t message_len, size_t n_check,
                    uint8_t *out, size_t *out_len);

/* rs_decode — correct up to t = n_check/2 errors in `received` and write the
 * recovered `received_len - n_check` message bytes into `out` (setting
 * *out_len). Returns RS_TOO_MANY_ERRORS if more than t bytes were corrupted. */
rs_status rs_decode(const uint8_t *received, size_t received_len, size_t n_check,
                    uint8_t *out, size_t *out_len);

/* rs_syndromes — compute the `n_check` syndromes of `received` into `out`
 * (which must hold `n_check` bytes). All zero means no errors detected. */
void rs_syndromes(const uint8_t *received, size_t received_len, size_t n_check,
                  uint8_t *out);

/* rs_error_locator — the error locator polynomial Lambda(x) (little-endian,
 * Lambda[0] = 1) from `syndromes`. Writes it into `out` (capacity >= nsyn + 1)
 * and returns its length. */
size_t rs_error_locator(const uint8_t *syndromes, size_t nsyn, uint8_t *out);

#endif /* REED_SOLOMON_H */
