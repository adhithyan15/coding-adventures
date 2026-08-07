/*
 * scrypt.c — implementation of scrypt (see scrypt.h). A faithful port of the
 * Rust `scrypt` crate: the same Salsa20/8 core, BlockMix, ROMix, and the
 * PBKDF2-HMAC-SHA256 expand/extract steps.
 */
#include "scrypt.h"

#include <stdlib.h> /* calloc, malloc, free */
#include <string.h> /* memcpy */

#include "pbkdf2.h" /* pbkdf2_hmac_sha256 */

/* ---- Salsa20/8 core --------------------------------------------------- */

static uint32_t rotl32(uint32_t v, unsigned c) {
    return (v << c) | (v >> (32 - c));
}

/* Apply the Salsa20/8 core to a 64-byte block: decode 16 little-endian words,
 * 4 double-rounds of the quarter-round, add the original words back, re-encode.
 * `in` and `out` must not alias. */
static void salsa20_8(const uint8_t in[64], uint8_t out[64]) {
    uint32_t x[16];
    uint32_t z[16];
    int i;
    int round;

    for (i = 0; i < 16; i++) {
        x[i] = (uint32_t)in[i * 4] | ((uint32_t)in[i * 4 + 1] << 8) |
               ((uint32_t)in[i * 4 + 2] << 16) |
               ((uint32_t)in[i * 4 + 3] << 24);
        z[i] = x[i];
    }

#define QR(a, b, c, d)                                    \
    do {                                                  \
        x[b] ^= rotl32(x[a] + x[d], 7);                   \
        x[c] ^= rotl32(x[b] + x[a], 9);                   \
        x[d] ^= rotl32(x[c] + x[b], 13);                  \
        x[a] ^= rotl32(x[d] + x[c], 18);                  \
    } while (0)

    for (round = 0; round < 4; round++) {
        /* Column rounds. */
        QR(0, 4, 8, 12);
        QR(5, 9, 13, 1);
        QR(10, 14, 2, 6);
        QR(15, 3, 7, 11);
        /* Row rounds. */
        QR(0, 1, 2, 3);
        QR(5, 6, 7, 4);
        QR(10, 11, 8, 9);
        QR(15, 12, 13, 14);
    }
#undef QR

    for (i = 0; i < 16; i++) {
        uint32_t v = x[i] + z[i]; /* uint32 addition wraps (well-defined) */
        out[i * 4] = (uint8_t)(v & 0xFF);
        out[i * 4 + 1] = (uint8_t)((v >> 8) & 0xFF);
        out[i * 4 + 2] = (uint8_t)((v >> 16) & 0xFF);
        out[i * 4 + 3] = (uint8_t)((v >> 24) & 0xFF);
    }
}

/* ---- BlockMix ---------------------------------------------------------- */

/* BlockMix over 2r 64-byte blocks: `in` and `out` are each 128*r bytes and must
 * not alias. `scratch` holds two 64-byte working blocks (X and the XOR temp).
 * The output is written in the RFC 7914 even-then-odd interleaving. */
static void block_mix(const uint8_t *in, uint8_t *out, size_t r,
                      uint8_t *scratch) {
    uint8_t *xcur = scratch;       /* 64 bytes: the rolling X */
    uint8_t *xored = scratch + 64; /* 64 bytes: X XOR B_i */
    size_t two_r = 2 * r;
    size_t i;
    size_t k;

    /* X starts as the last block. */
    memcpy(xcur, in + (two_r - 1) * 64, 64);

    for (i = 0; i < two_r; i++) {
        size_t dst;
        for (k = 0; k < 64; k++) {
            xored[k] = (uint8_t)(xcur[k] ^ in[i * 64 + k]);
        }
        salsa20_8(xored, xcur); /* X = Salsa20/8(X XOR B_i) */
        /* Even step i -> out block i/2; odd step -> out block r + i/2. */
        dst = (i % 2 == 0) ? (i / 2) : (r + i / 2);
        memcpy(out + dst * 64, xcur, 64);
    }
}

/* ---- ROMix ------------------------------------------------------------- */

/* integerify: interpret the last 64-byte block of `blocks` (128*r bytes) as a
 * little-endian integer; only the low 8 bytes are needed for the index. */
static uint64_t integerify(const uint8_t *blocks, size_t r) {
    const uint8_t *last = blocks + (2 * r - 1) * 64;
    return (uint64_t)last[0] | ((uint64_t)last[1] << 8) |
           ((uint64_t)last[2] << 16) | ((uint64_t)last[3] << 24) |
           ((uint64_t)last[4] << 32) | ((uint64_t)last[5] << 40) |
           ((uint64_t)last[6] << 48) | ((uint64_t)last[7] << 56);
}

