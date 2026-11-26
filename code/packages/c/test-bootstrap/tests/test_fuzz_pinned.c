/*
 * test_fuzz_pinned.c -- the blind spots fuzz-v1 does not pin (VER06j section 11), plus unit tests of the pure
 * pieces: real signature hashes (cross-checked with scripts/fuzz_conformance.py), power-schedule rows including
 * half-way ties, every interest and delta constant, unequal splice splits with target and donor swapped, bitmap
 * and admission rules, TAP sanitising, crash normalisation, work accounting and mutator limits.
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "fuzz.h"
#include "fuzz_hooks.h"
#include "iso_test.h"
#include "pbt_hooks.h"
#include "pbt_internal.h"

static fz_bytes unhex(const char *h)
{
    fz_bytes b;
    size_t n = strlen(h) / 2;
    size_t i;
    b.p = (uint8_t *)pbt_alloc(n, 1);
    b.n = n;
    for (i = 0; i < n; i++) {
        unsigned v = 0;
        (void)sscanf(h + 2 * i, "%2x", &v);
        b.p[i] = (uint8_t)v;
    }
    return b;
}

static int is_hex(const fz_bytes *b, const char *h)
{
    fz_bytes w = unhex(h);
    int ok = w.n == b->n && (w.n == 0 || memcmp(w.p, b->p, w.n) == 0);
    fz_bytes_free(&w);
    return ok;
}

/* ---- signatures ------------------------------------------------------------------------------------------ */

static void check_sig(const char *kind, const char *const *frames, size_t n, const char *want)
{
    char got[9];
    fz_crash_signature(got, kind, frames, n);
    if (strcmp(got, want) != 0) {
        printf("  signature of %s is %s, oracle says %s\n", kind, got, want);
    }
    ISO_CHECK_EQ_INT(strcmp(got, want), 0);
}

static uint64_t shift_hash(uint64_t h)
{
    return h >> 1;
}

static void test_signatures(void)
{
    static const char *const f1[] = {"eval", "step"};
    static const char *const f2[] = {"alloc", "write"};
    static const char *const f3[] = {"parse", "main"};
    static const char *const f4[] = {"parse"};
    static const char *const f5[] = {"a", "b", "c"};
    static const char *const f6[] = {"y"};
    char s[9];
    char t[9];
    /* values from scripts/fuzz_conformance.py crash_signature (the corpus pins none of them) */
    check_sig("panic", f1, 2, "4eca7ae8");
    check_sig("segv", f2, 2, "a0aa3aa2");
    check_sig("timeout", NULL, 0, "4e61ea76");
    check_sig("oom", NULL, 0, "7146a410");
    check_sig("invalid", NULL, 0, "43a76e9e");
    check_sig("unspecified", NULL, 0, "e2f83322");
    check_sig("oob", f3, 2, "54a9c7a9");
    check_sig("oob", f4, 1, "c151f40e");
    check_sig("assert", f5, 3, "0bf0dbc1");
    check_sig("", NULL, 0, "8601adad");
    check_sig("x", f6, 1, "7f6b2d3a");
    /* the definition: low 32 bits of fnv1a64("kind:frames joined by :") */
    ISO_CHECK_MSG(pbt_fnv1a64("panic:eval:step", 15) % 4294967296u == 0x4eca7ae8u, "fnv1a64 of the joined text");
    /* frames are not escaped: one frame "x:y" and two frames "x", "y" hash alike (documented limit) */
    {
        static const char *const one[] = {"x:y"};
        static const char *const two[] = {"x", "y"};
        fz_crash_signature(s, "k", one, 1);
        fz_crash_signature(t, "k", two, 2);
        ISO_CHECK_EQ_INT(strcmp(s, t), 0);
    }
    /* different kind, different frames, different order: different signatures */
    fz_crash_signature(s, "panic", f1, 2);
    fz_crash_signature(t, "panic", f2, 2);
    ISO_CHECK(strcmp(s, t) != 0);
    /* the sabotage seam moves the signature */
    fz_hooks.sig_post = shift_hash;
    fz_crash_signature(s, "panic", f1, 2);
    fz_hooks_reset();
    ISO_CHECK(strcmp(s, "4eca7ae8") != 0);
    fz_crash_signature(s, "panic", f1, 2);
    ISO_CHECK_EQ_INT(strcmp(s, "4eca7ae8"), 0);
}

/* ---- the power schedule ---------------------------------------------------------------------------------- */

static void check_energy(size_t base, size_t len, size_t nw, size_t tot, size_t want)
{
    size_t got = fz_calculate_energy(base, len, nw, tot);
    if (got != want) {
        printf("  energy(%lu,%lu,%lu,%lu) = %lu, oracle says %lu\n", (unsigned long)base, (unsigned long)len,
               (unsigned long)nw, (unsigned long)tot, (unsigned long)got, (unsigned long)want);
    }
    ISO_CHECK_EQ_INT(got, want);
}

static double floor_only(double v)
{
    return (double)(unsigned long)v;
}

