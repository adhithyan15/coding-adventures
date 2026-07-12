/*
 * bignum_rational.c — implementation of the exact fraction BigRational.
 * ===========================================================================
 *
 * A BigRational is a numerator over a denominator, both BigIntegers, held in
 * canonical form: lowest terms, positive denominator, zero pinned to 0/1. Every
 * routine reduces to BigInteger primitives (mul / add / sub / gcd / div / cmp /
 * pow). The interest is entirely in the canonicalization and the careful
 * ownership discipline across error paths.
 *
 * MEMORY DISCIPLINE. Fallible helpers return a RatStatus and, on success, write
 * a freshly-allocated result through an out-parameter; on any error they free
 * every intermediate they own and leave the out-parameter untouched. The
 * `*_owned` constructor CONSUMES the two BigIntegers handed to it (freeing them
 * on every path), which lets the public routines chain without leaks.
 */
#include "bignum_rational.h"

#include <stdlib.h>
#include <string.h>

#include "bignum_decimal.h" /* rat_to_f64 narrows through exact base-10 division */

/* ===========================================================================
 *  The representation
 * =========================================================================== */

struct BigRational {
    BigInteger *num; /* carries the sign; coprime with den */
    BigInteger *den; /* always strictly positive */
};

/* ===========================================================================
 *  Construction & canonicalization
 * =========================================================================== */

/* Reduce (num, den) to canonical form and wrap it. CONSUMES both `num` and
 * `den` (frees them on every path). */
static RatStatus rat_new_owned(BigInteger *num, BigInteger *den,
                               BigRational **out) {
    if (!num || !den) {
        bigint_free(num);
        bigint_free(den);
        return RAT_ERR_NOMEM;
    }
    if (bigint_is_zero(den)) {
        bigint_free(num);
        bigint_free(den);
        return RAT_ERR_ZERO_DENOMINATOR;
    }
    /* Step 1 — force the denominator positive, carrying the sign to the
     * numerator. (`-a/-b` and `a/-b` both need fixing; `a/b`, b>0, is left.) */
    if (bigint_is_negative(den)) {
        BigInteger *nn = bigint_neg(num);
        BigInteger *nd = bigint_neg(den);
        bigint_free(num);
        bigint_free(den);
        num = nn;
        den = nd;
        if (!num || !den) {
            bigint_free(num);
            bigint_free(den);
            return RAT_ERR_NOMEM;
        }
    }
    /* Step 2 — reduce to lowest terms. gcd is non-negative and gcd(0, d) == d,
     * so a zero numerator divides through to the canonical 0/1 automatically;
     * den > 0 here, so dividing by the positive gcd keeps it positive. */
    BigInteger *g = bigint_gcd(num, den);
    if (!g) {
        bigint_free(num);
        bigint_free(den);
        return RAT_ERR_NOMEM;
    }
    BigInteger *rn = NULL, *rd = NULL;
    /* g >= 1 for a non-zero denominator, so these divisions never divide by
     * zero and are always exact; only OOM can fail them. */
    int ok = bigint_div(num, g, &rn) == BIGINT_OK &&
             bigint_div(den, g, &rd) == BIGINT_OK;
    bigint_free(num);
    bigint_free(den);
    bigint_free(g);
    if (!ok) {
        bigint_free(rn);
        bigint_free(rd);
        return RAT_ERR_NOMEM;
    }
    BigRational *r = malloc(sizeof *r);
    if (!r) {
        bigint_free(rn);
        bigint_free(rd);
        return RAT_ERR_NOMEM;
    }
    r->num = rn;
    r->den = rd;
    *out = r;
    return RAT_OK;
}

/* Wrap two owned BigIntegers that are ALREADY canonical (coprime, den > 0),
 * skipping reduction. CONSUMES both. */
static BigRational *rat_wrap_canonical(BigInteger *num, BigInteger *den) {
    if (!num || !den) {
        bigint_free(num);
        bigint_free(den);
        return NULL;
    }
    BigRational *r = malloc(sizeof *r);
    if (!r) {
        bigint_free(num);
        bigint_free(den);
        return NULL;
    }
    r->num = num;
    r->den = den;
    return r;
}

BigRational *rat_zero(void) {
    return rat_wrap_canonical(bigint_zero(), bigint_one());
}
BigRational *rat_one(void) {
    return rat_wrap_canonical(bigint_one(), bigint_one());
}

BigRational *rat_from_integer(const BigInteger *n) {
    return rat_wrap_canonical(bigint_clone(n), bigint_one());
}

