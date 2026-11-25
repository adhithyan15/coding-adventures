/*
 * fuzz_mutate.c -- the mutator algebra (VER06 section 3, VER06j section 3).
 *
 * A mutator is a PURE function from bytes to bytes: it never touches its input
 * and returns a fresh buffer.  Together they are the moves of the fuzzer: flip
 * a bit, bump a byte, splice two inputs.  The shared fuzz-v1 corpus pins their
 * outputs byte for byte, so every language lane mutates identically.
 *
 * TWO FORMS
 * ---------
 * Each mutator exists for int64 arguments and for pbt_big arguments (the corpus
 * carries integers far wider than 64 bits).  Both reduce their arguments to the
 * same small unsigned numbers and then call ONE core, so they cannot disagree.
 *
 * FLOOR MODULO
 * ------------
 * Offsets wrap like a clock: -1 mod 8 is 7, not -1 (C's % would give -1).
 *
 *      floor_mod(-1, 8) = 7        floor_mod(9, 8) = 1        floor_mod(-8, 8) = 0
 *
 * FLIPS BY PARITY
 * ---------------
 * Flipping `count` consecutive bits, wrapping round a buffer of n bits, hits
 * every bit count/n times and the first count%n bits (counting from the start
 * of the window) once more.  A bit flipped an even number of times is back where
 * it began, so only the parity matters:
 *
 *      flips(step) = count/n + (1 if step < count%n else 0)
 *
 * That is O(len) work for ANY count; a loop of `count` single flips would never
 * finish for count = 10^18 (VER06 section 9.2).
 *
 * Worked example, 2 bytes = 16 bits, offset 0, count 17: 17/16 = 1 (every bit
 * once: odd) and 17%16 = 1 (bit 0 once more, so twice: even).  The result
 * inverts bits 1..15 and leaves bit 0.
 */
#include <stdlib.h>
#include <string.h>

#include "fuzz.h"
#include "fuzz_hooks.h"
#include "pbt_internal.h"

const int fz_deltas8[6] = {-35, -8, -1, 1, 8, 35};
const uint8_t fz_interest8[5] = {0, 1, 127, 128, 255};

static const uint64_t INTEREST16[7] = {0, 1, 255, 256, 32767, 32768, 65535};
static const uint64_t INTEREST32[7] = {0, 1, 65535, 65536, 0x7FFFFFFFu, 0x80000000u, 0xFFFFFFFFu};
static const uint64_t INTEREST64[7] = {0, 1, 0xFFFFFFFFu, 0x100000000u, 0x7FFFFFFFFFFFFFFFu, 0x8000000000000000u,
                                       0xFFFFFFFFFFFFFFFFu};

void fz_bytes_free(fz_bytes *b)
{
    pbt_free(b->p);
    b->p = NULL;
    b->n = 0;
}

size_t fz_interest_values(int width, uint64_t out[7])
{
    const uint64_t *src = NULL;
    size_t n = 7;
    size_t i;
    switch (width) {
    case 1:
        for (i = 0; i < 5; i++) {
            out[i] = fz_interest8[i];
        }
        return 5;
    case 2:
        src = INTEREST16;
        break;
    case 4:
        src = INTEREST32;
        break;
    case 8:
        src = INTEREST64;
        break;
    default:
        return 0;
    }
    for (i = 0; i < n; i++) {
        out[i] = src[i];
    }
    return n;
}

/* ---- helpers --------------------------------------------------------------------------------------- */

static int valid_width(int w)
{
    return w == 1 || w == 2 || w == 4 || w == 8;
}

/* a mod m for m > 0, always in [0, m).  m fits comfortably in int64 (inputs are capped at 2^30 bytes). */
static uint64_t floor_mod(int64_t a, uint64_t m)
{
    int64_t r = a % (int64_t)m;
    if (r < 0) {
        r += (int64_t)m;
    }
    return (uint64_t)r;
}

