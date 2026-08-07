/*
 * bignum_core.c — implementation of BigInteger (see bignum_core.h). A faithful
 * port of the Rust `bignum-core` crate's integer core: sign-magnitude with
 * little-endian base-2^32 limbs, schoolbook add/sub/mul, and Knuth Algorithm D
 * long division. All arithmetic uses 32-bit limbs and a 64-bit accumulator.
 */
#include "bignum_core.h"

#include <stdlib.h> /* malloc, calloc, free */
#include <string.h> /* memcpy */

struct BigInteger {
    int sign;      /* -1, 0, +1; 0 iff len == 0 */
    uint32_t *mag; /* little-endian base-2^32 limbs, no trailing zero; NULL if 0 */
    size_t len;
};

/* ---- tiny helpers ----------------------------------------------------- */

static unsigned clz32(uint32_t x) {
    unsigned n = 0;
    if (x == 0) {
        return 32;
    }
    while (!(x & 0x80000000u)) {
        x <<= 1;
        n++;
    }
    return n;
}

/* Trailing-zero-trimmed length of the first `n` limbs of `m`. */
static size_t mag_trim(const uint32_t *m, size_t n) {
    while (n > 0 && m[n - 1] == 0) {
        n--;
    }
    return n;
}

/* Duplicate `n` limbs; returns NULL for n==0 (a valid empty magnitude) or on
 * OOM — the caller distinguishes via a separate ok flag where it matters. */
static uint32_t *dup_arr(const uint32_t *m, size_t n) {
    uint32_t *p;
    if (n == 0) {
        return NULL;
    }
    p = malloc(n * sizeof *p);
    if (p) {
        memcpy(p, m, n * sizeof *p);
    }
    return p;
}

/* ---- magnitude primitives (operate on normalized little-endian slices) -
 * Each writes a freshly malloc'd, normalized result to (*out, *outlen) and
 * returns 1, or returns 0 on OOM (with *out=NULL, *outlen=0). An empty result
 * is (*out=NULL, *outlen=0) with return 1. */

static int mag_cmp(const uint32_t *a, size_t an, const uint32_t *b, size_t bn) {
    size_t i;
    if (an != bn) {
        return an < bn ? -1 : 1;
    }
    for (i = an; i > 0; i--) {
        if (a[i - 1] != b[i - 1]) {
            return a[i - 1] < b[i - 1] ? -1 : 1;
        }
    }
    return 0;
}

static int mag_add(const uint32_t *a, size_t an, const uint32_t *b, size_t bn,
                   uint32_t **out, size_t *outlen) {
    const uint32_t *lo;
    const uint32_t *sh;
    size_t ln, sn, i;
    uint32_t *r;
    uint64_t carry = 0;
    if (an >= bn) {
        lo = a; ln = an; sh = b; sn = bn;
    } else {
        lo = b; ln = bn; sh = a; sn = an;
    }
    r = malloc((ln + 1) * sizeof *r);
    if (!r) {
        *out = NULL; *outlen = 0; return 0;
    }
    for (i = 0; i < ln; i++) {
        uint64_t sum = (uint64_t)lo[i] + carry + (i < sn ? sh[i] : 0);
        r[i] = (uint32_t)sum;
        carry = sum >> 32;
    }
    r[ln] = (uint32_t)carry;
    *outlen = mag_trim(r, ln + 1);
    *out = r;
    return 1;
}

/* Requires a >= b (as magnitudes). */
static int mag_sub(const uint32_t *a, size_t an, const uint32_t *b, size_t bn,
                   uint32_t **out, size_t *outlen) {
    uint32_t *r = malloc(an ? an * sizeof *r : 1);
    int64_t borrow = 0;
    size_t i;
    if (!r) {
        *out = NULL; *outlen = 0; return 0;
    }
    for (i = 0; i < an; i++) {
        int64_t diff = (int64_t)a[i] - (i < bn ? (int64_t)b[i] : 0) - borrow;
        if (diff < 0) {
            diff += (int64_t)1 << 32;
            borrow = 1;
        } else {
            borrow = 0;
        }
        r[i] = (uint32_t)diff;
    }
    *outlen = mag_trim(r, an);
    *out = r;
    return 1;
}

