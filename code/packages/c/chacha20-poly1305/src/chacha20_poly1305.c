/*
 * chacha20_poly1305.c — ChaCha20 + Poly1305 + the ChaCha20-Poly1305 AEAD
 * (RFC 8439). ChaCha20 is the standard algorithm; Poly1305 uses the
 * "poly1305-donna" 32-bit representation (five 26-bit limbs) so no 128-bit
 * integer is required. Output matches the RFC 8439 test vectors.
 */
#include "chacha20_poly1305.h"

#include <stdint.h> /* SIZE_MAX */
#include <stdlib.h> /* malloc, free */
#include <string.h> /* memcpy, memset */

/* ── little-endian helpers ────────────────────────────────────────────────── */
static uint32_t load32(const uint8_t *p) {
    return (uint32_t)p[0] | ((uint32_t)p[1] << 8) | ((uint32_t)p[2] << 16) |
           ((uint32_t)p[3] << 24);
}
static void store32(uint8_t *p, uint32_t v) {
    p[0] = (uint8_t)v;
    p[1] = (uint8_t)(v >> 8);
    p[2] = (uint8_t)(v >> 16);
    p[3] = (uint8_t)(v >> 24);
}

/* ── ChaCha20 ─────────────────────────────────────────────────────────────── */
static uint32_t rotl32(uint32_t x, unsigned n) {
    return (x << n) | (x >> (32 - n));
}

#define QR(s, a, b, c, d)                                                      \
    do {                                                                       \
        s[a] += s[b];                                                          \
        s[d] = rotl32(s[d] ^ s[a], 16);                                        \
        s[c] += s[d];                                                          \
        s[b] = rotl32(s[b] ^ s[c], 12);                                        \
        s[a] += s[b];                                                          \
        s[d] = rotl32(s[d] ^ s[a], 8);                                         \
        s[c] += s[d];                                                          \
        s[b] = rotl32(s[b] ^ s[c], 7);                                         \
    } while (0)

static void chacha20_block(const uint8_t key[32], const uint8_t nonce[12],
                           uint32_t counter, uint8_t out[64]) {
    uint32_t state[16];
    uint32_t working[16];
    unsigned i;
    state[0] = 0x61707865u;
    state[1] = 0x3320646eu;
    state[2] = 0x79622d32u;
    state[3] = 0x6b206574u;
    for (i = 0; i < 8; i++) {
        state[4 + i] = load32(key + i * 4);
    }
    state[12] = counter;
    state[13] = load32(nonce + 0);
    state[14] = load32(nonce + 4);
    state[15] = load32(nonce + 8);

    memcpy(working, state, sizeof state);
    for (i = 0; i < 10; i++) {
        QR(working, 0, 4, 8, 12);
        QR(working, 1, 5, 9, 13);
        QR(working, 2, 6, 10, 14);
        QR(working, 3, 7, 11, 15);
        QR(working, 0, 5, 10, 15);
        QR(working, 1, 6, 11, 12);
        QR(working, 2, 7, 8, 13);
        QR(working, 3, 4, 9, 14);
    }
    for (i = 0; i < 16; i++) {
        store32(out + i * 4, working[i] + state[i]);
    }
}

void chacha20_encrypt(const uint8_t *input, size_t len, const uint8_t key[32],
                      const uint8_t nonce[12], uint32_t counter,
                      uint8_t *output) {
    uint8_t block[64];
    size_t offset = 0;
    while (offset < len) {
        size_t take = len - offset;
        size_t i;
        if (take > 64) {
            take = 64;
        }
        chacha20_block(key, nonce, counter, block);
        counter++;
        for (i = 0; i < take; i++) {
            output[offset + i] = input[offset + i] ^ block[i];
        }
        offset += take;
    }
}