/* The same for a bignum: |a| mod m, then mirrored for a negative a. */
static uint64_t floor_mod_big(const pbt_big *a, uint64_t m)
{
    pbt_big mb;
    pbt_big r;
    uint64_t v;
    pbt_big_set_u64(&mb, m);
    (void)pbt_big_divmod_abs(NULL, &r, a, &mb);
    v = pbt_big_low_u64(&r);
    if (pbt_big_sign(a) < 0 && v != 0) {
        v = m - v;
    }
    return v;
}

/* Clamp a bignum into the int64 range. */
static int64_t sat_i64(const pbt_big *v)
{
    if (pbt_big_fits_i64(v)) {
        return pbt_big_to_i64(v);
    }
    return pbt_big_sign(v) < 0 ? INT64_MIN : INT64_MAX;
}

/* A fresh output buffer of n bytes (never NULL on success). */
static int alloc_out(fz_bytes *out, size_t n)
{
    out->p = (uint8_t *)pbt_alloc(n, 1);
    out->n = 0;
    if (out->p == NULL) {
        return FZ_E_NOMEM;
    }
    out->n = n;
    return FZ_OK;
}

static int copy_out(fz_bytes *out, const uint8_t *d, size_t n)
{
    int rc = alloc_out(out, n);
    if (rc == FZ_OK && n > 0) {
        memcpy(out->p, d, n);
    }
    return rc;
}

static int too_big(size_t n)
{
    return n > FZ_MAX_MUTATE_BYTES;
}

/* ---- flips ------------------------------------------------------------------------------------------- */

/* Invert `rem` units from `base` (wrapping), after inverting everything when odd_full.  unit_bits is 1 or 8. */
static int flip_core(fz_bytes *out, const uint8_t *d, size_t n, int unit_bits, uint64_t base, int odd_full,
                     uint64_t rem)
{
    uint64_t units = (uint64_t)n * (unit_bits == 1 ? 8u : 1u);
    uint64_t step;
    size_t i;
    int rc = copy_out(out, d, n);
    if (rc != FZ_OK) {
        return rc;
    }
    if (odd_full) {
        for (i = 0; i < n; i++) {
            out->p[i] ^= 0xFF;
        }
    }
    for (step = 0; step < rem; step++) {
        uint64_t pos = (base + step) % units;
        if (unit_bits == 1) {
            out->p[pos / 8] ^= (uint8_t)(1u << (pos % 8));
        } else {
            out->p[pos] ^= 0xFF;
        }
    }
    return FZ_OK;
}

static int flip(fz_bytes *out, const uint8_t *d, size_t n, int unit_bits, int64_t offset, int64_t count)
{
    out->p = NULL;
    out->n = 0;
    uint64_t units;
    if (too_big(n)) {
        return FZ_E_LIMIT;
    }
    if (n == 0 || count <= 0) {
        return copy_out(out, d, n);
    }
    units = (uint64_t)n * (unit_bits == 1 ? 8u : 1u);
    return flip_core(out, d, n, unit_bits, floor_mod(offset, units), (((uint64_t)count / units) & 1u) != 0,
                     (uint64_t)count % units);
}

static int flip_big(fz_bytes *out, const uint8_t *d, size_t n, int unit_bits, const pbt_big *offset,
                    const pbt_big *count)
{
    out->p = NULL;
    out->n = 0;
    uint64_t units;
    pbt_big ub;
    pbt_big q;
    pbt_big r;
    if (too_big(n)) {
        return FZ_E_LIMIT;
    }
    if (n == 0 || pbt_big_sign(count) <= 0) {
        return copy_out(out, d, n);
    }
    units = (uint64_t)n * (unit_bits == 1 ? 8u : 1u);
    pbt_big_set_u64(&ub, units);
    (void)pbt_big_divmod_abs(&q, &r, count, &ub);
    return flip_core(out, d, n, unit_bits, floor_mod_big(offset, units), (pbt_big_low_u64(&q) & 1u) != 0,
                     pbt_big_low_u64(&r));
}

/*
 * Bits are numbered LSB-first within each byte: bit 9 of 00 00 is bit 1 of byte 1,
 * so BitFlip(00 00, 9, 1) is 00 02.  An empty input or count <= 0 is a no-op.
 */
