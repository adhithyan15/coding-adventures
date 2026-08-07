/*
 * bignum_decimal.c — implementation of the exact base-10 BigDecimal.
 * ===========================================================================
 *
 * The value is `mant × 10^(-scale)`, held in canonical form (mantissa carries
 * no trailing zero; zero is `(0, 0)`). Every routine here reduces to a handful
 * of BigInteger primitives (add/sub/mul/div_rem/pow/cmp) plus careful, checked
 * i64 scale bookkeeping — the arithmetic is deliberately boring; the interest
 * is in the invariants and the overflow guards.
 *
 * MEMORY DISCIPLINE. Fallible helpers return a DecStatus and, on success, write
 * a freshly-allocated result through an out-parameter; on any error they free
 * every intermediate they own and leave the out-parameter untouched. The
 * `*_owned` internal constructors CONSUME the BigInteger handed to them (freeing
 * it even on failure), which lets the public routines chain without leaks.
 */
#include "bignum_decimal.h"

#include <errno.h>
#include <limits.h>
#include <stdlib.h>
#include <string.h>

/* ===========================================================================
 *  The representation
 * =========================================================================== */

struct BigDecimal {
    BigInteger *mant; /* the unscaled integer, canonical (no trailing 0 digit) */
    int64_t scale;    /* value is mant × 10^(-scale) */
};

/* The largest exponent we will ever hand to `ten_pow`. Division/rounding apply
 * an exponent e = target_scale ± (two operand scales); every operand scale is
 * ceiling-bounded, so any e that could yield a REPRESENTABLE result (whose
 * canonical scale magnitude must be ≤ DEC_INTERNAL_SCALE_LIMIT) stays within 3×
 * the ceiling. A larger e is always doomed — the ceiling would reject the
 * result anyway — so we reject it up front rather than materialize a
 * multi-hundred-megabyte power of ten first. (Rust leaves this "caller's
 * responsibility", bounding only at u32::MAX; guarding here makes the
 * always-rejected case cheap instead of a DoS lever.) 3·8e6 = 24e6 caps any
 * such power of ten at ~24 MB. */
#define DEC_MATERIALIZE_LIMIT (3 * DEC_INTERNAL_SCALE_LIMIT)

/* ===========================================================================
 *  Checked i64 arithmetic
 *
 *  Rust uses `i128` intermediates and `checked_*` methods to keep scale
 *  bookkeeping from overflowing silently. C17 has no `__int128`, so we do the
 *  overflow tests by hand. Each returns 1 on success (result in *out), 0 on
 *  overflow.
 * =========================================================================== */

static int i64_add_checked(int64_t a, int64_t b, int64_t *out) {
    if (b > 0 && a > INT64_MAX - b) return 0;
    if (b < 0 && a < INT64_MIN - b) return 0;
    *out = a + b;
    return 1;
}

static int i64_sub_checked(int64_t a, int64_t b, int64_t *out) {
    /* a - b overflows exactly when a + (-b) would; handle b == INT64_MIN too. */
    if (b < 0) {
        if (a > INT64_MAX + b) return 0; /* b<0 ⇒ INT64_MAX+b is in range */
    } else {
        if (a < INT64_MIN + b) return 0;
    }
    *out = a - b;
    return 1;
}

static int i64_mul_checked(int64_t a, int64_t b, int64_t *out) {
    if (a == 0 || b == 0) {
        *out = 0;
        return 1;
    }
    /* Test BEFORE multiplying: forming an overflowing product is signed-overflow
     * UB, and an optimizer may delete a divide-back check placed after it. */
    if (a == -1 && b == INT64_MIN) return 0;
    if (b == -1 && a == INT64_MIN) return 0;
    if (a > 0) {
        if (b > 0 ? a > INT64_MAX / b : b < INT64_MIN / a) return 0;
    } else { /* a < 0 */
        if (b > 0 ? a < INT64_MIN / b : b < INT64_MAX / a) return 0;
    }
    *out = a * b;
    return 1;
}

/* The unsigned magnitude of an i64, without the UB of negating INT64_MIN. */
static uint64_t i64_abs_u64(int64_t s) {
    return s < 0 ? (0u - (uint64_t)s) : (uint64_t)s;
}

/* ===========================================================================
 *  BigInteger conveniences
 * =========================================================================== */

