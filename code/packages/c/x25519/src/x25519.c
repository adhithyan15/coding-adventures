/*
 * x25519.c — implementation of X25519 (Curve25519 ECDH).
 * ===========================================================================
 *
 * Layout mirrors the Rust crate:
 *   - a small emulated 128-bit unsigned integer (u128) — enough for the field
 *     multiply's 102-bit partial products — replacing Rust's native `u128`;
 *   - field elements in radix-2^51 (`Fe` = five u64 limbs) with add / sub /
 *     mul / square / invert / (de)serialize and a constant-time conditional
 *     swap; and
 *   - the constant-time Montgomery ladder plus the public entry points.
 */
#include "x25519.h"

#include <string.h>

/* ===========================================================================
 *  Emulated 128-bit unsigned integer
 *
 *  Only the operations the field arithmetic needs: a 64×64→128 multiply, add,
 *  left/right shift (by 0..127), bitwise or, and little-endian (de)serialize.
 * =========================================================================== */

typedef struct {
    uint64_t lo, hi;
} u128;

static u128 u128_from_u64(uint64_t v) {
    u128 r;
    r.lo = v;
    r.hi = 0;
    return r;
}

/* 64×64 → 128 via four 32×32 products. */
static u128 u128_mul_64(uint64_t a, uint64_t b) {
    uint64_t aL = a & 0xFFFFFFFFu, aH = a >> 32;
    uint64_t bL = b & 0xFFFFFFFFu, bH = b >> 32;
    uint64_t ll = aL * bL, lh = aL * bH, hl = aH * bL, hh = aH * bH;
    uint64_t cross = (ll >> 32) + (lh & 0xFFFFFFFFu) + (hl & 0xFFFFFFFFu);
    u128 r;
    r.lo = (ll & 0xFFFFFFFFu) | (cross << 32);
    r.hi = hh + (lh >> 32) + (hl >> 32) + (cross >> 32);
    return r;
}

static u128 u128_add(u128 a, u128 b) {
    u128 r;
    r.lo = a.lo + b.lo;
    r.hi = a.hi + b.hi + (r.lo < a.lo ? 1u : 0u);
    return r;
}

static u128 u128_or(u128 a, u128 b) {
    u128 r;
    r.lo = a.lo | b.lo;
    r.hi = a.hi | b.hi;
    return r;
}

/* Left shift by n (0 <= n < 128). Bits shifted past bit 127 are discarded. */
static u128 u128_shl(u128 a, unsigned n) {
    u128 r;
    if (n == 0) {
        r = a;
    } else if (n < 64) {
        r.hi = (a.hi << n) | (a.lo >> (64 - n));
        r.lo = a.lo << n;
    } else {
        r.hi = a.lo << (n - 64);
        r.lo = 0;
    }
    return r;
}

/* Right shift by n (0 <= n < 128). */
static u128 u128_shr(u128 a, unsigned n) {
    u128 r;
    if (n == 0) {
        r = a;
    } else if (n < 64) {
        r.lo = (a.lo >> n) | (a.hi << (64 - n));
        r.hi = a.hi >> n;
    } else {
        r.lo = a.hi >> (n - 64);
        r.hi = 0;
    }
    return r;
}

static u128 u128_from_le16(const uint8_t b[16]) {
    u128 r;
    r.lo = 0;
    r.hi = 0;
    for (int i = 0; i < 8; i++) r.lo |= (uint64_t)b[i] << (8 * i);
    for (int i = 0; i < 8; i++) r.hi |= (uint64_t)b[8 + i] << (8 * i);
    return r;
}

static void u128_to_le16(u128 v, uint8_t out[16]) {
    for (int i = 0; i < 8; i++) out[i] = (uint8_t)(v.lo >> (8 * i));
    for (int i = 0; i < 8; i++) out[8 + i] = (uint8_t)(v.hi >> (8 * i));
}

/* ===========================================================================
 *  Field element in GF(2^255 - 19): five 51-bit limbs
 * =========================================================================== */

typedef struct {
    uint64_t v[5];
} Fe;

#define MASK51 (((uint64_t)1 << 51) - 1)

static const Fe FE_ZERO = {{0, 0, 0, 0, 0}};
static const Fe FE_ONE = {{1, 0, 0, 0, 0}};

/* Propagate carries limb→limb, folding limb-4 overflow back into limb 0 (×19,
 * since 2^255 ≡ 19 mod p). */