int fz_bit_flip(fz_bytes *out, const uint8_t *d, size_t n, int64_t o, int64_t c)
{
    out->p = NULL;
    out->n = 0;
    return flip(out, d, n, 1, o, c);
}

int fz_bit_flip_big(fz_bytes *out, const uint8_t *d, size_t n, const pbt_big *o, const pbt_big *c)
{
    out->p = NULL;
    out->n = 0;
    return flip_big(out, d, n, 1, o, c);
}

int fz_byte_flip(fz_bytes *out, const uint8_t *d, size_t n, int64_t o, int64_t c)
{
    out->p = NULL;
    out->n = 0;
    return flip(out, d, n, 8, o, c);
}

int fz_byte_flip_big(fz_bytes *out, const uint8_t *d, size_t n, const pbt_big *o, const pbt_big *c)
{
    out->p = NULL;
    out->n = 0;
    return flip_big(out, d, n, 8, o, c);
}

/* ---- arithmetic and boundary values -------------------------------------------------------------------- */

static uint64_t read_uint(const uint8_t *b, int w, fz_endian e)
{
    uint64_t v = 0;
    int i;
    for (i = 0; i < w; i++) {
        if (e == FZ_BIG) {
            v = (v << 8) | b[i];
        } else {
            v |= (uint64_t)b[i] << (8 * i);
        }
    }
    return v;
}

/* Store the low w bytes of v: that is "v mod 2^(8w)". */
static void write_uint(uint8_t *b, int w, fz_endian e, uint64_t v)
{
    int i;
    for (i = 0; i < w; i++) {
        int shift = e == FZ_BIG ? 8 * (w - 1 - i) : 8 * i;
        b[i] = (uint8_t)(v >> shift);
    }
}

/*
 * Add delta (mod 2^64, which is also mod 2^(8w) after truncation) to the w-byte integer at `at`.
 * Worked example (u16, little endian): ff 00 is 0x00ff; +1 is 0x0100, stored little endian as 00 01.
 */
static int arith_at(fz_bytes *out, const uint8_t *d, size_t n, size_t at, int w, uint64_t delta, fz_endian e)
{
    int rc = copy_out(out, d, n);
    if (rc == FZ_OK) {
        write_uint(out->p + at, w, e, read_uint(out->p + at, w, e) + delta);
    }
    return rc;
}

static int boundary_at(fz_bytes *out, const uint8_t *d, size_t n, size_t at, int w, uint64_t val, fz_endian e)
{
    int rc = copy_out(out, d, n);
    if (rc == FZ_OK) {
        write_uint(out->p + at, w, e, val);
    }
    return rc;
}

int fz_byte_arithmetic(fz_bytes *out, const uint8_t *d, size_t n, int64_t off, int64_t delta, int w, fz_endian e)
{
    out->p = NULL;
    out->n = 0;
    if (too_big(n)) {
        return FZ_E_LIMIT;
    }
    if (!valid_width(w) || n < (size_t)w) {
        return copy_out(out, d, n);
    }
    return arith_at(out, d, n, (size_t)floor_mod(off, (uint64_t)(n - (size_t)w + 1)), w, (uint64_t)delta, e);
}

int fz_byte_arithmetic_big(fz_bytes *out, const uint8_t *d, size_t n, const pbt_big *off, const pbt_big *delta, int w,
                           fz_endian e)
{
    out->p = NULL;
    out->n = 0;
    if (too_big(n)) {
        return FZ_E_LIMIT;
    }
    if (!valid_width(w) || n < (size_t)w) {
        return copy_out(out, d, n);
    }
    return arith_at(out, d, n, (size_t)floor_mod_big(off, (uint64_t)(n - (size_t)w + 1)), w, pbt_big_mod_u64(delta),
                    e);
}

