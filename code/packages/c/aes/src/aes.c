/*
 * aes.c — implementation of AES (see aes.h). A faithful port of the Rust `aes`
 * crate: the S-box is built from GF(2^8) inverses (via the sibling gf256 field
 * on polynomial 0x11B) plus the AES affine transform; the round steps are the
 * standard FIPS 197 SubBytes / ShiftRows / MixColumns / AddRoundKey.
 */
#include "aes.h"

#include <string.h> /* memcpy */

#include "gf256.h" /* gf256_field for the S-box inverse */

/* Round constants: Rcon[i] = x^(i-1) in GF(2^8). Index 0 unused. */
static const uint8_t RCON[15] = {0x00, 0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40,
                                 0x80, 0x1B, 0x36, 0x6C, 0xD8, 0xAB, 0x4D};

/* ── S-box construction (lazy, single-threaded) ───────────────────────────── */
static uint8_t g_sbox[256];
static uint8_t g_inv_sbox[256];
static int g_sb_ready = 0;

static uint8_t rotl8(uint8_t x, unsigned n) {
    return (uint8_t)((x << n) | (x >> (8 - n)));
}

/* The AES affine transform: s = b ^ rotl(b,1) ^ rotl(b,2) ^ rotl(b,3) ^
 * rotl(b,4) ^ 0x63. */
static uint8_t affine_transform(uint8_t b) {
    return (uint8_t)(b ^ rotl8(b, 1) ^ rotl8(b, 2) ^ rotl8(b, 3) ^ rotl8(b, 4) ^
                     0x63);
}

static void ensure_sboxes(void) {
    gf256_field f;
    int b;
    if (g_sb_ready) {
        return;
    }
    f = gf256_field_new(0x11B);
    for (b = 0; b < 256; b++) {
        uint8_t inv = (b == 0) ? 0 : gf256_field_inverse(&f, (uint8_t)b);
        g_sbox[b] = affine_transform(inv);
    }
    for (b = 0; b < 256; b++) {
        g_inv_sbox[g_sbox[b]] = (uint8_t)b;
    }
    g_sb_ready = 1;
}

const uint8_t *aes_sbox(void) {
    ensure_sboxes();
    return g_sbox;
}
const uint8_t *aes_inv_sbox(void) {
    ensure_sboxes();
    return g_inv_sbox;
}

/* ── key schedule ─────────────────────────────────────────────────────────── */
int aes_expand_key(const uint8_t *key, size_t key_len,
                   uint8_t round_keys[15][4][4], int *nr_out) {
    uint8_t w[60][4]; /* up to 4*(14+1) = 60 words for AES-256 */
    size_t nk, total, i;
    int nr, rk, col, row;
    if (key_len != 16 && key_len != 24 && key_len != 32) {
        return 0;
    }
    ensure_sboxes();
    nk = key_len / 4;
    nr = (nk == 4) ? 10 : (nk == 6) ? 12 : 14;
    total = 4 * ((size_t)nr + 1);
    for (i = 0; i < nk; i++) {
        w[i][0] = key[4 * i];
        w[i][1] = key[4 * i + 1];
        w[i][2] = key[4 * i + 2];
        w[i][3] = key[4 * i + 3];
    }
    for (i = nk; i < total; i++) {
        uint8_t temp[4];
        temp[0] = w[i - 1][0];
        temp[1] = w[i - 1][1];
        temp[2] = w[i - 1][2];
        temp[3] = w[i - 1][3];
        if (i % nk == 0) {
            uint8_t t0 = temp[0]; /* RotWord */
            temp[0] = temp[1];
            temp[1] = temp[2];
            temp[2] = temp[3];
            temp[3] = t0;
            temp[0] = g_sbox[temp[0]]; /* SubWord */
            temp[1] = g_sbox[temp[1]];
            temp[2] = g_sbox[temp[2]];
            temp[3] = g_sbox[temp[3]];
            temp[0] ^= RCON[i / nk];
        } else if (nk == 8 && i % nk == 4) {
            temp[0] = g_sbox[temp[0]]; /* extra SubWord for AES-256 */
            temp[1] = g_sbox[temp[1]];
            temp[2] = g_sbox[temp[2]];
            temp[3] = g_sbox[temp[3]];
        }
        w[i][0] = (uint8_t)(w[i - nk][0] ^ temp[0]);
        w[i][1] = (uint8_t)(w[i - nk][1] ^ temp[1]);
        w[i][2] = (uint8_t)(w[i - nk][2] ^ temp[2]);
        w[i][3] = (uint8_t)(w[i - nk][3] ^ temp[3]);
    }
    /* Pack words into round keys: state[row][col] = w[4*rk + col][row]. */
    for (rk = 0; rk <= nr; rk++) {
        for (col = 0; col < 4; col++) {
            for (row = 0; row < 4; row++) {
                round_keys[rk][row][col] = w[4 * rk + col][row];
            }
        }
    }
    *nr_out = nr;
    return 1;
}