/* 10^n as a BigInteger (NULL on OOM). n is u32 because 10^(2^32) is already
 * astronomically large; the scale budgets keep every n reached here small. */
static BigInteger *ten_pow(uint32_t n) {
    BigInteger *ten = bigint_from_i64(10);
    if (!ten) return NULL;
    BigInteger *p = bigint_pow(ten, n);
    bigint_free(ten);
    return p; /* NULL propagates on OOM */
}

/* Attach `sign` (-1/+1) to a non-negative magnitude, CONSUMING `mag`
 * (freeing it) and returning a fresh value (NULL on OOM). */
static BigInteger *apply_sign_consume(BigInteger *mag, int sign) {
    if (!mag) return NULL;
    if (sign < 0) {
        BigInteger *n = bigint_neg(mag);
        bigint_free(mag);
        return n;
    }
    return mag;
}

/* ===========================================================================
 *  Construction & normalization
 * =========================================================================== */

/* Strip every trailing zero digit from *mant_io (lowering *scale_io by one each
 * time — value-preserving), and pin zero to (0, 0). CONSUMES/replaces *mant_io.
 * Returns 0 on OOM (in which case *mant_io is freed and set NULL). */
static int normalize_inplace(BigInteger **mant_io, int64_t *scale_io) {
    BigInteger *m = *mant_io;
    if (bigint_is_zero(m)) {
        bigint_free(m);
        *mant_io = bigint_zero();
        *scale_io = 0;
        return *mant_io != NULL;
    }
    BigInteger *ten = bigint_from_i64(10);
    if (!ten) {
        bigint_free(m);
        *mant_io = NULL;
        return 0;
    }
    for (;;) {
        BigInteger *q = NULL, *r = NULL;
        if (bigint_div_rem(m, ten, &q, &r) != BIGINT_OK) {
            bigint_free(ten);
            bigint_free(m);
            *mant_io = NULL;
            return 0;
        }
        if (!bigint_is_zero(r)) {
            bigint_free(q);
            bigint_free(r);
            break;
        }
        bigint_free(r);
        bigint_free(m);
        m = q;
        /* saturating_sub(scale, 1): a scale anywhere near INT64_MIN is
         * astronomically past every ceiling, so the caller's budget check
         * rejects it anyway — we only need to avoid the underflow itself. */
        if (*scale_io != INT64_MIN) {
            *scale_io -= 1;
        }
    }
    bigint_free(ten);
    *mant_io = m;
    return 1;
}

/* Wrap (mant, scale) into a heap BigDecimal after normalizing and enforcing the
 * internal ceiling. CONSUMES `mant` (frees it on every path). */
static DecStatus from_parts_owned(BigInteger *mant, int64_t scale,
                                  BigDecimal **out) {
    if (!mant) return DEC_ERR_NOMEM;
    if (!normalize_inplace(&mant, &scale)) return DEC_ERR_NOMEM;
    if (i64_abs_u64(scale) > (uint64_t)DEC_INTERNAL_SCALE_LIMIT) {
        bigint_free(mant);
        return DEC_ERR_SCALE_OVERFLOW;
    }
    BigDecimal *d = malloc(sizeof *d);
    if (!d) {
        bigint_free(mant);
        return DEC_ERR_NOMEM;
    }
    d->mant = mant;
    d->scale = scale;
    *out = d;
    return DEC_OK;
}

/* Build a BigDecimal directly from an owned mantissa and an ALREADY-canonical
 * scale, skipping normalization (used by abs/neg/clone, which cannot introduce
 * a trailing zero). CONSUMES `mant`. */
static BigDecimal *wrap_canonical(BigInteger *mant, int64_t scale) {
    if (!mant) return NULL;
    BigDecimal *d = malloc(sizeof *d);
    if (!d) {
        bigint_free(mant);
        return NULL;
    }
    d->mant = mant;
    d->scale = scale;
    return d;
}

BigDecimal *dec_zero(void) { return wrap_canonical(bigint_zero(), 0); }
BigDecimal *dec_one(void) { return wrap_canonical(bigint_one(), 0); }

BigDecimal *dec_from_i64(int64_t n) {
    BigDecimal *out = NULL;
    return from_parts_owned(bigint_from_i64(n), 0, &out) == DEC_OK ? out : NULL;
}