static int mag_mul(const uint32_t *a, size_t an, const uint32_t *b, size_t bn,
                   uint32_t **out, size_t *outlen) {
    uint32_t *r;
    size_t i, j;
    if (an == 0 || bn == 0) {
        *out = NULL; *outlen = 0; return 1;
    }
    r = calloc(an + bn, sizeof *r); /* checked multiply, zero-filled */
    if (!r) {
        *out = NULL; *outlen = 0; return 0;
    }
    for (i = 0; i < an; i++) {
        uint64_t ai = a[i];
        uint64_t carry = 0;
        for (j = 0; j < bn; j++) {
            uint64_t cur = (uint64_t)r[i + j] + ai * b[j] + carry;
            r[i + j] = (uint32_t)cur;
            carry = cur >> 32;
        }
        r[i + bn] = (uint32_t)((uint64_t)r[i + bn] + carry);
    }
    *outlen = mag_trim(r, an + bn);
    *out = r;
    return 1;
}

static int mag_mul_small(const uint32_t *a, size_t an, uint32_t factor,
                         uint32_t **out, size_t *outlen) {
    uint32_t *r;
    uint64_t carry = 0, f = factor;
    size_t i;
    if (factor == 0 || an == 0) {
        *out = NULL; *outlen = 0; return 1;
    }
    r = malloc((an + 1) * sizeof *r);
    if (!r) {
        *out = NULL; *outlen = 0; return 0;
    }
    for (i = 0; i < an; i++) {
        uint64_t cur = (uint64_t)a[i] * f + carry;
        r[i] = (uint32_t)cur;
        carry = cur >> 32;
    }
    r[an] = (uint32_t)carry;
    *outlen = mag_trim(r, an + 1);
    *out = r;
    return 1;
}

static int mag_add_small(const uint32_t *a, size_t an, uint32_t addend,
                         uint32_t **out, size_t *outlen) {
    uint32_t *r = malloc((an + 1) * sizeof *r);
    uint64_t carry = addend;
    size_t i;
    if (!r) {
        *out = NULL; *outlen = 0; return 0;
    }
    for (i = 0; i < an; i++) {
        uint64_t cur = (uint64_t)a[i] + carry;
        r[i] = (uint32_t)cur;
        carry = cur >> 32;
    }
    r[an] = (uint32_t)carry;
    *outlen = mag_trim(r, an + 1);
    *out = r;
    return 1;
}

/* q = mag / divisor, *rem = mag % divisor. */
static int mag_divmod_small(const uint32_t *a, size_t an, uint32_t divisor,
                            uint32_t **q_out, size_t *q_len, uint32_t *rem) {
    uint32_t *q = malloc(an ? an * sizeof *q : 1);
    uint64_t d = divisor, r = 0;
    size_t i;
    if (!q) {
        *q_out = NULL; *q_len = 0; return 0;
    }
    for (i = an; i > 0; i--) {
        uint64_t cur = (r << 32) | a[i - 1];
        q[i - 1] = (uint32_t)(cur / d);
        r = cur % d;
    }
    *q_len = mag_trim(q, an);
    *q_out = q;
    *rem = (uint32_t)r;
    return 1;
}

/* result = a << bits (0..31); may grow by one limb. len is the raw output
 * length (not trimmed) written to *outlen; buffer sized an+1. */