/* Overwrite width bytes with val mod 2^(8*width).  BoundaryInsert(00 00 00 00, 0, 4, 0x80000000, little) is 00 00 00 80. */
int fz_boundary_insert(fz_bytes *out, const uint8_t *d, size_t n, int64_t off, int w, uint64_t val, fz_endian e)
{
    out->p = NULL;
    out->n = 0;
    if (too_big(n)) {
        return FZ_E_LIMIT;
    }
    if (!valid_width(w) || n < (size_t)w) {
        return copy_out(out, d, n);
    }
    return boundary_at(out, d, n, (size_t)floor_mod(off, (uint64_t)(n - (size_t)w + 1)), w, val, e);
}

int fz_boundary_insert_big(fz_bytes *out, const uint8_t *d, size_t n, const pbt_big *off, int w, const pbt_big *val,
                           fz_endian e)
{
    out->p = NULL;
    out->n = 0;
    if (too_big(n)) {
        return FZ_E_LIMIT;
    }
    if (!valid_width(w) || n < (size_t)w) {
        return copy_out(out, d, n);
    }
    return boundary_at(out, d, n, (size_t)floor_mod_big(off, (uint64_t)(n - (size_t)w + 1)), w, pbt_big_mod_u64(val),
                       e);
}

/* ---- blocks and splice ----------------------------------------------------------------------------------- */

/* Delete up to `length` bytes at `at` (clamped at the end). */
static int delete_at(fz_bytes *out, const uint8_t *d, size_t n, size_t at, int64_t length)
{
    size_t room = n - at;
    size_t take = (uint64_t)length < (uint64_t)room ? (size_t)length : room;
    int rc = alloc_out(out, n - take);
    if (rc != FZ_OK) {
        return rc;
    }
    if (at > 0) {
        memcpy(out->p, d, at);
    }
    if (n - at - take > 0) {
        memcpy(out->p + at, d + at + take, n - at - take);
    }
    return FZ_OK;
}

int fz_block_delete(fz_bytes *out, const uint8_t *d, size_t n, int64_t off, int64_t len)
{
    out->p = NULL;
    out->n = 0;
    if (too_big(n)) {
        return FZ_E_LIMIT;
    }
    if (n == 0 || len <= 0) {
        return copy_out(out, d, n);
    }
    return delete_at(out, d, n, (size_t)floor_mod(off, (uint64_t)n), len);
}

int fz_block_delete_big(fz_bytes *out, const uint8_t *d, size_t n, const pbt_big *off, const pbt_big *len)
{
    out->p = NULL;
    out->n = 0;
    if (too_big(n)) {
        return FZ_E_LIMIT;
    }
    if (n == 0 || pbt_big_sign(len) <= 0) {
        return copy_out(out, d, n);
    }
    return delete_at(out, d, n, (size_t)floor_mod_big(off, (uint64_t)n), sat_i64(len));
}

/* data[:at] + ins + data[at:], the size overflow-checked. */
static int insert_at(fz_bytes *out, const uint8_t *d, size_t n, size_t at, const uint8_t *ins, size_t ins_n)
{
    int ok = 1;
    size_t total = pbt_add_size(n, ins_n, &ok);
    int rc;
    if (!ok || too_big(total)) {
        return FZ_E_LIMIT;
    }
    rc = alloc_out(out, total);
    if (rc != FZ_OK) {
        return rc;
    }
    if (at > 0) {
        memcpy(out->p, d, at);
    }
    if (ins_n > 0) {
        memcpy(out->p + at, ins, ins_n);
    }
    if (n - at > 0) {
        memcpy(out->p + at + ins_n, d + at, n - at);
    }
    return FZ_OK;
}

/* Insert at offset mod (len+1); an empty input becomes the insert. */
int fz_block_insert(fz_bytes *out, const uint8_t *d, size_t n, int64_t off, const uint8_t *ins, size_t ins_n)
{
    out->p = NULL;
    out->n = 0;
    if (too_big(n) || too_big(ins_n)) {
        return FZ_E_LIMIT;
    }
    if (n == 0) {
        return copy_out(out, ins, ins_n);
    }
    return insert_at(out, d, n, (size_t)floor_mod(off, (uint64_t)n + 1), ins, ins_n);
}