BigRational *rat_from_i64(int64_t n) {
    return rat_wrap_canonical(bigint_from_i64(n), bigint_one());
}

BigRational *rat_from_ints(int64_t num, int64_t den) {
    BigRational *out = NULL;
    RatStatus st =
        rat_new_owned(bigint_from_i64(num), bigint_from_i64(den), &out);
    return st == RAT_OK ? out : NULL; /* NULL on den==0 (Rust panic) or OOM */
}

RatStatus rat_new(const BigInteger *num, const BigInteger *den,
                  BigRational **out) {
    return rat_new_owned(bigint_clone(num), bigint_clone(den), out);
}

BigRational *rat_clone(const BigRational *a) {
    return rat_wrap_canonical(bigint_clone(a->num), bigint_clone(a->den));
}

void rat_free(BigRational *a) {
    if (!a) return;
    bigint_free(a->num);
    bigint_free(a->den);
    free(a);
}

/* ===========================================================================
 *  Accessors, predicates, sign
 * =========================================================================== */

const BigInteger *rat_numerator(const BigRational *a) { return a->num; }
const BigInteger *rat_denominator(const BigRational *a) { return a->den; }

int rat_is_zero(const BigRational *a) { return bigint_is_zero(a->num); }
int rat_is_negative(const BigRational *a) { return bigint_is_negative(a->num); }
int rat_is_positive(const BigRational *a) { return bigint_is_positive(a->num); }
int rat_signum(const BigRational *a) { return bigint_signum(a->num); }

int rat_is_integer(const BigRational *a) {
    /* denominator == 1: canonical, so den is positive and coprime — it is 1 iff
     * it has no more than the single limb holding the value one. Compare. */
    BigInteger *one = bigint_one();
    if (!one) return 0;
    int eq = bigint_cmp(a->den, one) == 0;
    bigint_free(one);
    return eq;
}

BigRational *rat_abs(const BigRational *a) {
    /* |num| over the (already positive, already coprime) denominator. */
    return rat_wrap_canonical(bigint_abs(a->num), bigint_clone(a->den));
}

RatStatus rat_recip(const BigRational *a, BigRational **out) {
    if (bigint_is_zero(a->num)) return RAT_ERR_DIV_BY_ZERO;
    /* Swap: den/num. num,den are already coprime, so no reduction happens —
     * only the sign may need moving off the denominator, which rat_new_owned
     * handles. */
    return rat_new_owned(bigint_clone(a->den), bigint_clone(a->num), out);
}

/* ===========================================================================
 *  Exact arithmetic
 * =========================================================================== */

/* Shared by add and sub: (a/b ± c/d) = (a·d ± c·b) / (b·d), then reduced. We
 * use the plain common denominator b·d (not the lcm) — canonicalization reduces
 * either way, and this keeps the code obviously correct. */
static RatStatus add_or_sub(const BigRational *a, const BigRational *b,
                            int subtract, BigRational **out) {
    BigInteger *ad = bigint_mul(a->num, b->den);
    BigInteger *cb = bigint_mul(b->num, a->den);
    BigInteger *den = bigint_mul(a->den, b->den);
    BigInteger *num = NULL;
    if (ad && cb) num = subtract ? bigint_sub(ad, cb) : bigint_add(ad, cb);
    bigint_free(ad);
    bigint_free(cb);
    /* num may be NULL (OOM) — rat_new_owned treats a NULL operand as NOMEM. */
    return rat_new_owned(num, den, out);
}

RatStatus rat_add(const BigRational *a, const BigRational *b, BigRational **out) {
    return add_or_sub(a, b, 0, out);
}
RatStatus rat_sub(const BigRational *a, const BigRational *b, BigRational **out) {
    return add_or_sub(a, b, 1, out);
}

RatStatus rat_mul(const BigRational *a, const BigRational *b, BigRational **out) {
    /* a/b · c/d = (a·c) / (b·d), then reduced. */
    return rat_new_owned(bigint_mul(a->num, b->num), bigint_mul(a->den, b->den),
                         out);
}

RatStatus rat_div(const BigRational *a, const BigRational *b, BigRational **out) {
    if (bigint_is_zero(b->num)) return RAT_ERR_DIV_BY_ZERO;
    /* a/b ÷ c/d = (a·d) / (b·c). The denominator b·c can be negative (if the
     * divisor was negative); rat_new_owned fixes the sign. */
    return rat_new_owned(bigint_mul(a->num, b->den), bigint_mul(a->den, b->num),
                         out);
}

