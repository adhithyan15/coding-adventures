/*
 * test_big.c -- unit suite for the fixed-size big integer (VER05i section 3).
 *
 * Expected values were computed with Python's arbitrary-precision int, never with
 * the code under test.
 */
#include <stdio.h>
#include <string.h>

#include "iso_test.h"
#include "pbt_big.h"

static pbt_big B(const char *dec)
{
    pbt_big b;
    int rc = pbt_big_parse_dec(&b, dec, strlen(dec));
    ISO_CHECK_MSG(rc == 0, "test literal must parse");
    return b;
}

static void expect_dec(const pbt_big *x, const char *want)
{
    char buf[PBT_BIG_DEC_BUF];
    size_t n = pbt_big_to_dec(x, buf, sizeof buf);
    ISO_CHECK_MSG(n > 0, "to_dec must succeed");
    if (n > 0 && strcmp(buf, want) != 0) {
        printf("  got  %s\n  want %s\n", buf, want);
    }
    ISO_CHECK(n > 0 && strcmp(buf, want) == 0);
}

static void test_basics(void)
{
    pbt_big z;
    pbt_big a;
    pbt_big m;
    pbt_big_zero(&z);
    ISO_CHECK(pbt_big_is_zero(&z));
    ISO_CHECK_EQ_INT(pbt_big_sign(&z), 0);
    ISO_CHECK_EQ_UINT(pbt_big_bitlen(&z), 0);
    expect_dec(&z, "0");

    pbt_big_set_u64(&a, UINT64_MAX);
    expect_dec(&a, "18446744073709551615");
    ISO_CHECK_EQ_UINT(pbt_big_bitlen(&a), 64);
    ISO_CHECK(pbt_big_fits_u64(&a));
    ISO_CHECK(!pbt_big_fits_i64(&a));
    ISO_CHECK(pbt_big_low_u64(&a) == UINT64_MAX);

    pbt_big_set_i64(&m, INT64_MIN);
    expect_dec(&m, "-9223372036854775808");
    ISO_CHECK(pbt_big_fits_i64(&m));
    ISO_CHECK(pbt_big_to_i64(&m) == INT64_MIN);
    pbt_big_set_i64(&m, INT64_MAX);
    expect_dec(&m, "9223372036854775807");
    ISO_CHECK(pbt_big_to_i64(&m) == INT64_MAX);
    pbt_big_set_i64(&m, -1);
    ISO_CHECK(pbt_big_to_i64(&m) == -1);
    ISO_CHECK_EQ_INT(pbt_big_sign(&m), -1);

    /* 2^63 does not fit an int64; -2^63 does. */
    a = B("9223372036854775808");
    ISO_CHECK(!pbt_big_fits_i64(&a));
    pbt_big_neg(&a, &a);
    ISO_CHECK(pbt_big_fits_i64(&a));
    /* Negating zero stays non-negative (there is no negative zero). */
    pbt_big_neg(&z, &z);
    ISO_CHECK(z.neg == 0);
    pbt_big_set_i64(&a, 0);
    ISO_CHECK(a.neg == 0 && a.n == 0);
}

static void test_compare(void)
{
    pbt_big n5 = B("-5");
    pbt_big n7 = B("-7");
    pbt_big p3 = B("3");
    pbt_big p3b = B("3");
    pbt_big big = B("4294967296"); /* 2^32: two limbs */
    pbt_big small = B("4294967295");
    pbt_big z;
    pbt_big_zero(&z);
    ISO_CHECK(pbt_big_cmp(&n5, &n7) > 0);
    ISO_CHECK(pbt_big_cmp(&n7, &n5) < 0);
    ISO_CHECK(pbt_big_cmp(&n5, &p3) < 0);
    ISO_CHECK(pbt_big_cmp(&p3, &n5) > 0);
    ISO_CHECK(pbt_big_cmp(&p3, &p3b) == 0);
    ISO_CHECK(pbt_big_cmp(&big, &small) > 0);
    ISO_CHECK(pbt_big_cmp(&z, &n5) > 0);
    ISO_CHECK(pbt_big_cmp(&z, &p3) < 0);
    ISO_CHECK(pbt_big_cmp(&z, &z) == 0);
    ISO_CHECK(pbt_big_cmp_abs(&n7, &p3) > 0);
    ISO_CHECK(pbt_big_cmp_abs(&n5, &n5) == 0);
}