static int shl_small(const uint32_t *a, size_t an, unsigned bits, uint32_t **out,
                     size_t *outlen) {
    uint32_t *r;
    uint32_t carry = 0;
    size_t i;
    if (bits == 0) {
        r = malloc(an ? an * sizeof *r : 1);
        if (!r) { *out = NULL; *outlen = 0; return 0; }
        if (an) memcpy(r, a, an * sizeof *r);
        *out = r; *outlen = an; return 1;
    }
    r = malloc((an + 1) * sizeof *r);
    if (!r) { *out = NULL; *outlen = 0; return 0; }
    for (i = 0; i < an; i++) {
        uint64_t v = ((uint64_t)a[i] << bits) | carry;
        r[i] = (uint32_t)v;
        carry = (uint32_t)(v >> 32);
    }
    r[an] = carry;
    *out = r; *outlen = an + 1; return 1;
}

static int shr_small(const uint32_t *a, size_t an, unsigned bits, uint32_t **out,
                     size_t *outlen) {
    uint32_t *r;
    uint32_t carry = 0;
    size_t i;
    if (bits == 0) {
        r = dup_arr(a, an);
        if (an && !r) { *out = NULL; *outlen = 0; return 0; }
        *out = r; *outlen = mag_trim(r, an); return 1;
    }
    r = malloc(an ? an * sizeof *r : 1);
    if (!r) { *out = NULL; *outlen = 0; return 0; }
    for (i = an; i > 0; i--) {
        uint32_t cur = a[i - 1];
        r[i - 1] = (cur >> bits) | carry;
        carry = cur << (32 - bits);
    }
    *out = r; *outlen = mag_trim(r, an); return 1;
}

/* Knuth Algorithm D. v must be normalized, non-empty. Writes q and r. */
static int mag_divmod(const uint32_t *u_in, size_t un, const uint32_t *v_in,
                      size_t vn, uint32_t **q_out, size_t *q_len,
                      uint32_t **r_out, size_t *r_len) {
    size_t n = vn, m, j;
    unsigned shift;
    uint64_t base = (uint64_t)1 << 32;
    uint32_t *v = NULL, *u = NULL, *q = NULL;
    size_t vlen, ulen;

    if (mag_cmp(u_in, mag_trim(u_in, un), v_in, vn) < 0) {
        /* u < v: quotient 0, remainder u. */
        *q_out = NULL; *q_len = 0;
        *r_out = dup_arr(u_in, mag_trim(u_in, un));
        *r_len = mag_trim(u_in, un);
        if (*r_len && !*r_out) { return 0; }
        return 1;
    }
    un = mag_trim(u_in, un);

    if (n == 1) {
        uint32_t rem;
        if (!mag_divmod_small(u_in, un, v_in[0], q_out, q_len, &rem)) {
            return 0;
        }
        *r_out = NULL; *r_len = 0;
        if (rem != 0) {
            *r_out = malloc(sizeof(uint32_t));
            if (!*r_out) { free(*q_out); *q_out = NULL; *q_len = 0; return 0; }
            (*r_out)[0] = rem;
            *r_len = 1;
        }
        return 1;
    }

    /* D1: normalize so v's top limb has its high bit set. */
    shift = clz32(v_in[n - 1]);
    if (!shl_small(v_in, vn, shift, &v, &vlen)) { return 0; }
    /* v stays length n after shift (its top limb's high bit was clear). */
    (void)vlen;
    if (!shl_small(u_in, un, shift, &u, &ulen)) { free(v); return 0; }
    /* Working dividend needs exactly un+1 limbs (one guard limb). */
    {
        uint32_t *u2 = malloc((un + 1) * sizeof *u2);
        if (!u2) { free(v); free(u); return 0; }
        memcpy(u2, u, ulen * sizeof *u2);
        {
            size_t k;
            for (k = ulen; k < un + 1; k++) {
                u2[k] = 0;
            }
        }
        free(u);
        u = u2;
    }
    m = un - n;

    q = calloc(m + 1, sizeof *q);
    if (!q) { free(v); free(u); return 0; }

    for (j = m + 1; j > 0; j--) {
        size_t jj = j - 1;
        uint64_t dividend = ((uint64_t)u[jj + n] << 32) | u[jj + n - 1];
        uint64_t qhat = dividend / v[n - 1];
        uint64_t rhat = dividend % v[n - 1];
        int64_t k;
        int64_t t;
        size_t i;
        for (;;) {
            if (qhat >= base ||
                qhat * v[n - 2] > (rhat << 32) + u[jj + n - 2]) {
                qhat -= 1;
                rhat += v[n - 1];
                if (rhat < base) {
                    continue;
                }
            }
            break;
        }
        /* D4: multiply-and-subtract. */
        k = 0;
        for (i = 0; i < n; i++) {
            uint64_t p = qhat * v[i];
            int64_t tt = (int64_t)u[jj + i] - k - (int64_t)(uint32_t)p;
            u[jj + i] = (uint32_t)tt;
            k = (int64_t)(p >> 32) - (tt >> 32);
        }
        t = (int64_t)u[jj + n] - k;
        u[jj + n] = (uint32_t)t;
        if (t < 0) {
            /* D5/D6: qhat was one too large; add v back. */
            uint64_t carry = 0;
            qhat -= 1;
            for (i = 0; i < n; i++) {
                uint64_t sum = (uint64_t)u[jj + i] + v[i] + carry;
                u[jj + i] = (uint32_t)sum;
                carry = sum >> 32;
            }
            u[jj + n] = (uint32_t)((uint64_t)u[jj + n] + carry);
        }
        q[jj] = (uint32_t)qhat;
    }

    *q_len = mag_trim(q, m + 1);
    *q_out = q;
    /* D8: remainder is the low n limbs of u, shifted right by `shift`. */
    if (!shr_small(u, n, shift, r_out, r_len)) {
        free(q); *q_out = NULL; *q_len = 0; free(v); free(u); return 0;
    }
    free(v);
    free(u);
    return 1;
}

