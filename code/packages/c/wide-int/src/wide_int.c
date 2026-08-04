/*
 * wide_int.c — implementation of portable 128-bit integers from two u64 halves.
 *
 * Nothing here uses `__int128` or any compiler extension: every operation is
 * expressed in standard 64-bit arithmetic, so it compiles identically under
 * GCC, Clang, and MSVC (and passes -pedantic-errors). See wide_int.h for the
 * surface and the representation (value = hi*2^64 + lo).
 */
#include "wide_int.h"

/* ================================================================== */
/* Construction / accessors                                           */
/* ================================================================== */

wi_u128 wi_u128_make(uint64_t hi, uint64_t lo) {
    wi_u128 r;
    r.hi = hi;
    r.lo = lo;
    return r;
}

wi_u128 wi_u128_from_u64(uint64_t v) { return wi_u128_make(0, v); }
wi_u128 wi_u128_zero(void) { return wi_u128_make(0, 0); }
wi_u128 wi_u128_max(void) { return wi_u128_make((uint64_t)-1, (uint64_t)-1); }
uint64_t wi_u128_lo(wi_u128 a) { return a.lo; }
uint64_t wi_u128_hi(wi_u128 a) { return a.hi; }
uint64_t wi_u128_to_u64(wi_u128 a) { return a.lo; }

/* ================================================================== */
/* Arithmetic                                                         */
/* ================================================================== */

wi_u128 wi_u128_add(wi_u128 a, wi_u128 b) {
    uint64_t lo = a.lo + b.lo;
    /* A carry out of the low word occurs iff the sum wrapped below an addend. */
    uint64_t carry = (lo < a.lo) ? 1u : 0u;
    return wi_u128_make(a.hi + b.hi + carry, lo);
}

wi_u128 wi_u128_sub(wi_u128 a, wi_u128 b) {
    uint64_t lo = a.lo - b.lo;
    uint64_t borrow = (a.lo < b.lo) ? 1u : 0u;
    return wi_u128_make(a.hi - b.hi - borrow, lo);
}

/* Exact 64x64 -> 128 multiply via four 32-bit partial products (schoolbook). */
wi_u128 wi_mul_u64(uint64_t a, uint64_t b) {
    uint64_t a_lo = a & 0xFFFFFFFFu;
    uint64_t a_hi = a >> 32;
    uint64_t b_lo = b & 0xFFFFFFFFu;
    uint64_t b_hi = b >> 32;

    uint64_t ll = a_lo * b_lo;
    uint64_t lh = a_lo * b_hi;
    uint64_t hl = a_hi * b_lo;
    uint64_t hh = a_hi * b_hi;

    /* Combine the middle terms, propagating the carry from the low 32 bits. */
    uint64_t cross = (ll >> 32) + (lh & 0xFFFFFFFFu) + (hl & 0xFFFFFFFFu);
    uint64_t lo = (ll & 0xFFFFFFFFu) | (cross << 32);
    uint64_t hi = hh + (lh >> 32) + (hl >> 32) + (cross >> 32);
    return wi_u128_make(hi, lo);
}

wi_u128 wi_u128_mul(wi_u128 a, wi_u128 b) {
    /* Low 128 bits of the product: a.lo*b.lo contributes a full 128-bit term;
     * the cross terms (a.lo*b.hi, a.hi*b.lo) are shifted left 64, so only their
     * low 64 bits survive; a.hi*b.hi is shifted 128 and drops entirely. */
    wi_u128 ll = wi_mul_u64(a.lo, b.lo);
    uint64_t mid = a.lo * b.hi + a.hi * b.lo;
    return wi_u128_make(ll.hi + mid, ll.lo);
}

/* ================================================================== */
/* Bitwise / shifts                                                   */
/* ================================================================== */

wi_u128 wi_u128_and(wi_u128 a, wi_u128 b) { return wi_u128_make(a.hi & b.hi, a.lo & b.lo); }
wi_u128 wi_u128_or(wi_u128 a, wi_u128 b) { return wi_u128_make(a.hi | b.hi, a.lo | b.lo); }
wi_u128 wi_u128_xor(wi_u128 a, wi_u128 b) { return wi_u128_make(a.hi ^ b.hi, a.lo ^ b.lo); }
wi_u128 wi_u128_not(wi_u128 a) { return wi_u128_make(~a.hi, ~a.lo); }

