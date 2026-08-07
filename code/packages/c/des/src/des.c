/*
 * des.c — implementation of DES (see des.h). A faithful port of the Rust `des`
 * crate: the standard FIPS 46 algorithm on a bit-array representation (each bit
 * held in one byte), so the permutation tables read exactly as published.
 */
#include "des.h"

#include <stdlib.h> /* malloc, free */
#include <string.h> /* memcpy */

/* ── permutation / substitution tables (0-indexed, from FIPS 46) ──────────── */
static const uint8_t IP[64] = {
    57, 49, 41, 33, 25, 17, 9,  1,  59, 51, 43, 35, 27, 19, 11, 3,
    61, 53, 45, 37, 29, 21, 13, 5,  63, 55, 47, 39, 31, 23, 15, 7,
    56, 48, 40, 32, 24, 16, 8,  0,  58, 50, 42, 34, 26, 18, 10, 2,
    60, 52, 44, 36, 28, 20, 12, 4,  62, 54, 46, 38, 30, 22, 14, 6};

static const uint8_t FP[64] = {
    39, 7, 47, 15, 55, 23, 63, 31, 38, 6, 46, 14, 54, 22, 62, 30,
    37, 5, 45, 13, 53, 21, 61, 29, 36, 4, 44, 12, 52, 20, 60, 28,
    35, 3, 43, 11, 51, 19, 59, 27, 34, 2, 42, 10, 50, 18, 58, 26,
    33, 1, 41, 9,  49, 17, 57, 25, 32, 0, 40, 8,  48, 16, 56, 24};

static const uint8_t PC1[56] = {
    56, 48, 40, 32, 24, 16, 8,  0,  57, 49, 41, 33, 25, 17,
    9,  1,  58, 50, 42, 34, 26, 18, 10, 2,  59, 51, 43, 35,
    62, 54, 46, 38, 30, 22, 14, 6,  61, 53, 45, 37, 29, 21,
    13, 5,  60, 52, 44, 36, 28, 20, 12, 4,  27, 19, 11, 3};

static const uint8_t PC2[48] = {
    13, 16, 10, 23, 0,  4,  2,  27, 14, 5,  20, 9,  22, 18, 11, 3,
    25, 7,  15, 6,  26, 19, 12, 1,  40, 51, 30, 36, 46, 54, 29, 39,
    50, 44, 32, 47, 43, 48, 38, 55, 33, 52, 45, 41, 49, 35, 28, 31};

static const uint8_t E[48] = {
    31, 0,  1,  2,  3,  4,  3,  4,  5,  6,  7,  8,  7,  8,  9,  10,
    11, 12, 11, 12, 13, 14, 15, 16, 15, 16, 17, 18, 19, 20, 19, 20,
    21, 22, 23, 24, 23, 24, 25, 26, 27, 28, 27, 28, 29, 30, 31, 0};

static const uint8_t P[32] = {15, 6,  19, 20, 28, 11, 27, 16, 0,  14, 22,
                              25, 4,  17, 30, 9,  1,  7,  23, 13, 31, 26,
                              2,  8,  18, 12, 29, 5,  21, 10, 3,  24};

static const uint8_t SHIFTS[16] = {1, 1, 2, 2, 2, 2, 2, 2,
                                   1, 2, 2, 2, 2, 2, 2, 1};