static void test_add_sub(void)
{
    pbt_big a;
    pbt_big b;
    pbt_big r;
    /* A carry across a limb and across two limbs. */
    a = B("4294967295");
    b = B("1");
    ISO_CHECK(pbt_big_add(&r, &a, &b) == 0);
    expect_dec(&r, "4294967296");
    a = B("18446744073709551615");
    ISO_CHECK(pbt_big_add(&r, &a, &b) == 0);
    expect_dec(&r, "18446744073709551616");
    /* Borrow back down. */
    ISO_CHECK(pbt_big_sub(&r, &r, &b) == 0);
    expect_dec(&r, "18446744073709551615");
    /* Mixed signs. */
    a = B("-10");
    b = B("3");
    ISO_CHECK(pbt_big_add(&r, &a, &b) == 0);
    expect_dec(&r, "-7");
    a = B("10");
    b = B("-30");
    ISO_CHECK(pbt_big_add(&r, &a, &b) == 0);
    expect_dec(&r, "-20");
    ISO_CHECK(pbt_big_sub(&r, &a, &a) == 0);
    expect_dec(&r, "0");
    ISO_CHECK(r.neg == 0);
    /* x - (-x) doubles. */
    a = B("-170141183460469231731687303715884105728");
    ISO_CHECK(pbt_big_sub(&r, &a, &a) == 0);
    ISO_CHECK(pbt_big_is_zero(&r));
    b = B("170141183460469231731687303715884105727");
    ISO_CHECK(pbt_big_sub(&r, &a, &b) == 0);
    expect_dec(&r, "-340282366920938463463374607431768211455");
    /* Output aliases an input. */
    a = B("99");
    ISO_CHECK(pbt_big_add(&a, &a, &a) == 0);
    expect_dec(&a, "198");
}

static void test_overflow(void)
{
    pbt_big p;
    pbt_big one;
    pbt_big r;
    pbt_big_set_u64(&one, 1);
    /* 2^1151 is the largest power that fits 36 limbs. */
    ISO_CHECK(pbt_big_pow2(&p, 1151) == 0);
    ISO_CHECK_EQ_UINT(pbt_big_bitlen(&p), 1152);
    ISO_CHECK(pbt_big_pow2(&r, 1152) != 0);
    ISO_CHECK(pbt_big_is_zero(&r)); /* overflow yields zero, never a wrapped value */
    ISO_CHECK(pbt_big_add(&r, &p, &p) != 0);
    ISO_CHECK(pbt_big_is_zero(&r));
    ISO_CHECK(pbt_big_shl(&r, &p, 1) != 0);
    ISO_CHECK(pbt_big_mul_add_u32(&r, &p, 2, 0) != 0);
    ISO_CHECK(pbt_big_shl(&r, &one, 4000000000u) != 0); /* a huge shift must not index out of range */
    ISO_CHECK(pbt_big_shl(&r, &one, 0xffffffffu) != 0);
    /* The largest in-range value still adds one to a smaller one. */
    ISO_CHECK(pbt_big_sub(&r, &p, &one) == 0);
    ISO_CHECK(pbt_big_add(&r, &r, &one) == 0);
    ISO_CHECK(pbt_big_cmp(&r, &p) == 0);
}

static void test_mul_shift_half(void)
{
    pbt_big a;
    pbt_big r;
    a = B("4294967295");
    ISO_CHECK(pbt_big_mul_add_u32(&r, &a, 4294967295u, 4294967295u) == 0);
    expect_dec(&r, "18446744069414584320"); /* (2^32-1)^2 + (2^32-1) = 2^64 - 2^32 */
    a = B("-12");
    ISO_CHECK(pbt_big_mul_add_u32(&r, &a, 10, 7) == 0);
    expect_dec(&r, "-127"); /* sign of a is kept: -(12*10+7) */

    a = B("1");
    ISO_CHECK(pbt_big_shl(&r, &a, 0) == 0);
    expect_dec(&r, "1");
    ISO_CHECK(pbt_big_shl(&r, &a, 31) == 0);
    expect_dec(&r, "2147483648");
    ISO_CHECK(pbt_big_shl(&r, &a, 32) == 0);
    expect_dec(&r, "4294967296");
    ISO_CHECK(pbt_big_shl(&r, &a, 33) == 0);
    expect_dec(&r, "8589934592");
    ISO_CHECK(pbt_big_shl(&r, &a, 127) == 0);
    expect_dec(&r, "170141183460469231731687303715884105728");
    a = B("-3");
    ISO_CHECK(pbt_big_shl(&r, &a, 65) == 0);
    expect_dec(&r, "-110680464442257309696");
    pbt_big_zero(&a);
    ISO_CHECK(pbt_big_shl(&r, &a, 100) == 0);
    ISO_CHECK(pbt_big_is_zero(&r));

    /* floor(x / 2) rounds toward negative infinity. */
    {
        static const struct {
            const char *in;
            const char *want;
        } cases[] = {{"0", "0"},   {"1", "0"},   {"2", "1"},     {"3", "1"},        {"-1", "-1"},
                     {"-2", "-1"}, {"-3", "-2"}, {"-4", "-2"},   {"4294967296", "2147483648"},
                     {"-4294967297", "-2147483649"}, {"18446744073709551615", "9223372036854775807"}};
        size_t i;
        for (i = 0; i < sizeof cases / sizeof cases[0]; i++) {
            pbt_big x = B(cases[i].in);
            pbt_big_floor_half(&r, &x);
            expect_dec(&r, cases[i].want);
        }
    }
}

