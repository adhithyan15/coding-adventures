/*
 * reed_solomon.c — implementation of Reed-Solomon coding (see reed_solomon.h).
 * A faithful port of the Rust `reed-solomon` crate: generator polynomial,
 * systematic encoding, and the syndromes -> Berlekamp-Massey -> Chien -> Forney
 * decode pipeline, all over GF(2^8) via the sibling gf256 package.
 *
 * Every polynomial here is bounded by the GF(256) block size (255 bytes), so all
 * working storage lives in fixed stack buffers — no heap allocation.
 */
#include "reed_solomon.h"

#include <string.h> /* memcpy, memset */

#include "gf256.h" /* gf256_add / multiply / divide / power (default 0x11D) */

#define RS_BUF 512 /* comfortably above any codeword/product length (<= ~382) */

/* ── GF(256) polynomial helpers ───────────────────────────────────────────── */
/* Evaluate a big-endian polynomial (p[0] = highest degree) at x (Horner). */
static uint8_t poly_eval_be(const uint8_t *p, size_t plen, uint8_t x) {
    uint8_t acc = 0;
    size_t i;
    for (i = 0; i < plen; i++) {
        acc = gf256_add(gf256_multiply(acc, x), p[i]);
    }
    return acc;
}
/* Evaluate a little-endian polynomial (p[i] = coeff of x^i) at x. */
static uint8_t poly_eval_le(const uint8_t *p, size_t plen, uint8_t x) {
    uint8_t acc = 0;
    size_t i;
    for (i = plen; i > 0; i--) {
        acc = gf256_add(gf256_multiply(acc, x), p[i - 1]);
    }
    return acc;
}
/* Multiply two little-endian polynomials (convolution) into out; returns len. */
static size_t poly_mul_le(const uint8_t *a, size_t alen, const uint8_t *b,
                          size_t blen, uint8_t *out) {
    size_t outlen, i, j;
    if (alen == 0 || blen == 0) {
        return 0;
    }
    outlen = alen + blen - 1;
    memset(out, 0, outlen);
    for (i = 0; i < alen; i++) {
        for (j = 0; j < blen; j++) {
            out[i + j] = gf256_add(out[i + j], gf256_multiply(a[i], b[j]));
        }
    }
    return outlen;
}
/* Remainder of big-endian division by a monic divisor; returns len (divlen-1). */
static size_t poly_mod_be(const uint8_t *dividend, size_t dlen,
                          const uint8_t *divisor, size_t divlen, uint8_t *out) {
    uint8_t rem[RS_BUF];
    size_t steps, i, j;
    memcpy(rem, dividend, dlen);
    if (dlen < divlen) {
        memcpy(out, rem, dlen);
        return dlen;
    }
    steps = dlen - divlen + 1;
    for (i = 0; i < steps; i++) {
        uint8_t coeff = rem[i];
        if (coeff == 0) {
            continue;
        }
        for (j = 0; j < divlen; j++) {
            rem[i + j] = gf256_add(rem[i + j], gf256_multiply(coeff, divisor[j]));
        }
    }
    memcpy(out, rem + (dlen - (divlen - 1)), divlen - 1);
    return divlen - 1;
}

/* ── generator polynomial ─────────────────────────────────────────────────── */
/* n_check must be even and >= 2, and small enough that a codeword still fits in
 * a GF(256) block (255 bytes) — with at least a 1-byte message, n_check <= 254.
 * The upper bound is also what keeps every fixed working buffer in bounds. */
static int valid_n_check(size_t n_check) {
    return n_check != 0 && n_check % 2 == 0 && n_check <= 254;
}

rs_status rs_build_generator(size_t n_check, uint8_t *out, size_t *out_len) {
    uint8_t g[RS_BUF];
    size_t glen = 1;
    size_t i, j;
    if (!valid_n_check(n_check)) {
        return RS_INVALID_INPUT;
    }
    g[0] = 1;
    for (i = 1; i <= n_check; i++) {
        uint8_t alpha_i = gf256_power(2, (uint32_t)i);
        uint8_t new_g[RS_BUF];
        memset(new_g, 0, glen + 1);
        for (j = 0; j < glen; j++) {
            new_g[j] = gf256_add(new_g[j], gf256_multiply(g[j], alpha_i));
            new_g[j + 1] = gf256_add(new_g[j + 1], g[j]);
        }
        memcpy(g, new_g, glen + 1);
        glen++;
    }
    memcpy(out, g, glen);
    *out_len = glen;
    return RS_OK;
}