/* ── state transforms (operate in place on a 4x4 state) ───────────────────── */
static void bytes_to_state(const uint8_t block[16], uint8_t s[4][4]) {
    int col, row;
    for (col = 0; col < 4; col++) {
        for (row = 0; row < 4; row++) {
            s[row][col] = block[row + 4 * col];
        }
    }
}
static void state_to_bytes(uint8_t s[4][4], uint8_t out[16]) {
    int col, row;
    for (col = 0; col < 4; col++) {
        for (row = 0; row < 4; row++) {
            out[row + 4 * col] = s[row][col];
        }
    }
}
/* `rk` is read-only but left unqualified: ISO C (before C23) forbids implicitly
 * converting uint8_t[4][4] to const uint8_t[4][4] at a call site. */
static void add_round_key(uint8_t s[4][4], uint8_t rk[4][4]) {
    int r, c;
    for (r = 0; r < 4; r++) {
        for (c = 0; c < 4; c++) {
            s[r][c] = (uint8_t)(s[r][c] ^ rk[r][c]);
        }
    }
}
static void sub_bytes(uint8_t s[4][4]) {
    int r, c;
    for (r = 0; r < 4; r++) {
        for (c = 0; c < 4; c++) {
            s[r][c] = g_sbox[s[r][c]];
        }
    }
}
static void inv_sub_bytes(uint8_t s[4][4]) {
    int r, c;
    for (r = 0; r < 4; r++) {
        for (c = 0; c < 4; c++) {
            s[r][c] = g_inv_sbox[s[r][c]];
        }
    }
}
static void shift_rows(uint8_t s[4][4]) {
    uint8_t t[4][4];
    int r, c;
    for (r = 0; r < 4; r++) {
        for (c = 0; c < 4; c++) {
            t[r][c] = s[r][(c + r) % 4];
        }
    }
    memcpy(s, t, 16);
}
static void inv_shift_rows(uint8_t s[4][4]) {
    uint8_t t[4][4];
    int r, c;
    for (r = 0; r < 4; r++) {
        for (c = 0; c < 4; c++) {
            t[r][c] = s[r][(c + 4 - r) % 4];
        }
    }
    memcpy(s, t, 16);
}

/* Multiply by x (=2) in the AES field. */
static uint8_t xtime(uint8_t b) {
    uint8_t shifted = (uint8_t)(b << 1);
    return (b & 0x80) ? (uint8_t)(shifted ^ 0x1B) : shifted;
}
/* General GF(2^8) multiply (AES polynomial) for InvMixColumns. */
static uint8_t aes_mul(uint8_t a, uint8_t b) {
    uint8_t result = 0;
    uint8_t aa = a;
    uint8_t bb = b;
    int i;
    for (i = 0; i < 8; i++) {
        if (bb & 1) {
            result = (uint8_t)(result ^ aa);
        }
        {
            uint8_t hi = (uint8_t)(aa & 0x80);
            aa = (uint8_t)(aa << 1);
            if (hi) {
                aa = (uint8_t)(aa ^ 0x1B);
            }
        }
        bb = (uint8_t)(bb >> 1);
    }
    return result;
}

