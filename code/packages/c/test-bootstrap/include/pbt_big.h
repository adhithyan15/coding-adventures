/*
 * pbt_big.h -- a small fixed-size big integer (VER05i section 3).
 * ===========================================================================
 *
 * WHY THIS EXISTS
 * ---------------
 * The pbt-v1 corpus draws signed 128-bit integers, and the unbiased sampler for
 * a span wider than 2^64 needs 2^(64k).  Pure ISO C has no 128-bit integer
 * (`unsigned __int128` is a compiler extension and -pedantic-errors rejects
 * it), so the engine carries its own.
 *
 * REPRESENTATION
 * --------------
 * Sign and magnitude.  The magnitude is a little-endian array of 32-bit limbs
 * (limb 0 is the least significant):
 *
 *     value = (-1)^neg * ( d[0] + d[1]*2^32 + d[2]*2^64 + ... )
 *
 * The struct is NORMALISED after every operation: there is no leading zero limb
 * (`d[n-1] != 0`), and zero is `n == 0, neg == 0` (there is no negative zero).
 * Because the limbs are an inline array, a `pbt_big` is a plain value:
 * assignment copies it, nothing is allocated, nothing is freed.
 *
 * CAPS
 * ----
 *   PBT_BIG_LIMBS      36 limbs = 1152 bits of storage
 *   PBT_BIG_MAX_BITS   1000    the widest value callers/fixtures may create
 *   PBT_BIG_MAX_DIGITS 300     the longest decimal token accepted
 *
 * Every arithmetic function returns 0 on success and nonzero if the result did
 * not fit; on overflow the output is set to zero, never to a wrapped value.
 * Outputs may alias inputs.
 */
#ifndef PBT_BIG_H
#define PBT_BIG_H

#include <stddef.h>
#include <stdint.h>

#define PBT_BIG_LIMBS 36
#define PBT_BIG_MAX_BITS 1000
#define PBT_BIG_MAX_DIGITS 300
/* Room for the longest decimal rendering plus sign and NUL. */
#define PBT_BIG_DEC_BUF (PBT_BIG_LIMBS * 10 + 3)

typedef struct pbt_big {
    int neg;
    uint32_t n;
    uint32_t d[PBT_BIG_LIMBS];
} pbt_big;

void pbt_big_zero(pbt_big *a);
void pbt_big_set_u64(pbt_big *a, uint64_t v);
void pbt_big_set_i64(pbt_big *a, int64_t v);

int pbt_big_is_zero(const pbt_big *a);
/* -1, 0 or +1. */
int pbt_big_sign(const pbt_big *a);
/* Three-way comparisons: negative, zero or positive. */
int pbt_big_cmp(const pbt_big *a, const pbt_big *b);
int pbt_big_cmp_abs(const pbt_big *a, const pbt_big *b);
/* Number of significant bits of the magnitude (0 for zero). */
uint32_t pbt_big_bitlen(const pbt_big *a);

/* out = a + b, a - b, -a, |a|. */
int pbt_big_add(pbt_big *out, const pbt_big *a, const pbt_big *b);
int pbt_big_sub(pbt_big *out, const pbt_big *a, const pbt_big *b);
void pbt_big_neg(pbt_big *out, const pbt_big *a);
void pbt_big_abs(pbt_big *out, const pbt_big *a);

/* out = a * m + add, with 32-bit m and add (magnitude arithmetic on |a|, sign of a kept). */
int pbt_big_mul_add_u32(pbt_big *out, const pbt_big *a, uint32_t m, uint32_t add);
/* out = a << bits (sign kept). */
int pbt_big_shl(pbt_big *out, const pbt_big *a, uint32_t bits);
/* out = floor(a / 2): -3 -> -2, 3 -> 1, -1 -> -1. */
void pbt_big_floor_half(pbt_big *out, const pbt_big *a);
/* out = 2^bits. */
int pbt_big_pow2(pbt_big *out, uint32_t bits);
/*
 * Magnitude division: q = |a| / |b|, r = |a| % |b| (both non-negative); b must
 * be non-zero (otherwise both outputs are zero and 1 is returned).  Either
 * output may be NULL.  Schoolbook shift-subtract: O(bits^2 / 32).
 */
int pbt_big_divmod_abs(pbt_big *q, pbt_big *r, const pbt_big *a, const pbt_big *b);

/* Magnitude fits in 64 bits / value fits in an int64_t. */
int pbt_big_fits_u64(const pbt_big *a);
int pbt_big_fits_i64(const pbt_big *a);
/* Low 64 bits of the magnitude. */
uint64_t pbt_big_low_u64(const pbt_big *a);
/* The value reduced modulo 2^64 (two's complement for negatives). */
uint64_t pbt_big_mod_u64(const pbt_big *a);
/* Valid only when pbt_big_fits_i64. */
int64_t pbt_big_to_i64(const pbt_big *a);

/*
 * Decimal: parse "[-]digits" (no canonical-form policy here: leading zeros are
 * accepted; callers that need canonical text check it first).  Returns nonzero
 * for an empty string, a non-digit, more than PBT_BIG_MAX_DIGITS digits or
 * more than PBT_BIG_MAX_BITS bits.
 */
int pbt_big_parse_dec(pbt_big *out, const char *s, size_t len);
/* Render into buf (cap bytes, NUL terminated); returns the length or 0 if cap is too small. */
size_t pbt_big_to_dec(const pbt_big *a, char *buf, size_t cap);

#endif /* PBT_BIG_H */