static void test_energy(void)
{
    /* rows from the Go formula evaluated in Python; the *tie* rows have x exactly k + 0.5 and round UP */
    check_energy(10, 4, 3, 6, 8);   /* 7.5: tie */
    check_energy(5, 4, 0, 0, 3);    /* 2.5: tie */
    check_energy(3, 4, 0, 0, 2);    /* 1.5: tie */
    check_energy(7, 4, 0, 0, 4);    /* 3.5: tie */
    check_energy(9, 36, 0, 0, 2);   /* 1.5: tie */
    check_energy(25, 100, 0, 0, 3); /* 2.5: tie */
    check_energy(1, 16, 1, 1, 1);   /* 0.5: tie, floor(1.0) */
    check_energy(10, 1, 0, 0, 10);
    check_energy(10, 100, 1, 2, 2);
    check_energy(0, 1, 0, 0, 1); /* base is at least 1 */
    check_energy(1, 1000000, 5, 5, 1);
    check_energy(10, 0, 0, 0, 10); /* len 0 counts as 1 */
    check_energy(1000000, 1, 5, 5, 1000000); /* clamped to FZ_MAX_ITERATIONS */
    check_energy(2, 4, 0, 0, 1);
    check_energy(1, 4, 0, 0, 1); /* 0.5 rounds up to 1 and the floor of 1 anyway */
    check_energy(15, 9, 0, 0, 5);
    check_energy(10, 9, 2, 4, 5);
    check_energy(11, 4, 1, 1, 11);
    check_energy(50, 4, 1, 2, 38);
    /* the total == 0 bonus is 1.0 even with "new" edges claimed */
    check_energy(10, 4, 9, 0, 5);
    /* sabotaging the rounding (floor instead of half-up) changes a tie row */
    fz_hooks.energy_round = floor_only;
    ISO_CHECK(fz_calculate_energy(10, 4, 3, 6) != 8);
    fz_hooks_reset();
    ISO_CHECK_EQ_INT(fz_calculate_energy(10, 4, 3, 6), 8);
}

/* ---- constants ---------------------------------------------------------------------------------------------- */

static void test_constants(void)
{
    uint64_t v[7];
    static const uint64_t i8[5] = {0, 1, 127, 128, 255};
    static const uint64_t i16[7] = {0, 1, 255, 256, 32767, 32768, 65535};
    static const uint64_t i32[7] = {0, 1, 65535, 65536, 0x7FFFFFFFu, 0x80000000u, 0xFFFFFFFFu};
    static const uint64_t i64[7] = {0, 1, 0xFFFFFFFFu, 0x100000000u, 0x7FFFFFFFFFFFFFFFu, 0x8000000000000000u,
                                    0xFFFFFFFFFFFFFFFFu};
    static const int d8[6] = {-35, -8, -1, 1, 8, 35};
    size_t i;
    ISO_CHECK_EQ_INT(fz_interest_values(1, v), 5);
    for (i = 0; i < 5; i++) {
        ISO_CHECK(v[i] == i8[i]);
        ISO_CHECK(fz_interest8[i] == i8[i]);
    }
    ISO_CHECK_EQ_INT(fz_interest_values(2, v), 7);
    for (i = 0; i < 7; i++) {
        ISO_CHECK(v[i] == i16[i]);
    }
    ISO_CHECK_EQ_INT(fz_interest_values(4, v), 7);
    for (i = 0; i < 7; i++) {
        ISO_CHECK(v[i] == i32[i]);
    }
    ISO_CHECK_EQ_INT(fz_interest_values(8, v), 7);
    for (i = 0; i < 7; i++) {
        ISO_CHECK(v[i] == i64[i]);
    }
    ISO_CHECK_EQ_INT(fz_interest_values(3, v), 0);
    ISO_CHECK_EQ_INT(fz_interest_values(0, v), 0);
    ISO_CHECK_EQ_INT(fz_interest_values(16, v), 0);
    for (i = 0; i < 6; i++) {
        ISO_CHECK_EQ_INT(fz_deltas8[i], d8[i]);
    }
    /* the limits themselves */
    ISO_CHECK_EQ_INT(FZ_COVERAGE_MAP_SIZE, 65536);
    ISO_CHECK_EQ_INT(FZ_COVERAGE_BYTES, 8192);
    ISO_CHECK_EQ_INT(FZ_MAX_ITERATIONS, 1000000);
    ISO_CHECK(FZ_MAX_INPUT_BYTES == ((size_t)1 << 20));
    ISO_CHECK_EQ_INT(FZ_MAX_CRASHES, 1000);
    ISO_CHECK(FZ_MAX_RETAINED_BYTES == ((uint64_t)256 << 20));
    ISO_CHECK(FZ_CORPUS_MAX_SEQUENCE_WORK == ((uint64_t)1 << 24));
    ISO_CHECK_EQ_INT(FZ_MAX_TARGET_LEN, 128);
}

/* Every table entry matters: changing ANY ONE interest value or delta changes some mutate_sequence output.
 * (Together with the 820 oracle vectors this pins every constant.) */