int fz_block_insert_big(fz_bytes *out, const uint8_t *d, size_t n, const pbt_big *off, const uint8_t *ins,
                        size_t ins_n)
{
    out->p = NULL;
    out->n = 0;
    if (too_big(n) || too_big(ins_n)) {
        return FZ_E_LIMIT;
    }
    if (n == 0) {
        return copy_out(out, ins, ins_n);
    }
    return insert_at(out, d, n, (size_t)floor_mod_big(off, (uint64_t)n + 1), ins, ins_n);
}

static int duplicate_at(fz_bytes *out, const uint8_t *d, size_t n, size_t src, int64_t length, size_t dst)
{
    size_t room = n - src;
    size_t take = (uint64_t)length < (uint64_t)room ? (size_t)length : room;
    return insert_at(out, d, n, dst, d + src, take);
}

/* Copy up to length bytes from src mod len and insert them at dst mod (len+1); the copy is clamped at the end. */
int fz_block_duplicate(fz_bytes *out, const uint8_t *d, size_t n, int64_t src, int64_t len, int64_t dst)
{
    out->p = NULL;
    out->n = 0;
    if (too_big(n)) {
        return FZ_E_LIMIT;
    }
    if (n == 0 || len <= 0) {
        return copy_out(out, d, n);
    }
    return duplicate_at(out, d, n, (size_t)floor_mod(src, (uint64_t)n), len, (size_t)floor_mod(dst, (uint64_t)n + 1));
}

int fz_block_duplicate_big(fz_bytes *out, const uint8_t *d, size_t n, const pbt_big *src, const pbt_big *len,
                           const pbt_big *dst)
{
    out->p = NULL;
    out->n = 0;
    if (too_big(n)) {
        return FZ_E_LIMIT;
    }
    if (n == 0 || pbt_big_sign(len) <= 0) {
        return copy_out(out, d, n);
    }
    return duplicate_at(out, d, n, (size_t)floor_mod_big(src, (uint64_t)n), sat_i64(len),
                        (size_t)floor_mod_big(dst, (uint64_t)n + 1));
}

static size_t clamp_split(int64_t split, size_t n)
{
    if (split <= 0) {
        return 0;
    }
    return (uint64_t)split > (uint64_t)n ? n : (size_t)split;
}

/* target[:ts] + donor[ds:], both splits clamped into their buffers. */
int fz_splice(fz_bytes *out, const uint8_t *t, size_t tn, const uint8_t *donor, size_t dn, int64_t ts, int64_t ds)
{
    out->p = NULL;
    out->n = 0;
    size_t a;
    size_t b;
    int ok = 1;
    size_t total;
    int rc;
    if (too_big(tn) || too_big(dn)) {
        return FZ_E_LIMIT;
    }
    a = clamp_split(ts, tn);
    b = clamp_split(ds, dn);
    total = pbt_add_size(a, dn - b, &ok);
    if (!ok) {
        return FZ_E_LIMIT;
    }
    rc = alloc_out(out, total);
    if (rc != FZ_OK) {
        return rc;
    }
    if (a > 0) {
        memcpy(out->p, t, a);
    }
    if (dn - b > 0) {
        memcpy(out->p + a, donor + b, dn - b);
    }
    return FZ_OK;
}

int fz_splice_big(fz_bytes *out, const uint8_t *t, size_t tn, const uint8_t *donor, size_t dn, const pbt_big *ts,
                  const pbt_big *ds)
{
    out->p = NULL;
    out->n = 0;
    return fz_splice(out, t, tn, donor, dn, sat_i64(ts), sat_i64(ds));
}

/* ---- mutate_sequence -------------------------------------------------------------------------------------- */

typedef struct seqbuf {
    uint8_t *p;
    size_t n;
    size_t cap;
} seqbuf;