/* ---- BigInteger construction ------------------------------------------ */

/* Take ownership of `mag` (length `len`, already trimmed); build a BigInteger.
 * On len==0 or OOM, frees mag. Returns NULL on OOM. */
static BigInteger *bi_make(int sign, uint32_t *mag, size_t len) {
    BigInteger *b;
    if (len == 0) {
        free(mag);
        b = malloc(sizeof *b);
        if (!b) { return NULL; }
        b->sign = 0; b->mag = NULL; b->len = 0;
        return b;
    }
    b = malloc(sizeof *b);
    if (!b) { free(mag); return NULL; }
    b->sign = sign; b->mag = mag; b->len = len;
    return b;
}

BigInteger *bigint_zero(void) { return bi_make(0, NULL, 0); }

static BigInteger *from_u64_signed(uint64_t value, int negative) {
    uint32_t tmp[2];
    size_t len = 0;
    uint64_t x = value;
    while (x != 0) {
        tmp[len++] = (uint32_t)x;
        x >>= 32;
    }
    if (len == 0) {
        return bigint_zero();
    }
    {
        uint32_t *mag = dup_arr(tmp, len);
        if (!mag) { return NULL; }
        return bi_make(negative ? -1 : 1, mag, len);
    }
}

BigInteger *bigint_from_u64(uint64_t value) {
    return from_u64_signed(value, 0);
}

BigInteger *bigint_from_i64(int64_t value) {
    if (value < 0) {
        /* Negate into u64 without overflow on INT64_MIN. */
        uint64_t mag = (uint64_t)(-(value + 1)) + 1;
        return from_u64_signed(mag, 1);
    }
    return from_u64_signed((uint64_t)value, 0);
}

BigInteger *bigint_one(void) { return bigint_from_u64(1); }

BigInteger *bigint_clone(const BigInteger *a) {
    uint32_t *mag;
    if (!a) { return NULL; }
    mag = dup_arr(a->mag, a->len);
    if (a->len && !mag) { return NULL; }
    return bi_make(a->sign, mag, a->len);
}

void bigint_free(BigInteger *a) {
    if (a) {
        free(a->mag);
        free(a);
    }
}

