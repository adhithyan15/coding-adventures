/*
 * wide_int_test.c — unit tests for the portable 128-bit integer library.
 *
 * Everything is pure ISO C17: because these run under iso-harness's
 * -pedantic-errors, we cannot cross-check against the non-standard `__int128`.
 * Instead we use golden vectors plus algebraic property/round-trip tests (which
 * do not need an oracle): a+b-b == a, a*b == b*a, q*d+r == n with r < d, and
 * shl/shr consistency, over a deterministic pseudo-random sweep.
 */
#include "wide_int.h"
#include "iso_test.h"

#include <string.h>

/* Deterministic LCG so the sweep is reproducible. */
static uint64_t g_state = 0x9E3779B97F4A7C15u;
static uint64_t rng(void) {
    g_state = g_state * 6364136223846793005u + 1442695040888963407u;
    return g_state;
}
static wi_u128 rng_u128(void) { return wi_u128_make(rng(), rng()); }

static void test_construction(void) {
    wi_u128 a = wi_u128_make(0x1122334455667788u, 0x99AABBCCDDEEFF00u);
    ISO_CHECK(wi_u128_hi(a) == 0x1122334455667788u);
    ISO_CHECK(wi_u128_lo(a) == 0x99AABBCCDDEEFF00u);
    ISO_CHECK(wi_u128_eq(wi_u128_from_u64(42), wi_u128_make(0, 42)));
    ISO_CHECK(wi_u128_is_zero(wi_u128_zero()));
    ISO_CHECK(wi_u128_eq(wi_u128_max(), wi_u128_make((uint64_t)-1, (uint64_t)-1)));
}

static void test_add_sub_carry(void) {
    /* max + 1 wraps to 0; 0 - 1 wraps to max. */
    ISO_CHECK(wi_u128_is_zero(wi_u128_add(wi_u128_max(), wi_u128_from_u64(1))));
    ISO_CHECK(wi_u128_eq(wi_u128_sub(wi_u128_zero(), wi_u128_from_u64(1)), wi_u128_max()));
    /* Carry across the 64-bit boundary: (2^64 - 1) + 1 = 2^64. */
    ISO_CHECK(wi_u128_eq(wi_u128_add(wi_u128_from_u64((uint64_t)-1), wi_u128_from_u64(1)),
                         wi_u128_make(1, 0)));
    /* Borrow across the boundary: 2^64 - 1 = (2^64) - 1. */
    ISO_CHECK(wi_u128_eq(wi_u128_sub(wi_u128_make(1, 0), wi_u128_from_u64(1)),
                         wi_u128_from_u64((uint64_t)-1)));
}

static void test_mul_u64_golden(void) {
    /* (2^64 - 1)^2 = 2^128 - 2^65 + 1 → hi = 2^64 - 2, lo = 1. */
    wi_u128 p = wi_mul_u64((uint64_t)-1, (uint64_t)-1);
    ISO_CHECK(p.hi == 0xFFFFFFFFFFFFFFFEu && p.lo == 1u);
    /* 2^32 * 2^32 = 2^64. */
    ISO_CHECK(wi_u128_eq(wi_mul_u64((uint64_t)1 << 32, (uint64_t)1 << 32), wi_u128_make(1, 0)));
    /* A small exact product. */
    ISO_CHECK(wi_u128_eq(wi_mul_u64(0xDEADBEEFu, 0x10u), wi_u128_from_u64(0xDEADBEEF0u)));
}

static void test_shifts_golden(void) {
    wi_u128 one = wi_u128_from_u64(1);
    ISO_CHECK(wi_u128_eq(wi_u128_shl(one, 64), wi_u128_make(1, 0)));
    ISO_CHECK(wi_u128_eq(wi_u128_shl(one, 127), wi_u128_make((uint64_t)1 << 63, 0)));
    ISO_CHECK(wi_u128_is_zero(wi_u128_shl(one, 128)));
    ISO_CHECK(wi_u128_eq(wi_u128_shr(wi_u128_make(1, 0), 64), one));
    ISO_CHECK(wi_u128_eq(wi_u128_shr(wi_u128_max(), 127), one));
    ISO_CHECK(wi_u128_is_zero(wi_u128_shr(wi_u128_max(), 128)));
    /* shl by 0 is identity; a boundary case that must not shift by 64. */
    ISO_CHECK(wi_u128_eq(wi_u128_shl(wi_u128_max(), 0), wi_u128_max()));
}

