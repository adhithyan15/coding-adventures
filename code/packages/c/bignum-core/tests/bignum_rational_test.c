/*
 * Tests for the C BigRational, using the header-only iso_test.h harness (pure
 * ISO). Vectors mirror the Rust crate's own unit tests — canonical/lowest-terms
 * form, sign placement, exact add/sub/mul/div (including big operands pinned
 * against Python's fractions.Fraction), ordering, reciprocal, integer powers
 * (with the try_pow DoS guard), parsing, and the lossy f64 export.
 */
#include "iso_test.h"

#include <stdlib.h>
#include <string.h>

#include "bignum_rational.h"

/* rat_from_ints, aborting the check on an unexpected NULL. */
static BigRational *rr(int64_t n, int64_t d) {
    BigRational *r = rat_from_ints(n, d);
    ISO_CHECK(r != NULL);
    return r;
}

/* Parse a literal expected to be valid. */
static BigRational *rp(const char *s) {
    BigRational *r = NULL;
    ISO_CHECK_MSG(rat_parse(s, &r) == RAT_PARSE_OK, s);
    return r;
}

/* Assert rat_to_string(r) == expected, then free r. */
static void expect_str(BigRational *r, const char *expected) {
    if (!r) {
        ISO_CHECK_MSG(0, expected);
        return;
    }
    char *s = rat_to_string(r);
    ISO_CHECK(s != NULL);
    if (s) ISO_CHECK_STR_EQ(s, expected);
    free(s);
    rat_free(r);
}