static void test_divmod(void)
{
    pbt_big a;
    pbt_big b;
    pbt_big q;
    pbt_big r;
    pbt_big z;
    pbt_big_zero(&z);

    a = B("10000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000012345");
    b = B("987654321987654321");
    ISO_CHECK(pbt_big_divmod_abs(&q, &r, &a, &b) == 0);
    expect_dec(&q, "10124999989748437510253144520996835703612539543579843265830251949052121860761848466");
    expect_dec(&r, "607878424607890759");

    /* 2^500 - 1 divided by 2^250 + 1 */
    {
        pbt_big p500;
        pbt_big p250;
        pbt_big one;
        pbt_big_set_u64(&one, 1);
        (void)pbt_big_pow2(&p500, 500);
        (void)pbt_big_sub(&a, &p500, &one);
        (void)pbt_big_pow2(&p250, 250);
        (void)pbt_big_add(&b, &p250, &one);
        ISO_CHECK(pbt_big_divmod_abs(&q, &r, &a, &b) == 0);
        expect_dec(&q, "1809251394333065553493296640760748560207343510400633813116524750123642650623");
        ISO_CHECK(pbt_big_is_zero(&r));
    }

    /* The magnitude of negative operands is used; both outputs are non-negative. */
    a = B("-17");
    b = B("5");
    ISO_CHECK(pbt_big_divmod_abs(&q, &r, &a, &b) == 0);
    expect_dec(&q, "3");
    expect_dec(&r, "2");
    /* Dividend smaller than divisor. */
    a = B("4");
    b = B("9");
    ISO_CHECK(pbt_big_divmod_abs(&q, &r, &a, &b) == 0);
    ISO_CHECK(pbt_big_is_zero(&q));
    expect_dec(&r, "4");
    /* Either output may be NULL; outputs may alias inputs. */
    a = B("100");
    b = B("7");
    ISO_CHECK(pbt_big_divmod_abs(NULL, &r, &a, &b) == 0);
    expect_dec(&r, "2");
    ISO_CHECK(pbt_big_divmod_abs(&a, NULL, &a, &b) == 0);
    expect_dec(&a, "14");
    /* Division by zero is refused and yields zeros. */
    a = B("5");
    ISO_CHECK(pbt_big_divmod_abs(&q, &r, &a, &z) != 0);
    ISO_CHECK(pbt_big_is_zero(&q) && pbt_big_is_zero(&r));
    ISO_CHECK(pbt_big_divmod_abs(NULL, NULL, &a, &z) != 0);
    /* Zero dividend. */
    ISO_CHECK(pbt_big_divmod_abs(&q, &r, &z, &b) == 0);
    ISO_CHECK(pbt_big_is_zero(&q) && pbt_big_is_zero(&r));
}