/* The magnitude of an i32 exponent as a u32, without the UB of negating
 * INT32_MIN. */
static uint32_t exp_abs_u32(int32_t exp) {
    return exp < 0 ? (uint32_t)(0u - (uint32_t)exp) : (uint32_t)exp;
}

RatStatus rat_pow(const BigRational *a, int32_t exp, BigRational **out) {
    if (exp == 0) {
        BigRational *one = rat_one();
        if (!one) return RAT_ERR_NOMEM;
        *out = one;
        return RAT_OK;
    }
    uint32_t n = exp_abs_u32(exp);
    BigInteger *num_pow = bigint_pow(a->num, n);
    BigInteger *den_pow = bigint_pow(a->den, n);
    if (exp > 0) {
        /* num^n / den^n: coprime because num,den are; den^n > 0. Canonical. */
        BigRational *r = rat_wrap_canonical(num_pow, den_pow);
        if (!r) return RAT_ERR_NOMEM;
        *out = r;
        return RAT_OK;
    }
    /* Reciprocal: den^n / num^n. num^n may be negative or (if a == 0) zero —
     * rat_new_owned restores the sign and reports 1/0 as ZERO_DENOMINATOR. */
    return rat_new_owned(den_pow, num_pow, out);
}

RatStatus rat_try_pow(const BigRational *a, int32_t exp, uint64_t max_bits,
                      BigRational **out) {
    if (exp == 0) {
        BigRational *one = rat_one();
        if (!one) return RAT_ERR_NOMEM;
        *out = one;
        return RAT_OK;
    }
    uint32_t n = exp_abs_u32(exp);
    BigInteger *num_pow = NULL, *den_pow = NULL;
    BigIntStatus s1 = bigint_try_pow(a->num, n, max_bits, &num_pow, NULL);
    BigIntStatus s2 = bigint_try_pow(a->den, n, max_bits, &den_pow, NULL);
    if (s1 != BIGINT_OK || s2 != BIGINT_OK) {
        bigint_free(num_pow);
        bigint_free(den_pow);
        if (s1 == BIGINT_POW_TOO_LARGE || s2 == BIGINT_POW_TOO_LARGE) {
            return RAT_ERR_POW_TOO_LARGE;
        }
        return RAT_ERR_NOMEM;
    }
    if (exp > 0) {
        BigRational *r = rat_wrap_canonical(num_pow, den_pow);
        if (!r) return RAT_ERR_NOMEM;
        *out = r;
        return RAT_OK;
    }
    return rat_new_owned(den_pow, num_pow, out);
}

/* ===========================================================================
 *  Ordering
 *
 *  a/b and c/d (both with POSITIVE denominators, guaranteed by canonical form)
 *  compare the same way their cross-products do: a/b < c/d exactly when
 *  a·d < c·b. Positive denominators are what make this valid.
 * =========================================================================== */

RatStatus rat_cmp(const BigRational *a, const BigRational *b, int *cmp_out) {
    BigInteger *left = bigint_mul(a->num, b->den);
    BigInteger *right = bigint_mul(b->num, a->den);
    if (!left || !right) {
        bigint_free(left);
        bigint_free(right);
        return RAT_ERR_NOMEM;
    }
    *cmp_out = bigint_cmp(left, right);
    bigint_free(left);
    bigint_free(right);
    return RAT_OK;
}

/* ===========================================================================
 *  Formatting
 * =========================================================================== */

char *rat_to_string(const BigRational *a) {
    char *num_s = bigint_to_string(a->num);
    if (!num_s) return NULL;
    if (rat_is_integer(a)) {
        return num_s; /* whole number: just the numerator */
    }
    char *den_s = bigint_to_string(a->den);
    if (!den_s) {
        free(num_s);
        return NULL;
    }
    size_t nlen = strlen(num_s), dlen = strlen(den_s);
    /* nlen + 1 ('/') + dlen + 1 (NUL); both lengths came from real strings so
     * the sum cannot overflow size_t on any real platform. */
    char *out = malloc(nlen + dlen + 2);
    if (out) {
        memcpy(out, num_s, nlen);
        out[nlen] = '/';
        memcpy(out + nlen + 1, den_s, dlen);
        out[nlen + 1 + dlen] = '\0';
    }
    free(num_s);
    free(den_s);
    return out;
}