static Fe fe_carry_propagate(Fe f) {
    uint64_t *l = f.v;
    l[1] += l[0] >> 51;
    l[0] &= MASK51;
    l[2] += l[1] >> 51;
    l[1] &= MASK51;
    l[3] += l[2] >> 51;
    l[2] &= MASK51;
    l[4] += l[3] >> 51;
    l[3] &= MASK51;
    uint64_t overflow = l[4] >> 51;
    l[4] &= MASK51;
    l[0] += overflow * 19;
    l[1] += l[0] >> 51;
    l[0] &= MASK51;
    return f;
}

static Fe fe_add(Fe a, Fe b) {
    Fe r;
    for (int i = 0; i < 5; i++) r.v[i] = a.v[i] + b.v[i];
    return r;
}

/* Subtraction: add a multiple of p to avoid u64 underflow, then subtract. */
static Fe fe_sub(Fe a, Fe b) {
    Fe r;
    r.v[0] = (a.v[0] + ((uint64_t)1 << 52) - 38) - b.v[0];
    r.v[1] = (a.v[1] + ((uint64_t)1 << 52) - 2) - b.v[1];
    r.v[2] = (a.v[2] + ((uint64_t)1 << 52) - 2) - b.v[2];
    r.v[3] = (a.v[3] + ((uint64_t)1 << 52) - 2) - b.v[3];
    r.v[4] = (a.v[4] + ((uint64_t)1 << 52) - 2) - b.v[4];
    return r;
}

/* Carry propagation from wide (u128) accumulators down to 51-bit limbs. */
static Fe fe_carry_wide(u128 r[5]) {
    Fe out;
    u128 carry = u128_from_u64(0);
    for (int i = 0; i < 5; i++) {
        u128 sum = u128_add(r[i], carry);
        out.v[i] = sum.lo & MASK51;
        carry = u128_shr(sum, 51);
    }
    out.v[0] += carry.lo * 19;
    out.v[1] += out.v[0] >> 51;
    out.v[0] &= MASK51;
    return out;
}

static Fe fe_mul(Fe fa, Fe fb) {
    uint64_t *a = fa.v, *b = fb.v;
    uint64_t b19[5];
    for (int i = 0; i < 5; i++) b19[i] = b[i] * 19;

    u128 r[5];
    /* r0 = a0*b0 + a1*b19_4 + a2*b19_3 + a3*b19_2 + a4*b19_1 */
    r[0] = u128_mul_64(a[0], b[0]);
    r[0] = u128_add(r[0], u128_mul_64(a[1], b19[4]));
    r[0] = u128_add(r[0], u128_mul_64(a[2], b19[3]));
    r[0] = u128_add(r[0], u128_mul_64(a[3], b19[2]));
    r[0] = u128_add(r[0], u128_mul_64(a[4], b19[1]));
    /* r1 = a0*b1 + a1*b0 + a2*b19_4 + a3*b19_3 + a4*b19_2 */
    r[1] = u128_mul_64(a[0], b[1]);
    r[1] = u128_add(r[1], u128_mul_64(a[1], b[0]));
    r[1] = u128_add(r[1], u128_mul_64(a[2], b19[4]));
    r[1] = u128_add(r[1], u128_mul_64(a[3], b19[3]));
    r[1] = u128_add(r[1], u128_mul_64(a[4], b19[2]));
    /* r2 = a0*b2 + a1*b1 + a2*b0 + a3*b19_4 + a4*b19_3 */
    r[2] = u128_mul_64(a[0], b[2]);
    r[2] = u128_add(r[2], u128_mul_64(a[1], b[1]));
    r[2] = u128_add(r[2], u128_mul_64(a[2], b[0]));
    r[2] = u128_add(r[2], u128_mul_64(a[3], b19[4]));
    r[2] = u128_add(r[2], u128_mul_64(a[4], b19[3]));
    /* r3 = a0*b3 + a1*b2 + a2*b1 + a3*b0 + a4*b19_4 */
    r[3] = u128_mul_64(a[0], b[3]);
    r[3] = u128_add(r[3], u128_mul_64(a[1], b[2]));
    r[3] = u128_add(r[3], u128_mul_64(a[2], b[1]));
    r[3] = u128_add(r[3], u128_mul_64(a[3], b[0]));
    r[3] = u128_add(r[3], u128_mul_64(a[4], b19[4]));
    /* r4 = a0*b4 + a1*b3 + a2*b2 + a3*b1 + a4*b0 */
    r[4] = u128_mul_64(a[0], b[4]);
    r[4] = u128_add(r[4], u128_mul_64(a[1], b[3]));
    r[4] = u128_add(r[4], u128_mul_64(a[2], b[2]));
    r[4] = u128_add(r[4], u128_mul_64(a[3], b[1]));
    r[4] = u128_add(r[4], u128_mul_64(a[4], b[0]));

    return fe_carry_wide(r);
}