int main(void) {
    /* ── canonical form: lowest terms, sign in numerator, zero == 0/1 ───── */
    expect_str(rr(50, 100), "1/2");
    expect_str(rr(462, 1071), "22/51"); /* gcd 21 */
    expect_str(rr(6, 3), "2");          /* integer collapses to "n" */
    expect_str(rr(3, -4), "-3/4");
    expect_str(rr(-3, -4), "3/4");
    expect_str(rr(-3, 4), "-3/4");
    expect_str(rr(0, 5), "0");
    {
        BigRational *z = rr(0, -7);
        char *den = bigint_to_string(rat_denominator(z));
        ISO_CHECK_STR_EQ(den, "1"); /* zero pins to 0/1 */
        free(den);
        ISO_CHECK(rat_is_zero(z));
        rat_free(z);
    }
    /* Different spellings are equal by value. */
    {
        BigRational *a = rr(2, 4), *b = rr(1, 2);
        int c = 99;
        ISO_CHECK(rat_cmp(a, b, &c) == RAT_OK);
        ISO_CHECK_EQ_INT(c, 0);
        rat_free(a);
        rat_free(b);
    }

    /* ── zero denominator is rejected, not accepted ─────────────────────── */
    {
        BigInteger *one = bigint_one(), *zero = bigint_zero();
        BigRational *r = NULL;
        ISO_CHECK(rat_new(one, zero, &r) == RAT_ERR_ZERO_DENOMINATOR);
        BigInteger *two = bigint_from_i64(2);
        ISO_CHECK(rat_new(one, two, &r) == RAT_OK);
        rat_free(r);
        ISO_CHECK(rat_from_ints(1, 0) == NULL); /* the Rust from_ints panic */
        bigint_free(one);
        bigint_free(zero);
        bigint_free(two);
    }

    /* ── small exact arithmetic ─────────────────────────────────────────── */
    {
        BigRational *a = rr(1, 3), *b = rr(1, 6), *r = NULL;
        ISO_CHECK(rat_add(a, b, &r) == RAT_OK);
        expect_str(r, "1/2");
        rat_free(a);
        rat_free(b);
    }
    {
        BigRational *a = rr(2, 7), *b = rr(14, 3), *r = NULL;
        ISO_CHECK(rat_mul(a, b, &r) == RAT_OK);
        expect_str(r, "4/3");
        rat_free(a);
        rat_free(b);
    }
    {
        BigRational *a = rr(355, 113), *b = rr(22, 7), *r = NULL;
        ISO_CHECK(rat_sub(a, b, &r) == RAT_OK);
        expect_str(r, "-1/791");
        rat_free(a);
        rat_free(b);
    }
    {
        BigRational *a = rr(1, 2), *b = rr(3, 4), *r = NULL;
        ISO_CHECK(rat_div(a, b, &r) == RAT_OK);
        expect_str(r, "2/3");
        rat_free(a);
        rat_free(b);
    }
    {
        /* The float-famous case: 0.1 + 0.2 is exactly 3/10. */
        BigRational *a = rr(1, 10), *b = rr(2, 10), *r = NULL;
        ISO_CHECK(rat_add(a, b, &r) == RAT_OK);
        expect_str(r, "3/10");
        rat_free(a);
        rat_free(b);
    }

    /* ── big operands, pinned against Python's fractions.Fraction ───────── */
    {
        BigRational *a =
            rp("1000000000000000000000000000001/100000000000000000000");
        BigRational *b = rp("6366805760909027985741435139224001/847288609443");
        BigRational *r = NULL;

        ISO_CHECK(rat_add(a, b, &r) == RAT_OK);
        expect_str(r, "636680576091750087183586513922400100000000847288609443/"
                      "84728860944300000000000000000000");
        r = NULL;
        ISO_CHECK(rat_sub(a, b, &r) == RAT_OK);
        expect_str(
            r, "-636680576090055509964700513922400099999999152711390557/"
               "84728860944300000000000000000000");
        r = NULL;
        ISO_CHECK(rat_mul(a, b, &r) == RAT_OK);
        expect_str(
            r,
            "6366805760909027985741435139230367805760909027985741435139224001/"
            "84728860944300000000000000000000");
        r = NULL;
        ISO_CHECK(rat_div(a, b, &r) == RAT_OK);
        expect_str(r, "847288609443000000000000000000847288609443/"
                      "636680576090902798574143513922400100000000000000000000");
        r = NULL;
        ISO_CHECK(rat_pow(a, 3, &r) == RAT_OK);
        expect_str(
            r,
            "1000000000000000000000000000003000000000000000000000000000003000000"
            "000000000000000000000001/"
            "1000000000000000000000000000000000000000000000000000000000000");
        r = NULL;
        ISO_CHECK(rat_pow(b, -2, &r) == RAT_OK);
        expect_str(r, "717897987691852588770249/"
                      "40536215597144386832065866109016673800875222251012083746"
                      "192454448001");

        rat_free(a);
        rat_free(b);
    }

    /* ── ordering ───────────────────────────────────────────────────────── */
    {
        BigRational *a = rr(22, 7), *b = rr(355, 113);
        int c = 0;
        ISO_CHECK(rat_cmp(a, b, &c) == RAT_OK);
        ISO_CHECK_EQ_INT(c, 1); /* 3.142857… > 3.14159… */
        rat_free(a);
        rat_free(b);
    }
    {
        BigRational *a = rr(-1, 3), *b = rr(-1, 2);
        int c = 0;
        ISO_CHECK(rat_cmp(a, b, &c) == RAT_OK);
        ISO_CHECK_EQ_INT(c, 1); /* -0.333… > -0.5 */
        rat_free(a);
        rat_free(b);
    }

    /* ── sign, reciprocal, predicates ───────────────────────────────────── */
    {
        BigRational *n = rr(-3, 4);
        expect_str(rat_abs(n), "3/4");
        ISO_CHECK_EQ_INT(rat_signum(n), -1);
        ISO_CHECK(rat_is_negative(n));
        BigRational *rec = NULL;
        ISO_CHECK(rat_recip(n, &rec) == RAT_OK);
        expect_str(rec, "-4/3");
        rat_free(n);
    }
    {
        BigRational *p = rr(3, 4);
        ISO_CHECK_EQ_INT(rat_signum(p), 1);
        ISO_CHECK(rat_is_positive(p));
        ISO_CHECK(!rat_is_integer(p));
        rat_free(p);
    }
    {
        BigRational *i = rr(6, 3);
        ISO_CHECK(rat_is_integer(i)); /* 6/3 == 2/1 */
        rat_free(i);
    }
    {
        BigRational *seven = rr(7, 1), *rec = NULL;
        ISO_CHECK(rat_recip(seven, &rec) == RAT_OK);
        expect_str(rec, "1/7");
        rat_free(seven);
    }
    {
        /* Reciprocal / division by zero are reported, not attempted. */
        BigRational *z = rr(0, 1), *out = NULL;
        ISO_CHECK(rat_recip(z, &out) == RAT_ERR_DIV_BY_ZERO);
        BigRational *half = rr(1, 2);
        ISO_CHECK(rat_div(half, z, &out) == RAT_ERR_DIV_BY_ZERO);
        rat_free(z);
        rat_free(half);
    }

    /* ── pow: positive, negative, zero exponents ────────────────────────── */
    {
        struct {
            int64_t n, d;
            int32_t exp;
            const char *want;
        } cases[] = {
            {2, 3, 0, "1"},   {2, 3, 3, "8/27"},   {2, 3, -3, "27/8"},
            {-2, 3, 2, "4/9"}, {-2, 3, 3, "-8/27"}, {0, 1, 5, "0"},
        };
        size_t i;
        for (i = 0; i < sizeof cases / sizeof cases[0]; i++) {
            BigRational *a = rr(cases[i].n, cases[i].d), *r = NULL;
            ISO_CHECK(rat_pow(a, cases[i].exp, &r) == RAT_OK);
            expect_str(r, cases[i].want);
            rat_free(a);
        }
    }
    {
        /* A negative power of zero is 1/0 — reported as ZERO_DENOMINATOR. */
        BigRational *z = rr(0, 1), *r = NULL;
        ISO_CHECK(rat_pow(z, -2, &r) == RAT_ERR_ZERO_DENOMINATOR);
        rat_free(z);
    }

    /* ── try_pow guards oversized results ───────────────────────────────── */
    {
        BigRational *two = rr(2, 1), *r = NULL;
        ISO_CHECK(rat_try_pow(two, 10, 64, &r) == RAT_OK);
        expect_str(r, "1024");
        rat_free(two);
    }
    {
        BigRational *a = rr(10, 3), *r = NULL;
        /* Millions of projected bits are refused up front — no allocation. */
        ISO_CHECK(rat_try_pow(a, 1000000, 4096, &r) == RAT_ERR_POW_TOO_LARGE);
        ISO_CHECK(rat_try_pow(a, -1000000, 4096, &r) == RAT_ERR_POW_TOO_LARGE);
        rat_free(a);
    }

    /* ── parsing & display round-trips ──────────────────────────────────── */
    expect_str(rp("22/7"), "22/7");
    expect_str(rp("-3/4"), "-3/4");
    expect_str(rp("5"), "5");
    expect_str(rp("0"), "0");
    expect_str(rp("42"), "42");         /* bare integer → n/1, no slash */
    expect_str(rp("50/100"), "1/2");    /* normalized on parse */
    expect_str(rp("3/-4"), "-3/4");
    expect_str(rp("1/1000000000000000000000"), "1/1000000000000000000000");

    /* ── parse errors are typed ─────────────────────────────────────────── */
    {
        BigRational *r = NULL;
        ISO_CHECK(rat_parse("", &r) == RAT_PARSE_EMPTY);
        ISO_CHECK(rat_parse("/3", &r) == RAT_PARSE_EMPTY);
        ISO_CHECK(rat_parse("5/", &r) == RAT_PARSE_EMPTY);
        ISO_CHECK(rat_parse("1/2/3", &r) == RAT_PARSE_TOO_MANY_SLASHES);
        ISO_CHECK(rat_parse("5/0", &r) == RAT_PARSE_ZERO_DENOMINATOR);
        ISO_CHECK(rat_parse("x/2", &r) == RAT_PARSE_INVALID_INTEGER);
        ISO_CHECK(rat_parse("1/y", &r) == RAT_PARSE_INVALID_INTEGER);
    }

    /* ── conversions & constants ────────────────────────────────────────── */
    expect_str(rat_from_i64(5), "5");
    expect_str(rat_from_i64(-5), "-5");
    {
        BigInteger *nine = bigint_from_i64(9);
        expect_str(rat_from_integer(nine), "9");
        bigint_free(nine);
    }
    expect_str(rat_one(), "1");
    expect_str(rat_zero(), "0");

    /* ── lossy f64 export ───────────────────────────────────────────────── */
    {
        BigRational *v = rr(0, 1);
        ISO_CHECK_EQ_DBL(rat_to_f64(v), 0.0, 0.0);
        rat_free(v);
    }
    {
        BigRational *v = rr(1, 2);
        ISO_CHECK_EQ_DBL(rat_to_f64(v), 0.5, 0.0);
        rat_free(v);
    }
    {
        BigRational *v = rr(-3, 4);
        ISO_CHECK_EQ_DBL(rat_to_f64(v), -0.75, 0.0);
        rat_free(v);
    }
    {
        BigRational *v = rr(10, 1);
        ISO_CHECK_EQ_DBL(rat_to_f64(v), 10.0, 0.0);
        rat_free(v);
    }
    {
        BigRational *v = rr(160, 7); /* the bmi(70,1.75) pin value */
        ISO_CHECK_EQ_DBL(rat_to_f64(v), 160.0 / 7.0, 0.0);
        rat_free(v);
    }
    {
        BigRational *v = rr(1, 3);
        ISO_CHECK_EQ_DBL(rat_to_f64(v), 1.0 / 3.0, 0.0);
        rat_free(v);
    }
    {
        BigRational *v = rr(2, 3);
        ISO_CHECK_EQ_DBL(rat_to_f64(v), 2.0 / 3.0, 0.0);
        rat_free(v);
    }
    {
        /* Extreme magnitudes narrow cleanly (saturate / underflow), no crash. */
        BigInteger *ten = bigint_from_i64(10);
        BigInteger *big = bigint_pow(ten, 400); /* 10^400 */
        bigint_free(ten);
        BigRational *huge = rat_from_integer(big);
        bigint_free(big);
        ISO_CHECK(rat_to_f64(huge) > 1e308); /* +inf */
        BigRational *tiny = NULL;
        ISO_CHECK(rat_recip(huge, &tiny) == RAT_OK); /* 10^-400 */
        ISO_CHECK_EQ_DBL(rat_to_f64(tiny), 0.0, 0.0);
        rat_free(huge);
        rat_free(tiny);
    }

    return ISO_TEST_RESULT();
}