/* ---- queries ---------------------------------------------------------- */

int bigint_is_zero(const BigInteger *a) { return a->sign == 0; }
int bigint_is_negative(const BigInteger *a) { return a->sign < 0; }
int bigint_is_positive(const BigInteger *a) { return a->sign > 0; }
int bigint_signum(const BigInteger *a) { return a->sign; }
size_t bigint_num_limbs(const BigInteger *a) { return a->len; }

uint64_t bigint_bit_len(const BigInteger *a) {
    if (a->len == 0) {
        return 0;
    }
    return (uint64_t)(a->len - 1) * 32 + (32 - clz32(a->mag[a->len - 1]));
}

int bigint_cmp(const BigInteger *a, const BigInteger *b) {
    if (a->sign != b->sign) {
        return a->sign < b->sign ? -1 : 1;
    }
    if (a->sign == 0) {
        return 0;
    }
    if (a->sign > 0) {
        return mag_cmp(a->mag, a->len, b->mag, b->len);
    }
    /* both negative: larger magnitude is the smaller value. */
    return mag_cmp(b->mag, b->len, a->mag, a->len);
}

/* ---- sign transforms -------------------------------------------------- */

BigInteger *bigint_abs(const BigInteger *a) {
    uint32_t *mag = dup_arr(a->mag, a->len);
    if (a->len && !mag) { return NULL; }
    return bi_make(a->sign == 0 ? 0 : 1, mag, a->len);
}

BigInteger *bigint_neg(const BigInteger *a) {
    uint32_t *mag = dup_arr(a->mag, a->len);
    if (a->len && !mag) { return NULL; }
    return bi_make(-a->sign, mag, a->len);
}

/* ---- arithmetic ------------------------------------------------------- */

BigInteger *bigint_add(const BigInteger *a, const BigInteger *b) {
    uint32_t *r;
    size_t rn;
    if (a->sign == 0) { return bigint_clone(b); }
    if (b->sign == 0) { return bigint_clone(a); }
    if (a->sign == b->sign) {
        if (!mag_add(a->mag, a->len, b->mag, b->len, &r, &rn)) { return NULL; }
        return bi_make(a->sign, r, rn);
    }
    /* opposite signs: subtract the smaller magnitude from the larger. */
    {
        int c = mag_cmp(a->mag, a->len, b->mag, b->len);
        if (c == 0) { return bigint_zero(); }
        if (c > 0) {
            if (!mag_sub(a->mag, a->len, b->mag, b->len, &r, &rn)) { return NULL; }
            return bi_make(a->sign, r, rn);
        }
        if (!mag_sub(b->mag, b->len, a->mag, a->len, &r, &rn)) { return NULL; }
        return bi_make(b->sign, r, rn);
    }
}

BigInteger *bigint_sub(const BigInteger *a, const BigInteger *b) {
    BigInteger *nb = bigint_neg(b);
    BigInteger *res;
    if (!nb) { return NULL; }
    res = bigint_add(a, nb);
    bigint_free(nb);
    return res;
}

BigInteger *bigint_mul(const BigInteger *a, const BigInteger *b) {
    uint32_t *r;
    size_t rn;
    if (a->sign == 0 || b->sign == 0) { return bigint_zero(); }
    if (!mag_mul(a->mag, a->len, b->mag, b->len, &r, &rn)) { return NULL; }
    return bi_make(a->sign == b->sign ? 1 : -1, r, rn);
}