static Fe fe_square(Fe fa) {
    uint64_t *a = fa.v;
    uint64_t a0_2 = a[0] * 2, a1_2 = a[1] * 2, a2_2 = a[2] * 2, a3_2 = a[3] * 2;
    uint64_t a3_19 = a[3] * 19, a4_19 = a[4] * 19;

    u128 r[5];
    r[0] = u128_mul_64(a[0], a[0]);
    r[0] = u128_add(r[0], u128_mul_64(a1_2, a4_19));
    r[0] = u128_add(r[0], u128_mul_64(a2_2, a3_19));

    r[1] = u128_mul_64(a0_2, a[1]);
    r[1] = u128_add(r[1], u128_mul_64(a2_2, a4_19));
    r[1] = u128_add(r[1], u128_mul_64(a[3], a3_19));

    r[2] = u128_mul_64(a0_2, a[2]);
    r[2] = u128_add(r[2], u128_mul_64(a[1], a[1]));
    r[2] = u128_add(r[2], u128_mul_64(a3_2, a4_19));

    r[3] = u128_mul_64(a0_2, a[3]);
    r[3] = u128_add(r[3], u128_mul_64(a1_2, a[2]));
    r[3] = u128_add(r[3], u128_mul_64(a[4], a4_19));

    r[4] = u128_mul_64(a0_2, a[4]);
    r[4] = u128_add(r[4], u128_mul_64(a1_2, a[3]));
    r[4] = u128_add(r[4], u128_mul_64(a[2], a[2]));

    return fe_carry_wide(r);
}

/* Multiply by a small constant (used for a24 = 121666). */
static Fe fe_mul_small(Fe fa, uint64_t small) {
    u128 r[5];
    for (int i = 0; i < 5; i++) r[i] = u128_mul_64(fa.v[i], small);
    return fe_carry_wide(r);
}

/* Inverse via Fermat: a^(p-2), with the standard addition chain. */
static Fe fe_invert(Fe a) {
    Fe t0 = fe_square(a);
    Fe t1 = fe_square(fe_square(t0));
    t1 = fe_mul(a, t1);
    t0 = fe_mul(t0, t1);
    Fe t2 = fe_square(t0);
    t1 = fe_mul(t1, t2);

    t2 = fe_square(t1);
    for (int i = 1; i < 5; i++) t2 = fe_square(t2);
    t1 = fe_mul(t2, t1);

    t2 = fe_square(t1);
    for (int i = 1; i < 10; i++) t2 = fe_square(t2);
    t2 = fe_mul(t2, t1);

    Fe t3 = fe_square(t2);
    for (int i = 1; i < 20; i++) t3 = fe_square(t3);
    t2 = fe_mul(t3, t2);

    t2 = fe_square(t2);
    for (int i = 1; i < 10; i++) t2 = fe_square(t2);
    t1 = fe_mul(t2, t1);

    t2 = fe_square(t1);
    for (int i = 1; i < 50; i++) t2 = fe_square(t2);
    t2 = fe_mul(t2, t1);

    t3 = fe_square(t2);
    for (int i = 1; i < 100; i++) t3 = fe_square(t3);
    t2 = fe_mul(t3, t2);

    t2 = fe_square(t2);
    for (int i = 1; i < 50; i++) t2 = fe_square(t2);
    t1 = fe_mul(t2, t1);

    t1 = fe_square(t1);
    t1 = fe_square(t1);
    t1 = fe_square(t1);
    t1 = fe_square(t1);
    t1 = fe_square(t1);
    return fe_mul(t1, t0);
}

/* Canonical reduction + little-endian 32-byte serialization. */
static void fe_to_bytes(Fe f, uint8_t out[32]) {
    Fe t = fe_carry_propagate(fe_carry_propagate(f));
    uint64_t *l = t.v;

    /* Conditional subtraction of p (= 2^255 - 19). */
    uint64_t q = (l[0] + 19) >> 51;
    q = (l[1] + q) >> 51;
    q = (l[2] + q) >> 51;
    q = (l[3] + q) >> 51;
    q = (l[4] + q) >> 51;

    l[0] += 19 * q;
    l[1] += l[0] >> 51;
    l[0] &= MASK51;
    l[2] += l[1] >> 51;
    l[1] &= MASK51;
    l[3] += l[2] >> 51;
    l[2] &= MASK51;
    l[4] += l[3] >> 51;
    l[3] &= MASK51;
    l[4] &= MASK51;

    u128 v0 = u128_from_u64(l[0]);
    u128 v1 = u128_from_u64(l[1]);
    u128 v2 = u128_from_u64(l[2]);
    u128 v3 = u128_from_u64(l[3]);
    u128 v4 = u128_from_u64(l[4]);

    u128 lo = u128_or(u128_or(v0, u128_shl(v1, 51)), u128_shl(v2, 102));
    u128 hi = u128_or(u128_or(u128_shr(v2, 26), u128_shl(v3, 25)),
                      u128_shl(v4, 76));

    u128_to_le16(lo, out);
    u128_to_le16(hi, out + 16);
}