/* Make room for `want` bytes (doubling, overflow-checked). */
static int seq_reserve(seqbuf *s, size_t want)
{
    size_t cap;
    uint8_t *np;
    if (want <= s->cap) {
        return FZ_OK;
    }
    cap = s->cap < 16 ? 16 : s->cap;
    while (cap < want) {
        if (cap > ((size_t)-1) / 2) {
            return FZ_E_LIMIT;
        }
        cap *= 2;
    }
    np = (uint8_t *)pbt_realloc(s->p, cap, 1);
    if (np == NULL) {
        return FZ_E_NOMEM;
    }
    s->p = np;
    s->cap = cap;
    return FZ_OK;
}

static uint8_t interest_at(size_t i)
{
    return fz_hooks.interest8 != NULL ? fz_hooks.interest8[i] : fz_interest8[i];
}

static int delta_at(size_t i)
{
    return fz_hooks.deltas8 != NULL ? fz_hooks.deltas8[i] : fz_deltas8[i];
}

/*
 * Each step is chosen by one bounded(7) draw:
 *
 *   draw  step
 *   0     bit flip, count 1
 *   1     byte flip, count 1
 *   2     byte arithmetic, width 1, delta from (-35 -8 -1 1 8 35)
 *   3     boundary insert, width 1, value from (0 1 127 128 255)
 *   4     block delete (length 1..len/2); skipped if len <= 1
 *   5     block insert of one random byte
 *   6     block duplicate (length 1..len/2); skipped if len <= 1
 *
 * An empty buffer is refilled to 00 before every step.  The draw order is the cross-lane contract.
 * Draws for a skipped step are still consumed.  max_len skips a growing step AFTER its draws.
 *
 * The work charged per step is the length of the buffer the step leaves (at least 1), and the initial
 * copy is charged first; see VER06j section 3.
 */