BigDecimal *dec_from_integer(const BigInteger *n) {
    BigDecimal *out = NULL;
    return from_parts_owned(bigint_clone(n), 0, &out) == DEC_OK ? out : NULL;
}

DecStatus dec_from_parts(const BigInteger *mant, int64_t scale,
                         BigDecimal **out) {
    return from_parts_owned(bigint_clone(mant), scale, out);
}

BigDecimal *dec_clone(const BigDecimal *a) {
    return wrap_canonical(bigint_clone(a->mant), a->scale);
}

void dec_free(BigDecimal *a) {
    if (!a) return;
    bigint_free(a->mant);
    free(a);
}

/* ===========================================================================
 *  Accessors, predicates, sign
 * =========================================================================== */

const BigInteger *dec_mantissa(const BigDecimal *a) { return a->mant; }
int64_t dec_scale(const BigDecimal *a) { return a->scale; }

int dec_is_zero(const BigDecimal *a) { return bigint_is_zero(a->mant); }
int dec_is_negative(const BigDecimal *a) { return bigint_is_negative(a->mant); }
int dec_is_positive(const BigDecimal *a) { return bigint_is_positive(a->mant); }
int dec_signum(const BigDecimal *a) { return bigint_signum(a->mant); }

BigDecimal *dec_abs(const BigDecimal *a) {
    return wrap_canonical(bigint_abs(a->mant), a->scale);
}
BigDecimal *dec_neg(const BigDecimal *a) {
    return wrap_canonical(bigint_neg(a->mant), a->scale);
}

/* ===========================================================================
 *  Scale alignment (shared by +, -, and comparison)
 * =========================================================================== */

/* Re-express both mantissas at the common scale max(a.scale, b.scale) — exact,
 * since raising to a finer scale only appends zeros. Writes fresh mantissas to
 * am and bm, and the common scale to scale. Both scales are ceiling-bounded, so
 * their difference fits comfortably in i64 and (defensively guarded) in u32. */
static DecStatus aligned_mantissas(const BigDecimal *a, const BigDecimal *b,
                                   BigInteger **am, BigInteger **bm,
                                   int64_t *scale) {
    int64_t target = a->scale > b->scale ? a->scale : b->scale;
    int64_t da = target - a->scale; /* >= 0, bounded by 2·ceiling */
    int64_t db = target - b->scale;
    if (da > (int64_t)UINT32_MAX || db > (int64_t)UINT32_MAX) {
        return DEC_ERR_SCALE_OVERFLOW; /* unreachable for constructed values */
    }
    BigInteger *pa = ten_pow((uint32_t)da);
    if (!pa) return DEC_ERR_NOMEM;
    BigInteger *pb = ten_pow((uint32_t)db);
    if (!pb) {
        bigint_free(pa);
        return DEC_ERR_NOMEM;
    }
    BigInteger *ra = bigint_mul(a->mant, pa);
    BigInteger *rb = bigint_mul(b->mant, pb);
    bigint_free(pa);
    bigint_free(pb);
    if (!ra || !rb) {
        bigint_free(ra);
        bigint_free(rb);
        return DEC_ERR_NOMEM;
    }
    *am = ra;
    *bm = rb;
    *scale = target;
    return DEC_OK;
}

/* ===========================================================================
 *  Exact arithmetic
 * =========================================================================== */

/* Shared by add and sub: align, combine the mantissas, renormalize. */
static DecStatus add_or_sub(const BigDecimal *a, const BigDecimal *b, int subtract,
                            BigDecimal **out) {
    BigInteger *am = NULL, *bm = NULL;
    int64_t scale = 0;
    DecStatus st = aligned_mantissas(a, b, &am, &bm, &scale);
    if (st != DEC_OK) return st;
    BigInteger *combined = subtract ? bigint_sub(am, bm) : bigint_add(am, bm);
    bigint_free(am);
    bigint_free(bm);
    return from_parts_owned(combined, scale, out); /* NULL combined ⇒ NOMEM */
}

DecStatus dec_add(const BigDecimal *a, const BigDecimal *b, BigDecimal **out) {
    return add_or_sub(a, b, 0, out);
}
DecStatus dec_sub(const BigDecimal *a, const BigDecimal *b, BigDecimal **out) {
    return add_or_sub(a, b, 1, out);
}