/* Decode a field element from 32 little-endian bytes. */
static Fe fe_from_bytes(const uint8_t bytes[32]) {
    u128 lo = u128_from_le16(bytes);
    u128 hi = u128_from_le16(bytes + 16);

    Fe f;
    f.v[0] = lo.lo & MASK51;
    f.v[1] = u128_shr(lo, 51).lo & MASK51;
    f.v[2] = (u128_shr(lo, 102).lo | (hi.lo << 26)) & MASK51;
    f.v[3] = u128_shr(hi, 25).lo & MASK51;
    f.v[4] = u128_shr(hi, 76).lo & MASK51;
    return f;
}

/* Constant-time conditional swap (no branch on `swap`). */
static void fe_cswap(uint64_t swap, Fe *a, Fe *b) {
    uint64_t mask = (uint64_t)0 - swap; /* all-ones if swap==1, else 0 */
    for (int i = 0; i < 5; i++) {
        uint64_t dummy = mask & (a->v[i] ^ b->v[i]);
        a->v[i] ^= dummy;
        b->v[i] ^= dummy;
    }
}

/* ===========================================================================
 *  Scalar clamping and the Montgomery ladder
 * =========================================================================== */

static void clamp_scalar(const uint8_t k[32], uint8_t out[32]) {
    memcpy(out, k, 32);
    out[0] &= 248;  /* clear the low 3 bits (cofactor clearing) */
    out[31] &= 127; /* clear bit 255 */
    out[31] |= 64;  /* set bit 254 (constant bit-length) */
}

static void montgomery_ladder(const uint8_t k_bytes[32],
                              const uint8_t u_bytes[32], uint8_t out[32]) {
    uint8_t k[32];
    clamp_scalar(k_bytes, k);
    uint8_t u_masked[32];
    memcpy(u_masked, u_bytes, 32);
    u_masked[31] &= 0x7F;
    Fe u = fe_from_bytes(u_masked);

    Fe x_1 = u;
    Fe x_2 = FE_ONE;
    Fe z_2 = FE_ZERO;
    Fe x_3 = u;
    Fe z_3 = FE_ONE;
    uint64_t swap = 0;

    for (int t = 254; t >= 0; t--) {
        uint64_t k_t = (uint64_t)((k[t / 8] >> (t % 8)) & 1);
        swap ^= k_t;
        fe_cswap(swap, &x_2, &x_3);
        fe_cswap(swap, &z_2, &z_3);
        swap = k_t;

        Fe a = fe_add(x_2, z_2);
        Fe aa = fe_square(a);
        Fe b = fe_sub(x_2, z_2);
        Fe bb = fe_square(b);
        Fe e = fe_sub(aa, bb);

        Fe c = fe_add(x_3, z_3);
        Fe d = fe_sub(x_3, z_3);
        Fe da = fe_mul(d, a);
        Fe cb = fe_mul(c, b);

        x_3 = fe_square(fe_add(da, cb));
        z_3 = fe_mul(x_1, fe_square(fe_sub(da, cb)));
        x_2 = fe_mul(aa, bb);
        z_2 = fe_mul(e, fe_add(bb, fe_mul_small(e, 121666)));
    }

    fe_cswap(swap, &x_2, &x_3);
    fe_cswap(swap, &z_2, &z_3);

    Fe result = fe_mul(x_2, fe_invert(z_2));
    fe_to_bytes(result, out);
}

/* ===========================================================================
 *  Public API
 * =========================================================================== */

const uint8_t X25519_BASE_POINT[32] = {9, 0};

int x25519(uint8_t out[32], const uint8_t scalar[32],
           const uint8_t u_coordinate[32]) {
    montgomery_ladder(scalar, u_coordinate, out);
    int all_zero = 1;
    for (int i = 0; i < 32; i++) {
        if (out[i] != 0) {
            all_zero = 0;
            break;
        }
    }
    return all_zero ? -1 : 0; /* all zeros ⇒ low-order point input */
}

int x25519_base(uint8_t out[32], const uint8_t scalar[32]) {
    return x25519(out, scalar, X25519_BASE_POINT);
}

int x25519_generate_keypair(uint8_t out[32], const uint8_t private_key[32]) {
    return x25519_base(out, private_key);
}
