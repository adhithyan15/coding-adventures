/* Tests for the C bignum-core, using the header-only iso_test.h harness (pure
 * ISO). The known "big" values are cross-checked against Python's
 * arbitrary-precision integers, matching the Rust crate's oracle tests. */
#include "iso_test.h"

#include <stdlib.h> /* free */
#include <string.h> /* strcmp */

#include "bignum_core.h"

/* Parse a decimal string into a BigInteger (asserting success). */
static BigInteger *dec(const char *s) {
    BigInteger *b = NULL;
    BigIntStatus st = bigint_parse_radix(s, 10, &b, NULL);
    ISO_CHECK_EQ_INT((int)st, (int)BIGINT_OK);
    return b;
}

/* Assert that to_string(b) equals `want`, then free b. */
static void check_str(BigInteger *b, const char *want) {
    char *s = bigint_to_string(b);
    ISO_CHECK(s != NULL && strcmp(s, want) == 0);
    free(s);
    bigint_free(b);
}

int main(void) {
    /* ── construction / to_string / signum ──────────────────────────────── */
    {
        check_str(bigint_from_u64(255), "255");
        check_str(bigint_from_i64(-255), "-255");
        check_str(bigint_zero(), "0");
        check_str(bigint_one(), "1");
        {
            BigInteger *n = bigint_from_i64(-42);
            ISO_CHECK(bigint_is_negative(n) && !bigint_is_positive(n));
            ISO_CHECK_EQ_INT(bigint_signum(n), -1);
            check_str(bigint_abs(n), "42");
            bigint_free(n);
        }
        {
            BigInteger *z = bigint_zero();
            ISO_CHECK(bigint_is_zero(z) && bigint_signum(z) == 0);
            ISO_CHECK_EQ_UINT(bigint_num_limbs(z), 0u);
            ISO_CHECK_EQ_UINT((unsigned)bigint_bit_len(z), 0u);
            bigint_free(z);
        }
    }

    /* ── bit_len boundaries ─────────────────────────────────────────────── */
    {
        BigInteger *a = bigint_from_u64(255);
        BigInteger *b = bigint_from_u64(256);
        ISO_CHECK_EQ_UINT((unsigned)bigint_bit_len(a), 8u);
        ISO_CHECK_EQ_UINT((unsigned)bigint_bit_len(b), 9u);
        bigint_free(a);
        bigint_free(b);
    }

    /* ── factorials (big multiply + decimal formatting) ─────────────────── */
    {
        BigInteger *acc = bigint_one();
        int i;
        for (i = 2; i <= 50; i++) {
            BigInteger *f = bigint_from_i64(i);
            BigInteger *nn = bigint_mul(acc, f);
            bigint_free(acc);
            bigint_free(f);
            acc = nn;
        }
        {
            char *s = bigint_to_string(acc);
            ISO_CHECK(s != NULL &&
                      strcmp(s, "3041409320171337804361260816606476884437764"
                                "1568960512000000000000") == 0);
            free(s);
        }
        bigint_free(acc);
    }

    /* ── powers beyond 64/128 bits ──────────────────────────────────────── */
    {
        BigInteger *two = bigint_from_u64(2);
        BigInteger *ten = bigint_from_u64(10);
        BigInteger *neg2 = bigint_from_i64(-2);
        check_str(bigint_pow(two, 128),
                  "340282366920938463463374607431768211456");
        check_str(bigint_pow(ten, 50),
                  "100000000000000000000000000000000000000000000000000");
        check_str(bigint_pow(neg2, 7), "-128");
        check_str(bigint_pow(neg2, 8), "256");
        {
            BigInteger *z = bigint_zero();
            check_str(bigint_pow(z, 0), "1"); /* anything^0 == 1 */
            bigint_free(z);
        }
        {
            BigInteger *z = bigint_zero();
            check_str(bigint_pow(z, 5), "0");
            bigint_free(z);
        }
        bigint_free(two);
        bigint_free(ten);
        bigint_free(neg2);
    }

    /* ── Python-oracle: mul / div_rem / gcd / radix ─────────────────────── */
    {
        BigInteger *a = dec("123456789012345678901234567890123456789");
        BigInteger *b = dec("98765432109876543210987654321");
        BigInteger *q = NULL, *r = NULL;
        BigInteger *na;

        check_str(bigint_mul(a, b),
                  "121932631137021795226185032733744855963362292333223746380"
                  "11112635269");

        ISO_CHECK_EQ_INT((int)bigint_div_rem(a, b, &q, &r), (int)BIGINT_OK);
        {
            char *qs = bigint_to_string(q);
            char *rs = bigint_to_string(r);
            ISO_CHECK(qs && strcmp(qs, "1249999988") == 0);
            ISO_CHECK(rs && strcmp(rs, "60185185206018518520725308641") == 0);
            free(qs);
            free(rs);
        }
        /* reconstruction: a == q*b + r. */
        {
            BigInteger *qb = bigint_mul(q, b);
            BigInteger *recon = bigint_add(qb, r);
            ISO_CHECK_EQ_INT(bigint_cmp(recon, a), 0);
            bigint_free(qb);
            bigint_free(recon);
        }
        bigint_free(q);
        bigint_free(r);

        /* negative dividend: truncation toward zero, remainder takes -sign. */
        na = bigint_neg(a);
        ISO_CHECK_EQ_INT((int)bigint_div_rem(na, b, &q, &r), (int)BIGINT_OK);
        {
            char *qs = bigint_to_string(q);
            char *rs = bigint_to_string(r);
            ISO_CHECK(qs && strcmp(qs, "-1249999988") == 0);
            ISO_CHECK(rs && strcmp(rs, "-60185185206018518520725308641") == 0);
            free(qs);
            free(rs);
        }
        bigint_free(q);
        bigint_free(r);
        bigint_free(na);

        check_str(bigint_gcd(a, b), "9");

        /* radix renderings of b. */
        {
            char *h = bigint_to_str_radix(b, 16);
            char *t = bigint_to_str_radix(b, 36);
            ISO_CHECK(h && strcmp(h, "13f20d9c2fff89d38e1c70cb1") == 0);
            ISO_CHECK(t && strcmp(t, "9kpsz865lt7jkxk0gq9") == 0);
            free(h);
            free(t);
        }
        /* parse back from hex. */
        {
            BigInteger *pb = NULL;
            ISO_CHECK_EQ_INT(
                (int)bigint_parse_radix("13f20d9c2fff89d38e1c70cb1", 16, &pb,
                                        NULL),
                (int)BIGINT_OK);
            ISO_CHECK_EQ_INT(bigint_cmp(pb, b), 0);
            bigint_free(pb);
        }
        bigint_free(a);
        bigint_free(b);
    }

    /* ── 7^99 and 2^200-in-base36 (Python oracle) ───────────────────────── */
    {
        BigInteger *seven = bigint_from_u64(7);
        BigInteger *two = bigint_from_u64(2);
        check_str(bigint_pow(seven, 99),
                  "46206807280353685590637825272860240155102902841494648584769"
                  "9333055955922805275437143");
        {
            BigInteger *p = bigint_pow(two, 200);
            char *s = bigint_to_str_radix(p, 36);
            ISO_CHECK(s &&
                      strcmp(s, "bnklg118comha6gqury14067gur54n8won6guf4") == 0);
            free(s);
            bigint_free(p);
        }
        bigint_free(seven);
        bigint_free(two);
    }

    /* ── radix known renderings + parse ─────────────────────────────────── */
    {
        check_str(bigint_neg(dec("42")), "-42");
        {
            BigInteger *n = bigint_from_u64(255);
            char *b2 = bigint_to_str_radix(n, 2);
            char *b16 = bigint_to_str_radix(n, 16);
            ISO_CHECK(b2 && strcmp(b2, "11111111") == 0);
            ISO_CHECK(b16 && strcmp(b16, "ff") == 0);
            free(b2);
            free(b16);
            bigint_free(n);
        }
        {
            BigInteger *n = NULL;
            ISO_CHECK_EQ_INT((int)bigint_parse_radix("FF", 16, &n, NULL),
                             (int)BIGINT_OK);
            check_str(n, "255");
        }
        {
            BigInteger *n = NULL;
            ISO_CHECK_EQ_INT((int)bigint_parse_radix("+7B", 16, &n, NULL),
                             (int)BIGINT_OK);
            check_str(n, "123");
        }
    }

    /* ── parse errors (never crash) ─────────────────────────────────────── */
    {
        BigInteger *n = NULL;
        char bad = 0;
        ISO_CHECK_EQ_INT((int)bigint_parse_radix("", 10, &n, NULL),
                         (int)BIGINT_PARSE_EMPTY);
        ISO_CHECK_EQ_INT((int)bigint_parse_radix("-", 10, &n, NULL),
                         (int)BIGINT_PARSE_EMPTY);
        ISO_CHECK_EQ_INT((int)bigint_parse_radix("12x3", 10, &n, &bad),
                         (int)BIGINT_PARSE_INVALID_DIGIT);
        ISO_CHECK(bad == 'x');
        ISO_CHECK_EQ_INT((int)bigint_parse_radix("102", 2, &n, NULL),
                         (int)BIGINT_PARSE_INVALID_DIGIT);
        ISO_CHECK_EQ_INT((int)bigint_parse_radix("10", 1, &n, NULL),
                         (int)BIGINT_PARSE_INVALID_RADIX);
        ISO_CHECK_EQ_INT((int)bigint_parse_radix("10", 37, &n, NULL),
                         (int)BIGINT_PARSE_INVALID_RADIX);
        /* "-0" and "+000" parse to canonical zero. */
        ISO_CHECK_EQ_INT((int)bigint_parse_radix("-0", 10, &n, NULL),
                         (int)BIGINT_OK);
        ISO_CHECK(bigint_is_zero(n));
        bigint_free(n);
    }

    /* ── div by zero + try_pow guard ────────────────────────────────────── */
    {
        BigInteger *five = bigint_from_u64(5);
        BigInteger *z = bigint_zero();
        BigInteger *q = NULL;
        ISO_CHECK_EQ_INT((int)bigint_div_rem(five, z, &q, NULL),
                         (int)BIGINT_DIV_BY_ZERO);

        {
            BigInteger *two = bigint_from_u64(2);
            uint64_t proj = 0;
            BigInteger *out = NULL;
            ISO_CHECK_EQ_INT(
                (int)bigint_try_pow(two, 4000000000u, (uint64_t)1 << 20, &out,
                                    &proj),
                (int)BIGINT_POW_TOO_LARGE);
            ISO_CHECK(proj > ((uint64_t)1 << 20));
            /* a modest exponent succeeds and equals plain pow. */
            ISO_CHECK_EQ_INT((int)bigint_try_pow(two, 200, 4096, &out, NULL),
                             (int)BIGINT_OK);
            {
                BigInteger *p = bigint_pow(two, 200);
                ISO_CHECK_EQ_INT(bigint_cmp(out, p), 0);
                bigint_free(p);
            }
            bigint_free(out);
            bigint_free(two);
        }
        bigint_free(five);
        bigint_free(z);
    }

    /* ── ordering across signs ──────────────────────────────────────────── */
    {
        BigInteger *neg = bigint_from_i64(-5);
        BigInteger *zero = bigint_zero();
        BigInteger *pos = bigint_from_u64(1000000);
        ISO_CHECK_EQ_INT(bigint_cmp(neg, zero), -1);
        ISO_CHECK_EQ_INT(bigint_cmp(zero, pos), -1);
        ISO_CHECK_EQ_INT(bigint_cmp(pos, neg), 1);
        ISO_CHECK_EQ_INT(bigint_cmp(pos, pos), 0);
        bigint_free(neg);
        bigint_free(zero);
        bigint_free(pos);
    }

    return ISO_TEST_RESULT();
}