int fz_mutate_sequence_budget(fz_bytes *out, const uint8_t *d, size_t n, uint64_t seed, size_t steps, size_t max_len,
                              uint64_t work_cap, uint64_t *work_done)
{
    pbt_rng rng;
    seqbuf s;
    uint64_t work = 0;
    size_t i;
    int rc = FZ_OK;
    out->p = NULL;
    out->n = 0;
    *work_done = 0;
    if (max_len > FZ_MAX_MUTATE_BYTES) {
        max_len = FZ_MAX_MUTATE_BYTES; /* a growth cap above what any mutator accepts is no cap at all */
    }
    if (steps > FZ_MAX_ITERATIONS || n > FZ_MAX_INPUT_BYTES) {
        return FZ_E_LIMIT;
    }
    if (n > work_cap) {
        return FZ_E_WORK;
    }
    work = n;
    *work_done = work;
    s.p = NULL;
    s.n = n;
    s.cap = 0;
    rc = seq_reserve(&s, n == 0 ? 1 : n);
    if (rc != FZ_OK) {
        return rc;
    }
    if (n > 0) {
        memcpy(s.p, d, n);
    }
    pbt_rng_init(&rng, seed);
    for (i = 0; i < steps; i++) {
        size_t len;
        uint64_t cost;
        size_t after;
        if (s.n == 0) {
            s.p[0] = 0;
            s.n = 1;
        }
        len = s.n;
        switch (pbt_rng_bounded(&rng, 7)) {
        case 0: {
            uint64_t pos = pbt_rng_bounded(&rng, (uint64_t)len * 8);
            cost = len;
            after = len;
            if (cost > work_cap - work) {
                rc = FZ_E_WORK;
                break;
            }
            s.p[pos / 8] ^= (uint8_t)(1u << (pos % 8));
            break;
        }
        case 1: {
            uint64_t pos = pbt_rng_bounded(&rng, (uint64_t)len);
            cost = len;
            after = len;
            if (cost > work_cap - work) {
                rc = FZ_E_WORK;
                break;
            }
            s.p[pos] ^= 0xFF;
            break;
        }
        case 2: {
            int delta = delta_at((size_t)pbt_rng_bounded(&rng, 6));
            uint64_t at = pbt_rng_bounded(&rng, (uint64_t)len);
            cost = len;
            after = len;
            if (cost > work_cap - work) {
                rc = FZ_E_WORK;
                break;
            }
            s.p[at] = (uint8_t)(s.p[at] + delta);
            break;
        }
        case 3: {
            uint8_t val = interest_at((size_t)pbt_rng_bounded(&rng, 5));
            uint64_t at = pbt_rng_bounded(&rng, (uint64_t)len);
            cost = len;
            after = len;
            if (cost > work_cap - work) {
                rc = FZ_E_WORK;
                break;
            }
            s.p[at] = val;
            break;
        }
        case 4:
            cost = len;
            after = len;
            if (len > 1) {
                uint64_t length = 1 + pbt_rng_bounded(&rng, (uint64_t)(len / 2 > 1 ? len / 2 : 1));
                size_t at = (size_t)pbt_rng_bounded(&rng, (uint64_t)len);
                size_t room = len - at;
                size_t take = length < room ? (size_t)length : room;
                after = len - take;
                cost = after > 0 ? after : 1;
                if (cost > work_cap - work) {
                    rc = FZ_E_WORK;
                    break;
                }
                memmove(s.p + at, s.p + at + take, len - at - take);
                s.n = len - take;
            } else {
                cost = len;
            }
            break;
        case 5: {
            uint8_t ins = (uint8_t)pbt_rng_bounded(&rng, 256);
            size_t at = (size_t)pbt_rng_bounded(&rng, (uint64_t)len + 1);
            cost = len;
            after = len;
            if (len + 1 <= max_len) {
                cost = (uint64_t)len + 1;
                if (cost > work_cap - work) {
                    rc = FZ_E_WORK;
                    break;
                }
                rc = seq_reserve(&s, len + 1);
                if (rc != FZ_OK) {
                    break;
                }
                memmove(s.p + at + 1, s.p + at, len - at);
                s.p[at] = ins;
                s.n = len + 1;
                after = len + 1;
            } else if (cost > work_cap - work) {
                rc = FZ_E_WORK;
            }
            break;
        }
        default:
            cost = len;
            after = len;
            if (len > 1) {
                uint64_t length = 1 + pbt_rng_bounded(&rng, (uint64_t)(len / 2 > 1 ? len / 2 : 1));
                size_t src = (size_t)pbt_rng_bounded(&rng, (uint64_t)len);
                size_t dst = (size_t)pbt_rng_bounded(&rng, (uint64_t)len + 1);
                size_t room = len - src;
                size_t take = length < room ? (size_t)length : room;
                if (len + take <= max_len) {
                    cost = (uint64_t)len + take;
                    if (cost > work_cap - work) {
                        rc = FZ_E_WORK;
                        break;
                    }
                    rc = seq_reserve(&s, len + take);
                    if (rc != FZ_OK) {
                        break;
                    }
                    /* the source range may overlap the insertion point: move the tail first, then copy
                     * from the (possibly shifted) source */
                    memmove(s.p + dst + take, s.p + dst, len - dst);
                    if (src >= dst) {
                        memmove(s.p + dst, s.p + src + take, take);
                    } else if (src + take <= dst) {
                        memmove(s.p + dst, s.p + src, take);
                    } else {
                        /* source straddles the insertion point: head part unmoved, tail part shifted */
                        size_t head = dst - src;
                        memmove(s.p + dst, s.p + src, head);
                        memmove(s.p + dst + head, s.p + dst + take, take - head);
                    }
                    s.n = len + take;
                    after = len + take;
                } else if (cost > work_cap - work) {
                    rc = FZ_E_WORK;
                }
            } else if (cost > work_cap - work) {
                rc = FZ_E_WORK;
            }
            break;
        }
        if (rc != FZ_OK) {
            break;
        }
        (void)after;
        work += cost;
        *work_done = work;
    }
    if (rc != FZ_OK) {
        pbt_free(s.p);
        return rc;
    }
    out->p = s.p;
    out->n = s.n;
    return FZ_OK;
}

int fz_mutate_sequence(fz_bytes *out, const uint8_t *d, size_t n, uint64_t seed, size_t steps, size_t max_len)
{
    uint64_t done;
    return fz_mutate_sequence_budget(out, d, n, seed, steps, max_len, UINT64_MAX, &done);
}