DecStatus dec_mul(const BigDecimal *a, const BigDecimal *b, BigDecimal **out) {
    /* (m1·10^-s1)·(m2·10^-s2) = (m1·m2)·10^-(s1+s2). */
    int64_t scale;
    if (!i64_add_checked(a->scale, b->scale, &scale)) return DEC_ERR_SCALE_OVERFLOW;
    return from_parts_owned(bigint_mul(a->mant, b->mant), scale, out);
}

DecStatus dec_pow(const BigDecimal *a, uint32_t exp, BigDecimal **out) {
    /* (m·10^-s)^e = m^e · 10^-(s·e). */
    int64_t scale;
    if (!i64_mul_checked(a->scale, (int64_t)exp, &scale)) return DEC_ERR_SCALE_OVERFLOW;
    return from_parts_owned(bigint_pow(a->mant, exp), scale, out);
}

/* ===========================================================================
 *  Rounding division core
 *
 *  Round the exact quotient n/d (d != 0) to the nearest integer under `mode`.
 *  Everything is decided from the truncating quotient q and remainder r of the
 *  magnitudes: |n| = q·|d| + r with 0 ≤ r < |d|. If r == 0 the quotient is
 *  exact. Otherwise 2r vs |d| places the fraction below / at / above halfway.
 *  The result's sign is sign(n·d). Returns NULL on OOM.
 * =========================================================================== */

static BigInteger *round_div(const BigInteger *n, const BigInteger *d,
                             DecRoundingMode mode) {
    int sign = bigint_signum(n) * bigint_signum(d);
    if (sign == 0) return bigint_zero(); /* n == 0 */

    BigInteger *na = bigint_abs(n);
    BigInteger *da = bigint_abs(d);
    if (!na || !da) {
        bigint_free(na);
        bigint_free(da);
        return NULL;
    }
    BigInteger *q = NULL, *r = NULL;
    if (bigint_div_rem(na, da, &q, &r) != BIGINT_OK) {
        bigint_free(na);
        bigint_free(da);
        return NULL;
    }
    bigint_free(na);
    if (bigint_is_zero(r)) {
        bigint_free(da);
        bigint_free(r);
        return apply_sign_consume(q, sign); /* exact */
    }

    /* Decide whether to round the magnitude away from zero. */
    BigInteger *two = bigint_from_i64(2);
    BigInteger *two_r = two ? bigint_mul(r, two) : NULL;
    BigInteger *q_mod2 = NULL; /* q parity; two != 0 so failure means OOM */
    if (two) {
        (void)bigint_rem(q, two, &q_mod2);
    }
    bigint_free(r);
    if (!two || !two_r || !q_mod2) {
        bigint_free(da);
        bigint_free(q);
        bigint_free(two);
        bigint_free(two_r);
        bigint_free(q_mod2);
        return NULL;
    }
    int half_cmp = bigint_cmp(two_r, da); /* r vs d/2: -1 below, 0 at, +1 above */
    int q_is_odd = !bigint_is_zero(q_mod2);
    bigint_free(da);
    bigint_free(two);
    bigint_free(two_r);
    bigint_free(q_mod2);

    int round_away;
    switch (mode) {
        case DEC_ROUND_DOWN: round_away = 0; break;
        case DEC_ROUND_UP: round_away = 1; break;
        case DEC_ROUND_FLOOR: round_away = sign < 0; break;   /* toward -inf */
        case DEC_ROUND_CEILING: round_away = sign > 0; break; /* toward +inf */
        case DEC_ROUND_HALF_UP: round_away = half_cmp >= 0; break;
        case DEC_ROUND_HALF_DOWN: round_away = half_cmp > 0; break;
        case DEC_ROUND_HALF_EVEN:
            round_away = half_cmp > 0 || (half_cmp == 0 && q_is_odd);
            break;
        default: round_away = 0; break;
    }

    BigInteger *magnitude;
    if (round_away) {
        BigInteger *one = bigint_one();
        magnitude = one ? bigint_add(q, one) : NULL;
        bigint_free(one);
        bigint_free(q);
    } else {
        magnitude = q;
    }
    return apply_sign_consume(magnitude, sign); /* frees magnitude, NULL on OOM */
}