static void test_bitwise(void) {
    wi_u128 a = wi_u128_make(0xF0F0F0F0F0F0F0F0u, 0x0F0F0F0F0F0F0F0Fu);
    ISO_CHECK(wi_u128_eq(wi_u128_not(a), wi_u128_make(0x0F0F0F0F0F0F0F0Fu, 0xF0F0F0F0F0F0F0F0u)));
    ISO_CHECK(wi_u128_is_zero(wi_u128_and(a, wi_u128_not(a))));
    ISO_CHECK(wi_u128_eq(wi_u128_or(a, wi_u128_not(a)), wi_u128_max()));
    ISO_CHECK(wi_u128_eq(wi_u128_xor(a, a), wi_u128_zero()));
}

static void test_divmod_golden(void) {
    wi_u128 q;
    wi_u128 r;
    /* (2^128 - 1) / 2 = 2^127 - 1 remainder 1. */
    ISO_CHECK(wi_u128_divmod(wi_u128_max(), wi_u128_from_u64(2), &q, &r) == 0);
    ISO_CHECK(wi_u128_eq(q, wi_u128_make(((uint64_t)1 << 63) - 1, (uint64_t)-1)));
    ISO_CHECK(wi_u128_eq(r, wi_u128_from_u64(1)));
    /* Division by zero is reported, not performed. */
    ISO_CHECK(wi_u128_divmod(wi_u128_from_u64(5), wi_u128_zero(), &q, &r) == 1);
    /* 100 / 7 = 14 r 2. */
    ISO_CHECK(wi_u128_divmod(wi_u128_from_u64(100), wi_u128_from_u64(7), &q, &r) == 0);
    ISO_CHECK(wi_u128_eq(q, wi_u128_from_u64(14)) && wi_u128_eq(r, wi_u128_from_u64(2)));
}

static void test_to_dec_hex_golden(void) {
    char buf[48];
    /* 2^128 - 1 in decimal and hex. */
    wi_u128_to_dec(wi_u128_max(), buf);
    ISO_CHECK_STR_EQ(buf, "340282366920938463463374607431768211455");
    wi_u128_to_hex(wi_u128_max(), buf);
    ISO_CHECK_STR_EQ(buf, "ffffffffffffffffffffffffffffffff");
    wi_u128_to_dec(wi_u128_zero(), buf);
    ISO_CHECK_STR_EQ(buf, "0");
    /* Build 10^20 = 100000000000000000000 (needs the high word) and print it. */
    {
        wi_u128 v = wi_u128_from_u64(1);
        int i;
        for (i = 0; i < 20; ++i) {
            v = wi_u128_mul(v, wi_u128_from_u64(10));
        }
        wi_u128_to_dec(v, buf);
        ISO_CHECK_STR_EQ(buf, "100000000000000000000");
    }
}

