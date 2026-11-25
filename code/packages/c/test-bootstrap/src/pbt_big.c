/*
 * pbt_big.c -- fixed-size sign-magnitude big integer (VER05i section 3).
 *
 * Everything is schoolbook arithmetic on 32-bit limbs, widened to uint64_t for
 * the carry.  A 32x32 product plus two 32-bit addends always fits in 64 bits:
 *
 *     (2^32-1)^2 + 2*(2^32-1) = 2^64 - 1
 *
 * so `acc = limb * m + carry + add` never wraps.
 */
#include "pbt_big.h"

#include <string.h>

/* ---- magnitude helpers ---------------------------------------------------- */

static void norm(pbt_big *a)
{
    while (a->n > 0 && a->d[a->n - 1] == 0) {
        a->n--;
    }
    if (a->n == 0) {
        a->neg = 0;
    }
}

void pbt_big_zero(pbt_big *a)
{
    memset(a, 0, sizeof *a);
}

void pbt_big_set_u64(pbt_big *a, uint64_t v)
{
    pbt_big_zero(a);
    a->d[0] = (uint32_t)(v & 0xffffffffu);
    a->d[1] = (uint32_t)(v >> 32);
    a->n = 2;
    norm(a);
}

void pbt_big_set_i64(pbt_big *a, int64_t v)
{
    /* Negate in unsigned arithmetic so INT64_MIN is well defined. */
    if (v < 0) {
        pbt_big_set_u64(a, (uint64_t)0 - (uint64_t)v);
        a->neg = 1;
        norm(a);
    } else {
        pbt_big_set_u64(a, (uint64_t)v);
    }
}

int pbt_big_is_zero(const pbt_big *a)
{
    return a->n == 0;
}

int pbt_big_sign(const pbt_big *a)
{
    if (a->n == 0) {
        return 0;
    }
    return a->neg ? -1 : 1;
}

int pbt_big_cmp_abs(const pbt_big *a, const pbt_big *b)
{
    uint32_t i;
    if (a->n != b->n) {
        return a->n < b->n ? -1 : 1;
    }
    for (i = a->n; i > 0; i--) {
        if (a->d[i - 1] != b->d[i - 1]) {
            return a->d[i - 1] < b->d[i - 1] ? -1 : 1;
        }
    }
    return 0;
}

int pbt_big_cmp(const pbt_big *a, const pbt_big *b)
{
    int sa = pbt_big_sign(a);
    int sb = pbt_big_sign(b);
    if (sa != sb) {
        return sa < sb ? -1 : 1;
    }
    if (sa == 0) {
        return 0;
    }
    /* Same non-zero sign: larger magnitude wins for positives, loses for negatives. */
    return sa > 0 ? pbt_big_cmp_abs(a, b) : pbt_big_cmp_abs(b, a);
}

uint32_t pbt_big_bitlen(const pbt_big *a)
{
    uint32_t top;
    uint32_t bits = 0;
    if (a->n == 0) {
        return 0;
    }
    top = a->d[a->n - 1];
    while (top != 0) {
        bits++;
        top >>= 1;
    }
    return (a->n - 1) * 32u + bits;
}

/* r = |a| + |b|; returns 1 on overflow.  Result is non-negative. */
static int mag_add(pbt_big *r, const pbt_big *a, const pbt_big *b)
{
    pbt_big t;
    uint32_t i;
    uint32_t n = a->n > b->n ? a->n : b->n;
    uint64_t carry = 0;
    pbt_big_zero(&t);
    for (i = 0; i < n; i++) {
        uint64_t s = carry;
        if (i < a->n) {
            s += a->d[i];
        }
        if (i < b->n) {
            s += b->d[i];
        }
        t.d[i] = (uint32_t)(s & 0xffffffffu);
        carry = s >> 32;
    }
    t.n = n;
    if (carry != 0) {
        if (n >= PBT_BIG_LIMBS) {
            pbt_big_zero(r);
            return 1;
        }
        t.d[n] = (uint32_t)carry;
        t.n = n + 1;
    }
    norm(&t);
    *r = t;
    return 0;
}

/* r = |a| - |b| where |a| >= |b|.  Result is non-negative. */
static void mag_sub(pbt_big *r, const pbt_big *a, const pbt_big *b)
{
    pbt_big t;
    uint32_t i;
    int64_t borrow = 0;
    pbt_big_zero(&t);
    for (i = 0; i < a->n; i++) {
        int64_t s = (int64_t)a->d[i] - borrow - (i < b->n ? (int64_t)b->d[i] : 0);
        if (s < 0) {
            s += (int64_t)1 << 32;
            borrow = 1;
        } else {
            borrow = 0;
        }
        t.d[i] = (uint32_t)s;
    }
    t.n = a->n;
    norm(&t);
    *r = t;
}