/* ── encoding ─────────────────────────────────────────────────────────────── */
rs_status rs_encode(const uint8_t *message, size_t message_len, size_t n_check,
                    uint8_t *out, size_t *out_len) {
    uint8_t g_le[RS_BUF], g_be[RS_BUF], shifted[RS_BUF], rem[RS_BUF];
    size_t glen, remlen, n, i;
    if (!valid_n_check(n_check)) {
        return RS_INVALID_INPUT;
    }
    n = message_len + n_check;
    if (n > 255 || n < message_len) { /* second test guards size_t overflow */
        return RS_INVALID_INPUT;
    }
    rs_build_generator(n_check, g_le, &glen); /* glen = n_check + 1 */
    for (i = 0; i < glen; i++) {
        g_be[i] = g_le[glen - 1 - i]; /* big-endian, monic (g_be[0] = 1) */
    }
    /* shifted = message || n_check zeros  (= M(x) * x^n_check). */
    if (message_len) {
        memcpy(shifted, message, message_len);
    }
    memset(shifted + message_len, 0, n - message_len);
    remlen = poly_mod_be(shifted, n, g_be, glen, rem); /* remlen == n_check */
    /* codeword = message || (pad) || remainder. */
    if (message_len) {
        memcpy(out, message, message_len);
    }
    {
        size_t pad = n_check - remlen; /* 0 in the common case */
        memset(out + message_len, 0, pad);
        memcpy(out + message_len + pad, rem, remlen);
    }
    *out_len = n;
    return RS_OK;
}

/* ── decoding ─────────────────────────────────────────────────────────────── */
void rs_syndromes(const uint8_t *received, size_t received_len, size_t n_check,
                  uint8_t *out) {
    size_t i;
    for (i = 1; i <= n_check; i++) {
        out[i - 1] = poly_eval_be(received, received_len, gf256_power(2, (uint32_t)i));
    }
}

static int all_zero(const uint8_t *s, size_t n) {
    size_t i;
    for (i = 0; i < n; i++) {
        if (s[i] != 0) {
            return 0;
        }
    }
    return 1;
}

/* Berlekamp-Massey: fills `c_out` with the locator (LE, c[0]=1), sets *c_len,
 * returns the error count L. */
static size_t berlekamp_massey(const uint8_t *synds, size_t nsyn, uint8_t *c_out,
                               size_t *c_len) {
    uint8_t c[RS_BUF], b[RS_BUF];
    size_t clen = 1, blen = 1;
    size_t big_l = 0, x = 1;
    uint8_t b_scale = 1;
    size_t nn;
    c[0] = 1;
    b[0] = 1;
    for (nn = 0; nn < nsyn; nn++) {
        uint8_t d = synds[nn];
        size_t j, k;
        for (j = 1; j <= big_l; j++) {
            if (j < clen && nn >= j) {
                d = gf256_add(d, gf256_multiply(c[j], synds[nn - j]));
            }
        }
        if (d == 0) {
            x++;
        } else if (2 * big_l <= nn) {
            uint8_t tsave[RS_BUF];
            size_t tlen = clen;
            uint8_t scale = gf256_divide(d, b_scale);
            size_t shifted_len = x + blen;
            memcpy(tsave, c, clen);
            if (clen < shifted_len) {
                memset(c + clen, 0, shifted_len - clen);
                clen = shifted_len;
            }
            for (k = 0; k < blen; k++) {
                c[x + k] = gf256_add(c[x + k], gf256_multiply(scale, b[k]));
            }
            big_l = nn + 1 - big_l;
            memcpy(b, tsave, tlen);
            blen = tlen;
            b_scale = d;
            x = 1;
        } else {
            uint8_t scale = gf256_divide(d, b_scale);
            size_t shifted_len = x + blen;
            if (clen < shifted_len) {
                memset(c + clen, 0, shifted_len - clen);
                clen = shifted_len;
            }
            for (k = 0; k < blen; k++) {
                c[x + k] = gf256_add(c[x + k], gf256_multiply(scale, b[k]));
            }
            x++;
        }
    }
    memcpy(c_out, c, clen);
    *c_len = clen;
    return big_l;
}