BigIntStatus bigint_div_rem(const BigInteger *a, const BigInteger *b,
                            BigInteger **q_out, BigInteger **r_out) {
    uint32_t *qm, *rm;
    size_t qn, rn;
    BigInteger *q, *r;
    if (b->sign == 0) { return BIGINT_DIV_BY_ZERO; }
    if (a->sign == 0) {
        q = bigint_zero();
        r = bigint_zero();
        if (!q || !r) { bigint_free(q); bigint_free(r); return BIGINT_ALLOC_ERROR; }
        if (q_out) { *q_out = q; } else { bigint_free(q); }
        if (r_out) { *r_out = r; } else { bigint_free(r); }
        return BIGINT_OK;
    }
    if (!mag_divmod(a->mag, a->len, b->mag, b->len, &qm, &qn, &rm, &rn)) {
        return BIGINT_ALLOC_ERROR;
    }
    q = bi_make(a->sign == b->sign ? 1 : -1, qm, qn);
    r = bi_make(a->sign, rm, rn);
    if (!q || !r) { bigint_free(q); bigint_free(r); return BIGINT_ALLOC_ERROR; }
    if (q_out) { *q_out = q; } else { bigint_free(q); }
    if (r_out) { *r_out = r; } else { bigint_free(r); }
    return BIGINT_OK;
}

BigIntStatus bigint_div(const BigInteger *a, const BigInteger *b,
                        BigInteger **out) {
    return bigint_div_rem(a, b, out, NULL);
}

BigIntStatus bigint_rem(const BigInteger *a, const BigInteger *b,
                        BigInteger **out) {
    return bigint_div_rem(a, b, NULL, out);
}

BigInteger *bigint_pow(const BigInteger *a, uint32_t exp) {
    BigInteger *result = bigint_one();
    BigInteger *base = bigint_clone(a);
    uint32_t e = exp;
    if (!result || !base) { bigint_free(result); bigint_free(base); return NULL; }
    while (e > 0) {
        if (e & 1u) {
            BigInteger *nr = bigint_mul(result, base);
            bigint_free(result);
            if (!nr) { bigint_free(base); return NULL; }
            result = nr;
        }
        e >>= 1;
        if (e > 0) {
            BigInteger *nb = bigint_mul(base, base);
            bigint_free(base);
            if (!nb) { bigint_free(result); return NULL; }
            base = nb;
        }
    }
    bigint_free(base);
    return result;
}

BigIntStatus bigint_try_pow(const BigInteger *a, uint32_t exp, uint64_t max_bits,
                            BigInteger **out, uint64_t *projected_out) {
    uint64_t projected;
    uint64_t bl = bigint_bit_len(a);
    if (exp == 0 || a->sign == 0 || bl <= 1) {
        projected = 1;
    } else {
        /* saturating multiply bl * exp. */
        uint64_t e = exp;
        if (bl > (uint64_t)-1 / e) {
            projected = (uint64_t)-1;
        } else {
            projected = bl * e;
        }
    }
    if (projected_out) { *projected_out = projected; }
    if (projected > max_bits) {
        return BIGINT_POW_TOO_LARGE;
    }
    {
        BigInteger *r = bigint_pow(a, exp);
        if (!r) { return BIGINT_ALLOC_ERROR; }
        *out = r;
        return BIGINT_OK;
    }
}

BigInteger *bigint_gcd(const BigInteger *a, const BigInteger *b) {
    BigInteger *x = bigint_abs(a);
    BigInteger *y = bigint_abs(b);
    if (!x || !y) { bigint_free(x); bigint_free(y); return NULL; }
    while (y->sign != 0) {
        BigInteger *r = NULL;
        if (bigint_div_rem(x, y, NULL, &r) != BIGINT_OK) {
            bigint_free(x); bigint_free(y); return NULL;
        }
        bigint_free(x);
        x = y;
        y = r;
    }
    bigint_free(y);
    return x;
}

/* ---- parsing / formatting --------------------------------------------- */

static int digit_value(char c, uint32_t radix, uint32_t *out) {
    uint32_t d;
    if (c >= '0' && c <= '9') {
        d = (uint32_t)(c - '0');
    } else if (c >= 'a' && c <= 'z') {
        d = (uint32_t)(c - 'a') + 10;
    } else if (c >= 'A' && c <= 'Z') {
        d = (uint32_t)(c - 'A') + 10;
    } else {
        return 0;
    }
    if (d >= radix) {
        return 0;
    }
    *out = d;
    return 1;
}