int pbt_big_add(pbt_big *out, const pbt_big *a, const pbt_big *b)
{
    pbt_big t;
    if (a->neg == b->neg) {
        int neg = a->neg;
        if (mag_add(&t, a, b)) {
            pbt_big_zero(out);
            return 1;
        }
        t.neg = neg;
        norm(&t);
    } else if (pbt_big_cmp_abs(a, b) >= 0) {
        int neg = a->neg;
        mag_sub(&t, a, b);
        t.neg = neg;
        norm(&t);
    } else {
        int neg = b->neg;
        mag_sub(&t, b, a);
        t.neg = neg;
        norm(&t);
    }
    *out = t;
    return 0;
}

void pbt_big_neg(pbt_big *out, const pbt_big *a)
{
    *out = *a;
    if (out->n != 0) {
        out->neg = !out->neg;
    }
}

void pbt_big_abs(pbt_big *out, const pbt_big *a)
{
    *out = *a;
    out->neg = 0;
}

int pbt_big_sub(pbt_big *out, const pbt_big *a, const pbt_big *b)
{
    pbt_big nb;
    pbt_big_neg(&nb, b);
    return pbt_big_add(out, a, &nb);
}

int pbt_big_mul_add_u32(pbt_big *out, const pbt_big *a, uint32_t m, uint32_t add)
{
    pbt_big t;
    uint32_t i;
    uint64_t carry = add;
    pbt_big_zero(&t);
    for (i = 0; i < a->n; i++) {
        uint64_t acc = (uint64_t)a->d[i] * m + carry;
        t.d[i] = (uint32_t)(acc & 0xffffffffu);
        carry = acc >> 32;
    }
    t.n = a->n;
    if (carry != 0) {
        if (a->n >= PBT_BIG_LIMBS) {
            pbt_big_zero(out);
            return 1;
        }
        t.d[a->n] = (uint32_t)carry;
        t.n = a->n + 1;
    }
    t.neg = a->neg;
    norm(&t);
    *out = t;
    return 0;
}

int pbt_big_shl(pbt_big *out, const pbt_big *a, uint32_t bits)
{
    pbt_big t;
    uint32_t limbs = bits / 32u;
    uint32_t rem = bits % 32u;
    uint32_t i;
    if (a->n == 0) {
        pbt_big_zero(out);
        return 0;
    }
    /* Check the final width first, so no index below can leave the array. */
    if (bits > PBT_BIG_LIMBS * 32u || pbt_big_bitlen(a) + bits > PBT_BIG_LIMBS * 32u) {
        pbt_big_zero(out);
        return 1;
    }
    pbt_big_zero(&t);
    for (i = 0; i < a->n; i++) {
        uint64_t v = (uint64_t)a->d[i] << rem;
        t.d[i + limbs] |= (uint32_t)(v & 0xffffffffu);
        if (i + limbs + 1 < PBT_BIG_LIMBS) {
            t.d[i + limbs + 1] |= (uint32_t)(v >> 32);
        }
    }
    t.n = a->n + limbs + 1;
    if (t.n > PBT_BIG_LIMBS) {
        t.n = PBT_BIG_LIMBS;
    }
    t.neg = a->neg;
    norm(&t);
    *out = t;
    return 0;
}

void pbt_big_floor_half(pbt_big *out, const pbt_big *a)
{
    pbt_big t = *a;
    uint32_t i;
    int odd = (a->n > 0) && (a->d[0] & 1u);
    for (i = 0; i < t.n; i++) {
        uint32_t hi = (i + 1 < t.n) ? t.d[i + 1] : 0;
        t.d[i] = (t.d[i] >> 1) | (hi << 31);
    }
    norm(&t);
    /* floor of a negative odd value rounds away from zero: -3/2 -> -2. */
    if (a->neg && odd) {
        pbt_big one;
        pbt_big_set_u64(&one, 1);
        (void)mag_add(&t, &t, &one); /* shrank by half: cannot overflow */
        t.neg = 1;
    }
    norm(&t);
    *out = t;
}

int pbt_big_pow2(pbt_big *out, uint32_t bits)
{
    pbt_big one;
    pbt_big_set_u64(&one, 1);
    return pbt_big_shl(out, &one, bits);
}