static int sequence_differs(const uint8_t *tbl8, const int *deltas)
{
    uint8_t in[8] = {10, 20, 30, 40, 50, 60, 70, 80};
    uint64_t seed;
    for (seed = 0; seed < 3000; seed++) {
        fz_bytes a;
        fz_bytes b;
        int diff;
        fz_hooks_reset();
        ISO_CHECK(fz_mutate_sequence(&a, in, 8, seed, 12, 4096) == FZ_OK);
        fz_hooks.interest8 = tbl8;
        fz_hooks.deltas8 = deltas;
        ISO_CHECK(fz_mutate_sequence(&b, in, 8, seed, 12, 4096) == FZ_OK);
        fz_hooks_reset();
        diff = a.n != b.n || memcmp(a.p, b.p, a.n) != 0;
        fz_bytes_free(&a);
        fz_bytes_free(&b);
        if (diff) {
            return 1;
        }
    }
    return 0;
}

static void test_every_table_entry_matters(void)
{
    uint8_t t[5];
    int d[6];
    size_t i;
    for (i = 0; i < 5; i++) {
        memcpy(t, fz_interest8, 5);
        t[i] = (uint8_t)(t[i] ^ 0x5A);
        ISO_CHECK_MSG(sequence_differs(t, fz_deltas8), "an interest value change must be visible");
    }
    for (i = 0; i < 6; i++) {
        memcpy(d, fz_deltas8, sizeof d);
        d[i] += 3;
        ISO_CHECK_MSG(sequence_differs(fz_interest8, d), "a delta change must be visible");
    }
    /* the unmodified tables, passed through the seam, change nothing */
    ISO_CHECK(!sequence_differs(fz_interest8, fz_deltas8));
}

/* ---- splice ------------------------------------------------------------------------------------------------ */

static void check_splice(const char *t, const char *d, int64_t ts, int64_t ds, const char *want)
{
    fz_bytes tb = unhex(t);
    fz_bytes db = unhex(d);
    fz_bytes out;
    ISO_CHECK(fz_splice(&out, tb.p, tb.n, db.p, db.n, ts, ds) == FZ_OK);
    if (!is_hex(&out, want)) {
        printf("  splice(%s,%s,%ld,%ld) differs from oracle %s\n", t, d, (long)ts, (long)ds, want);
    }
    ISO_CHECK(is_hex(&out, want));
    fz_bytes_free(&out);
    fz_bytes_free(&tb);
    fz_bytes_free(&db);
}

static void test_splice_unequal_and_swapped(void)
{
    const char *t = "0102030405";
    const char *d = "a0b1c2d3";
    /* oracle values (scripts/fuzz_conformance.py splice); the corpus case has equal splits 2 and 2 */
    check_splice(t, d, 1, 3, "01d3");
    check_splice(d, t, 3, 1, "a0b1c202030405"); /* target and donor swapped, splits swapped */
    check_splice(t, d, 3, 1, "010203b1c2d3");
    check_splice(d, t, 1, 3, "a00405");
    check_splice(t, d, 0, 0, "a0b1c2d3");
    check_splice(d, t, 0, 0, "0102030405");
    check_splice(t, d, 5, 4, "0102030405");
    check_splice(t, d, 2, -1, "0102a0b1c2d3"); /* negative splits clamp to 0 */
    check_splice(d, t, -1, 2, "030405");
    check_splice(t, d, -3, 2, "c2d3");
    check_splice(t, d, 9, 9, "0102030405"); /* splits past the end clamp */
    check_splice(t, d, 2, 2, "0102c2d3");
    check_splice(d, t, 2, 2, "a0b1030405");
}

/* ---- mutator edges and limits ------------------------------------------------------------------------------- */