static void mix_columns(uint8_t s[4][4]) {
    uint8_t t[4][4];
    int col;
    for (col = 0; col < 4; col++) {
        uint8_t s0 = s[0][col], s1 = s[1][col], s2 = s[2][col], s3 = s[3][col];
        t[0][col] = (uint8_t)(xtime(s0) ^ (xtime(s1) ^ s1) ^ s2 ^ s3);
        t[1][col] = (uint8_t)(s0 ^ xtime(s1) ^ (xtime(s2) ^ s2) ^ s3);
        t[2][col] = (uint8_t)(s0 ^ s1 ^ xtime(s2) ^ (xtime(s3) ^ s3));
        t[3][col] = (uint8_t)((xtime(s0) ^ s0) ^ s1 ^ s2 ^ xtime(s3));
    }
    memcpy(s, t, 16);
}
static void inv_mix_columns(uint8_t s[4][4]) {
    uint8_t t[4][4];
    int col;
    for (col = 0; col < 4; col++) {
        uint8_t s0 = s[0][col], s1 = s[1][col], s2 = s[2][col], s3 = s[3][col];
        t[0][col] = (uint8_t)(aes_mul(0x0e, s0) ^ aes_mul(0x0b, s1) ^
                              aes_mul(0x0d, s2) ^ aes_mul(0x09, s3));
        t[1][col] = (uint8_t)(aes_mul(0x09, s0) ^ aes_mul(0x0e, s1) ^
                              aes_mul(0x0b, s2) ^ aes_mul(0x0d, s3));
        t[2][col] = (uint8_t)(aes_mul(0x0d, s0) ^ aes_mul(0x09, s1) ^
                              aes_mul(0x0e, s2) ^ aes_mul(0x0b, s3));
        t[3][col] = (uint8_t)(aes_mul(0x0b, s0) ^ aes_mul(0x0d, s1) ^
                              aes_mul(0x09, s2) ^ aes_mul(0x0e, s3));
    }
    memcpy(s, t, 16);
}

/* ── block cipher ─────────────────────────────────────────────────────────── */
int aes_encrypt_block(const uint8_t block[16], const uint8_t *key,
                      size_t key_len, uint8_t out[16]) {
    uint8_t round_keys[15][4][4];
    uint8_t state[4][4];
    int nr, rnd;
    if (!aes_expand_key(key, key_len, round_keys, &nr)) {
        return 0;
    }
    bytes_to_state(block, state);
    add_round_key(state, round_keys[0]);
    for (rnd = 1; rnd < nr; rnd++) {
        sub_bytes(state);
        shift_rows(state);
        mix_columns(state);
        add_round_key(state, round_keys[rnd]);
    }
    sub_bytes(state);
    shift_rows(state);
    add_round_key(state, round_keys[nr]);
    state_to_bytes(state, out);
    return 1;
}

int aes_decrypt_block(const uint8_t block[16], const uint8_t *key,
                      size_t key_len, uint8_t out[16]) {
    uint8_t round_keys[15][4][4];
    uint8_t state[4][4];
    int nr, rnd;
    if (!aes_expand_key(key, key_len, round_keys, &nr)) {
        return 0;
    }
    bytes_to_state(block, state);
    add_round_key(state, round_keys[nr]);
    for (rnd = nr - 1; rnd >= 1; rnd--) {
        inv_shift_rows(state);
        inv_sub_bytes(state);
        add_round_key(state, round_keys[rnd]);
        inv_mix_columns(state);
    }
    inv_shift_rows(state);
    inv_sub_bytes(state);
    add_round_key(state, round_keys[0]);
    state_to_bytes(state, out);
    return 1;
}