static const uint8_t SBOXES[8][4][16] = {
    {{14, 4, 13, 1, 2, 15, 11, 8, 3, 10, 6, 12, 5, 9, 0, 7},
     {0, 15, 7, 4, 14, 2, 13, 1, 10, 6, 12, 11, 9, 5, 3, 8},
     {4, 1, 14, 8, 13, 6, 2, 11, 15, 12, 9, 7, 3, 10, 5, 0},
     {15, 12, 8, 2, 4, 9, 1, 7, 5, 11, 3, 14, 10, 0, 6, 13}},
    {{15, 1, 8, 14, 6, 11, 3, 4, 9, 7, 2, 13, 12, 0, 5, 10},
     {3, 13, 4, 7, 15, 2, 8, 14, 12, 0, 1, 10, 6, 9, 11, 5},
     {0, 14, 7, 11, 10, 4, 13, 1, 5, 8, 12, 6, 9, 3, 2, 15},
     {13, 8, 10, 1, 3, 15, 4, 2, 11, 6, 7, 12, 0, 5, 14, 9}},
    {{10, 0, 9, 14, 6, 3, 15, 5, 1, 13, 12, 7, 11, 4, 2, 8},
     {13, 7, 0, 9, 3, 4, 6, 10, 2, 8, 5, 14, 12, 11, 15, 1},
     {13, 6, 4, 9, 8, 15, 3, 0, 11, 1, 2, 12, 5, 10, 14, 7},
     {1, 10, 13, 0, 6, 9, 8, 7, 4, 15, 14, 3, 11, 5, 2, 12}},
    {{7, 13, 14, 3, 0, 6, 9, 10, 1, 2, 8, 5, 11, 12, 4, 15},
     {13, 8, 11, 5, 6, 15, 0, 3, 4, 7, 2, 12, 1, 10, 14, 9},
     {10, 6, 9, 0, 12, 11, 7, 13, 15, 1, 3, 14, 5, 2, 8, 4},
     {3, 15, 0, 6, 10, 1, 13, 8, 9, 4, 5, 11, 12, 7, 2, 14}},
    {{2, 12, 4, 1, 7, 10, 11, 6, 8, 5, 3, 15, 13, 0, 14, 9},
     {14, 11, 2, 12, 4, 7, 13, 1, 5, 0, 15, 10, 3, 9, 8, 6},
     {4, 2, 1, 11, 10, 13, 7, 8, 15, 9, 12, 5, 6, 3, 0, 14},
     {11, 8, 12, 7, 1, 14, 2, 13, 6, 15, 0, 9, 10, 4, 5, 3}},
    {{12, 1, 10, 15, 9, 2, 6, 8, 0, 13, 3, 4, 14, 7, 5, 11},
     {10, 15, 4, 2, 7, 12, 9, 5, 6, 1, 13, 14, 0, 11, 3, 8},
     {9, 14, 15, 5, 2, 8, 12, 3, 7, 0, 4, 10, 1, 13, 11, 6},
     {4, 3, 2, 12, 9, 5, 15, 10, 11, 14, 1, 7, 6, 0, 8, 13}},
    {{4, 11, 2, 14, 15, 0, 8, 13, 3, 12, 9, 7, 5, 10, 6, 1},
     {13, 0, 11, 7, 4, 9, 1, 10, 14, 3, 5, 12, 2, 15, 8, 6},
     {1, 4, 11, 13, 12, 3, 7, 14, 10, 15, 6, 8, 0, 5, 9, 2},
     {6, 11, 13, 8, 1, 4, 10, 7, 9, 5, 0, 15, 14, 2, 3, 12}},
    {{13, 2, 8, 4, 6, 15, 11, 1, 10, 9, 3, 14, 5, 0, 12, 7},
     {1, 15, 13, 8, 10, 3, 7, 4, 12, 5, 6, 11, 0, 14, 9, 2},
     {7, 11, 4, 1, 9, 12, 14, 2, 0, 6, 10, 13, 15, 3, 5, 8},
     {2, 1, 14, 7, 4, 10, 8, 13, 15, 12, 9, 0, 3, 5, 6, 11}}};

/* ── bit helpers (each bit is one byte, MSB first) ────────────────────────── */
static void bytes_to_bits(const uint8_t *data, size_t nbytes, uint8_t *bits) {
    size_t b, k;
    for (b = 0; b < nbytes; b++) {
        for (k = 0; k < 8; k++) {
            bits[b * 8 + k] = (uint8_t)((data[b] >> (7 - k)) & 1);
        }
    }
}

static void bits_to_bytes(const uint8_t *bits, size_t nbits, uint8_t *bytes) {
    size_t b, k;
    for (b = 0; b < nbits / 8; b++) {
        uint8_t byte = 0;
        for (k = 0; k < 8; k++) {
            byte = (uint8_t)((byte << 1) | bits[b * 8 + k]);
        }
        bytes[b] = byte;
    }
}