static void test_mutator_edges(void)
{
    fz_bytes o;
    const uint8_t two[2] = {0, 0};
    const uint8_t three[3] = {0x12, 0x34, 0x56};
    /* huge counts finish at once (parity), not after 10^18 flips: bit_flip(00 00, 0, INT64_MAX) */
    ISO_CHECK(fz_bit_flip(&o, two, 2, 0, INT64_MAX) == FZ_OK);
    ISO_CHECK(o.n == 2);
    fz_bytes_free(&o);
    /* count 17 over 16 bits: bits 1..15 inverted, bit 0 twice (even) */
    ISO_CHECK(fz_bit_flip(&o, two, 2, 0, 17) == FZ_OK);
    ISO_CHECK(is_hex(&o, "feff"));
    fz_bytes_free(&o);
    /* floor modulo: byte_flip offset -1 over 3 bytes is offset 2 */
    ISO_CHECK(fz_byte_flip(&o, three, 3, -1, 1) == FZ_OK);
    ISO_CHECK(is_hex(&o, "1234a9"));
    fz_bytes_free(&o);
    /* count <= 0 and empty input are copies */
    ISO_CHECK(fz_byte_flip(&o, three, 3, 0, 0) == FZ_OK && is_hex(&o, "123456"));
    fz_bytes_free(&o);
    ISO_CHECK(fz_byte_flip(&o, three, 0, 0, 5) == FZ_OK && o.n == 0 && o.p != NULL);
    fz_bytes_free(&o);
    /* unsupported width / short buffer: unchanged */
    ISO_CHECK(fz_byte_arithmetic(&o, three, 3, 0, 1, 3, FZ_LITTLE) == FZ_OK && is_hex(&o, "123456"));
    fz_bytes_free(&o);
    ISO_CHECK(fz_byte_arithmetic(&o, three, 3, 0, 1, 4, FZ_LITTLE) == FZ_OK && is_hex(&o, "123456"));
    fz_bytes_free(&o);
    ISO_CHECK(fz_boundary_insert(&o, three, 3, 0, 0, 1, FZ_BIG) == FZ_OK && is_hex(&o, "123456"));
    fz_bytes_free(&o);
    /* arithmetic wraps: u16 big endian 0xffff + 1 */
    {
        const uint8_t ff[2] = {0xff, 0xff};
        ISO_CHECK(fz_byte_arithmetic(&o, ff, 2, 0, 1, 2, FZ_BIG) == FZ_OK && is_hex(&o, "0000"));
        fz_bytes_free(&o);
        ISO_CHECK(fz_byte_arithmetic(&o, ff, 2, 0, -1, 2, FZ_LITTLE) == FZ_OK && is_hex(&o, "feff"));
        fz_bytes_free(&o);
    }
    /* block_insert into an empty input is the insert; duplicate at the very end is allowed */
    {
        const uint8_t ins[2] = {7, 8};
        ISO_CHECK(fz_block_insert(&o, NULL, 0, 5, ins, 2) == FZ_OK && is_hex(&o, "0708"));
        fz_bytes_free(&o);
        ISO_CHECK(fz_block_duplicate(&o, three, 3, 1, 5, 3) == FZ_OK && is_hex(&o, "1234563456"));
        fz_bytes_free(&o);
        ISO_CHECK(fz_block_delete(&o, three, 3, 2, 99) == FZ_OK && is_hex(&o, "1234"));
        fz_bytes_free(&o);
    }
    /* limits: an input over FZ_MAX_MUTATE_BYTES is refused before anything is allocated */
    ISO_CHECK_EQ_INT(fz_bit_flip(&o, three, FZ_MAX_MUTATE_BYTES + 1, 0, 1), FZ_E_LIMIT);
    ISO_CHECK(o.p == NULL);
    ISO_CHECK_EQ_INT(fz_splice(&o, three, 3, three, FZ_MAX_MUTATE_BYTES + 1, 0, 0), FZ_E_LIMIT);
    ISO_CHECK_EQ_INT(fz_block_insert(&o, three, 3, 0, three, FZ_MAX_MUTATE_BYTES + 1), FZ_E_LIMIT);
    ISO_CHECK_EQ_INT(fz_mutate_sequence(&o, three, 3, 1, FZ_MAX_ITERATIONS + 1, 100), FZ_E_LIMIT);
    ISO_CHECK_EQ_INT(fz_mutate_sequence(&o, three, FZ_MAX_INPUT_BYTES + 1, 1, 1, 100), FZ_E_LIMIT);
    /* the mutators never touch their input */
    {
        uint8_t keep[4] = {1, 2, 3, 4};
        ISO_CHECK(fz_byte_flip(&o, keep, 4, 0, 4) == FZ_OK);
        ISO_CHECK(keep[0] == 1 && keep[3] == 4);
        ISO_CHECK(o.p != keep);
        fz_bytes_free(&o);
    }
}

/* ---- mutate_sequence work accounting ---------------------------------------------------------------------- */