DecStatus dec_div_round(const BigDecimal *a, const BigDecimal *b,
                        int64_t target_scale, DecRoundingMode mode,
                        BigDecimal **out) {
    if (bigint_is_zero(b->mant)) return DEC_ERR_DIV_BY_ZERO;
    /* We want R with R·10^-target_scale ≈ a/b. With a=m1·10^-s1, b=m2·10^-s2,
     * R = round( m1 · 10^(s2 - s1 + target_scale) / m2 ). Apply the exponent
     * e = target_scale + s2 - s1 to whichever side keeps both integers. */
    int64_t e;
    if (!i64_add_checked(target_scale, b->scale, &e)) return DEC_ERR_SCALE_OVERFLOW;
    if (!i64_sub_checked(e, a->scale, &e)) return DEC_ERR_SCALE_OVERFLOW;

    BigInteger *rounded;
    if (e >= 0) {
        if (e > DEC_MATERIALIZE_LIMIT) return DEC_ERR_SCALE_OVERFLOW;
        BigInteger *tp = ten_pow((uint32_t)e);
        if (!tp) return DEC_ERR_NOMEM;
        BigInteger *num = bigint_mul(a->mant, tp);
        bigint_free(tp);
        if (!num) return DEC_ERR_NOMEM;
        rounded = round_div(num, b->mant, mode);
        bigint_free(num);
    } else {
        uint64_t mag = i64_abs_u64(e);
        if (mag > (uint64_t)DEC_MATERIALIZE_LIMIT) return DEC_ERR_SCALE_OVERFLOW;
        BigInteger *tp = ten_pow((uint32_t)mag);
        if (!tp) return DEC_ERR_NOMEM;
        BigInteger *den = bigint_mul(b->mant, tp);
        bigint_free(tp);
        if (!den) return DEC_ERR_NOMEM;
        rounded = round_div(a->mant, den, mode);
        bigint_free(den);
    }
    if (!rounded) return DEC_ERR_NOMEM;
    return from_parts_owned(rounded, target_scale, out);
}

DecStatus dec_round_to_scale(const BigDecimal *a, int64_t target_scale,
                             DecRoundingMode mode, BigDecimal **out) {
    if (target_scale >= a->scale) {
        /* Already exactly representable at this (or a coarser) scale. */
        BigDecimal *c = dec_clone(a);
        if (!c) return DEC_ERR_NOMEM;
        *out = c;
        return DEC_OK;
    }
    int64_t drop;
    if (!i64_sub_checked(a->scale, target_scale, &drop)) return DEC_ERR_SCALE_OVERFLOW;
    if (drop > DEC_MATERIALIZE_LIMIT) return DEC_ERR_SCALE_OVERFLOW;
    BigInteger *tp = ten_pow((uint32_t)drop);
    if (!tp) return DEC_ERR_NOMEM;
    BigInteger *rounded = round_div(a->mant, tp, mode);
    bigint_free(tp);
    if (!rounded) return DEC_ERR_NOMEM;
    return from_parts_owned(rounded, target_scale, out);
}

/* ===========================================================================
 *  Ordering
 * =========================================================================== */

DecStatus dec_cmp(const BigDecimal *a, const BigDecimal *b, int *cmp_out) {
    BigInteger *am = NULL, *bm = NULL;
    int64_t scale = 0;
    DecStatus st = aligned_mantissas(a, b, &am, &bm, &scale);
    if (st != DEC_OK) return st;
    *cmp_out = bigint_cmp(am, bm);
    bigint_free(am);
    bigint_free(bm);
    return DEC_OK;
}

/* ===========================================================================
 *  Formatting
 * =========================================================================== */

/* Grow-and-fill a byte buffer with overflow-guarded sizing. Returns a malloc'd,
 * NUL-terminated string, NULL on OOM or on an implausibly large length. */