wi_u128 wi_u128_shl(wi_u128 a, unsigned n) {
    if (n == 0) {
        return a;
    }
    if (n >= 128) {
        return wi_u128_zero();
    }
    if (n >= 64) {
        /* Everything moves into the high word; the low word becomes zero. A
         * shift of exactly 64 is handled here (n-64 in [0,63], never 64). */
        return wi_u128_make(a.lo << (n - 64), 0);
    }
    /* 1 <= n <= 63: 64-n is in [1,63], so both sub-shifts are well-defined. */
    return wi_u128_make((a.hi << n) | (a.lo >> (64 - n)), a.lo << n);
}

wi_u128 wi_u128_shr(wi_u128 a, unsigned n) {
    if (n == 0) {
        return a;
    }
    if (n >= 128) {
        return wi_u128_zero();
    }
    if (n >= 64) {
        return wi_u128_make(0, a.hi >> (n - 64));
    }
    return wi_u128_make(a.hi >> n, (a.lo >> n) | (a.hi << (64 - n)));
}

/* ================================================================== */
/* Comparison                                                         */
/* ================================================================== */

int wi_u128_cmp(wi_u128 a, wi_u128 b) {
    if (a.hi != b.hi) {
        return a.hi < b.hi ? -1 : 1;
    }
    if (a.lo != b.lo) {
        return a.lo < b.lo ? -1 : 1;
    }
    return 0;
}

int wi_u128_eq(wi_u128 a, wi_u128 b) { return a.hi == b.hi && a.lo == b.lo; }
int wi_u128_is_zero(wi_u128 a) { return a.hi == 0 && a.lo == 0; }

/* ================================================================== */
/* Division (binary long division, shift-and-subtract)                */
/* ================================================================== */

/* Extract bit `i` (0..127) of `a`. */
static uint64_t wi__bit(wi_u128 a, unsigned i) {
    if (i >= 64) {
        return (a.hi >> (i - 64)) & 1u;
    }
    return (a.lo >> i) & 1u;
}

/* Set bit `i` (0..127) of *a. */
static void wi__set_bit(wi_u128 *a, unsigned i) {
    if (i >= 64) {
        a->hi |= (uint64_t)1 << (i - 64);
    } else {
        a->lo |= (uint64_t)1 << i;
    }
}

int wi_u128_divmod(wi_u128 a, wi_u128 b, wi_u128 *q, wi_u128 *r) {
    wi_u128 quotient;
    wi_u128 remainder;
    int i;
    if (wi_u128_is_zero(b)) {
        return 1;
    }
    quotient = wi_u128_zero();
    remainder = wi_u128_zero();
    /* Process bits from most- to least-significant: shift the remainder left,
     * pull in the next dividend bit, and subtract the divisor when it fits. */
    for (i = 127; i >= 0; --i) {
        remainder = wi_u128_shl(remainder, 1);
        remainder.lo |= wi__bit(a, (unsigned)i);
        if (wi_u128_cmp(remainder, b) >= 0) {
            remainder = wi_u128_sub(remainder, b);
            wi__set_bit(&quotient, (unsigned)i);
        }
    }
    *q = quotient;
    *r = remainder;
    return 0;
}

/* ================================================================== */
/* Formatting                                                         */
/* ================================================================== */

size_t wi_u128_to_hex(wi_u128 a, char *buf) {
    static const char DIGITS[] = "0123456789abcdef";
    int i;
    for (i = 0; i < 16; ++i) {
        buf[i] = DIGITS[(a.hi >> (60 - i * 4)) & 0xFu];
    }
    for (i = 0; i < 16; ++i) {
        buf[16 + i] = DIGITS[(a.lo >> (60 - i * 4)) & 0xFu];
    }
    buf[32] = '\0';
    return 32;
}

size_t wi_u128_to_dec(wi_u128 a, char *buf) {
    char tmp[40];
    size_t n = 0;
    size_t i;
    wi_u128 ten = wi_u128_from_u64(10);
    if (wi_u128_is_zero(a)) {
        buf[0] = '0';
        buf[1] = '\0';
        return 1;
    }
    /* Repeatedly divide by 10, collecting least-significant digits first. */
    while (!wi_u128_is_zero(a)) {
        wi_u128 q;
        wi_u128 rem;
        (void)wi_u128_divmod(a, ten, &q, &rem);
        tmp[n++] = (char)('0' + (int)rem.lo);
        a = q;
    }
    /* Reverse into the output. */
    for (i = 0; i < n; ++i) {
        buf[i] = tmp[n - 1 - i];
    }
    buf[n] = '\0';
    return n;
}

/* ================================================================== */
/* Signed (two's complement)                                          */
/* ================================================================== */