static void test_sequence_work(void)
{
    uint8_t in[8] = {1, 2, 3, 4, 5, 6, 7, 8};
    uint64_t done = 999;
    fz_bytes o;
    int rc;
    /* the initial copy is charged: a cap below the length fails before any step, having done no work */
    rc = fz_mutate_sequence_budget(&o, in, 8, 42, 4, 100, 7, &done);
    ISO_CHECK_EQ_INT(rc, FZ_E_WORK);
    ISO_CHECK_EQ_INT(done, 0);
    ISO_CHECK(o.p == NULL);
    /* zero steps cost exactly the initial copy */
    rc = fz_mutate_sequence_budget(&o, in, 8, 42, 0, 100, 8, &done);
    ISO_CHECK_EQ_INT(rc, FZ_OK);
    ISO_CHECK_EQ_INT(done, 8);
    fz_bytes_free(&o);
    /* every step costs at least 1: the work grows with the steps and a failed call still reports it */
    rc = fz_mutate_sequence_budget(&o, in, 8, 42, 4, 100, UINT64_MAX, &done);
    ISO_CHECK_EQ_INT(rc, FZ_OK);
    ISO_CHECK(done >= 8 + 4);
    fz_bytes_free(&o);
    {
        uint64_t full = done;
        uint64_t cap;
        for (cap = 8; cap < full; cap++) {
            rc = fz_mutate_sequence_budget(&o, in, 8, 42, 4, 100, cap, &done);
            ISO_CHECK_EQ_INT(rc, FZ_E_WORK);
            ISO_CHECK(done <= cap);
            ISO_CHECK(o.p == NULL);
        }
        rc = fz_mutate_sequence_budget(&o, in, 8, 42, 4, 100, full, &done);
        ISO_CHECK_EQ_INT(rc, FZ_OK);
        ISO_CHECK_EQ_INT(done, full);
        fz_bytes_free(&o);
    }
    /* the work of a failed call is written, and no output survives */
    rc = fz_mutate_sequence_budget(&o, in, 8, 42, 1000, 100, 50, &done);
    ISO_CHECK_EQ_INT(rc, FZ_E_WORK);
    ISO_CHECK(done > 0 && done <= 50);
    /* steps above the ceiling: limit, work 0 */
    rc = fz_mutate_sequence_budget(&o, in, 8, 42, FZ_MAX_ITERATIONS + 1, 100, UINT64_MAX, &done);
    ISO_CHECK_EQ_INT(rc, FZ_E_LIMIT);
    ISO_CHECK_EQ_INT(done, 0);
    /* the cap on growth: with max_len equal to the input length nothing ever grows */
    {
        uint64_t seed;
        for (seed = 0; seed < 200; seed++) {
            rc = fz_mutate_sequence(&o, in, 8, seed, 40, 8);
            ISO_CHECK_EQ_INT(rc, FZ_OK);
            ISO_CHECK(o.n <= 8);
            fz_bytes_free(&o);
        }
    }
}

/* ---- bitmap and admission ---------------------------------------------------------------------------------- */

static void test_bitmap(void)
{
    fz_bitmap *b = (fz_bitmap *)malloc(sizeof *b);
    fz_bitmap *o = (fz_bitmap *)malloc(sizeof *o);
    uint16_t edges[16];
    uint16_t trace[4];
    size_t fresh = 99;
    ISO_CHECK(b != NULL && o != NULL);
    if (b == NULL || o == NULL) {
        free(b);
        free(o);
        return;
    }
    fz_bitmap_init(b);
    fz_bitmap_init(o);
    fz_bitmap_record(b, 70000); /* 70000 mod 65536 = 4464 */
    fz_bitmap_record(b, 4464);  /* the same edge */
    fz_bitmap_record(b, 7);
    ISO_CHECK_EQ_INT(b->nedges, 2);
    ISO_CHECK_EQ_INT(fz_bitmap_count(b), 2);
    ISO_CHECK(fz_bitmap_has(b, 4464) && fz_bitmap_has(b, 7 + 65536) && !fz_bitmap_has(b, 8));
    ISO_CHECK_EQ_INT(fz_bitmap_edges(b, edges), 2);
    ISO_CHECK(edges[0] == 7 && edges[1] == 4464); /* ascending, whatever the order recorded */
    ISO_CHECK(b->m[4464 / 8] == (1u << 0));
    trace[0] = 7;
    trace[1] = 9;
    trace[2] = 9;
    trace[3] = 4464;
    ISO_CHECK(fz_bitmap_admit(b, trace, 4, &fresh) == 1);
    ISO_CHECK_EQ_INT(fresh, 1); /* 9 once, although it appears twice */
    ISO_CHECK(fz_bitmap_admit(b, trace, 4, &fresh) == 0);
    ISO_CHECK_EQ_INT(fresh, 0);
    ISO_CHECK(fz_bitmap_admit(b, trace, 0, NULL) == 0);
    fz_bitmap_record(o, 100);
    fz_bitmap_merge(b, o);
    ISO_CHECK(fz_bitmap_has(b, 100));
    ISO_CHECK_EQ_INT(fz_bitmap_count(b), 4);
    fz_bitmap_reset(b);
    ISO_CHECK_EQ_INT(fz_bitmap_count(b), 0);
    ISO_CHECK_EQ_INT(b->nedges, 0);
    ISO_CHECK(!fz_bitmap_has(b, 4464));
    /* every id round-trips through the full range */
    {
        uint64_t e;
        for (e = 0; e < 70000; e += 997) {
            fz_bitmap_record(b, e);
        }
        ISO_CHECK(fz_bitmap_has(b, 997));
        ISO_CHECK_EQ_INT(fz_bitmap_count(b), b->nedges);
    }
    free(b);
    free(o);
}

/* Security review: a racing target thread (misuse) could push nedges past the map.  Drive more than 65,536 colliding
 * records, then corrupt nedges to the limit directly: record must refuse to write past edges[]. */