/* ── Poly1305 (poly1305-donna, 32-bit) ────────────────────────────────────── */
void poly1305_mac(const uint8_t *message, size_t len, const uint8_t key[32],
                  uint8_t tag[16]) {
    uint32_t r0, r1, r2, r3, r4;
    uint32_t s1, s2, s3, s4;
    uint32_t h0 = 0, h1 = 0, h2 = 0, h3 = 0, h4 = 0;
    uint32_t t0, t1, t2, t3;
    uint64_t d0, d1, d2, d3, d4;
    uint32_t c;
    uint32_t g0, g1, g2, g3, g4;
    uint32_t mask;
    uint64_t f;
    size_t offset = 0;

    /* Clamp r and split into 26-bit limbs. */
    t0 = load32(key + 0);
    t1 = load32(key + 4);
    t2 = load32(key + 8);
    t3 = load32(key + 12);
    r0 = t0 & 0x3ffffffu;
    r1 = ((t0 >> 26) | (t1 << 6)) & 0x3ffff03u;
    r2 = ((t1 >> 20) | (t2 << 12)) & 0x3ffc0ffu;
    r3 = ((t2 >> 14) | (t3 << 18)) & 0x3f03fffu;
    r4 = (t3 >> 8) & 0x00fffffu;
    s1 = r1 * 5;
    s2 = r2 * 5;
    s3 = r3 * 5;
    s4 = r4 * 5;

    while (offset < len) {
        uint8_t buf[16];
        uint32_t hibit;
        size_t take = len - offset;
        if (take >= 16) {
            take = 16;
            memcpy(buf, message + offset, 16);
            hibit = (uint32_t)1 << 24; /* full block: the 2^128 bit */
        } else {
            memset(buf, 0, sizeof buf);
            memcpy(buf, message + offset, take);
            buf[take] = 1; /* partial block: the padding 1 bit */
            hibit = 0;
        }
        offset += take;

        t0 = load32(buf + 0);
        t1 = load32(buf + 4);
        t2 = load32(buf + 8);
        t3 = load32(buf + 12);
        h0 += t0 & 0x3ffffffu;
        h1 += ((t0 >> 26) | (t1 << 6)) & 0x3ffffffu;
        h2 += ((t1 >> 20) | (t2 << 12)) & 0x3ffffffu;
        h3 += ((t2 >> 14) | (t3 << 18)) & 0x3ffffffu;
        h4 += (t3 >> 8) | hibit;

        /* h *= r  (mod 2^130 - 5) */
        d0 = (uint64_t)h0 * r0 + (uint64_t)h1 * s4 + (uint64_t)h2 * s3 +
             (uint64_t)h3 * s2 + (uint64_t)h4 * s1;
        d1 = (uint64_t)h0 * r1 + (uint64_t)h1 * r0 + (uint64_t)h2 * s4 +
             (uint64_t)h3 * s3 + (uint64_t)h4 * s2;
        d2 = (uint64_t)h0 * r2 + (uint64_t)h1 * r1 + (uint64_t)h2 * r0 +
             (uint64_t)h3 * s4 + (uint64_t)h4 * s3;
        d3 = (uint64_t)h0 * r3 + (uint64_t)h1 * r2 + (uint64_t)h2 * r1 +
             (uint64_t)h3 * r0 + (uint64_t)h4 * s4;
        d4 = (uint64_t)h0 * r4 + (uint64_t)h1 * r3 + (uint64_t)h2 * r2 +
             (uint64_t)h3 * r1 + (uint64_t)h4 * r0;

        c = (uint32_t)(d0 >> 26);
        h0 = (uint32_t)d0 & 0x3ffffffu;
        d1 += c;
        c = (uint32_t)(d1 >> 26);
        h1 = (uint32_t)d1 & 0x3ffffffu;
        d2 += c;
        c = (uint32_t)(d2 >> 26);
        h2 = (uint32_t)d2 & 0x3ffffffu;
        d3 += c;
        c = (uint32_t)(d3 >> 26);
        h3 = (uint32_t)d3 & 0x3ffffffu;
        d4 += c;
        c = (uint32_t)(d4 >> 26);
        h4 = (uint32_t)d4 & 0x3ffffffu;
        h0 += c * 5;
        c = h0 >> 26;
        h0 &= 0x3ffffffu;
        h1 += c;
    }

    /* Fully carry h. */
    c = h1 >> 26;
    h1 &= 0x3ffffffu;
    h2 += c;
    c = h2 >> 26;
    h2 &= 0x3ffffffu;
    h3 += c;
    c = h3 >> 26;
    h3 &= 0x3ffffffu;
    h4 += c;
    c = h4 >> 26;
    h4 &= 0x3ffffffu;
    h0 += c * 5;
    c = h0 >> 26;
    h0 &= 0x3ffffffu;
    h1 += c;

    /* Compute h - p (p = 2^130 - 5); select h if it was < p, else h - p. */
    g0 = h0 + 5;
    c = g0 >> 26;
    g0 &= 0x3ffffffu;
    g1 = h1 + c;
    c = g1 >> 26;
    g1 &= 0x3ffffffu;
    g2 = h2 + c;
    c = g2 >> 26;
    g2 &= 0x3ffffffu;
    g3 = h3 + c;
    c = g3 >> 26;
    g3 &= 0x3ffffffu;
    g4 = h4 + c - ((uint32_t)1 << 26);

    mask = (g4 >> 31) - 1; /* 0 if h < p (keep h), all-ones if h >= p (use g) */
    g0 &= mask;
    g1 &= mask;
    g2 &= mask;
    g3 &= mask;
    g4 &= mask;
    mask = ~mask;
    h0 = (h0 & mask) | g0;
    h1 = (h1 & mask) | g1;
    h2 = (h2 & mask) | g2;
    h3 = (h3 & mask) | g3;
    h4 = (h4 & mask) | g4;

    /* Reassemble the 128-bit value and add s (key bytes 16..32) mod 2^128. */
    h0 = (h0 | (h1 << 26)) & 0xffffffffu;
    h1 = ((h1 >> 6) | (h2 << 20)) & 0xffffffffu;
    h2 = ((h2 >> 12) | (h3 << 14)) & 0xffffffffu;
    h3 = ((h3 >> 18) | (h4 << 8)) & 0xffffffffu;

    f = (uint64_t)h0 + load32(key + 16);
    h0 = (uint32_t)f;
    f = (uint64_t)h1 + load32(key + 20) + (f >> 32);
    h1 = (uint32_t)f;
    f = (uint64_t)h2 + load32(key + 24) + (f >> 32);
    h2 = (uint32_t)f;
    f = (uint64_t)h3 + load32(key + 28) + (f >> 32);
    h3 = (uint32_t)f;

    store32(tag + 0, h0);
    store32(tag + 4, h1);
    store32(tag + 8, h2);
    store32(tag + 12, h3);
}