int pbt_big_divmod_abs(pbt_big *q, pbt_big *r, const pbt_big *a, const pbt_big *b)
{
    pbt_big quo;
    pbt_big rem;
    pbt_big bb;
    uint32_t bits;
    uint32_t i;
    pbt_big_zero(&quo);
    pbt_big_zero(&rem);
    if (b->n == 0) {
        if (q != NULL) {
            pbt_big_zero(q);
        }
        if (r != NULL) {
            pbt_big_zero(r);
        }
        return 1;
    }
    pbt_big_abs(&bb, b);
    bits = pbt_big_bitlen(a);
    for (i = bits; i > 0; i--) {
        uint32_t bit = i - 1;
        uint32_t limb = bit / 32u;
        /* rem = rem * 2 + bit.  rem < b <= 2^1000 so this cannot overflow. */
        (void)pbt_big_mul_add_u32(&rem, &rem, 2, (a->d[limb] >> (bit % 32u)) & 1u);
        if (pbt_big_cmp_abs(&rem, &bb) >= 0) {
            mag_sub(&rem, &rem, &bb);
            quo.d[limb] |= (uint32_t)1 << (bit % 32u);
            if (quo.n < limb + 1) {
                quo.n = limb + 1;
            }
        }
    }
    norm(&quo);
    if (q != NULL) {
        *q = quo;
    }
    if (r != NULL) {
        *r = rem;
    }
    return 0;
}

int pbt_big_fits_u64(const pbt_big *a)
{
    return a->n <= 2;
}

int pbt_big_fits_i64(const pbt_big *a)
{
    uint64_t m;
    if (a->n > 2) {
        return 0;
    }
    m = pbt_big_low_u64(a);
    if (a->neg) {
        return m <= (uint64_t)1 << 63;
    }
    return m < (uint64_t)1 << 63;
}

uint64_t pbt_big_low_u64(const pbt_big *a)
{
    uint64_t lo = a->n > 0 ? a->d[0] : 0;
    uint64_t hi = a->n > 1 ? a->d[1] : 0;
    return lo | (hi << 32);
}

uint64_t pbt_big_mod_u64(const pbt_big *a)
{
    uint64_t m = pbt_big_low_u64(a);
    return a->neg ? (uint64_t)0 - m : m;
}

int64_t pbt_big_to_i64(const pbt_big *a)
{
    uint64_t m = pbt_big_low_u64(a);
    if (a->neg) {
        if (m == (uint64_t)1 << 63) {
            return INT64_MIN;
        }
        return -(int64_t)m;
    }
    return (int64_t)m;
}

int pbt_big_parse_dec(pbt_big *out, const char *s, size_t len)
{
    pbt_big t;
    size_t i = 0;
    int neg = 0;
    pbt_big_zero(&t);
    if (len > 0 && s[0] == '-') {
        neg = 1;
        i = 1;
    }
    if (i >= len || len - i > PBT_BIG_MAX_DIGITS) {
        pbt_big_zero(out);
        return 1;
    }
    for (; i < len; i++) {
        if (s[i] < '0' || s[i] > '9') {
            pbt_big_zero(out);
            return 1;
        }
        if (pbt_big_mul_add_u32(&t, &t, 10, (uint32_t)(s[i] - '0'))) {
            pbt_big_zero(out);
            return 1;
        }
    }
    t.neg = neg;
    norm(&t);
    if (pbt_big_bitlen(&t) > PBT_BIG_MAX_BITS) {
        pbt_big_zero(out);
        return 1;
    }
    *out = t;
    return 0;
}

size_t pbt_big_to_dec(const pbt_big *a, char *buf, size_t cap)
{
    /* Repeated division of the magnitude by 10^9, collecting 9-digit groups. */
    uint32_t work[PBT_BIG_LIMBS];
    uint32_t n = a->n;
    uint32_t groups[PBT_BIG_LIMBS * 2];
    size_t ng = 0;
    size_t len = 0;
    size_t i;
    char tmp[PBT_BIG_DEC_BUF];
    size_t tl = 0;
    memcpy(work, a->d, sizeof work);
    if (n == 0) {
        if (cap < 2) {
            return 0;
        }
        buf[0] = '0';
        buf[1] = '\0';
        return 1;
    }
    while (n > 0) {
        uint64_t rem = 0;
        uint32_t k;
        for (k = n; k > 0; k--) {
            uint64_t cur = (rem << 32) | work[k - 1];
            work[k - 1] = (uint32_t)(cur / 1000000000u);
            rem = cur % 1000000000u;
        }
        while (n > 0 && work[n - 1] == 0) {
            n--;
        }
        groups[ng++] = (uint32_t)rem;
    }
    if (a->neg) {
        tmp[tl++] = '-';
    }
    for (i = ng; i > 0; i--) {
        uint32_t g = groups[i - 1];
        char digits[9];
        int k = 9;
        int started = (i != ng); /* only the most significant group drops leading zeros */
        while (k > 0) {
            digits[--k] = (char)('0' + g % 10);
            g /= 10;
        }
        for (k = 0; k < 9; k++) {
            if (!started && digits[k] == '0') {
                continue;
            }
            started = 1;
            tmp[tl++] = digits[k];
        }
    }
    len = tl;
    if (len + 1 > cap) {
        return 0;
    }
    memcpy(buf, tmp, len);
    buf[len] = '\0';
    return len;
}