static void test_bitmap_never_outgrows_the_map(void)
{
    fz_bitmap *b = (fz_bitmap *)malloc(sizeof *b);
    uint64_t e;
    ISO_CHECK(b != NULL);
    if (b == NULL) {
        return;
    }
    fz_bitmap_init(b);
    for (e = 0; e < (uint64_t)FZ_COVERAGE_MAP_SIZE * 3 + 5; e++) {
        fz_bitmap_record(b, e * 7919u); /* distinct ids that collide modulo 65,536 after the first lap */
        ISO_CHECK_MSG(b->nedges <= FZ_COVERAGE_MAP_SIZE, "nedges outgrew the map");
    }
    ISO_CHECK(b->nedges <= FZ_COVERAGE_MAP_SIZE);
    fz_bitmap_reset(b);
    fz_bitmap_record(b, 3);
    b->nedges = FZ_COVERAGE_MAP_SIZE; /* the state a lost race could leave */
    fz_bitmap_record(b, 4);
    ISO_CHECK_EQ_INT(b->nedges, FZ_COVERAGE_MAP_SIZE);
    ISO_CHECK(!fz_bitmap_has(b, 4));
    free(b);
}

/* A growth cap above FZ_MAX_MUTATE_BYTES is clamped: it behaves exactly like the clamp value. */
static void test_max_len_clamped(void)
{
    uint8_t in[4] = {1, 2, 3, 4};
    fz_bytes a;
    fz_bytes b;
    uint64_t seed;
    for (seed = 0; seed < 50; seed++) {
        ISO_CHECK_EQ_INT(fz_mutate_sequence(&a, in, 4, seed, 30, (size_t)-1), FZ_OK);
        ISO_CHECK_EQ_INT(fz_mutate_sequence(&b, in, 4, seed, 30, FZ_MAX_MUTATE_BYTES), FZ_OK);
        ISO_CHECK(a.n == b.n && memcmp(a.p, b.p, a.n) == 0);
        ISO_CHECK(a.n <= FZ_MAX_MUTATE_BYTES);
        fz_bytes_free(&a);
        fz_bytes_free(&b);
    }
}

/* NULL data with length 0 is legal for every mutator (the sanitizer build proves no memcpy(NULL, ...)). */
static void test_null_empty_inputs(void)
{
    fz_bytes o;
    ISO_CHECK(fz_block_insert(&o, NULL, 0, 0, NULL, 0) == FZ_OK && o.n == 0);
    fz_bytes_free(&o);
    ISO_CHECK(fz_block_delete(&o, NULL, 0, 0, 3) == FZ_OK && o.n == 0);
    fz_bytes_free(&o);
    ISO_CHECK(fz_block_duplicate(&o, NULL, 0, 0, 3, 0) == FZ_OK && o.n == 0);
    fz_bytes_free(&o);
    ISO_CHECK(fz_splice(&o, NULL, 0, NULL, 0, 0, 0) == FZ_OK && o.n == 0);
    fz_bytes_free(&o);
    ISO_CHECK(fz_bit_flip(&o, NULL, 0, 0, 5) == FZ_OK && o.n == 0);
    fz_bytes_free(&o);
    ISO_CHECK(fz_byte_arithmetic(&o, NULL, 0, 0, 1, 1, FZ_BIG) == FZ_OK && o.n == 0);
    fz_bytes_free(&o);
    ISO_CHECK(fz_block_insert(&o, (const uint8_t *)"ab", 2, 2, NULL, 0) == FZ_OK && o.n == 2);
    fz_bytes_free(&o);
    ISO_CHECK(fz_block_insert(&o, (const uint8_t *)"ab", 2, 0, (const uint8_t *)"x", 1) == FZ_OK && o.n == 3 &&
              o.p[0] == 'x');
    fz_bytes_free(&o);
    ISO_CHECK(fz_mutate_sequence(&o, NULL, 0, 9, 20, 64) == FZ_OK);
    fz_bytes_free(&o);
}

static void test_corpus_admit(void)
{
    static const uint64_t ex[] = {1, 2, 3};
    static const uint64_t t1[] = {2, 4};
    static const uint64_t t2[] = {1, 2};
    static const uint64_t t3[] = {4, 4, 4, 5};
    static const uint64_t big1[] = {0xFFFFFFFFFFFFFFFFu, 0xFFFFFFFFFFFFFFFFu};
    int a;
    size_t n;
    ISO_CHECK(fz_corpus_admit(ex, 3, t1, 2, &a, &n) == FZ_OK && a == 1 && n == 1);
    ISO_CHECK(fz_corpus_admit(ex, 3, t2, 2, &a, &n) == FZ_OK && a == 0 && n == 0);
    ISO_CHECK(fz_corpus_admit(ex, 3, t3, 4, &a, &n) == FZ_OK && a == 1 && n == 2); /* 4 once, 5 once */
    ISO_CHECK(fz_corpus_admit(NULL, 0, t1, 2, &a, &n) == FZ_OK && a == 1 && n == 2);
    ISO_CHECK(fz_corpus_admit(ex, 3, NULL, 0, &a, &n) == FZ_OK && a == 0 && n == 0);
    ISO_CHECK(fz_corpus_admit(ex, 3, big1, 2, &a, &n) == FZ_OK && a == 1 && n == 1);
    /* ids are compared exactly here (no masking): 65537 is not 1 */
    {
        static const uint64_t wide[] = {65537};
        ISO_CHECK(fz_corpus_admit(ex, 3, wide, 1, &a, &n) == FZ_OK && a == 1 && n == 1);
    }
}