size_t rs_error_locator(const uint8_t *syndromes, size_t nsyn, uint8_t *out) {
    size_t clen;
    /* A real syndrome sequence has length n_check <= 254; reject longer inputs
     * so the fixed Berlekamp-Massey buffers cannot overflow. */
    if (nsyn > 254) {
        return 0;
    }
    berlekamp_massey(syndromes, nsyn, out, &clen);
    return clen;
}

/* Inverse locator number for big-endian position p in a length-n codeword. */
static uint8_t inv_locator(size_t p, size_t n) {
    size_t exp = (p + 256 - n) % 255;
    return gf256_power(2, (uint32_t)exp);
}

/* Chien search: fills `positions` with error positions, returns the count. */
static size_t chien_search(const uint8_t *lambda, size_t llen, size_t n,
                           size_t *positions) {
    size_t p, count = 0;
    for (p = 0; p < n; p++) {
        uint8_t xi_inv = inv_locator(p, n);
        if (poly_eval_le(lambda, llen, xi_inv) == 0) {
            positions[count++] = p;
        }
    }
    return count;
}

/* Forney: error magnitudes for each position into `mags`. */
static rs_status forney(const uint8_t *lambda, size_t llen, const uint8_t *synds,
                        size_t nsyn, const size_t *positions, size_t npos,
                        size_t n, uint8_t *mags) {
    uint8_t omega_full[RS_BUF], lambda_prime[RS_BUF];
    size_t two_t = nsyn;
    size_t of_len = poly_mul_le(synds, nsyn, lambda, llen, omega_full);
    size_t omega_len = of_len < two_t ? of_len : two_t; /* Omega mod x^{2t} */
    size_t lplen = llen > 0 ? llen - 1 : 0;
    size_t j, i;
    memset(lambda_prime, 0, lplen ? lplen : 1);
    for (j = 0; j < llen; j++) {
        if (j % 2 == 1) {
            size_t out_idx = j - 1;
            if (out_idx < lplen) {
                lambda_prime[out_idx] = gf256_add(lambda_prime[out_idx], lambda[j]);
            }
        }
    }
    for (i = 0; i < npos; i++) {
        uint8_t xi_inv = inv_locator(positions[i], n);
        uint8_t omega_val = poly_eval_le(omega_full, omega_len, xi_inv);
        uint8_t lp_val = poly_eval_le(lambda_prime, lplen, xi_inv);
        if (lp_val == 0) {
            return RS_TOO_MANY_ERRORS;
        }
        mags[i] = gf256_divide(omega_val, lp_val);
    }
    return RS_OK;
}

rs_status rs_decode(const uint8_t *received, size_t received_len, size_t n_check,
                    uint8_t *out, size_t *out_len) {
    uint8_t synds[RS_BUF], lambda[RS_BUF], corrected[RS_BUF], mags[RS_BUF / 2];
    size_t positions[RS_BUF / 2];
    size_t t, n, k, llen, num_errors, npos, i;
    rs_status st;
    if (!valid_n_check(n_check)) {
        return RS_INVALID_INPUT;
    }
    /* A codeword cannot exceed the GF(256) block size; this also bounds every
     * fixed working buffer (received is copied into corrected[RS_BUF]). */
    if (received_len < n_check || received_len > 255) {
        return RS_INVALID_INPUT;
    }
    t = n_check / 2;
    n = received_len;
    k = n - n_check;

    rs_syndromes(received, received_len, n_check, synds);
    if (all_zero(synds, n_check)) {
        if (k) {
            memcpy(out, received, k);
        }
        *out_len = k;
        return RS_OK;
    }
    num_errors = berlekamp_massey(synds, n_check, lambda, &llen);
    if (num_errors > t) {
        return RS_TOO_MANY_ERRORS;
    }
    npos = chien_search(lambda, llen, n, positions);
    if (npos != num_errors) {
        return RS_TOO_MANY_ERRORS;
    }
    st = forney(lambda, llen, synds, n_check, positions, npos, n, mags);
    if (st != RS_OK) {
        return st;
    }
    memcpy(corrected, received, n);
    for (i = 0; i < npos; i++) {
        corrected[positions[i]] = gf256_add(corrected[positions[i]], mags[i]);
    }
    if (k) {
        memcpy(out, corrected, k);
    }
    *out_len = k;
    return RS_OK;
}