BigIntStatus bigint_parse_radix(const char *s, uint32_t radix, BigInteger **out,
                                char *bad_char_out) {
    size_t n, i, start = 0;
    int negative = 0;
    uint32_t *mag = NULL;
    size_t maglen = 0;
    if (radix < 2 || radix > 36) {
        return BIGINT_PARSE_INVALID_RADIX;
    }
    n = 0;
    while (s[n] != '\0') { n++; }
    if (n == 0) {
        return BIGINT_PARSE_EMPTY;
    }
    if (s[0] == '+') {
        start = 1;
    } else if (s[0] == '-') {
        negative = 1;
        start = 1;
    }
    if (start >= n) {
        return BIGINT_PARSE_EMPTY;
    }
    for (i = start; i < n; i++) {
        uint32_t digit;
        uint32_t *t1, *t2;
        size_t l1, l2;
        if (!digit_value(s[i], radix, &digit)) {
            if (bad_char_out) { *bad_char_out = s[i]; }
            free(mag);
            return BIGINT_PARSE_INVALID_DIGIT;
        }
        if (!mag_mul_small(mag, maglen, radix, &t1, &l1)) {
            free(mag); return BIGINT_ALLOC_ERROR;
        }
        free(mag);
        if (!mag_add_small(t1, l1, digit, &t2, &l2)) {
            free(t1); return BIGINT_ALLOC_ERROR;
        }
        free(t1);
        mag = t2;
        maglen = l2;
    }
    maglen = mag_trim(mag, maglen);
    {
        BigInteger *b = bi_make(maglen == 0 ? 0 : (negative ? -1 : 1), mag, maglen);
        if (!b) { return BIGINT_ALLOC_ERROR; }
        *out = b;
        return BIGINT_OK;
    }
}

char *bigint_to_str_radix(const BigInteger *a, uint32_t radix) {
    static const char DIGITS[] = "0123456789abcdefghijklmnopqrstuvwxyz";
    uint32_t *mag;
    size_t maglen;
    char *digits;
    size_t ndigits = 0, cap;
    char *out;
    size_t oi = 0, k;
    if (radix < 2 || radix > 36) {
        return NULL;
    }
    if (a->sign == 0) {
        out = malloc(2);
        if (out) { out[0] = '0'; out[1] = '\0'; }
        return out;
    }
    mag = dup_arr(a->mag, a->len);
    if (!mag) { return NULL; }
    maglen = a->len;
    /* Upper bound on digit count: bit_len / log2(radix) + 1 <= bit_len + 1.
     * Compute in 64 bits and guard before narrowing to size_t so a > 2^32-bit
     * value cannot truncate the cap (and overflow the digit buffer) on a 32-bit
     * platform. */
    {
        uint64_t need = bigint_bit_len(a) + 2;
        if (need > (uint64_t)(size_t)-1) {
            free(mag);
            return NULL;
        }
        cap = (size_t)need;
    }
    digits = malloc(cap);
    if (!digits) { free(mag); return NULL; }
    while (maglen != 0) {
        uint32_t rem;
        uint32_t *q;
        size_t qlen;
        if (!mag_divmod_small(mag, maglen, radix, &q, &qlen, &rem)) {
            free(mag); free(digits); return NULL;
        }
        free(mag);
        mag = q;
        maglen = qlen;
        digits[ndigits++] = DIGITS[rem];
    }
    free(mag);
    out = malloc(ndigits + 2);
    if (!out) { free(digits); return NULL; }
    if (a->sign < 0) { out[oi++] = '-'; }
    for (k = ndigits; k > 0; k--) {
        out[oi++] = digits[k - 1];
    }
    out[oi] = '\0';
    free(digits);
    return out;
}

char *bigint_to_string(const BigInteger *a) {
    return bigint_to_str_radix(a, 10);
}