/* ---- failure ids and TAP ----------------------------------------------------------------------------------- */

static void test_tap(void)
{
    char buf[256];
    static const char *const ids[5] = {"fuzz-crash", "fuzz-timeout", "fuzz-oom", "fuzz-leak", "fuzz-panic"};
    size_t i;
    for (i = 0; i < 5; i++) {
        ISO_CHECK(fz_is_failure_id(ids[i], strlen(ids[i])));
    }
    ISO_CHECK(!fz_is_failure_id("fuzz-arbitrary-error", 20));
    ISO_CHECK(!fz_is_failure_id("fuzz-crash\0x", 12));
    ISO_CHECK(!fz_is_failure_id("fuzz-crash ", 11));
    ISO_CHECK(!fz_is_failure_id("", 0));
    ISO_CHECK(!fz_is_failure_id("FUZZ-CRASH", 10));
    ISO_CHECK(!fz_is_failure_id("property-counterexample", 23));
    /* the pinned corpus line */
    ISO_CHECK(fz_format_tap_failure(buf, sizeof buf, 1, "ct_compare", 10, "fuzz-crash", 10, "3a8f19b2", 8) == FZ_OK);
    ISO_CHECK_EQ_INT(strcmp(buf, "not ok 1 - ct_compare [fuzz-crash:3a8f19b2]"), 0);
    /* control characters and newlines in the target cannot forge a line: each byte becomes '_' */
    ISO_CHECK(fz_format_tap_failure(buf, sizeof buf, 7, "a\nnot ok 9 - x\r\t b\x01", 19, "fuzz-oom", 8, "00ff00ff", 8) ==
              FZ_OK);
    ISO_CHECK(strchr(buf, '\n') == NULL && strchr(buf, '\r') == NULL && strchr(buf, '\t') == NULL);
    ISO_CHECK_EQ_INT(strncmp(buf, "not ok 7 - a_not_ok_9_-_x___b_ [fuzz-oom:00ff00ff]", 20), 0);
    ISO_CHECK_EQ_INT(strcmp(buf, "not ok 7 - a_not_ok_9_-_x___b_ [fuzz-oom:00ff00ff]"), 0);
    /* an embedded NUL and non-ASCII bytes are replaced too */
    ISO_CHECK(fz_format_tap_failure(buf, sizeof buf, 1, "a\0b\xc3\xa9", 5, "fuzz-oom", 8, "00ff00ff", 8) == FZ_OK);
    ISO_CHECK_EQ_INT(strcmp(buf, "not ok 1 - a_b__ [fuzz-oom:00ff00ff]"), 0);
    /* the target is truncated to 128 bytes */
    {
        char longt[300];
        memset(longt, 'x', sizeof longt);
        ISO_CHECK(fz_format_tap_failure(buf, sizeof buf, 1, longt, 300, "fuzz-oom", 8, "00ff00ff", 8) == FZ_OK);
        ISO_CHECK_EQ_INT(strlen(buf), strlen("not ok 1 -  [fuzz-oom:00ff00ff]") + 128);
    }
    /* refusals: unregistered id, uppercase / short / long / non-hex signature */
    ISO_CHECK_EQ_INT(fz_format_tap_failure(buf, sizeof buf, 1, "t", 1, "fuzz-nope", 9, "00ff00ff", 8), FZ_E_ARG);
    ISO_CHECK_EQ_INT(fz_format_tap_failure(buf, sizeof buf, 1, "t", 1, "fuzz-oom", 8, "00FF00FF", 8), FZ_E_ARG);
    ISO_CHECK_EQ_INT(fz_format_tap_failure(buf, sizeof buf, 1, "t", 1, "fuzz-oom", 8, "00ff00f", 7), FZ_E_ARG);
    ISO_CHECK_EQ_INT(fz_format_tap_failure(buf, sizeof buf, 1, "t", 1, "fuzz-oom", 8, "00ff00ff0", 9), FZ_E_ARG);
    ISO_CHECK_EQ_INT(fz_format_tap_failure(buf, sizeof buf, 1, "t", 1, "fuzz-oom", 8, "00ff00fg", 8), FZ_E_ARG);
    /* a buffer too small is a limit error, never an overrun */
    {
        char small[10];
        ISO_CHECK_EQ_INT(fz_format_tap_failure(small, sizeof small, 1, "ct_compare", 10, "fuzz-crash", 10, "3a8f19b2", 8),
                         FZ_E_LIMIT);
        ISO_CHECK_EQ_INT(small[0], 0);
    }
}

/* ---- crash normalisation ----------------------------------------------------------------------------------- */