static char *build_display(int neg, const char *digits, int64_t scale) {
    size_t dlen = strlen(digits);

    /* Compute the total content length in u64 first, so an 8-million-place
     * scale on a 32-bit size_t cannot silently truncate the allocation. */
    uint64_t content; /* excludes sign and NUL */
    /* We assemble one of three shapes; compute each length precisely. */
    uint64_t zeros = 0, lead = 0;
    int put_dot = 0, whole = 0;
    if (scale <= 0) {
        whole = 1;
        zeros = i64_abs_u64(scale); /* trailing zeros after the digits */
        content = (uint64_t)dlen + zeros;
    } else {
        put_dot = 1;
        uint64_t s = (uint64_t)scale;
        if ((uint64_t)dlen > s) {
            content = (uint64_t)dlen + 1; /* one '.' inside the digits */
        } else {
            lead = s - (uint64_t)dlen; /* "0." then leading zeros */
            content = 2 + lead + (uint64_t)dlen; /* '0' '.' + zeros + digits */
        }
    }

    uint64_t total = content + (uint64_t)(neg ? 1 : 0) + 1u; /* + sign + NUL */
    if (total > (uint64_t)((size_t)-1)) return NULL;

    char *out = malloc((size_t)total);
    if (!out) return NULL;
    char *p = out;
    if (neg) *p++ = '-';

    if (whole) {
        memcpy(p, digits, dlen);
        p += dlen;
        for (uint64_t i = 0; i < zeros; i++) *p++ = '0';
    } else if (put_dot) {
        uint64_t s = (uint64_t)scale;
        if ((uint64_t)dlen > s) {
            size_t intlen = dlen - (size_t)s; /* dlen > s ⇒ fits size_t */
            memcpy(p, digits, intlen);
            p += intlen;
            *p++ = '.';
            memcpy(p, digits + intlen, (size_t)s);
            p += s;
        } else {
            *p++ = '0';
            *p++ = '.';
            for (uint64_t i = 0; i < lead; i++) *p++ = '0';
            memcpy(p, digits, dlen);
            p += dlen;
        }
    }
    *p = '\0';
    return out;
}

char *dec_to_string(const BigDecimal *a) {
    if (bigint_is_zero(a->mant)) {
        char *z = malloc(2);
        if (z) {
            z[0] = '0';
            z[1] = '\0';
        }
        return z;
    }
    int neg = bigint_is_negative(a->mant);
    BigInteger *absm = bigint_abs(a->mant);
    if (!absm) return NULL;
    char *digits = bigint_to_string(absm); /* base-10, no sign */
    bigint_free(absm);
    if (!digits) return NULL;
    char *out = build_display(neg, digits, a->scale);
    free(digits);
    return out;
}

double dec_to_f64(const BigDecimal *a) {
    char *s = dec_to_string(a);
    if (!s) return strtod("nan", NULL); /* only reachable on OOM */
    double v = strtod(s, NULL); /* correctly rounded; ±inf/0 on out-of-range */
    free(s);
    return v;
}

/* ===========================================================================
 *  Parsing
 *
 *  Accepts an optional sign, integer and fractional digit groups around at most
 *  one '.', and an optional 'e'/'E' exponent. The mantissa is the integer and
 *  fractional digits concatenated; the scale is (fractional digit count) minus
 *  the exponent. The DEC_MAX_SCALE budget is enforced on the CANONICAL scale
 *  (after trailing-zero stripping), the untrusted-input boundary.
 * =========================================================================== */

static int all_ascii_digits(const char *s, size_t len) {
    for (size_t i = 0; i < len; i++) {
        if (s[i] < '0' || s[i] > '9') return 0;
    }
    return 1;
}