/* ── AEAD (RFC 8439 §2.8) ─────────────────────────────────────────────────── */
static size_t pad16(size_t n) {
    return (n % 16 == 0) ? 0 : (16 - (n % 16));
}

/* Build the Poly1305 input: aad || pad || ciphertext || pad || le64(aad_len) ||
 * le64(ct_len). Returns a malloc'd buffer and its length, or NULL. */
static uint8_t *build_mac_data(const uint8_t *aad, size_t aad_len,
                               const uint8_t *ct, size_t ct_len, size_t *out_len) {
    size_t aad_pad = pad16(aad_len);
    size_t ct_pad = pad16(ct_len);
    size_t total;
    uint8_t *data;
    size_t off = 0;
    unsigned i;

    /* total = aad_len + aad_pad + ct_len + ct_pad + 16; guard every addition so
     * the final sum can never wrap size_t (which would under-size the malloc). */
    if (aad_len > SIZE_MAX - aad_pad || ct_len > SIZE_MAX - ct_pad) {
        return NULL;
    }
    total = aad_len + aad_pad;
    if (ct_len + ct_pad > SIZE_MAX - 16 ||
        total > SIZE_MAX - (ct_len + ct_pad) - 16) {
        return NULL;
    }
    total += ct_len + ct_pad + 16;

    data = (uint8_t *)malloc(total);
    if (data == NULL) {
        return NULL;
    }
    if (aad_len > 0) {
        memcpy(data + off, aad, aad_len);
    }
    off += aad_len;
    memset(data + off, 0, aad_pad);
    off += aad_pad;
    if (ct_len > 0) {
        memcpy(data + off, ct, ct_len);
    }
    off += ct_len;
    memset(data + off, 0, ct_pad);
    off += ct_pad;
    for (i = 0; i < 8; i++) {
        data[off + i] = (uint8_t)((uint64_t)aad_len >> (i * 8));
    }
    off += 8;
    for (i = 0; i < 8; i++) {
        data[off + i] = (uint8_t)((uint64_t)ct_len >> (i * 8));
    }
    *out_len = total;
    return data;
}

/* Derive the one-time Poly1305 key: the first 32 bytes of ChaCha20 block 0. */
static void poly_key_gen(const uint8_t key[32], const uint8_t nonce[12],
                         uint8_t poly_key[32]) {
    uint8_t block[64];
    chacha20_block(key, nonce, 0, block);
    memcpy(poly_key, block, 32);
}

int aead_encrypt(const uint8_t *plaintext, size_t plaintext_len,
                 const uint8_t key[32], const uint8_t nonce[12],
                 const uint8_t *aad, size_t aad_len, uint8_t *ciphertext,
                 uint8_t tag[16]) {
    uint8_t poly_key[32];
    uint8_t *mac_data;
    size_t mac_len;

    poly_key_gen(key, nonce, poly_key);
    chacha20_encrypt(plaintext, plaintext_len, key, nonce, 1, ciphertext);
    mac_data = build_mac_data(aad, aad_len, ciphertext, plaintext_len, &mac_len);
    if (mac_data == NULL) {
        return 0;
    }
    poly1305_mac(mac_data, mac_len, poly_key, tag);
    free(mac_data);
    return 1;
}

int aead_decrypt(const uint8_t *ciphertext, size_t ciphertext_len,
                 const uint8_t key[32], const uint8_t nonce[12],
                 const uint8_t *aad, size_t aad_len, const uint8_t tag[16],
                 uint8_t *plaintext) {
    uint8_t poly_key[32];
    uint8_t expected[16];
    uint8_t *mac_data;
    size_t mac_len;
    uint8_t diff = 0;
    unsigned i;

    poly_key_gen(key, nonce, poly_key);
    mac_data = build_mac_data(aad, aad_len, ciphertext, ciphertext_len, &mac_len);
    if (mac_data == NULL) {
        return 0;
    }
    poly1305_mac(mac_data, mac_len, poly_key, expected);
    free(mac_data);

    /* Constant-time tag comparison. */
    for (i = 0; i < 16; i++) {
        diff = (uint8_t)(diff | (expected[i] ^ tag[i]));
    }
    /* Decrypt regardless (caller must ignore plaintext when we return 0). */
    chacha20_encrypt(ciphertext, ciphertext_len, key, nonce, 1, plaintext);
    return diff == 0 ? 1 : 0;
}