/* out[i] = in[table[i]] */
static void permute(const uint8_t *in, const uint8_t *table, size_t tlen,
                    uint8_t *out) {
    size_t i;
    for (i = 0; i < tlen; i++) {
        out[i] = in[table[i]];
    }
}

static void left_rotate_28(uint8_t half[28], unsigned n) {
    uint8_t tmp[28];
    memcpy(tmp, half + n, 28 - n);
    memcpy(tmp + (28 - n), half, n);
    memcpy(half, tmp, 28);
}

/* ── key schedule ─────────────────────────────────────────────────────────── */
void des_expand_key(const uint8_t key[8], uint8_t subkeys[16][6]) {
    uint8_t key_bits[64];
    uint8_t permuted[56];
    uint8_t c[28], d[28];
    unsigned i;
    bytes_to_bits(key, 8, key_bits);
    permute(key_bits, PC1, 56, permuted);
    memcpy(c, permuted, 28);
    memcpy(d, permuted + 28, 28);
    for (i = 0; i < 16; i++) {
        uint8_t cd[56];
        uint8_t subkey_bits[48];
        left_rotate_28(c, SHIFTS[i]);
        left_rotate_28(d, SHIFTS[i]);
        memcpy(cd, c, 28);
        memcpy(cd + 28, d, 28);
        permute(cd, PC2, 48, subkey_bits);
        bits_to_bytes(subkey_bits, 48, subkeys[i]);
    }
}

/* ── round function f(R, K) ───────────────────────────────────────────────── */
static void feistel_f(const uint8_t right[32], const uint8_t subkey[6],
                      uint8_t out[32]) {
    uint8_t expanded[48];
    uint8_t sk_bits[48];
    uint8_t xored[48];
    uint8_t sbox_out[32];
    unsigned box, i, k;
    permute(right, E, 48, expanded);
    bytes_to_bits(subkey, 6, sk_bits);
    for (i = 0; i < 48; i++) {
        xored[i] = (uint8_t)(expanded[i] ^ sk_bits[i]);
    }
    for (box = 0; box < 8; box++) {
        const uint8_t *chunk = &xored[box * 6];
        unsigned row = (unsigned)((chunk[0] << 1) | chunk[5]);
        unsigned col = (unsigned)((chunk[1] << 3) | (chunk[2] << 2) |
                                  (chunk[3] << 1) | chunk[4]);
        uint8_t val = SBOXES[box][row][col];
        for (k = 0; k < 4; k++) {
            sbox_out[box * 4 + k] = (uint8_t)((val >> (3 - k)) & 1);
        }
    }
    permute(sbox_out, P, 32, out);
}

/* ── core block cipher (subkeys applied in the given order) ───────────────── */
/* `subkeys` is read-only, but is left unqualified: ISO C (before C23) forbids
 * implicitly converting uint8_t[16][6] to const uint8_t[16][6] at a call. */
static void des_block(const uint8_t block[8], uint8_t subkeys[16][6],
                      uint8_t out[8]) {
    uint8_t bits[64];
    uint8_t perm[64];
    uint8_t left[32], right[32];
    unsigned r, i;
    bytes_to_bits(block, 8, bits);
    permute(bits, IP, 64, perm);
    memcpy(left, perm, 32);
    memcpy(right, perm + 32, 32);
    for (r = 0; r < 16; r++) {
        uint8_t f_out[32];
        uint8_t new_right[32];
        feistel_f(right, subkeys[r], f_out);
        for (i = 0; i < 32; i++) {
            new_right[i] = (uint8_t)(left[i] ^ f_out[i]);
        }
        memcpy(left, right, 32);
        memcpy(right, new_right, 32);
    }
    /* Swap halves (R ∥ L), then the final permutation. */
    memcpy(perm, right, 32);
    memcpy(perm + 32, left, 32);
    {
        uint8_t result_bits[64];
        permute(perm, FP, 64, result_bits);
        bits_to_bytes(result_bits, 64, out);
    }
}