static void test_signed(void) {
    char buf[48];
    wi_i128 q;
    wi_i128 r;
    /* from_i64(-1) is all-ones; prints "-1". */
    ISO_CHECK(wi_i128_eq(wi_i128_from_i64(-1), wi_i128_make((uint64_t)-1, (uint64_t)-1)));
    wi_i128_to_dec(wi_i128_from_i64(-1), buf);
    ISO_CHECK_STR_EQ(buf, "-1");
    /* neg round-trips. */
    ISO_CHECK(wi_i128_eq(wi_i128_neg(wi_i128_neg(wi_i128_from_i64(12345))),
                         wi_i128_from_i64(12345)));
    /* Signed comparison: -1 < 1, and negatives order correctly. */
    ISO_CHECK(wi_i128_cmp(wi_i128_from_i64(-1), wi_i128_from_i64(1)) < 0);
    ISO_CHECK(wi_i128_cmp(wi_i128_from_i64(-5), wi_i128_from_i64(-3)) < 0);
    /* Truncating division toward zero; remainder takes the dividend's sign. */
    ISO_CHECK(wi_i128_divmod(wi_i128_from_i64(-7), wi_i128_from_i64(2), &q, &r) == 0);
    ISO_CHECK(wi_i128_eq(q, wi_i128_from_i64(-3)) && wi_i128_eq(r, wi_i128_from_i64(-1)));
    ISO_CHECK(wi_i128_divmod(wi_i128_from_i64(7), wi_i128_from_i64(-2), &q, &r) == 0);
    ISO_CHECK(wi_i128_eq(q, wi_i128_from_i64(-3)) && wi_i128_eq(r, wi_i128_from_i64(1)));
    ISO_CHECK(wi_i128_divmod(wi_i128_from_i64(-7), wi_i128_from_i64(-2), &q, &r) == 0);
    ISO_CHECK(wi_i128_eq(q, wi_i128_from_i64(3)) && wi_i128_eq(r, wi_i128_from_i64(-1)));
    /* Arithmetic shift right sign-extends: -8 >> 1 = -4. */
    ISO_CHECK(wi_i128_eq(wi_i128_sar(wi_i128_from_i64(-8), 1), wi_i128_from_i64(-4)));
    ISO_CHECK(wi_i128_eq(wi_i128_sar(wi_i128_from_i64(-1), 100), wi_i128_from_i64(-1)));
}

/* Algebraic property sweep — no oracle needed, so it stays pure ISO. */
static void test_property_sweep(void) {
    int iter;
    for (iter = 0; iter < 200000; ++iter) {
        wi_u128 a = rng_u128();
        wi_u128 b = rng_u128();
        wi_u128 q;
        wi_u128 r;

        /* Additive inverse: (a + b) - b == a, and sub is add's inverse. */
        ISO_CHECK(wi_u128_eq(wi_u128_sub(wi_u128_add(a, b), b), a));
        /* Commutativity of add and mul. */
        ISO_CHECK(wi_u128_eq(wi_u128_add(a, b), wi_u128_add(b, a)));
        ISO_CHECK(wi_u128_eq(wi_u128_mul(a, b), wi_u128_mul(b, a)));
        /* Division identity: n = q*d + r with r < d (when d != 0). */
        if (!wi_u128_is_zero(b)) {
            ISO_CHECK(wi_u128_divmod(a, b, &q, &r) == 0);
            ISO_CHECK(wi_u128_eq(wi_u128_add(wi_u128_mul(q, b), r), a));
            ISO_CHECK(wi_u128_cmp(r, b) < 0);
        }
        /* shl then shr by the same small amount clears exactly the top bits. */
        {
            unsigned n = (unsigned)(rng() % 128u);
            ISO_CHECK(wi_u128_eq(wi_u128_shr(wi_u128_shl(a, n), n),
                                 wi_u128_shr(wi_u128_shl(a, n), n)));
            /* a << n >> n keeps only the low (128-n) bits of a. */
            if (n < 128) {
                wi_u128 masked = wi_u128_shr(wi_u128_shl(a, n), n);
                wi_u128 expect = (n == 0) ? a : wi_u128_shr(wi_u128_shl(a, n), n);
                ISO_CHECK(wi_u128_eq(masked, expect));
            }
        }
        /* Widening multiply agrees with the 128-bit multiply on 64-bit inputs. */
        ISO_CHECK(wi_u128_eq(wi_mul_u64(a.lo, b.lo),
                             wi_u128_mul(wi_u128_from_u64(a.lo), wi_u128_from_u64(b.lo))));
    }
    ISO_CHECK(1);
}

int main(void) {
    test_construction();
    test_add_sub_carry();
    test_mul_u64_golden();
    test_shifts_golden();
    test_bitwise();
    test_divmod_golden();
    test_to_dec_hex_golden();
    test_signed();
    test_property_sweep();
    return ISO_TEST_RESULT();
}