/* ROMix on a 128*r-byte block in place. Returns SCRYPT_OK or SCRYPT_ALLOC_ERROR
 * (the only failure — a working set too large for available memory). */
static ScryptStatus ro_mix(uint8_t *block, size_t n, size_t r) {
    size_t block_len = 128 * r; /* bounded by the caller's p*128*r <= 2^30 */
    uint8_t *v;                 /* N snapshots, N * block_len bytes */
    uint8_t *x;                 /* current state, block_len bytes */
    uint8_t *t;                 /* BlockMix output, block_len bytes */
    uint8_t scratch[128];       /* two 64-byte working blocks for BlockMix */
    size_t i;
    size_t k;

    /* calloc performs a checked multiply: NULL on overflow or OOM. */
    v = calloc(n, block_len);
    x = malloc(block_len);
    t = malloc(block_len);
    if (!v || !x || !t) {
        free(v);
        free(x);
        free(t);
        return SCRYPT_ALLOC_ERROR;
    }

    memcpy(x, block, block_len);

    /* Phase 1: fill the V table with N snapshots of the evolving state. */
    for (i = 0; i < n; i++) {
        memcpy(v + i * block_len, x, block_len);
        block_mix(x, t, r, scratch);
        {
            uint8_t *tmp = x; /* swap x and t (avoid a copy) */
            x = t;
            t = tmp;
        }
    }

    /* Phase 2: N data-dependent mixing steps. */
    for (i = 0; i < n; i++) {
        size_t j = (size_t)(integerify(x, r) % (uint64_t)n);
        const uint8_t *vj = v + j * block_len;
        for (k = 0; k < block_len; k++) {
            x[k] ^= vj[k];
        }
        block_mix(x, t, r, scratch);
        {
            uint8_t *tmp = x;
            x = t;
            t = tmp;
        }
    }

    memcpy(block, x, block_len);

    free(v);
    free(x);
    free(t);
    return SCRYPT_OK;
}

/* ---- public API -------------------------------------------------------- */

ScryptStatus scrypt(const uint8_t *password, size_t password_len,
                    const uint8_t *salt, size_t salt_len, size_t n, size_t r,
                    size_t p, size_t dk_len, uint8_t *out) {
    size_t b_len;
    uint8_t *b;
    size_t i;
    ScryptStatus rc;

    if (!out) {
        return SCRYPT_HMAC_ERROR; /* no output buffer */
    }

    /* ── parameter validation (order matters, per the Rust crate) ──────── */
    if (n > SCRYPT_MAX_N) {
        return SCRYPT_N_TOO_LARGE;
    }
    if (n < 2 || (n & (n - 1)) != 0) {
        return SCRYPT_INVALID_N;
    }
    if (r == 0) {
        return SCRYPT_INVALID_R;
    }
    if (p == 0) {
        return SCRYPT_INVALID_P;
    }
    if (dk_len == 0) {
        return SCRYPT_INVALID_KEY_LENGTH;
    }
    if (dk_len > SCRYPT_MAX_DK_LEN) {
        return SCRYPT_KEY_LENGTH_TOO_LARGE;
    }
    /* RFC 7914 §2: p*r <= 2^30. r >= 1 here, so `p > 2^30/r` is exactly
     * `p*r > 2^30` and never overflows. */
    if (p > ((size_t)1 << 30) / r) {
        return SCRYPT_PR_TOO_LARGE;
    }
    {
        size_t pr = p * r; /* now <= 2^30, so the product is safe */
        /* p*128*r = 128*pr is the Step-1 PBKDF2 output size; cap at 2^30. */
        if (pr > ((size_t)1 << 30) / 128) {
            return SCRYPT_PR_TOO_LARGE;
        }
        b_len = 128 * pr; /* <= 2^30, fits a 32-bit size_t */
    }

    /* ── Step 1: expand the password into B (p blocks of 128*r bytes) ──── */
    b = malloc(b_len);
    if (!b) {
        return SCRYPT_ALLOC_ERROR;
    }
    if (pbkdf2_hmac_sha256(password, password_len, salt, salt_len, 1, b, b_len,
                           1) != PBKDF2_OK) {
        free(b);
        return SCRYPT_HMAC_ERROR;
    }

    /* ── Step 2: ROMix each 128*r-byte block independently ─────────────── */
    for (i = 0; i < p; i++) {
        rc = ro_mix(b + i * 128 * r, n, r);
        if (rc != SCRYPT_OK) {
            free(b);
            return rc;
        }
    }

    /* ── Step 3: extract the final key (salt = B) ──────────────────────── */
    if (pbkdf2_hmac_sha256(password, password_len, b, b_len, 1, out, dk_len,
                           1) != PBKDF2_OK) {
        free(b);
        return SCRYPT_HMAC_ERROR;
    }

    free(b);
    return SCRYPT_OK;
}