static void reverse_subkeys(uint8_t subkeys[16][6]) {
    unsigned i;
    for (i = 0; i < 8; i++) {
        uint8_t tmp[6];
        memcpy(tmp, subkeys[i], 6);
        memcpy(subkeys[i], subkeys[15 - i], 6);
        memcpy(subkeys[15 - i], tmp, 6);
    }
}

void des_encrypt_block(const uint8_t block[8], const uint8_t key[8],
                       uint8_t out[8]) {
    uint8_t subkeys[16][6];
    des_expand_key(key, subkeys);
    des_block(block, subkeys, out);
}

void des_decrypt_block(const uint8_t block[8], const uint8_t key[8],
                       uint8_t out[8]) {
    uint8_t subkeys[16][6];
    des_expand_key(key, subkeys);
    reverse_subkeys(subkeys);
    des_block(block, subkeys, out);
}

/* ── ECB mode with PKCS#7 padding ─────────────────────────────────────────── */
uint8_t *des_ecb_encrypt(const uint8_t *plaintext, size_t len,
                         const uint8_t key[8], size_t *out_len) {
    uint8_t subkeys[16][6];
    size_t pad_len = 8 - (len % 8); /* in 1..8 (a full block if aligned) */
    size_t total = len + pad_len;
    uint8_t *out;
    size_t off;
    if (total < len) {
        return NULL; /* size overflow (len within 8 of SIZE_MAX) */
    }
    out = (uint8_t *)malloc(total);
    if (out == NULL) {
        return NULL;
    }
    des_expand_key(key, subkeys);
    for (off = 0; off < total; off += 8) {
        uint8_t blk[8];
        size_t i;
        for (i = 0; i < 8; i++) {
            blk[i] = (off + i < len) ? plaintext[off + i] : (uint8_t)pad_len;
        }
        des_block(blk, subkeys, out + off);
    }
    *out_len = total;
    return out;
}

int des_ecb_decrypt(const uint8_t *ciphertext, size_t len, const uint8_t key[8],
                    uint8_t **out, size_t *out_len) {
    uint8_t subkeys[16][6];
    uint8_t *plain;
    size_t off, pad_len, i;
    if (len == 0 || len % 8 != 0) {
        return 0;
    }
    plain = (uint8_t *)malloc(len);
    if (plain == NULL) {
        return 0;
    }
    des_expand_key(key, subkeys);
    reverse_subkeys(subkeys);
    for (off = 0; off < len; off += 8) {
        des_block(ciphertext + off, subkeys, plain + off);
    }
    /* Validate and strip PKCS#7 padding. */
    pad_len = plain[len - 1];
    if (pad_len == 0 || pad_len > 8) {
        free(plain);
        return 0;
    }
    for (i = 0; i < pad_len; i++) {
        if (plain[len - 1 - i] != (uint8_t)pad_len) {
            free(plain);
            return 0;
        }
    }
    *out = plain;
    *out_len = len - pad_len;
    return 1;
}

/* ── Triple DES (EDE) ─────────────────────────────────────────────────────── */
void des_tdea_encrypt_block(const uint8_t block[8], const uint8_t k1[8],
                            const uint8_t k2[8], const uint8_t k3[8],
                            uint8_t out[8]) {
    uint8_t s1[8], s2[8];
    des_encrypt_block(block, k3, s1); /* E_k3(P) */
    des_decrypt_block(s1, k2, s2);    /* D_k2(...) */
    des_encrypt_block(s2, k1, out);   /* E_k1(...) */
}

void des_tdea_decrypt_block(const uint8_t block[8], const uint8_t k1[8],
                            const uint8_t k2[8], const uint8_t k3[8],
                            uint8_t out[8]) {
    uint8_t s1[8], s2[8];
    des_decrypt_block(block, k1, s1); /* D_k1(C) */
    des_encrypt_block(s1, k2, s2);    /* E_k2(...) */
    des_decrypt_block(s2, k3, out);   /* D_k3(...) */
}