DecParseStatus dec_parse(const char *s, BigDecimal **out) {
    if (!s || s[0] == '\0') return DEC_PARSE_EMPTY;
    size_t n = strlen(s);
    size_t i = 0;
    int negative = 0;
    if (s[0] == '+' || s[0] == '-') {
        negative = s[0] == '-';
        i = 1;
    }

    /* Split off an optional exponent at the first 'e'/'E'. */
    const char *rest = s + i;
    size_t rest_len = n - i;
    const char *epos = NULL;
    for (size_t k = 0; k < rest_len; k++) {
        if (rest[k] == 'e' || rest[k] == 'E') {
            epos = rest + k;
            break;
        }
    }
    const char *digits_part = rest;
    size_t digits_len = epos ? (size_t)(epos - rest) : rest_len;
    const char *exp_part = epos ? epos + 1 : NULL;
    size_t exp_len = epos ? rest_len - digits_len - 1 : 0;

    /* Integer and fractional groups around at most one '.'. */
    const char *dot = NULL;
    for (size_t k = 0; k < digits_len; k++) {
        if (digits_part[k] == '.') {
            if (dot) return DEC_PARSE_MALFORMED_SHAPE; /* two dots */
            dot = digits_part + k;
        }
    }
    const char *int_digits = digits_part;
    size_t int_len = dot ? (size_t)(dot - digits_part) : digits_len;
    const char *frac_digits = dot ? dot + 1 : digits_part + digits_len;
    size_t frac_len = dot ? digits_len - int_len - 1 : 0;

    if (int_len == 0 && frac_len == 0) return DEC_PARSE_EMPTY;
    if (!all_ascii_digits(int_digits, int_len) ||
        !all_ascii_digits(frac_digits, frac_len)) {
        return DEC_PARSE_INVALID_DIGIT;
    }

    /* The exponent, if present. */
    int64_t exp = 0;
    if (exp_part) {
        if (exp_len == 0) return DEC_PARSE_INVALID_DIGIT; /* "1e" */
        /* Validate shape: optional sign then digits. */
        size_t j = 0;
        if (exp_part[0] == '+' || exp_part[0] == '-') j = 1;
        if (j == exp_len || !all_ascii_digits(exp_part + j, exp_len - j)) {
            return DEC_PARSE_INVALID_DIGIT;
        }
        /* Parse into i64; a well-formed but too-big exponent is an overflow.
         * `exp_part` is the tail of the NUL-terminated input, so `strtoll` can
         * consume it directly and must reach its terminator. */
        errno = 0;
        char *endp = NULL;
        long long v = strtoll(exp_part, &endp, 10);
        if (endp != exp_part + exp_len) return DEC_PARSE_INVALID_DIGIT;
        if (errno == ERANGE) return DEC_PARSE_EXPONENT_OVERFLOW;
#if LLONG_MAX > INT64_MAX
        /* Only reachable where `long long` is wider than 64 bits; on such a
         * platform `strtoll` would not have flagged an i64-range overflow. */
        if (v > INT64_MAX || v < INT64_MIN) return DEC_PARSE_EXPONENT_OVERFLOW;
#endif
        exp = (int64_t)v;
    }

    /* Assemble the mantissa integer string: [-] int_digits frac_digits. */
    size_t mant_len = int_len + frac_len;
    char *mbuf = malloc(mant_len + 2); /* sign + NUL */
    if (!mbuf) return DEC_PARSE_NOMEM;
    char *mp = mbuf;
    if (negative) *mp++ = '-';
    memcpy(mp, int_digits, int_len);
    mp += int_len;
    memcpy(mp, frac_digits, frac_len);
    mp += frac_len;
    *mp = '\0';

    BigInteger *mant = NULL;
    if (mbuf[0] == '\0' || (mbuf[0] == '-' && mbuf[1] == '\0')) {
        mant = bigint_zero(); /* ".5"/"5." leaves one side empty — guard "" */
    } else {
        BigIntStatus bs = bigint_parse_radix(mbuf, 10, &mant, NULL);
        if (bs != BIGINT_OK) {
            free(mbuf);
            return bs == BIGINT_ALLOC_ERROR ? DEC_PARSE_NOMEM
                                            : DEC_PARSE_INVALID_DIGIT;
        }
    }
    free(mbuf);
    if (!mant) return DEC_PARSE_NOMEM;

    /* scale = frac_len - exp; fractional digits push the point right, the
     * exponent pushes it left. */
    int64_t scale;
    if (!i64_sub_checked((int64_t)frac_len, exp, &scale)) {
        bigint_free(mant);
        return DEC_PARSE_EXPONENT_OVERFLOW;
    }

    /* Canonicalize (this can change the stored scale) THEN enforce the strict
     * MAX_SCALE budget on the canonical scale. */
    BigDecimal *d = NULL;
    DecStatus st = from_parts_owned(mant, scale, &d); /* consumes mant */
    if (st == DEC_ERR_NOMEM) return DEC_PARSE_NOMEM;
    if (st != DEC_OK) return DEC_PARSE_EXPONENT_OVERFLOW;
    if (i64_abs_u64(d->scale) > (uint64_t)DEC_MAX_SCALE) {
        dec_free(d);
        return DEC_PARSE_EXPONENT_OVERFLOW;
    }
    *out = d;
    return DEC_PARSE_OK;
}