/* ===========================================================================
 *  Lossy f64 export
 *
 *  The Rust method divides num/den as a `BigDouble` (binary, f64 width + guard
 *  bits). This port has no float rung yet, so it takes the same nearest-f64
 *  result through the exact base-10 `BigDecimal` division and `strtod`: we pick
 *  a decimal precision that captures the value's leading significant digit plus
 *  a comfortable guard, divide exactly to that many places (round-half-even),
 *  and let `strtod` do the final correctly-rounded narrowing (and the ±inf / 0
 *  saturation for out-of-range magnitudes). For rationals of practical size the
 *  result is the correctly-rounded nearest f64 — identical to hardware division.
 * =========================================================================== */

double rat_to_f64(const BigRational *a) {
    if (bigint_is_zero(a->num)) return 0.0;

    uint64_t nbits = bigint_bit_len(a->num);
    uint64_t dbits = bigint_bit_len(a->den);
    /* Fractional places needed: the leading significant digit of a value < 1
     * sits ~ (dbits - nbits)·log10(2) places past the point; add a 45-digit
     * guard so `strtod` rounds to the correct f64. Beyond ~400 places every
     * value is below the smallest subnormal and `strtod` returns 0 anyway. */
    int64_t scale = 45;
    if (dbits > nbits) {
        uint64_t lead = ((dbits - nbits) * 30103u) / 100000u;
        if (lead > 400) lead = 400;
        scale = (int64_t)lead + 45;
    }

    BigDecimal *n = dec_from_integer(a->num);
    BigDecimal *d = dec_from_integer(a->den);
    BigDecimal *q = NULL;
    double v;
    if (n && d &&
        dec_div_round(n, d, scale, DEC_ROUND_HALF_EVEN, &q) == DEC_OK) {
        v = dec_to_f64(q);
    } else {
        v = strtod("nan", NULL); /* only reachable on OOM */
    }
    dec_free(n);
    dec_free(d);
    dec_free(q);
    return v;
}

/* ===========================================================================
 *  Parsing
 *
 *  "num/den" or a bare integer "num" (base 10); a bare integer n becomes n/1.
 *  Whitespace is not trimmed, matching the Rust `FromStr`.
 * =========================================================================== */

RatParseStatus rat_parse(const char *s, BigRational **out) {
    if (!s) return RAT_PARSE_EMPTY;

    /* Find the (single) slash. */
    const char *slash = strchr(s, '/');
    if (slash) {
        /* Reject a second slash. */
        if (strchr(slash + 1, '/')) return RAT_PARSE_TOO_MANY_SLASHES;
    }

    size_t num_len = slash ? (size_t)(slash - s) : strlen(s);
    if (num_len == 0) return RAT_PARSE_EMPTY;

    /* Copy the numerator token out so we can NUL-terminate it for the parser. */
    char *num_tok = malloc(num_len + 1);
    if (!num_tok) return RAT_PARSE_NOMEM;
    memcpy(num_tok, s, num_len);
    num_tok[num_len] = '\0';

    BigInteger *num = NULL;
    BigIntStatus ns = bigint_parse_radix(num_tok, 10, &num, NULL);
    free(num_tok);
    if (ns != BIGINT_OK) {
        return ns == BIGINT_ALLOC_ERROR ? RAT_PARSE_NOMEM
                                        : RAT_PARSE_INVALID_INTEGER;
    }

    if (!slash) {
        /* Bare integer → n/1. */
        BigRational *r = rat_wrap_canonical(num, bigint_one());
        if (!r) return RAT_PARSE_NOMEM;
        *out = r;
        return RAT_PARSE_OK;
    }

    const char *den_str = slash + 1;
    if (den_str[0] == '\0') {
        bigint_free(num);
        return RAT_PARSE_EMPTY;
    }
    BigInteger *den = NULL;
    BigIntStatus ds = bigint_parse_radix(den_str, 10, &den, NULL);
    if (ds != BIGINT_OK) {
        bigint_free(num);
        return ds == BIGINT_ALLOC_ERROR ? RAT_PARSE_NOMEM
                                        : RAT_PARSE_INVALID_INTEGER;
    }

    BigRational *r = NULL;
    RatStatus st = rat_new_owned(num, den, &r); /* consumes num, den */
    if (st == RAT_OK) {
        *out = r;
        return RAT_PARSE_OK;
    }
    if (st == RAT_ERR_ZERO_DENOMINATOR) return RAT_PARSE_ZERO_DENOMINATOR;
    return RAT_PARSE_NOMEM;
}
