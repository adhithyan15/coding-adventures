/*
 * Tests for the C BigDecimal, using the header-only iso_test.h harness (pure
 * ISO). Vectors mirror the Rust crate's own unit tests — canonical form,
 * display, parsing (plain and scientific), exact add/sub/mul, pow, rounding,
 * rounding division, ordering, the lossy f64 export, and the MAX_SCALE budget
 * that rejects scale-amplification payloads.
 */
#include "iso_test.h"

#include <stdlib.h>
#include <string.h>

#include "bignum_decimal.h"

/* Parse a literal that is expected to be valid; aborts the check on failure. */
static BigDecimal *dp(const char *s) {
    BigDecimal *d = NULL;
    DecParseStatus st = dec_parse(s, &d);
    ISO_CHECK_MSG(st == DEC_PARSE_OK, s);
    return d; /* NULL if the parse unexpectedly failed */
}

/* Assert dec_to_string(d) == expected, then free d. */
static void expect_str(BigDecimal *d, const char *expected) {
    if (!d) {
        ISO_CHECK_MSG(0, expected);
        return;
    }
    char *s = dec_to_string(d);
    ISO_CHECK(s != NULL);
    if (s) ISO_CHECK_STR_EQ(s, expected);
    free(s);
    dec_free(d);
}

/* Parse `s`, expect success, assert its string round-trips to `expected`. */
static void expect_parse_str(const char *s, const char *expected) {
    expect_str(dp(s), expected);
}