static void test_crash_normalize(void)
{
    fz_crash c;
    size_t i;
    char sig_a[9];
    char sig_b[9];
    memset(&c, 0, sizeof c);
    fz_crash_set(&c, "bad:kind with spaces\n");
    ISO_CHECK_EQ_INT(strcmp(c.kind, "bad_kind_with_spaces_"), 0);
    fz_crash_push(&c, "f:g");
    fz_crash_push(&c, "f:g"); /* consecutive repeat collapses */
    fz_crash_push(&c, "");    /* empty is dropped */
    fz_crash_push(&c, "h");
    fz_crash_push(&c, NULL);
    ISO_CHECK_EQ_INT(c.nframes, 2);
    ISO_CHECK_EQ_INT(strcmp(c.frames[0], "f_g"), 0);
    /* a long kind and frame are cut */
    {
        char longs[200];
        memset(longs, 'k', sizeof longs - 1);
        longs[sizeof longs - 1] = '\0';
        memset(&c, 0, sizeof c);
        fz_crash_set(&c, longs);
        fz_crash_push(&c, longs);
        ISO_CHECK_EQ_INT(strlen(c.kind), FZ_KIND_MAX);
        ISO_CHECK_EQ_INT(strlen(c.frames[0]), FZ_FRAME_MAX);
    }
    /* more than 64 frames: the innermost 64 are kept */
    memset(&c, 0, sizeof c);
    for (i = 0; i < 100; i++) {
        char f[16];
        (void)snprintf(f, sizeof f, "fn%lu", (unsigned long)i);
        fz_crash_push(&c, f);
    }
    ISO_CHECK_EQ_INT(c.nframes, FZ_MAX_FRAMES);
    ISO_CHECK_EQ_INT(strcmp(c.frames[63], "fn63"), 0);
    /* a recursive crash (the same frame at any depth) hashes alike */
    {
        fz_crash r1;
        fz_crash r2;
        size_t d;
        memset(&r1, 0, sizeof r1);
        memset(&r2, 0, sizeof r2);
        fz_crash_set(&r1, "stack");
        fz_crash_set(&r2, "stack");
        for (d = 0; d < 3; d++) {
            fz_crash_push(&r1, "rec");
        }
        for (d = 0; d < 50; d++) {
            fz_crash_push(&r2, "rec");
        }
        fz_crash_push(&r1, "main");
        fz_crash_push(&r2, "main");
        fz_crash_signature_of(sig_a, &r1);
        fz_crash_signature_of(sig_b, &r2);
        ISO_CHECK_EQ_INT(strcmp(sig_a, sig_b), 0);
    }
    /* a hostile struct written directly: unterminated strings, a wild frame count, ':' and empties */
    memset(&c, 'A', sizeof c);
    c.nframes = (size_t)-1;
    fz_crash_normalize(&c);
    ISO_CHECK(strlen(c.kind) <= FZ_KIND_MAX);
    ISO_CHECK(c.nframes <= FZ_MAX_FRAMES);
    for (i = 0; i < c.nframes; i++) {
        ISO_CHECK(strlen(c.frames[i]) <= FZ_FRAME_MAX && strchr(c.frames[i], ':') == NULL);
    }
    memset(&c, 0, sizeof c);
    fz_crash_normalize(&c);
    ISO_CHECK_EQ_INT(strcmp(c.kind, "unspecified"), 0);
    ISO_CHECK_EQ_INT(c.nframes, 0);
    /* signature of an empty-kind normalised crash equals the oracle value of "unspecified" with no frames */
    fz_crash_signature_of(sig_a, &c);
    ISO_CHECK_EQ_INT(strcmp(sig_a, "e2f83322"), 0);
    /* the corpus-style duplicate check */
    {
        fz_str k = {"panic", 5};
        fz_str fa[2] = {{"eval", 4}, {"step", 4}};
        fz_str fb[2] = {{"alloc", 5}, {"write", 5}};
        fz_str k2 = {"segv", 4};
        ISO_CHECK(fz_is_duplicate_crash(k, fa, 2, k, fa, 2));
        ISO_CHECK(!fz_is_duplicate_crash(k, fa, 2, k2, fb, 2));
        ISO_CHECK(!fz_is_duplicate_crash(k, fa, 2, k, fa, 1));
    }
}

static void test_edge_id(void)
{
    ISO_CHECK(fz_edge_id("parse") == pbt_fnv1a64("parse", 5));
    ISO_CHECK(fz_edge_id("a") != fz_edge_id("b"));
    ISO_CHECK(fz_edge_id("") == 0xcbf29ce484222325u);
}

int main(void)
{
    test_signatures();
    test_energy();
    test_constants();
    test_every_table_entry_matters();
    test_splice_unequal_and_swapped();
    test_mutator_edges();
    test_sequence_work();
    test_bitmap();
    test_bitmap_never_outgrows_the_map();
    test_max_len_clamped();
    test_null_empty_inputs();
    test_corpus_admit();
    test_tap();
    test_crash_normalize();
    test_edge_id();
    fz_hooks_reset();
    ISO_CHECK_EQ_INT(pbt_hooks.live_allocs, 0);
    return ISO_TEST_RESULT();
}