wi_i128 wi_i128_make(uint64_t hi, uint64_t lo) {
    wi_i128 r;
    r.hi = hi;
    r.lo = lo;
    return r;
}

wi_i128 wi_i128_from_i64(int64_t v) {
    /* Sign-extend: negative values fill the high word with ones. */
    uint64_t hi = (v < 0) ? (uint64_t)-1 : 0;
    return wi_i128_make(hi, (uint64_t)v);
}

wi_i128 wi_i128_zero(void) { return wi_i128_make(0, 0); }
int wi_i128_is_negative(wi_i128 a) { return (a.hi >> 63) != 0; }

wi_u128 wi_i128_bits(wi_i128 a) { return wi_u128_make(a.hi, a.lo); }
wi_i128 wi_u128_as_i128(wi_u128 a) { return wi_i128_make(a.hi, a.lo); }

/* Add/sub/mul are two's-complement identical to the unsigned versions. */
wi_i128 wi_i128_add(wi_i128 a, wi_i128 b) {
    return wi_u128_as_i128(wi_u128_add(wi_i128_bits(a), wi_i128_bits(b)));
}
wi_i128 wi_i128_sub(wi_i128 a, wi_i128 b) {
    return wi_u128_as_i128(wi_u128_sub(wi_i128_bits(a), wi_i128_bits(b)));
}
wi_i128 wi_i128_mul(wi_i128 a, wi_i128 b) {
    return wi_u128_as_i128(wi_u128_mul(wi_i128_bits(a), wi_i128_bits(b)));
}
wi_i128 wi_i128_neg(wi_i128 a) {
    /* -a = ~a + 1 */
    wi_u128 t = wi_u128_add(wi_u128_not(wi_i128_bits(a)), wi_u128_from_u64(1));
    return wi_u128_as_i128(t);
}

int wi_i128_cmp(wi_i128 a, wi_i128 b) {
    int na = wi_i128_is_negative(a);
    int nb = wi_i128_is_negative(b);
    if (na != nb) {
        /* The negative one is smaller. */
        return na ? -1 : 1;
    }
    /* Same sign: the unsigned bit-comparison gives the correct signed order. */
    return wi_u128_cmp(wi_i128_bits(a), wi_i128_bits(b));
}

int wi_i128_eq(wi_i128 a, wi_i128 b) { return a.hi == b.hi && a.lo == b.lo; }

wi_i128 wi_i128_sar(wi_i128 a, unsigned n) {
    int negative = wi_i128_is_negative(a);
    wi_u128 shifted = wi_u128_shr(wi_i128_bits(a), n);
    if (negative) {
        /* Fill the vacated high bits with ones: OR in ~(2^(128-n) - 1). */
        wi_u128 ones = wi_u128_max();
        wi_u128 fill;
        if (n >= 128) {
            fill = wi_u128_max();
        } else {
            fill = wi_u128_shl(ones, 128 - n);
        }
        shifted = wi_u128_or(shifted, fill);
    }
    return wi_u128_as_i128(shifted);
}

int wi_i128_divmod(wi_i128 a, wi_i128 b, wi_i128 *q, wi_i128 *r) {
    int neg_a = wi_i128_is_negative(a);
    int neg_b = wi_i128_is_negative(b);
    wi_u128 ua;
    wi_u128 ub;
    wi_u128 uq;
    wi_u128 ur;
    if (wi_i128_eq(b, wi_i128_zero())) {
        return 1;
    }
    /* Work with magnitudes, then reapply signs (C truncates toward zero, and the
     * remainder takes the dividend's sign). */
    ua = wi_i128_bits(neg_a ? wi_i128_neg(a) : a);
    ub = wi_i128_bits(neg_b ? wi_i128_neg(b) : b);
    (void)wi_u128_divmod(ua, ub, &uq, &ur);
    {
        wi_i128 sq = wi_u128_as_i128(uq);
        wi_i128 sr = wi_u128_as_i128(ur);
        if (neg_a != neg_b) {
            sq = wi_i128_neg(sq);
        }
        if (neg_a) {
            sr = wi_i128_neg(sr);
        }
        *q = sq;
        *r = sr;
    }
    return 0;
}

size_t wi_i128_to_dec(wi_i128 a, char *buf) {
    if (wi_i128_is_negative(a)) {
        size_t n;
        wi_u128 mag = wi_i128_bits(wi_i128_neg(a));
        buf[0] = '-';
        n = wi_u128_to_dec(mag, buf + 1);
        return n + 1;
    }
    return wi_u128_to_dec(wi_i128_bits(a), buf);
}