static void test_decimal(void)
{
    pbt_big x;
    char buf[PBT_BIG_DEC_BUF];
    char tiny[4];
    char digits[PBT_BIG_MAX_DIGITS + 2];
    static const char *const good[] = {"0", "1", "-1", "340282366920938463463374607431768211456",
                                       "-340282366920938463463374607431768211456", "4294967295", "4294967296",
                                       "1000000000", "999999999", "1000000000000000000"};
    size_t i;
    for (i = 0; i < sizeof good / sizeof good[0]; i++) {
        x = B(good[i]);
        expect_dec(&x, good[i]);
    }
    /* Leading zeros are accepted by the parser (callers check canonical form); "-0" is plain zero. */
    ISO_CHECK(pbt_big_parse_dec(&x, "007", 3) == 0);
    expect_dec(&x, "7");
    ISO_CHECK(pbt_big_parse_dec(&x, "-0", 2) == 0);
    ISO_CHECK(x.neg == 0 && x.n == 0);
    /* Rejections: empty, sign only, non-digits, whitespace, plus sign. */
    ISO_CHECK(pbt_big_parse_dec(&x, "", 0) != 0);
    ISO_CHECK(pbt_big_parse_dec(&x, "-", 1) != 0);
    ISO_CHECK(pbt_big_parse_dec(&x, "12a", 3) != 0);
    ISO_CHECK(pbt_big_parse_dec(&x, " 1", 2) != 0);
    ISO_CHECK(pbt_big_parse_dec(&x, "+1", 2) != 0);
    ISO_CHECK(pbt_big_parse_dec(&x, "1.5", 3) != 0);
    ISO_CHECK(pbt_big_is_zero(&x)); /* failure leaves zero */
    /* The digit cap: 300 digits fit, 301 do not. */
    memset(digits, '9', PBT_BIG_MAX_DIGITS);
    ISO_CHECK(pbt_big_parse_dec(&x, digits, PBT_BIG_MAX_DIGITS) == 0);
    ISO_CHECK(pbt_big_to_dec(&x, buf, sizeof buf) == PBT_BIG_MAX_DIGITS);
    memset(digits, '9', PBT_BIG_MAX_DIGITS + 1);
    ISO_CHECK(pbt_big_parse_dec(&x, digits, PBT_BIG_MAX_DIGITS + 1) != 0);
    /* A rendering buffer that is too small yields 0 and writes nothing past it. */
    x = B("123456");
    ISO_CHECK_EQ_UINT(pbt_big_to_dec(&x, tiny, sizeof tiny), 0);
    ISO_CHECK_EQ_UINT(pbt_big_to_dec(&x, tiny, 0), 0);
    {
        pbt_big z;
        pbt_big_zero(&z);
        ISO_CHECK_EQ_UINT(pbt_big_to_dec(&z, tiny, 1), 0);
        ISO_CHECK_EQ_UINT(pbt_big_to_dec(&z, tiny, 2), 1);
    }
    /* Exactly enough room: length + NUL. */
    ISO_CHECK_EQ_UINT(pbt_big_to_dec(&x, tiny, 7 > sizeof tiny ? sizeof tiny : 7), 0);
    {
        char seven[7];
        ISO_CHECK_EQ_UINT(pbt_big_to_dec(&x, seven, sizeof seven), 6);
        ISO_CHECK_STR_EQ(seven, "123456");
    }
}

static void test_mod_u64(void)
{
    pbt_big x;
    x = B("-1");
    ISO_CHECK(pbt_big_mod_u64(&x) == UINT64_MAX);
    x = B("-18446744073709551616");
    ISO_CHECK(pbt_big_mod_u64(&x) == 0);
    x = B("-18446744073709551621"); /* -(2^64 + 5) */
    ISO_CHECK(pbt_big_mod_u64(&x) == 18446744073709551611u);
    x = B("18446744073709551621");
    ISO_CHECK(pbt_big_mod_u64(&x) == 5u);
    x = B("42");
    ISO_CHECK(pbt_big_mod_u64(&x) == 42u);
    pbt_big_zero(&x);
    ISO_CHECK(pbt_big_mod_u64(&x) == 0);
}

static void test_bitlen(void)
{
    pbt_big x;
    ISO_CHECK(pbt_big_pow2(&x, 0) == 0);
    ISO_CHECK_EQ_UINT(pbt_big_bitlen(&x), 1);
    ISO_CHECK(pbt_big_pow2(&x, 31) == 0);
    ISO_CHECK_EQ_UINT(pbt_big_bitlen(&x), 32);
    ISO_CHECK(pbt_big_pow2(&x, 32) == 0);
    ISO_CHECK_EQ_UINT(pbt_big_bitlen(&x), 33);
    x = B("-255");
    ISO_CHECK_EQ_UINT(pbt_big_bitlen(&x), 8);
}

int main(void)
{
    test_basics();
    test_compare();
    test_add_sub();
    test_overflow();
    test_mul_shift_half();
    test_divmod();
    test_decimal();
    test_mod_u64();
    test_bitlen();
    return ISO_TEST_RESULT();
}