int main(void) {
    /* ── canonical form & display ──────────────────────────────────────── */
    expect_parse_str("1.230", "1.23");
    expect_parse_str("12300", "12300");
    expect_parse_str("123.45", "123.45");
    expect_parse_str("0.001", "0.001");
    expect_parse_str("0.0123", "0.0123");
    expect_parse_str("-0.5", "-0.5");
    expect_parse_str("0", "0");
    expect_parse_str("-0", "0"); /* negative zero collapses */
    expect_parse_str("0.00", "0");
    expect_parse_str("0e5", "0");

    /* 100 canonicalizes to mantissa 1, scale -2, but still displays "100". */
    {
        BigDecimal *h = dp("100");
        ISO_CHECK(h != NULL);
        if (h) {
            char *m = bigint_to_string(dec_mantissa(h));
            ISO_CHECK_STR_EQ(m, "1");
            free(m);
            ISO_CHECK_EQ_INT((int)dec_scale(h), -2);
            char *s = dec_to_string(h);
            ISO_CHECK_STR_EQ(s, "100");
            free(s);
            dec_free(h);
        }
    }
    {
        BigDecimal *a = dp("1.230");
        if (a) {
            ISO_CHECK_EQ_INT((int)dec_scale(a), 2);
            dec_free(a);
        }
    }

    /* ── from_i64 & from_integer ───────────────────────────────────────── */
    expect_str(dec_from_i64(42), "42");
    expect_str(dec_from_i64(-9), "-9");
    {
        BigInteger *n = bigint_from_i64(250);
        expect_str(dec_from_integer(n), "250");
        bigint_free(n);
    }

    /* ── parse plain & scientific ──────────────────────────────────────── */
    expect_parse_str("1.5e-3", "0.0015");
    expect_parse_str("6.022E23", "602200000000000000000000");
    expect_parse_str("1e3", "1000");
    expect_parse_str("-0.001", "-0.001");
    expect_parse_str("+42", "42");
    expect_parse_str(".5", "0.5");
    expect_parse_str("5.", "5");

    /* ── parse errors are typed ────────────────────────────────────────── */
    {
        BigDecimal *d = NULL;
        ISO_CHECK(dec_parse("", &d) == DEC_PARSE_EMPTY);
        ISO_CHECK(dec_parse("1.2.3", &d) == DEC_PARSE_MALFORMED_SHAPE);
        ISO_CHECK(dec_parse("1x2", &d) == DEC_PARSE_INVALID_DIGIT);
        ISO_CHECK(dec_parse(".", &d) == DEC_PARSE_EMPTY);
        ISO_CHECK(dec_parse("1e", &d) == DEC_PARSE_INVALID_DIGIT);
        ISO_CHECK(dec_parse("abc", &d) == DEC_PARSE_INVALID_DIGIT);
    }

    /* ── exact +, -, * (the float trap and friends) ────────────────────── */
    {
        BigDecimal *a = dp("0.1"), *b = dp("0.2"), *r = NULL;
        ISO_CHECK(dec_add(a, b, &r) == DEC_OK);
        expect_str(r, "0.3");
        dec_free(a);
        dec_free(b);
    }
    {
        BigDecimal *a = dp("1.23"), *b = dp("4.5"), *r = NULL;
        ISO_CHECK(dec_add(a, b, &r) == DEC_OK);
        expect_str(r, "5.73");
        dec_free(a);
        dec_free(b);
    }
    {
        BigDecimal *a = dp("100"), *b = dp("0.01"), *r = NULL;
        ISO_CHECK(dec_sub(a, b, &r) == DEC_OK);
        expect_str(r, "99.99");
        dec_free(a);
        dec_free(b);
    }
    {
        BigDecimal *a = dp("1.5"), *b = dp("1.5"), *r = NULL;
        ISO_CHECK(dec_mul(a, b, &r) == DEC_OK);
        expect_str(r, "2.25");
        dec_free(a);
        dec_free(b);
    }
    {
        BigDecimal *a = dp("12345.678"), *b = dp("1000"), *r = NULL;
        ISO_CHECK(dec_mul(a, b, &r) == DEC_OK);
        expect_str(r, "12345678");
        dec_free(a);
        dec_free(b);
    }
    {
        BigDecimal *a = dp("-1.5"), *b = dp("0.2"), *r = NULL;
        ISO_CHECK(dec_mul(a, b, &r) == DEC_OK);
        expect_str(r, "-0.3");
        dec_free(a);
        dec_free(b);
    }

    /* ── pow is exact ──────────────────────────────────────────────────── */
    {
        BigDecimal *a = dp("1.1"), *r = NULL;
        ISO_CHECK(dec_pow(a, 2, &r) == DEC_OK);
        expect_str(r, "1.21");
        dec_free(a);
    }
    {
        BigDecimal *a = dp("2"), *r = NULL;
        ISO_CHECK(dec_pow(a, 10, &r) == DEC_OK);
        expect_str(r, "1024");
        dec_free(a);
    }
    {
        BigDecimal *a = dp("0.5"), *r = NULL;
        ISO_CHECK(dec_pow(a, 3, &r) == DEC_OK);
        expect_str(r, "0.125");
        dec_free(a);
    }
    {
        BigDecimal *a = dp("10"), *r = NULL;
        ISO_CHECK(dec_pow(a, 0, &r) == DEC_OK);
        expect_str(r, "1");
        dec_free(a);
    }

    /* ── rounding modes on halves (Python's decimal.quantize truth table) ─ */
    {
        struct {
            const char *val;
            DecRoundingMode mode;
            const char *want;
        } cases[] = {
            {"2.5", DEC_ROUND_HALF_UP, "3"},
            {"2.5", DEC_ROUND_HALF_EVEN, "2"},
            {"2.5", DEC_ROUND_HALF_DOWN, "2"},
            {"2.5", DEC_ROUND_DOWN, "2"},
            {"2.5", DEC_ROUND_UP, "3"},
            {"2.5", DEC_ROUND_FLOOR, "2"},
            {"2.5", DEC_ROUND_CEILING, "3"},
            {"-2.5", DEC_ROUND_HALF_UP, "-3"},
            {"-2.5", DEC_ROUND_HALF_EVEN, "-2"},
            {"-2.5", DEC_ROUND_HALF_DOWN, "-2"},
            {"-2.5", DEC_ROUND_DOWN, "-2"},
            {"-2.5", DEC_ROUND_UP, "-3"},
            {"-2.5", DEC_ROUND_FLOOR, "-3"},
            {"-2.5", DEC_ROUND_CEILING, "-2"},
        };
        size_t i;
        for (i = 0; i < sizeof cases / sizeof cases[0]; i++) {
            BigDecimal *v = dp(cases[i].val), *r = NULL;
            ISO_CHECK(dec_round_to_scale(v, 0, cases[i].mode, &r) == DEC_OK);
            expect_str(r, cases[i].want);
            dec_free(v);
        }
    }

    /* ── rounding to one place ─────────────────────────────────────────── */
    {
        BigDecimal *v = dp("1.25"), *r = NULL;
        ISO_CHECK(dec_round_to_scale(v, 1, DEC_ROUND_HALF_UP, &r) == DEC_OK);
        expect_str(r, "1.3");
        dec_free(v);
    }
    {
        BigDecimal *v = dp("1.25"), *r = NULL;
        ISO_CHECK(dec_round_to_scale(v, 1, DEC_ROUND_HALF_EVEN, &r) == DEC_OK);
        expect_str(r, "1.2"); /* 2 is even */
        dec_free(v);
    }
    {
        BigDecimal *v = dp("1.35"), *r = NULL;
        ISO_CHECK(dec_round_to_scale(v, 1, DEC_ROUND_HALF_EVEN, &r) == DEC_OK);
        expect_str(r, "1.4"); /* 4 is even */
        dec_free(v);
    }
    /* Rounding to a larger scale is a no-op. */
    {
        BigDecimal *v = dp("1.5"), *r = NULL;
        ISO_CHECK(dec_round_to_scale(v, 5, DEC_ROUND_HALF_UP, &r) == DEC_OK);
        expect_str(r, "1.5");
        dec_free(v);
    }

    /* ── rounding division (pinned against Python) ─────────────────────── */
    {
        struct {
            const char *a, *b;
            int64_t scale;
            DecRoundingMode mode;
            const char *want;
        } cases[] = {
            {"10", "3", 4, DEC_ROUND_HALF_EVEN, "3.3333"},
            {"2", "3", 2, DEC_ROUND_HALF_UP, "0.67"},
            {"1", "8", 3, DEC_ROUND_HALF_EVEN, "0.125"},
            {"100", "7", 6, DEC_ROUND_DOWN, "14.285714"},
            {"-10", "3", 2, DEC_ROUND_FLOOR, "-3.34"},
            {"1", "3", 0, DEC_ROUND_HALF_UP, "0"},
            {"1", "4", 10, DEC_ROUND_HALF_EVEN, "0.25"},
        };
        size_t i;
        for (i = 0; i < sizeof cases / sizeof cases[0]; i++) {
            BigDecimal *a = dp(cases[i].a), *b = dp(cases[i].b), *r = NULL;
            ISO_CHECK(dec_div_round(a, b, cases[i].scale, cases[i].mode, &r) ==
                      DEC_OK);
            expect_str(r, cases[i].want);
            dec_free(a);
            dec_free(b);
        }
    }
    /* Division by zero is reported, not attempted. */
    {
        BigDecimal *a = dp("1"), *b = dp("0"), *r = NULL;
        ISO_CHECK(dec_div_round(a, b, 2, DEC_ROUND_HALF_UP, &r) ==
                  DEC_ERR_DIV_BY_ZERO);
        dec_free(a);
        dec_free(b);
    }
    /* An extreme target_scale is rejected up front (the DoS guard), NOT
     * materialized as a ~gigabyte power of ten. The result would be past the
     * ceiling anyway, so nothing legitimate is lost. */
    {
        BigDecimal *a = dp("1"), *b = dp("3"), *r = NULL;
        ISO_CHECK(dec_div_round(a, b, 2000000000, DEC_ROUND_HALF_UP, &r) ==
                  DEC_ERR_SCALE_OVERFLOW);
        ISO_CHECK(dec_round_to_scale(a, -2000000000, DEC_ROUND_HALF_UP, &r) ==
                  DEC_ERR_SCALE_OVERFLOW);
        dec_free(a);
        dec_free(b);
    }

    /* ── ordering & sign ───────────────────────────────────────────────── */
    {
        BigDecimal *a = dp("0.1"), *b = dp("0.2");
        int c = 0;
        ISO_CHECK(dec_cmp(a, b, &c) == DEC_OK);
        ISO_CHECK_EQ_INT(c, -1);
        dec_free(a);
        dec_free(b);
    }
    {
        BigDecimal *a = dp("100"), *b = dp("99.99");
        int c = 0;
        ISO_CHECK(dec_cmp(a, b, &c) == DEC_OK);
        ISO_CHECK_EQ_INT(c, 1);
        dec_free(a);
        dec_free(b);
    }
    {
        BigDecimal *a = dp("1.20"), *b = dp("1.2");
        int c = 99;
        ISO_CHECK(dec_cmp(a, b, &c) == DEC_OK);
        ISO_CHECK_EQ_INT(c, 0); /* equal by value */
        dec_free(a);
        dec_free(b);
    }
    {
        BigDecimal *a = dp("-0.5"), *b = dp("0");
        int c = 0;
        ISO_CHECK(dec_cmp(a, b, &c) == DEC_OK);
        ISO_CHECK_EQ_INT(c, -1);
        dec_free(a);
        dec_free(b);
    }
    {
        BigDecimal *n = dp("-3.14");
        ISO_CHECK_EQ_INT(dec_signum(n), -1);
        ISO_CHECK(dec_is_negative(n));
        expect_str(dec_abs(n), "3.14");
        dec_free(n);
    }
    {
        BigDecimal *z = dp("0");
        ISO_CHECK_EQ_INT(dec_signum(z), 0);
        ISO_CHECK(dec_is_zero(z));
        dec_free(z);
    }
    {
        BigDecimal *p = dp("1");
        ISO_CHECK(dec_is_positive(p));
        expect_str(dec_neg(p), "-1");
        dec_free(p);
    }

    /* ── lossy f64 export ──────────────────────────────────────────────── */
    {
        BigDecimal *a = dp("0.5");
        ISO_CHECK_EQ_DBL(dec_to_f64(a), 0.5, 0.0);
        dec_free(a);
    }
    {
        BigDecimal *a = dp("-2.25");
        ISO_CHECK_EQ_DBL(dec_to_f64(a), -2.25, 0.0);
        dec_free(a);
    }
    {
        BigDecimal *a = dp("123.456");
        ISO_CHECK_EQ_DBL(dec_to_f64(a), 123.456, 1e-12);
        dec_free(a);
    }
    {
        BigDecimal *a = dp("0.1");
        ISO_CHECK_EQ_DBL(dec_to_f64(a), 0.1, 0.0); /* same nearest f64 as 0.1 */
        dec_free(a);
    }

    /* ── security: the MAX_SCALE budget rejects amplification payloads ──── */
    {
        BigDecimal *d = NULL;
        /* A few bytes must NOT store a billions-of-digits scale. */
        ISO_CHECK(dec_parse("1e-2000000000", &d) == DEC_PARSE_EXPONENT_OVERFLOW);
        ISO_CHECK(dec_parse("1e2000000000", &d) == DEC_PARSE_EXPONENT_OVERFLOW);
        /* An exponent that doesn't even fit i64 is rejected, not wrapped. */
        ISO_CHECK(dec_parse("1e99999999999999999999", &d) ==
                  DEC_PARSE_EXPONENT_OVERFLOW);
        /* Trailing-zero normalization drives the stored scale over budget. */
        ISO_CHECK(dec_parse("100e999999", &d) == DEC_PARSE_EXPONENT_OVERFLOW);
        /* A parsed scale exactly at the budget is fine. */
        ISO_CHECK(dec_parse("1e-1000000", &d) == DEC_PARSE_OK);
        dec_free(d);
        d = NULL;
        /* Near-i64::MIN parsed scale with trailing zeros: error, not a crash. */
        ISO_CHECK(dec_parse("100e9223372036854775807", &d) ==
                  DEC_PARSE_EXPONENT_OVERFLOW);
        ISO_CHECK(dec_parse("1000e9223372036854775806", &d) ==
                  DEC_PARSE_EXPONENT_OVERFLOW);
    }

    /* ── from_parts enforces the internal ceiling (Rust's checked form) ── */
    {
        BigInteger *one = bigint_one();
        BigDecimal *d = NULL;
        ISO_CHECK(dec_from_parts(one, DEC_MAX_SCALE + 1, &d) == DEC_OK);
        dec_free(d);
        d = NULL;
        ISO_CHECK(dec_from_parts(one, DEC_INTERNAL_SCALE_LIMIT, &d) == DEC_OK);
        dec_free(d);
        d = NULL;
        ISO_CHECK(dec_from_parts(one, DEC_INTERNAL_SCALE_LIMIT + 1, &d) ==
                  DEC_ERR_SCALE_OVERFLOW);
        ISO_CHECK(dec_from_parts(one, INT64_MIN, &d) == DEC_ERR_SCALE_OVERFLOW);
        bigint_free(one);
    }

    return ISO_TEST_RESULT();
}
