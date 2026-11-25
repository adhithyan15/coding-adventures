/*
 * test_engine.c -- the generator algebra, integrated shrinking and the honest
 * runner (VER05 sections 3, 4, 7; VER05i sections 4, 5, 7).
 *
 * Every test is bounded by construction (finite case counts, finite shrink
 * budgets), so a regression FAILS rather than hangs.
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "iso_test.h"
#include "pbt.h"
#include "pbt_hooks.h"
#include "pbt_internal.h"
#include "test_util.h"

/* ---- small helpers -------------------------------------------------------------------- */

static pbt_big I(long v)
{
    pbt_big b;
    pbt_big_set_i64(&b, v);
    return b;
}

static long as_long(const pbt_val *v)
{
    return (long)pbt_big_to_i64(&v->u.i);
}

static pbt_run new_run(uint64_t seed)
{
    pbt_run r;
    pbt_rng_init(&r.rng, seed);
    r.budget = NULL;
    r.depth = 0;
    return r;
}

/* ---- properties ------------------------------------------------------------------------ */

static int p_lt(const pbt_val *v, long t)
{
    return pbt_big_cmp(&v->u.i, &(pbt_big){0, 0, {0}}) == 0 ? 0 < t : as_long(v) < t;
}

typedef struct counter {
    long calls;
    long threshold;
} counter;

/* holds iff x < threshold */
static int prop_int_lt(void *user, pbt_budget *scope, const pbt_val *v)
{
    counter *c = (counter *)user;
    (void)scope;
    c->calls++;
    return p_lt(v, c->threshold);
}

/* holds iff x > threshold (fails for small values) */
static int prop_int_gt(void *user, pbt_budget *scope, const pbt_val *v)
{
    counter *c = (counter *)user;
    (void)scope;
    c->calls++;
    return as_long(v) > c->threshold;
}

static int prop_always_holds(void *user, pbt_budget *scope, const pbt_val *v)
{
    (void)user;
    (void)scope;
    (void)v;
    return 1;
}

static int prop_always_fails(void *user, pbt_budget *scope, const pbt_val *v)
{
    (void)user;
    (void)scope;
    (void)v;
    return 0;
}

static int prop_errors(void *user, pbt_budget *scope, const pbt_val *v)
{
    (void)user;
    (void)scope;
    (void)v;
    return -1;
}

static pbt_prop mk(pbt_prop_fn fn, void *user)
{
    pbt_prop p;
    p.fn = fn;
    p.user = user;
    return p;
}

static pbt_options opts(int n, int steps, pbt_ci_mode ci)
{
    pbt_options o;
    o.num_tests = n;
    o.max_shrink_steps = steps;
    o.ci = ci;
    return o;
}

/* ============================ generators ============================ */

static void test_primitive_ranges(void)
{
    pbt_run run = new_run(1);
    pbt_gen *g;
    int i;
    int seen_true = 0;
    int seen_false = 0;
    int hit_lo = 0;
    int hit_hi = 0;
    int neg = 0;
    int pos = 0;

    g = pbt_gen_bool();
    for (i = 0; i < 100; i++) {
        pbt_val v;
        ISO_CHECK(pbt_gen_sample(&run, g, &v) == 0);
        ISO_CHECK(v.kind == PBT_V_BOOL);
        if (v.u.b) {
            seen_true = 1;
        } else {
            seen_false = 1;
        }
    }
    ISO_CHECK(seen_true && seen_false);
    pbt_gen_release(g);

    g = pbt_gen_u8();
    for (i = 0; i < 2000; i++) {
        pbt_val v;
        ISO_CHECK(pbt_gen_sample(&run, g, &v) == 0);
        ISO_CHECK(v.kind == PBT_V_INT && as_long(&v) >= 0 && as_long(&v) <= 255);
    }
    pbt_gen_release(g);

    /* int(-5, 5) is INCLUSIVE at both ends. */
    {
        pbt_big lo = I(-5);
        pbt_big hi = I(5);
        g = pbt_gen_int(&lo, &hi);
    }
    for (i = 0; i < 3000; i++) {
        pbt_val v;
        ISO_CHECK(pbt_gen_sample(&run, g, &v) == 0);
        ISO_CHECK(as_long(&v) >= -5 && as_long(&v) <= 5);
        hit_lo |= as_long(&v) == -5;
        hit_hi |= as_long(&v) == 5;
    }
    ISO_CHECK(hit_lo && hit_hi);
    pbt_gen_release(g);

    /* A single-value range is constant (and still consumes a word). */
    g = pbt_gen_int_i64(7, 7);
    {
        pbt_rng twin = run.rng;
        pbt_val v;
        ISO_CHECK(pbt_gen_sample(&run, g, &v) == 0);
        ISO_CHECK(as_long(&v) == 7);
        (void)pbt_rng_next(&twin);
        ISO_CHECK(run.rng.state == twin.state);
    }
    pbt_gen_release(g);

    /* The full int64 range, and i128's sign. */
    g = pbt_gen_int_i64(INT64_MIN, INT64_MAX);
    for (i = 0; i < 200; i++) {
        pbt_val v;
        ISO_CHECK(pbt_gen_sample(&run, g, &v) == 0);
        ISO_CHECK(pbt_big_fits_i64(&v.u.i));
    }
    pbt_gen_release(g);
    g = pbt_gen_i128();
    for (i = 0; i < 400; i++) {
        pbt_val v;
        ISO_CHECK(pbt_gen_sample(&run, g, &v) == 0);
        ISO_CHECK(pbt_big_bitlen(&v.u.i) <= 128);
        if (pbt_big_sign(&v.u.i) < 0) {
            neg = 1;
        } else {
            pos = 1;
        }
    }
    ISO_CHECK(neg && pos);
    pbt_gen_release(g);

    /* Beyond 128 bits: [-2^200, 2^200]. */
    {
        pbt_big hi;
        pbt_big lo;
        (void)pbt_big_pow2(&hi, 200);
        pbt_big_neg(&lo, &hi);
        g = pbt_gen_int(&lo, &hi);
        ISO_CHECK(g != NULL);
        for (i = 0; i < 100; i++) {
            pbt_val v;
            ISO_CHECK(pbt_gen_sample(&run, g, &v) == 0);
            ISO_CHECK(pbt_big_cmp(&v.u.i, &lo) >= 0 && pbt_big_cmp(&v.u.i, &hi) <= 0);
        }
        pbt_gen_release(g);
    }

    /* Invalid ranges are refused with NULL. */
    {
        pbt_big a = I(5);
        pbt_big b = I(4);
        pbt_big huge;
        (void)pbt_big_pow2(&huge, 1100 > 1151 ? 1151 : 1100);
        ISO_CHECK(pbt_gen_int(&a, &b) == NULL);
        ISO_CHECK(pbt_gen_int(NULL, &b) == NULL);
        ISO_CHECK(pbt_gen_int(&a, NULL) == NULL);
        ISO_CHECK(pbt_gen_int(&a, &huge) == NULL); /* more than 1000 bits */
        ISO_CHECK(pbt_gen_int_i64(3, 2) == NULL);
    }
}

static void test_bytes_and_list_lengths(void)
{
    pbt_run run = new_run(11);
    pbt_gen *g;
    pbt_gen *u8;
    int i;
    int seen_len[6] = {0, 0, 0, 0, 0, 0};

    g = pbt_gen_bytes(2, 5);
    for (i = 0; i < 600; i++) {
        pbt_val v;
        ISO_CHECK(pbt_gen_sample(&run, g, &v) == 0);
        ISO_CHECK(v.kind == PBT_V_BYTES && v.u.bytes.n >= 2 && v.u.bytes.n <= 5);
        seen_len[v.u.bytes.n]++;
        pbt_val_free(&v);
    }
    ISO_CHECK(seen_len[2] > 0 && seen_len[3] > 0 && seen_len[4] > 0 && seen_len[5] > 0); /* inclusive */
    pbt_gen_release(g);

    g = pbt_gen_bytes(4, 4);
    {
        pbt_val v;
        pbt_rng twin = run.rng;
        ISO_CHECK(pbt_gen_sample(&run, g, &v) == 0);
        ISO_CHECK_EQ_UINT(v.u.bytes.n, 4);
        /* The fixed length still consumes one word, then four byte draws. */
        (void)pbt_rng_bounded(&twin, 1);
        for (i = 0; i < 4; i++) {
            (void)pbt_rng_bounded(&twin, 256);
        }
        ISO_CHECK(twin.state == run.rng.state);
        pbt_val_free(&v);
    }
    pbt_gen_release(g);

    ISO_CHECK(pbt_gen_bytes(5, 4) == NULL);
    ISO_CHECK(pbt_gen_bytes(0, PBT_MAX_INPUT_BYTES + 1) == NULL);

    u8 = pbt_gen_u8();
    g = pbt_gen_list(u8, 1, 3);
    memset(seen_len, 0, sizeof seen_len);
    for (i = 0; i < 600; i++) {
        pbt_val v;
        ISO_CHECK(pbt_gen_sample(&run, g, &v) == 0);
        ISO_CHECK(v.kind == PBT_V_LIST && v.u.list.n >= 1 && v.u.list.n <= 3);
        seen_len[v.u.list.n]++;
        pbt_val_free(&v);
    }
    ISO_CHECK(seen_len[1] > 0 && seen_len[2] > 0 && seen_len[3] > 0);
    pbt_gen_release(g);
    ISO_CHECK(pbt_gen_list(u8, 3, 2) == NULL);
    ISO_CHECK(pbt_gen_list(u8, 0, PBT_MAX_LIST_LEN + 1) == NULL);
    ISO_CHECK(pbt_gen_list(NULL, 0, 3) == NULL);
    pbt_gen_release(u8);
}

static int count_calls(void *self, pbt_budget *b, const pbt_val *v)
{
    (void)b;
    (void)v;
    ++*(long *)self;
    return 0;
}

static int keep_even(void *self, pbt_budget *b, const pbt_val *v)
{
    (void)self;
    (void)b;
    return as_long(v) % 2 == 0;
}

static int keep_all(void *self, pbt_budget *b, const pbt_val *v)
{
    (void)self;
    (void)b;
    (void)v;
    return 1;
}

static int filter_errors(void *self, pbt_budget *b, const pbt_val *v)
{
    (void)self;
    (void)b;
    (void)v;
    return -1;
}

static void test_filter(void)
{
    pbt_run run = new_run(5);
    pbt_gen *u8 = pbt_gen_u8();
    pbt_gen *g;
    pbt_val v;
    long calls = 0;
    int i;

    /* After EXACTLY max_retries rejected samples: exhausted. */
    g = pbt_gen_filter(u8, count_calls, &calls, 100);
    ISO_CHECK(g != NULL);
    ISO_CHECK(pbt_gen_sample(&run, g, &v) == PBT_E_EXHAUSTED);
    ISO_CHECK(v.kind == PBT_V_NIL);
    ISO_CHECK_EQ_INT(calls, 100);
    pbt_gen_release(g);

    g = pbt_gen_filter(u8, keep_even, NULL, 100);
    for (i = 0; i < 200; i++) {
        ISO_CHECK(pbt_gen_sample(&run, g, &v) == 0);
        ISO_CHECK(as_long(&v) % 2 == 0);
    }
    pbt_gen_release(g);

    /* A predicate error is a generator error. */
    g = pbt_gen_filter(u8, filter_errors, NULL, 5);
    ISO_CHECK(pbt_gen_sample(&run, g, &v) == PBT_E_GEN);
    pbt_gen_release(g);

    /* Retry bounds. */
    ISO_CHECK(pbt_gen_filter(u8, keep_all, NULL, 0) == NULL);
    ISO_CHECK(pbt_gen_filter(u8, keep_all, NULL, PBT_MAX_FILTER_RETRIES + 1) == NULL);
    ISO_CHECK(pbt_gen_filter(u8, NULL, NULL, 5) == NULL);
    ISO_CHECK(pbt_gen_filter(NULL, keep_all, NULL, 5) == NULL);
    g = pbt_gen_filter(u8, keep_all, NULL, PBT_MAX_FILTER_RETRIES);
    ISO_CHECK(g != NULL);
    pbt_gen_release(g);

    /* Shrinking is preserved and the predicate is honoured: only EVEN values are valid shrinks. */
    {
        pbt_big lo = I(0);
        pbt_big hi = I(1000);
        pbt_gen *ints = pbt_gen_int(&lo, &hi);
        pbt_gen *evens = pbt_gen_filter(ints, keep_even, NULL, 100);
        counter c;
        pbt_verdict vd;
        c.calls = 0;
        c.threshold = 10;
        ISO_CHECK(pbt_check(NULL, 3, evens, mk(prop_int_lt, &c), &(pbt_options){100, 1000, PBT_CI_OFF}, NULL, &vd) == 0);
        ISO_CHECK(!vd.passed);
        ISO_CHECK(vd.failure_id != NULL && strcmp(vd.failure_id, PBT_FAIL_COUNTEREXAMPLE) == 0);
        /* Only EVEN candidates are valid shrinks, so the walk can stall above 10, but never on an odd or passing value. */
        ISO_CHECK(vd.minimal.kind == PBT_V_INT && as_long(&vd.minimal) >= 10 && as_long(&vd.minimal) % 2 == 0);
        pbt_verdict_free(&vd);
        pbt_gen_release(evens);
        pbt_gen_release(ints);
    }
    pbt_gen_release(u8);
}

static int double_it(void *self, const pbt_val *in, pbt_val *out)
{
    (void)self;
    pbt_val_set_i64(out, as_long(in) * 2);
    return 0;
}

static int map_fails(void *self, const pbt_val *in, pbt_val *out)
{
    (void)self;
    (void)in;
    (void)out;
    return -3;
}

/* flat_map: a length n in [1,4], then a list of exactly n small ints. */
static int make_list_gen(void *self, const pbt_val *in, pbt_gen **out)
{
    pbt_gen *u8 = pbt_gen_u8();
    size_t n = (size_t)as_long(in);
    (void)self;
    *out = pbt_gen_list(u8, n, n);
    pbt_gen_release(u8);
    return *out == NULL ? PBT_E_NOMEM : 0;
}

/* A flat_map that returns a generator built from ITSELF: unbounded recursion must stop at the depth limit. */
static pbt_gen *g_self_ref;
static int recurse_forever(void *self, const pbt_val *in, pbt_gen **out)
{
    (void)self;
    (void)in;
    pbt_gen_retain(g_self_ref);
    *out = g_self_ref;
    return 0;
}

static void test_map_flat_map_constant(void)
{
    pbt_run run = new_run(8);
    pbt_gen *u8 = pbt_gen_u8();
    pbt_gen *m;
    pbt_gen *f;
    pbt_val v;
    int i;
    pbt_rng twin = run.rng;

    m = pbt_gen_map(u8, double_it, NULL);
    for (i = 0; i < 100; i++) {
        long raw = (long)pbt_rng_bounded(&twin, 256);
        ISO_CHECK(pbt_gen_sample(&run, m, &v) == 0);
        ISO_CHECK(as_long(&v) == raw * 2); /* no extra draws */
    }
    pbt_gen_release(m);
    m = pbt_gen_map(u8, map_fails, NULL);
    ISO_CHECK(pbt_gen_sample(&run, m, &v) == PBT_E_GEN);
    pbt_gen_release(m);
    ISO_CHECK(pbt_gen_map(NULL, double_it, NULL) == NULL);
    ISO_CHECK(pbt_gen_map(u8, NULL, NULL) == NULL);

    {
        pbt_big lo = I(1);
        pbt_big hi = I(4);
        pbt_gen *len = pbt_gen_int(&lo, &hi);
        f = pbt_gen_flat_map(len, make_list_gen, NULL);
        pbt_gen_release(len);
    }
    for (i = 0; i < 100; i++) {
        ISO_CHECK(pbt_gen_sample(&run, f, &v) == 0);
        ISO_CHECK(v.kind == PBT_V_LIST && v.u.list.n >= 1 && v.u.list.n <= 4);
        pbt_val_free(&v);
    }
    pbt_gen_release(f);
    ISO_CHECK(pbt_gen_flat_map(NULL, make_list_gen, NULL) == NULL);

    /* Unbounded recursion is a LIMIT error, not a stack overflow. */
    g_self_ref = pbt_gen_flat_map(u8, recurse_forever, NULL);
    ISO_CHECK(g_self_ref != NULL);
    {
        pbt_val r;
        int rc = pbt_gen_sample(&run, g_self_ref, &r);
        ISO_CHECK(rc == PBT_E_LIMIT);
        ISO_CHECK(run.depth == 0); /* the depth counter unwinds */
    }
    pbt_gen_release(g_self_ref);
    g_self_ref = NULL;

    /* constant: clones, draws nothing. */
    {
        pbt_val c;
        pbt_gen *k;
        pbt_rng before;
        pbt_val_set_i64(&c, 42);
        k = pbt_gen_constant(&c);
        before = run.rng;
        ISO_CHECK(pbt_gen_sample(&run, k, &v) == 0);
        ISO_CHECK(as_long(&v) == 42);
        ISO_CHECK(run.rng.state == before.state);
        pbt_gen_release(k);
        ISO_CHECK(pbt_gen_constant(NULL) == NULL);
        ISO_CHECK(pbt_val_set_bytes(&c, (const uint8_t *)"xy", 2) == 0);
        k = pbt_gen_constant(&c);
        pbt_val_free(&c); /* the generator owns its own copy */
        ISO_CHECK(pbt_gen_sample(&run, k, &v) == 0);
        ISO_CHECK(v.kind == PBT_V_BYTES && v.u.bytes.n == 2 && v.u.bytes.p[0] == 'x');
        pbt_val_free(&v);
        pbt_gen_release(k);
    }
    pbt_gen_release(u8);
}

static void test_choice(void)
{
    pbt_run run = new_run(77);
    pbt_gen *a;
    pbt_gen *b;
    pbt_gen *c;
    pbt_gen *g;
    pbt_val va;
    pbt_val vb;
    pbt_val vc;
    pbt_gen *arms3[3];
    int hits[3] = {0, 0, 0};
    int i;
    pbt_arm arms[3];

    pbt_val_set_i64(&va, 0);
    pbt_val_set_i64(&vb, 1);
    pbt_val_set_i64(&vc, 2);
    a = pbt_gen_constant(&va);
    b = pbt_gen_constant(&vb);
    c = pbt_gen_constant(&vc);
    arms3[0] = a;
    arms3[1] = b;
    arms3[2] = c;
    g = pbt_gen_one_of(arms3, 3);
    ISO_CHECK(g != NULL);
    for (i = 0; i < 3000; i++) {
        pbt_val v;
        ISO_CHECK(pbt_gen_sample(&run, g, &v) == 0);
        hits[as_long(&v)]++;
    }
    for (i = 0; i < 3; i++) {
        ISO_CHECK(hits[i] > 850 && hits[i] < 1150); /* uniform */
    }
    pbt_gen_release(g);
    ISO_CHECK(pbt_gen_one_of(arms3, 0) == NULL);
    ISO_CHECK(pbt_gen_one_of(NULL, 3) == NULL);
    arms3[1] = NULL;
    ISO_CHECK(pbt_gen_one_of(arms3, 3) == NULL);
    arms3[1] = b;

    /* frequency follows its weights and NEVER selects a zero-weight arm. */
    arms[0].weight = 1;
    arms[0].gen = a;
    arms[1].weight = 0;
    arms[1].gen = b;
    arms[2].weight = 3;
    arms[2].gen = c;
    g = pbt_gen_frequency(arms, 3);
    ISO_CHECK(g != NULL);
    hits[0] = hits[1] = hits[2] = 0;
    for (i = 0; i < 4000; i++) {
        pbt_val v;
        ISO_CHECK(pbt_gen_sample(&run, g, &v) == 0);
        hits[as_long(&v)]++;
    }
    ISO_CHECK_EQ_INT(hits[1], 0);
    ISO_CHECK(hits[0] > 850 && hits[0] < 1150);
    ISO_CHECK(hits[2] > 2850 && hits[2] < 3150);
    pbt_gen_release(g);

    /* Weight validation: all-zero, a single weight above 2^32, a total above 2^32. */
    arms[0].weight = 0;
    arms[2].weight = 0;
    ISO_CHECK(pbt_gen_frequency(arms, 3) == NULL);
    arms[0].weight = PBT_MAX_FREQUENCY_TOTAL + 1;
    ISO_CHECK(pbt_gen_frequency(arms, 3) == NULL);
    arms[0].weight = PBT_MAX_FREQUENCY_TOTAL;
    arms[2].weight = 1;
    ISO_CHECK(pbt_gen_frequency(arms, 3) == NULL);
    arms[2].weight = 0;
    g = pbt_gen_frequency(arms, 3); /* exactly 2^32 is the largest legal total */
    ISO_CHECK(g != NULL);
    pbt_gen_release(g);
    ISO_CHECK(pbt_gen_frequency(NULL, 3) == NULL);
    ISO_CHECK(pbt_gen_frequency(arms, 0) == NULL);

    pbt_gen_release(a);
    pbt_gen_release(b);
    pbt_gen_release(c);
    pbt_val_free(&va);
}

static int g_destroyed;
static void count_destroy(void *self)
{
    (void)self;
    g_destroyed++;
}

static int const_sample(void *self, pbt_run *run, pbt_val *out)
{
    (void)self;
    (void)run;
    pbt_val_set_i64(out, 1);
    return 0;
}

static void test_ownership(void)
{
    pbt_gen *inner;
    pbt_gen *list;
    pbt_run run = new_run(1);
    pbt_val v;
    g_destroyed = 0;
    ISO_CHECK(pbt_gen_custom(NULL, NULL, NULL, NULL) == NULL);
    inner = pbt_gen_custom(const_sample, NULL, NULL, count_destroy);
    list = pbt_gen_list(inner, 1, 1); /* borrows: retains inner */
    pbt_gen_release(inner);           /* the creator lets go; the list still holds one reference */
    ISO_CHECK_EQ_INT(g_destroyed, 0);
    ISO_CHECK(pbt_gen_sample(&run, list, &v) == 0);
    pbt_val_free(&v);
    pbt_gen_release(list);
    ISO_CHECK_EQ_INT(g_destroyed, 1); /* destroyed exactly once, when the last reference went */
    pbt_gen_retain(NULL);             /* harmless */
    pbt_gen_release(NULL);
    {
        pbt_gen *g = pbt_gen_custom(const_sample, NULL, NULL, count_destroy);
        pbt_gen_retain(g);
        pbt_gen_release(g);
        ISO_CHECK_EQ_INT(g_destroyed, 1);
        pbt_gen_release(g);
        ISO_CHECK_EQ_INT(g_destroyed, 2);
    }
}

static void test_tuple(void)
{
    pbt_run run = new_run(4);
    pbt_gen *u8 = pbt_gen_u8();
    pbt_gen *bytes = pbt_gen_bytes(1, 1);
    pbt_gen *two[2];
    pbt_gen *nine[9];
    pbt_gen *g;
    pbt_val v;
    pbt_rng twin = run.rng;
    size_t i;
    two[0] = u8;
    two[1] = bytes;
    g = pbt_gen_tuple(two, 2);
    ISO_CHECK(g != NULL);
    ISO_CHECK(pbt_gen_sample(&run, g, &v) == 0);
    ISO_CHECK(v.kind == PBT_V_TUPLE && v.u.list.n == 2);
    /* Components are drawn left to right. */
    ISO_CHECK(as_long(&v.u.list.items[0]) == (long)pbt_rng_bounded(&twin, 256));
    ISO_CHECK(v.u.list.items[1].kind == PBT_V_BYTES);
    pbt_val_free(&v);
    pbt_gen_release(g);
    for (i = 0; i < 9; i++) {
        nine[i] = u8;
    }
    g = pbt_gen_tuple(nine, 8);
    ISO_CHECK(g != NULL);
    pbt_gen_release(g);
    ISO_CHECK(pbt_gen_tuple(nine, 9) == NULL);
    ISO_CHECK(pbt_gen_tuple(nine, 0) == NULL);
    ISO_CHECK(pbt_gen_tuple(NULL, 2) == NULL);
    nine[0] = NULL;
    ISO_CHECK(pbt_gen_tuple(nine, 2) == NULL);
    pbt_gen_release(bytes);
    pbt_gen_release(u8);
}

/* fails iff a + b >= 10 (as a tuple of two ints) */
static int prop_sum_lt_10(void *user, pbt_budget *scope, const pbt_val *v)
{
    (void)user;
    (void)scope;
    return as_long(&v->u.list.items[0]) + as_long(&v->u.list.items[1]) < 10;
}

static void test_tuple_shrinks_coordinatewise(void)
{
    pbt_big lo = I(0);
    pbt_big hi = I(1000);
    pbt_gen *ints = pbt_gen_int(&lo, &hi);
    pbt_gen *pair[2];
    pbt_gen *g;
    pbt_verdict vd;
    pair[0] = ints;
    pair[1] = ints;
    g = pbt_gen_tuple(pair, 2);
    ISO_CHECK(pbt_check(NULL, 17, g, mk(prop_sum_lt_10, NULL), &(pbt_options){200, 1000, PBT_CI_OFF}, NULL, &vd) == 0);
    ISO_CHECK(!vd.passed);
    ISO_CHECK(vd.minimal.kind == PBT_V_TUPLE);
    if (vd.minimal.kind == PBT_V_TUPLE) {
        long a = as_long(&vd.minimal.u.list.items[0]);
        long b = as_long(&vd.minimal.u.list.items[1]);
        ISO_CHECK(a + b == 10); /* a local minimum of a monotone property is its threshold */
        ISO_CHECK(a >= 0 && b >= 0);
    }
    ISO_CHECK(vd.shrink_steps > 0);
    pbt_verdict_free(&vd);
    pbt_gen_release(g);
    pbt_gen_release(ints);
}

/* ============================ shrinking ============================ */

/* fails iff x >= t (holds iff x < t) over [lo, hi] */
static void shrink_threshold(long t, long start, long lo, long hi, long want, int *steps_out)
{
    counter c;
    pbt_big minimal;
    pbt_big b_lo = I(lo);
    pbt_big b_hi = I(hi);
    pbt_big b_start = I(start);
    pbt_big target = I(0);
    int steps = 0;
    int rc;
    c.calls = 0;
    c.threshold = t;
    rc = pbt_shrink_int(NULL, &b_start, &target, &b_lo, &b_hi, mk(prop_int_lt, &c), 1000, &minimal, &steps);
    ISO_CHECK(rc == 0);
    ISO_CHECK(pbt_big_to_i64(&minimal) == want);
    if (steps_out != NULL) {
        *steps_out = steps;
    }
}

static void test_monotone_thresholds_exact(void)
{
    long t;
    long start;
    /* For a MONOTONE property the result is exactly the threshold from every failing start. */
    for (t = 1; t <= 120; t++) {
        for (start = t; start <= t + 260; start++) {
            shrink_threshold(t, start, -1000, 1000, t, NULL);
        }
    }
    /* The mirror image: holds iff x > -t, so the threshold is -t, approached from below. */
    for (t = 1; t <= 120; t++) {
        for (start = -t; start >= -t - 260; start--) {
            counter c;
            pbt_big minimal;
            pbt_big b_lo = I(-1000);
            pbt_big b_hi = I(1000);
            pbt_big b_start = I(start);
            pbt_big target = I(0);
            int steps = 0;
            c.calls = 0;
            c.threshold = -t;
            ISO_CHECK(pbt_shrink_int(NULL, &b_start, &target, &b_lo, &b_hi, mk(prop_int_gt, &c), 1000, &minimal, &steps) == 0);
            ISO_CHECK(pbt_big_to_i64(&minimal) == -t);
        }
    }
}

static int prop_is_odd_holds(void *user, pbt_budget *scope, const pbt_val *v)
{
    (void)user;
    (void)scope;
    return as_long(v) % 2 != 0; /* fails on EVEN values: a non-monotone failing set */
}

static void test_arbitrary_failing_set_is_a_local_minimum(void)
{
    long start;
    for (start = -200; start <= 200; start += 2) {
        pbt_big minimal;
        pbt_big lo = I(-500);
        pbt_big hi = I(500);
        pbt_big b_start = I(start);
        pbt_big target = I(0);
        int steps = 0;
        long m;
        ISO_CHECK(pbt_shrink_int(NULL, &b_start, &target, &lo, &hi, mk(prop_is_odd_holds, NULL), 1000, &minimal, &steps) == 0);
        m = (long)pbt_big_to_i64(&minimal);
        ISO_CHECK(m % 2 == 0);                           /* still falsifies */
        ISO_CHECK(m >= -500 && m <= 500);                /* inside the bounds */
        ISO_CHECK(labs(m) <= labs(start));               /* never farther from the target */
        ISO_CHECK(m == 0);                               /* the target itself fails, one step */
    }
}

static int prop_gt_min_holds(void *user, pbt_budget *scope, const pbt_val *v)
{
    (void)user;
    (void)scope;
    (void)v;
    return 0;
}

static void test_extremes_and_errors(void)
{
    pbt_big lo;
    pbt_big hi;
    pbt_big one = I(1);
    pbt_big minimal;
    pbt_big target;
    pbt_big start;
    int steps;
    counter c;
    c.calls = 0;
    c.threshold = 0;
    (void)pbt_big_pow2(&hi, 127);
    pbt_big_neg(&lo, &hi);
    (void)pbt_big_sub(&hi, &hi, &one);

    /* i128 extremes: start at one end with the opposite extreme as target, and the reverse. */
    steps = 0;
    ISO_CHECK(pbt_shrink_int(NULL, &hi, &lo, &lo, &hi, mk(prop_gt_min_holds, NULL), 1000, &minimal, &steps) == 0);
    ISO_CHECK(pbt_big_cmp(&minimal, &lo) == 0 && steps == 1);
    ISO_CHECK(pbt_shrink_int(NULL, &lo, &hi, &lo, &hi, mk(prop_gt_min_holds, NULL), 1000, &minimal, &steps) == 0);
    ISO_CHECK(pbt_big_cmp(&minimal, &hi) == 0 && steps == 1);
    /* Walking the full span: fails iff x != target (target holds), from each extreme. */
    target = I(0);
    steps = 0;
    ISO_CHECK(pbt_shrink_int(NULL, &hi, &target, &lo, &hi, mk(prop_gt_min_holds, NULL), 1000, &minimal, &steps) == 0);
    ISO_CHECK(pbt_big_is_zero(&minimal));

    /* NULL arguments are refused, not dereferenced. */
    ISO_CHECK(pbt_shrink_int(NULL, NULL, &target, &lo, &hi, mk(prop_gt_min_holds, NULL), 10, &minimal, &steps) == PBT_E_ARG);
    ISO_CHECK(pbt_shrink_int(NULL, &hi, NULL, &lo, &hi, mk(prop_gt_min_holds, NULL), 10, &minimal, &steps) == PBT_E_ARG);
    ISO_CHECK(pbt_shrink_int(NULL, &hi, &target, NULL, &hi, mk(prop_gt_min_holds, NULL), 10, &minimal, &steps) == PBT_E_ARG);
    ISO_CHECK(pbt_shrink_int(NULL, &hi, &target, &lo, NULL, mk(prop_gt_min_holds, NULL), 10, &minimal, &steps) == PBT_E_ARG);
    ISO_CHECK(pbt_shrink_int(NULL, &hi, &target, &lo, &hi, mk(prop_gt_min_holds, NULL), 10, NULL, &steps) == PBT_E_ARG);
    ISO_CHECK(pbt_shrink_int(NULL, &hi, &target, &lo, &hi, mk(prop_gt_min_holds, NULL), 10, &minimal, NULL) == PBT_E_ARG);
    /* A negative step count cannot wrap into a huge attempt cap. */
    ISO_CHECK(pbt_attempt_budget(-1) == pbt_attempt_budget(0));
    ISO_CHECK(pbt_attempt_budget(-2147483647 - 1) == PBT_ATTEMPT_SLACK);
    ISO_CHECK(pbt_attempt_budget(10) == 16 * 10 + PBT_ATTEMPT_SLACK);

    /* A budget of zero returns the start unchanged. */
    start = I(900);
    ISO_CHECK(pbt_shrink_int(NULL, &start, &target, &lo, &hi, mk(prop_gt_min_holds, NULL), 0, &minimal, &steps) == 0);
    ISO_CHECK(pbt_big_cmp(&minimal, &start) == 0 && steps == 0);
    /* A budget of one: one accepted move only. */
    c.threshold = 10;
    ISO_CHECK(pbt_shrink_int(NULL, &start, &target, &lo, &hi, mk(prop_int_lt, &c), 1, &minimal, &steps) == 0);
    ISO_CHECK(steps == 1 && as_long(&(pbt_val){PBT_V_INT, {0}}) == 0 ? 1 : steps == 1);

    /* Errors: out-of-range budget, lo > hi, an initial outside [lo, hi], a start that HOLDS, a property error, no property. */
    ISO_CHECK(pbt_shrink_int(NULL, &start, &target, &lo, &hi, mk(prop_gt_min_holds, NULL), PBT_MAX_SHRINK_STEPS + 1, &minimal, &steps) == PBT_E_ARG);
    ISO_CHECK(pbt_shrink_int(NULL, &start, &target, &lo, &hi, mk(prop_gt_min_holds, NULL), -1, &minimal, &steps) == PBT_E_ARG);
    ISO_CHECK(pbt_shrink_int(NULL, &start, &target, &hi, &lo, mk(prop_gt_min_holds, NULL), 10, &minimal, &steps) == PBT_E_ARG);
    {
        pbt_big small_hi = I(100);
        ISO_CHECK(pbt_shrink_int(NULL, &start, &target, &lo, &small_hi, mk(prop_gt_min_holds, NULL), 10, &minimal, &steps) == PBT_E_ARG);
    }
    ISO_CHECK(pbt_shrink_int(NULL, &start, &target, &lo, &hi, mk(prop_always_holds, NULL), 10, &minimal, &steps) == PBT_E_NOT_FALSIFYING);
    ISO_CHECK(pbt_shrink_int(NULL, &start, &target, &lo, &hi, mk(prop_errors, NULL), 10, &minimal, &steps) == PBT_E_GEN);
    {
        pbt_prop none;
        none.fn = NULL;
        none.user = NULL;
        ISO_CHECK(pbt_shrink_int(NULL, &start, &target, &lo, &hi, none, 10, &minimal, &steps) == PBT_E_ARG);
    }
}

/* ---- list shrinking: properties over a list of ints ------------------------------------------ */

typedef int (*list_fn)(const pbt_val *l);

static int l_len_lt_2(const pbt_val *l) { return l->u.list.n < 2; }
static int l_no_three(const pbt_val *l)
{
    size_t i;
    for (i = 0; i < l->u.list.n; i++) {
        if (as_long(&l->u.list.items[i]) == 3) {
            return 0;
        }
    }
    return 1;
}
static int l_sum_le_3(const pbt_val *l)
{
    long s = 0;
    size_t i;
    for (i = 0; i < l->u.list.n; i++) {
        s += as_long(&l->u.list.items[i]);
    }
    return s <= 3;
}
static int l_no_adjacent_equal(const pbt_val *l)
{
    size_t i;
    for (i = 0; i + 1 < l->u.list.n; i++) {
        if (as_long(&l->u.list.items[i]) == as_long(&l->u.list.items[i + 1])) {
            return 0;
        }
    }
    return 1;
}
static int l_all_below_2(const pbt_val *l)
{
    size_t i;
    for (i = 0; i < l->u.list.n; i++) {
        if (as_long(&l->u.list.items[i]) >= 2) {
            return 0;
        }
    }
    return 1;
}
static int l_sum_lt_5_or_short(const pbt_val *l)
{
    long s = 0;
    size_t i;
    for (i = 0; i < l->u.list.n; i++) {
        s += as_long(&l->u.list.items[i]);
    }
    return s < 5;
}

static list_fn g_list_fn;
static int list_prop_adapter(void *user, pbt_budget *scope, const pbt_val *v)
{
    (void)user;
    (void)scope;
    return g_list_fn(v);
}

/* Every list of length 0..4 over {0,1,2,3}: 341 of them. */
static void test_list_one_minimality(void)
{
    static const list_fn shapes[] = {l_len_lt_2, l_no_three, l_sum_le_3, l_no_adjacent_equal, l_all_below_2,
                                     l_sum_lt_5_or_short};
    size_t s;
    for (s = 0; s < sizeof shapes / sizeof shapes[0]; s++) {
        size_t len;
        int checked = 0;
        g_list_fn = shapes[s];
        for (len = 0; len <= 4; len++) {
            long total = 1;
            long code;
            size_t k;
            for (k = 0; k < len; k++) {
                total *= 4;
            }
            for (code = 0; code < total; code++) {
                pbt_val l;
                pbt_val orig;
                long rest = code;
                int steps = 0;
                size_t i;
                ISO_CHECK(pbt_val_set_list(&l, PBT_V_LIST, len) == 0);
                for (i = 0; i < len; i++) {
                    pbt_val_set_i64(&l.u.list.items[i], rest % 4);
                    rest /= 4;
                }
                if (shapes[s](&l)) { /* holds: nothing to shrink */
                    pbt_val_free(&l);
                    continue;
                }
                checked++;
                ISO_CHECK(pbt_val_clone(&orig, &l) == 0);
                {
                    pbt_big zero = I(0);
                    ISO_CHECK(pbt_shrink_list_ints(NULL, &l, &zero, mk(list_prop_adapter, NULL), 1000, &steps) == 0);
                }
                ISO_CHECK(!shapes[s](&l));                           /* the result still falsifies */
                ISO_CHECK(l.u.list.n <= orig.u.list.n);               /* never longer */
                for (i = 0; i < l.u.list.n; i++) {
                    ISO_CHECK(as_long(&l.u.list.items[i]) >= 0 && as_long(&l.u.list.items[i]) <= 3);
                }
                /* 1-minimal by deletion: removing any single element makes the property hold. */
                for (i = 0; i < l.u.list.n; i++) {
                    pbt_val cand;
                    size_t j;
                    size_t w = 0;
                    ISO_CHECK(pbt_val_set_list(&cand, PBT_V_LIST, l.u.list.n - 1) == 0);
                    for (j = 0; j < l.u.list.n; j++) {
                        if (j != i) {
                            cand.u.list.items[w++] = l.u.list.items[j];
                        }
                    }
                    if (!shapes[s](&cand)) {
                        printf("  shape %lu: not 1-minimal after shrinking code %ld len %lu\n", (unsigned long)s, code,
                               (unsigned long)len);
                    }
                    ISO_CHECK(shapes[s](&cand));
                    pbt_free(cand.u.list.items); /* the elements are shared with l: free only the array */
                }
                pbt_val_free(&l);
                pbt_val_free(&orig);
            }
        }
        ISO_CHECK(checked > 0);
    }
}

/* A property that records every view it is shown, so the order of candidates is observable. */
typedef struct log_ctx {
    char text[512];
    size_t n;
    int contains;
} log_ctx;

static int prop_logs_contains_9(void *user, pbt_budget *scope, const pbt_val *v)
{
    log_ctx *c = (log_ctx *)user;
    size_t i;
    int found = 0;
    (void)scope;
    for (i = 0; i < v->u.list.n; i++) {
        if (as_long(&v->u.list.items[i]) == 9) {
            found = 1;
        }
    }
    if (c->n + v->u.list.n + 2 < sizeof c->text) {
        for (i = 0; i < v->u.list.n; i++) {
            c->text[c->n++] = (char)('0' + as_long(&v->u.list.items[i]));
        }
        c->text[c->n++] = '|';
        c->text[c->n] = '\0';
    }
    return !found; /* fails iff the list contains 9 */
}

static pbt_val list_of(const long *xs, size_t n)
{
    pbt_val l;
    size_t i;
    ISO_CHECK(pbt_val_set_list(&l, PBT_V_LIST, n) == 0);
    for (i = 0; i < n; i++) {
        pbt_val_set_i64(&l.u.list.items[i], xs[i]);
    }
    return l;
}

static int prop_sum_lt_18(void *user, pbt_budget *scope, const pbt_val *v)
{
    long s = 0;
    size_t i;
    (void)user;
    (void)scope;
    for (i = 0; i < v->u.list.n; i++) {
        s += as_long(&v->u.list.items[i]);
    }
    return s < 18;
}

/* BOTH bisection halves: the left half is tried first, then the right. */
static void test_both_bisection_halves(void)
{
    pbt_big zero = I(0);
    int steps = 0;
    log_ctx c;
    pbt_val l;
    static const long left_case[] = {9, 1, 1, 1, 1, 1, 1, 1};
    static const long right_case[] = {1, 1, 1, 1, 1, 1, 1, 9};

    /* The 9 sits in the LEFT half: the left half is accepted at once, then again, then again. */
    memset(&c, 0, sizeof c);
    l = list_of(left_case, 8);
    ISO_CHECK(pbt_shrink_list_ints(NULL, &l, &zero, mk(prop_logs_contains_9, &c), 1000, &steps) == 0);
    ISO_CHECK(l.u.list.n == 1 && as_long(&l.u.list.items[0]) == 9);
    ISO_CHECK_EQ_INT(steps, 3);
    /* The log starts: the initial check, the empty probe, then left halves only. */
    ISO_CHECK(strncmp(c.text, "91111111||9111|91|9|", 20) == 0);
    pbt_val_free(&l);

    /* The 9 sits in the RIGHT half: the left half is REJECTED, the right half accepted. */
    memset(&c, 0, sizeof c);
    l = list_of(right_case, 8);
    ISO_CHECK(pbt_shrink_list_ints(NULL, &l, &zero, mk(prop_logs_contains_9, &c), 1000, &steps) == 0);
    ISO_CHECK(l.u.list.n == 1 && as_long(&l.u.list.items[0]) == 9);
    ISO_CHECK_EQ_INT(steps, 3); /* three bisections, no deletion phase needed */
    ISO_CHECK(strncmp(c.text, "11111119||1111|1119|11|19|", 25) == 0);
    pbt_val_free(&l);

    /* Neither half fails on its own: the shrinker falls through to single deletion, right to left. */
    {
        static const long mid_case[] = {9, 0, 0, 9};
        memset(&c, 0, sizeof c);
        l = list_of(mid_case, 4);
        ISO_CHECK(pbt_shrink_list_ints(NULL, &l, &zero, mk(prop_sum_lt_18, NULL), 1000, &steps) == 0);
        ISO_CHECK(l.u.list.n == 2 && as_long(&l.u.list.items[0]) == 9 && as_long(&l.u.list.items[1]) == 9);
        ISO_CHECK_EQ_INT(steps, 2); /* two single deletions */
        pbt_val_free(&l);
    }
}

/* A rejected candidate must leave the working value EXACTLY as it was (the in-place views are restored). */
typedef struct exact {
    const uint8_t *data;
    size_t n;
} exact;

static int prop_fails_only_original(void *user, pbt_budget *scope, const pbt_val *v)
{
    exact *e = (exact *)user;
    (void)scope;
    /* Fails (returns 0) ONLY for the original content; every other candidate holds and is rejected. */
    return !(v->u.bytes.n == e->n && memcmp(v->u.bytes.p, e->data, e->n) == 0);
}

static void test_rejected_candidates_leave_the_value_intact(void)
{
    uint8_t data[40];
    pbt_val b;
    exact e;
    int steps = 0;
    size_t i;
    for (i = 0; i < 40; i++) {
        data[i] = (uint8_t)(i * 7 + 1);
    }
    e.data = data;
    e.n = 40;
    ISO_CHECK(pbt_val_set_bytes(&b, data, 40) == 0);
    ISO_CHECK(pbt_shrink_bytes(NULL, &b, mk(prop_fails_only_original, &e), 1000, &steps) == 0);
    ISO_CHECK_EQ_INT(steps, 0);
    ISO_CHECK_EQ_UINT(b.u.bytes.n, 40);
    ISO_CHECK(memcmp(b.u.bytes.p, data, 40) == 0);
    pbt_val_free(&b);
}

/* min_len is respected by list and bytes generators. */
static int prop_bytes_sum_lt_5(void *user, pbt_budget *scope, const pbt_val *v)
{
    long s = 0;
    size_t i;
    (void)user;
    (void)scope;
    for (i = 0; i < v->u.bytes.n; i++) {
        s += v->u.bytes.p[i];
    }
    return s < 5;
}

static int prop_list_sum_lt_5(void *user, pbt_budget *scope, const pbt_val *v)
{
    long s = 0;
    size_t i;
    (void)user;
    (void)scope;
    for (i = 0; i < v->u.list.n; i++) {
        s += as_long(&v->u.list.items[i]);
    }
    return s < 5;
}

static void test_min_len_and_bounds_respected(void)
{
    pbt_gen *g;
    pbt_gen *u8;
    pbt_verdict vd;
    pbt_options o = opts(100, 1000, PBT_CI_OFF);

    g = pbt_gen_bytes(3, 12);
    ISO_CHECK(pbt_check(NULL, 5, g, mk(prop_bytes_sum_lt_5, NULL), &o, NULL, &vd) == 0);
    ISO_CHECK(!vd.passed && vd.minimal.kind == PBT_V_BYTES);
    ISO_CHECK(vd.minimal.u.bytes.n >= 3); /* never below min_len */
    {
        long s = 0;
        size_t i;
        for (i = 0; i < vd.minimal.u.bytes.n; i++) {
            s += vd.minimal.u.bytes.p[i];
        }
        ISO_CHECK(s >= 5);
    }
    pbt_verdict_free(&vd);
    pbt_gen_release(g);

    u8 = pbt_gen_u8();
    g = pbt_gen_list(u8, 2, 9);
    ISO_CHECK(pbt_check(NULL, 5, g, mk(prop_list_sum_lt_5, NULL), &o, NULL, &vd) == 0);
    ISO_CHECK(!vd.passed && vd.minimal.kind == PBT_V_LIST);
    ISO_CHECK(vd.minimal.u.list.n >= 2);
    pbt_verdict_free(&vd);
    pbt_gen_release(g);
    pbt_gen_release(u8);

    /* int(5, 10) never shrinks below 5, even when every value fails. */
    {
        pbt_big lo = I(5);
        pbt_big hi = I(10);
        g = pbt_gen_int(&lo, &hi);
        ISO_CHECK(pbt_check(NULL, 3, g, mk(prop_always_fails, NULL), &o, NULL, &vd) == 0);
        ISO_CHECK(as_long(&vd.minimal) == 5);
        pbt_verdict_free(&vd);
        pbt_gen_release(g);
        lo = I(-10);
        hi = I(-3);
        g = pbt_gen_int(&lo, &hi);
        ISO_CHECK(pbt_check(NULL, 3, g, mk(prop_always_fails, NULL), &o, NULL, &vd) == 0);
        ISO_CHECK(as_long(&vd.minimal) == -3); /* the in-range point nearest zero */
        pbt_verdict_free(&vd);
        pbt_gen_release(g);
    }
}

static int prop_not_true(void *user, pbt_budget *scope, const pbt_val *v)
{
    (void)user;
    (void)scope;
    return !v->u.b; /* fails on true */
}

static int prop_true_only_when_false_is_ok(void *user, pbt_budget *scope, const pbt_val *v)
{
    (void)user;
    (void)scope;
    return v->u.b ? 0 : 1; /* same as not_true */
}

static void test_bool_and_unshrinkable(void)
{
    pbt_gen *g = pbt_gen_bool();
    pbt_verdict vd;
    pbt_options o = opts(50, 100, PBT_CI_OFF);
    ISO_CHECK(pbt_check(NULL, 1, g, mk(prop_not_true, NULL), &o, NULL, &vd) == 0);
    ISO_CHECK(!vd.passed && vd.minimal.kind == PBT_V_BOOL);
    /* true -> false would make the property HOLD, so it must stay true: shrinking steps 0. */
    ISO_CHECK(vd.minimal.u.b == 1 && vd.shrink_steps == 0);
    pbt_verdict_free(&vd);
    pbt_gen_release(g);
    /* A property false for everything: true shrinks to false. */
    g = pbt_gen_bool();
    ISO_CHECK(pbt_check(NULL, 1, g, mk(prop_always_fails, NULL), &o, NULL, &vd) == 0);
    ISO_CHECK(vd.minimal.u.b == 0);
    pbt_verdict_free(&vd);
    pbt_gen_release(g);
    (void)prop_true_only_when_false_is_ok;

    /* map / one_of / frequency / flat_map have no shrinker: the witness is reported unshrunk. */
    {
        pbt_gen *u8 = pbt_gen_u8();
        pbt_gen *m = pbt_gen_map(u8, double_it, NULL);
        counter c;
        c.calls = 0;
        c.threshold = 10;
        ISO_CHECK(pbt_check(NULL, 2, m, mk(prop_int_lt, &c), &o, NULL, &vd) == 0);
        ISO_CHECK(!vd.passed);
        ISO_CHECK(vd.shrink_steps == 0);
        ISO_CHECK(pbt_val_equal(&vd.minimal, &vd.counterexample));
        pbt_verdict_free(&vd);
        pbt_gen_release(m);
        pbt_gen_release(u8);
    }
}

/* ============================ the honest runner ============================ */

static void test_passes_and_basic_verdicts(void)
{
    pbt_big lo = I(0);
    pbt_big hi = I(1000);
    pbt_gen *g = pbt_gen_int(&lo, &hi);
    pbt_verdict vd;
    counter c;
    c.calls = 0;
    c.threshold = 5000;

    ISO_CHECK(pbt_check(NULL, 1, g, mk(prop_int_lt, &c), &(pbt_options){40, 100, PBT_CI_OFF}, NULL, &vd) == 0);
    ISO_CHECK(vd.passed);
    ISO_CHECK(vd.failure_id == NULL);
    ISO_CHECK_EQ_INT(vd.cases_run, 40);
    ISO_CHECK_EQ_INT(c.calls, 40);
    pbt_verdict_free(&vd);
    pbt_verdict_free(&vd); /* freeing twice is harmless */

    /* NULL options means defaults: 100 cases. */
    c.calls = 0;
    ISO_CHECK(pbt_check(NULL, 1, g, mk(prop_int_lt, &c), NULL, NULL, &vd) == 0);
    ISO_CHECK(vd.passed && c.calls == 100);
    pbt_verdict_free(&vd);

    /* A falsifiable property is NEVER a pass. */
    c.threshold = 500;
    ISO_CHECK(pbt_check(NULL, 1, g, mk(prop_int_lt, &c), &(pbt_options){200, 1000, PBT_CI_OFF}, NULL, &vd) == 0);
    ISO_CHECK(!vd.passed);
    ISO_CHECK(vd.failure_id != NULL && strcmp(vd.failure_id, PBT_FAIL_COUNTEREXAMPLE) == 0);
    ISO_CHECK(as_long(&vd.minimal) == 500);
    ISO_CHECK(as_long(&vd.counterexample) >= 500);
    ISO_CHECK(vd.has_seed && vd.seed == 1);
    ISO_CHECK(vd.iteration >= 1 && vd.cases_run == vd.iteration);
    ISO_CHECK(vd.message != NULL && strstr(vd.message, "seed=1") != NULL);
    ISO_CHECK(strstr(vd.message, "minimal=500") != NULL);
    pbt_verdict_free(&vd);

    /* The same seed reproduces the identical verdict. */
    {
        pbt_verdict a;
        pbt_verdict b;
        c.threshold = 500;
        ISO_CHECK(pbt_check(NULL, 99, g, mk(prop_int_lt, &c), &(pbt_options){200, 1000, PBT_CI_OFF}, NULL, &a) == 0);
        ISO_CHECK(pbt_check(NULL, 99, g, mk(prop_int_lt, &c), &(pbt_options){200, 1000, PBT_CI_OFF}, NULL, &b) == 0);
        ISO_CHECK_STR_EQ(a.message, b.message);
        ISO_CHECK(a.shrink_steps == b.shrink_steps && a.iteration == b.iteration);
        pbt_verdict_free(&a);
        pbt_verdict_free(&b);
    }
    pbt_gen_release(g);
}

static void test_zero_cases_and_invalid_options(void)
{
    pbt_gen *g = pbt_gen_u8();
    pbt_verdict vd;
    /* A property that never ran proves nothing. */
    ISO_CHECK(pbt_check(NULL, 1, g, mk(prop_always_holds, NULL), &(pbt_options){0, 100, PBT_CI_OFF}, NULL, &vd) == 0);
    ISO_CHECK(!vd.passed);
    ISO_CHECK(vd.failure_id != NULL && strcmp(vd.failure_id, PBT_FAIL_EXHAUSTED) == 0);
    pbt_verdict_free(&vd);

    /* Invalid options: a status AND a failing verdict, never a pass. */
    {
        pbt_options bad[4];
        int i;
        bad[0] = opts(-1, 10, PBT_CI_OFF);
        bad[1] = opts(PBT_MAX_CASES + 1, 10, PBT_CI_OFF);
        bad[2] = opts(10, -1, PBT_CI_OFF);
        bad[3] = opts(10, PBT_MAX_SHRINK_STEPS + 1, PBT_CI_OFF);
        for (i = 0; i < 4; i++) {
            ISO_CHECK(pbt_check(NULL, 1, g, mk(prop_always_holds, NULL), &bad[i], NULL, &vd) == PBT_E_ARG);
            ISO_CHECK(!vd.passed);
            ISO_CHECK(vd.failure_id != NULL);
            pbt_verdict_free(&vd);
        }
    }
    {
        pbt_prop none;
        none.fn = NULL;
        none.user = NULL;
        ISO_CHECK(pbt_check(NULL, 1, g, none, NULL, NULL, &vd) == PBT_E_ARG);
        ISO_CHECK(!vd.passed);
        pbt_verdict_free(&vd);
        ISO_CHECK(pbt_check(NULL, 1, NULL, mk(prop_always_holds, NULL), NULL, NULL, &vd) == PBT_E_ARG);
        ISO_CHECK(!vd.passed);
        pbt_verdict_free(&vd);
    }
    /* The largest legal option values are accepted. */
    ISO_CHECK(pbt_check(NULL, 1, g, mk(prop_always_holds, NULL), &(pbt_options){PBT_MAX_CASES, PBT_MAX_SHRINK_STEPS, PBT_CI_OFF}, NULL, &vd) == 0);
    ISO_CHECK(vd.passed);
    pbt_verdict_free(&vd);
    pbt_gen_release(g);
}

static void test_generator_error_and_property_error(void)
{
    pbt_gen *u8 = pbt_gen_u8();
    pbt_gen *never_ok = pbt_gen_filter(u8, count_calls, &(long){0}, 10);
    pbt_verdict vd;
    /* A filter that never accepts: property-exhausted, naming the iteration. */
    ISO_CHECK(pbt_check(NULL, 1, never_ok, mk(prop_always_holds, NULL), &(pbt_options){10, 10, PBT_CI_OFF}, NULL, &vd) == 0);
    ISO_CHECK(!vd.passed && strcmp(vd.failure_id, PBT_FAIL_EXHAUSTED) == 0);
    ISO_CHECK(vd.iteration == 1 && vd.cases_run == 0);
    ISO_CHECK(strstr(vd.message, "filter precondition") != NULL);
    pbt_verdict_free(&vd);
    pbt_gen_release(never_ok);

    /* A generator that reports an error (not exhaustion). */
    {
        pbt_gen *m = pbt_gen_map(u8, map_fails, NULL);
        ISO_CHECK(pbt_check(NULL, 1, m, mk(prop_always_holds, NULL), &(pbt_options){10, 10, PBT_CI_OFF}, NULL, &vd) == 0);
        ISO_CHECK(!vd.passed && strcmp(vd.failure_id, PBT_FAIL_EXHAUSTED) == 0);
        ISO_CHECK(strstr(vd.message, "generator error") != NULL);
        pbt_verdict_free(&vd);
        pbt_gen_release(m);
    }

    /* A property that errors: property-panic; the seed only in local mode. */
    ISO_CHECK(pbt_check(NULL, 7, u8, mk(prop_errors, NULL), &(pbt_options){10, 10, PBT_CI_OFF}, NULL, &vd) == 0);
    ISO_CHECK(!vd.passed && strcmp(vd.failure_id, PBT_FAIL_PANIC) == 0);
    ISO_CHECK(vd.has_seed && vd.seed == 7);
    pbt_verdict_free(&vd);
    ISO_CHECK(pbt_check(NULL, 7, u8, mk(prop_errors, NULL), &(pbt_options){10, 10, PBT_CI_ON}, NULL, &vd) == 0);
    ISO_CHECK(!vd.passed && strcmp(vd.failure_id, PBT_FAIL_PANIC) == 0);
    ISO_CHECK(!vd.has_seed);
    ISO_CHECK(strstr(vd.message, "seed") == NULL);
    pbt_verdict_free(&vd);
    pbt_gen_release(u8);
}

/* A property that errors on every value below 50 but only fails (returns 0) at 50 and above. */
static int prop_error_below_50_fails_at_or_above(void *user, pbt_budget *scope, const pbt_val *v)
{
    (void)user;
    (void)scope;
    return as_long(v) < 50 ? -1 : 0;
}

static void test_errors_while_shrinking_are_not_falsifying(void)
{
    pbt_big lo = I(50);
    pbt_big hi = I(1000);
    pbt_gen *g = pbt_gen_int(&lo, &hi);
    pbt_verdict vd;
    /* Every value fails, so the first case is falsified; smaller candidates error and are never accepted. */
    ISO_CHECK(pbt_check(NULL, 4, g, mk(prop_error_below_50_fails_at_or_above, NULL), &(pbt_options){10, 1000, PBT_CI_OFF}, NULL, &vd) == 0);
    ISO_CHECK(strcmp(vd.failure_id, PBT_FAIL_COUNTEREXAMPLE) == 0);
    ISO_CHECK(as_long(&vd.minimal) >= 50); /* a real counterexample, never an erroring value */
    pbt_verdict_free(&vd);
    pbt_gen_release(g);
}

/* ---- the shrinker as an untrusted party -------------------------------------------------------- */

typedef struct lie {
    int kind;
} lie;

static int lying_shrinker(void *self, pbt_budget *b, pbt_val *value, pbt_pred falsifies, int max_steps, int *steps)
{
    lie *l = (lie *)self;
    (void)b;
    (void)falsifies;
    (void)max_steps;
    switch (l->kind) {
    case 0: /* returns a value that no longer fails, claiming steps */
        pbt_val_set_i64(value, 1);
        *steps = 5;
        return 0;
    case 1: /* reports a nonsense step count for a genuinely failing value */
        *steps = 1000000;
        return 0;
    case 2: /* negative steps */
        *steps = -3;
        return 0;
    case 3: /* fails outright, leaving the value destroyed */
        pbt_val_free(value);
        *steps = 0;
        return PBT_E_GEN;
    case 4: /* an honest shrink to a smaller failing value */
        pbt_val_set_i64(value, 777);
        *steps = 2;
        return 0;
    default:
        return PBT_E_ARG;
    }
}

static void test_a_lying_shrinker_cannot_change_the_verdict(void)
{
    pbt_big lo = I(900);
    pbt_big hi = I(900);
    pbt_gen *g = pbt_gen_int(&lo, &hi);
    counter c;
    pbt_shrinker sh;
    lie l;
    pbt_verdict vd;
    int k;
    c.calls = 0;
    c.threshold = 500; /* holds iff x < 500: 900 fails, 777 fails, 1 holds */
    sh.fn = lying_shrinker;
    sh.self = &l;
    for (k = 0; k <= 3; k++) {
        l.kind = k;
        ISO_CHECK(pbt_check(NULL, 1, g, mk(prop_int_lt, &c), &(pbt_options){5, 100, PBT_CI_OFF}, &sh, &vd) == 0);
        ISO_CHECK(strcmp(vd.failure_id, PBT_FAIL_COUNTEREXAMPLE) == 0);
        /* The ORIGINAL candidate is reported, with zero steps. */
        ISO_CHECK(as_long(&vd.minimal) == 900);
        ISO_CHECK(as_long(&vd.counterexample) == 900);
        ISO_CHECK_EQ_INT(vd.shrink_steps, 0);
        pbt_verdict_free(&vd);
    }
    /* An honest result that really fails IS adopted. */
    l.kind = 4;
    ISO_CHECK(pbt_check(NULL, 1, g, mk(prop_int_lt, &c), &(pbt_options){5, 100, PBT_CI_OFF}, &sh, &vd) == 0);
    ISO_CHECK(as_long(&vd.minimal) == 777 && as_long(&vd.counterexample) == 900 && vd.shrink_steps == 2);
    pbt_verdict_free(&vd);
    /* An out-of-range step count is rejected even when the value is fine: max steps 1 < 2. */
    ISO_CHECK(pbt_check(NULL, 1, g, mk(prop_int_lt, &c), &(pbt_options){5, 1, PBT_CI_OFF}, &sh, &vd) == 0);
    ISO_CHECK(as_long(&vd.minimal) == 900 && vd.shrink_steps == 0);
    pbt_verdict_free(&vd);
    pbt_gen_release(g);
}

/* ---- CI redaction ---------------------------------------------------------------------------------- */

static void test_ci_helper(void)
{
    ISO_CHECK(!pbt_ci_from_value(NULL));
    ISO_CHECK(!pbt_ci_from_value(""));
    ISO_CHECK(!pbt_ci_from_value("0"));
    ISO_CHECK(!pbt_ci_from_value("false"));
    ISO_CHECK(!pbt_ci_from_value("FALSE"));
    ISO_CHECK(!pbt_ci_from_value("  False \n"));
    ISO_CHECK(!pbt_ci_from_value("   "));
    ISO_CHECK(pbt_ci_from_value("true"));
    ISO_CHECK(pbt_ci_from_value("1"));
    ISO_CHECK(pbt_ci_from_value("yes"));
    ISO_CHECK(pbt_ci_from_value("falsey"));
    ISO_CHECK(pbt_ci_from_value("00"));
    ISO_CHECK(pbt_ci_from_value("a long enough value to overflow the small buffer"));
}

static void test_redaction_leaks_nothing(void)
{
    pbt_big lo = I(987654321);
    pbt_big hi = I(987654321);
    pbt_gen *g = pbt_gen_int(&lo, &hi);
    pbt_ci_mode modes[2];
    int m;
    modes[0] = PBT_CI_UNSET; /* the zero value fails closed */
    modes[1] = PBT_CI_ON;
    for (m = 0; m < 2; m++) {
        pbt_verdict vd;
        char want[17];
        char pre[100];
        uint64_t h;
        pbt_options o;
        o.num_tests = 5;
        o.max_shrink_steps = 100;
        o.ci = modes[m];
        ISO_CHECK(pbt_check(NULL, 123456789, g, mk(prop_always_fails, NULL), &o, NULL, &vd) == 0);
        ISO_CHECK(!vd.passed && strcmp(vd.failure_id, PBT_FAIL_COUNTEREXAMPLE) == 0);
        ISO_CHECK(!vd.has_seed && vd.seed == 0);
        ISO_CHECK(vd.counterexample.kind == PBT_V_NIL && vd.minimal.kind == PBT_V_NIL);
        ISO_CHECK(vd.message != NULL);
        ISO_CHECK(strstr(vd.message, "987654321") == NULL); /* no witness */
        ISO_CHECK(strstr(vd.message, "123456789") == NULL); /* no seed */
        ISO_CHECK(strstr(vd.message, "seed") == NULL);
        ISO_CHECK(strstr(vd.message, "initial") == NULL && strstr(vd.message, "minimal") == NULL);
        /* The only thing carried is the digest of iter;steps;shrunk (a correlation id, not a secret). */
        ISO_CHECK_EQ_UINT(strlen(vd.digest), 16);
        (void)snprintf(pre, sizeof pre, "iter:%d;steps:%d;shrunk:987654321", vd.iteration, vd.shrink_steps);
        h = pbt_fnv1a64(pre, strlen(pre));
        (void)snprintf(want, sizeof want, "%016llx", (unsigned long long)h);
        ISO_CHECK_STR_EQ(vd.digest, want);
        ISO_CHECK(strstr(vd.message, want) != NULL);
        pbt_verdict_free(&vd);
    }
    /* Local mode shows both. */
    {
        pbt_verdict vd;
        ISO_CHECK(pbt_check(NULL, 123456789, g, mk(prop_always_fails, NULL), &(pbt_options){5, 100, PBT_CI_OFF}, NULL, &vd) == 0);
        ISO_CHECK(vd.has_seed && vd.seed == 123456789);
        ISO_CHECK(strstr(vd.message, "987654321") != NULL && strstr(vd.message, "seed=123456789") != NULL);
        ISO_CHECK(vd.digest[0] == '\0');
        pbt_verdict_free(&vd);
    }
    pbt_gen_release(g);
}

static void test_hostile_witnesses_still_get_a_verdict(void)
{
    pbt_verdict vd;
    pbt_gen *g;
    pbt_val deep;
    pbt_val *slot;
    int i;

    /* A 1 MiB witness. */
    g = pbt_gen_bytes(PBT_MAX_INPUT_BYTES, PBT_MAX_INPUT_BYTES);
    ISO_CHECK(g != NULL);
    ISO_CHECK(pbt_check(NULL, 3, g, mk(prop_always_fails, NULL), &(pbt_options){1, 1000, PBT_CI_OFF}, NULL, &vd) == 0);
    ISO_CHECK(strcmp(vd.failure_id, PBT_FAIL_COUNTEREXAMPLE) == 0);
    ISO_CHECK(vd.message != NULL && strlen(vd.message) < 2 * PBT_CANON_MAX_BYTES + 200);
    pbt_verdict_free(&vd);
    ISO_CHECK(pbt_check(NULL, 3, g, mk(prop_always_fails, NULL), &(pbt_options){1, 1000, PBT_CI_ON}, NULL, &vd) == 0);
    ISO_CHECK(strlen(vd.digest) == 16);
    pbt_verdict_free(&vd);
    pbt_gen_release(g);

    /* A witness nested far deeper than the renderer's depth limit. */
    ISO_CHECK(pbt_val_set_list(&deep, PBT_V_LIST, 1) == 0);
    slot = &deep;
    for (i = 0; i < 50; i++) {
        ISO_CHECK(pbt_val_set_list(&slot->u.list.items[0], PBT_V_LIST, 1) == 0);
        slot = &slot->u.list.items[0];
    }
    g = pbt_gen_constant(&deep);
    pbt_val_free(&deep);
    ISO_CHECK(g != NULL);
    ISO_CHECK(pbt_check(NULL, 3, g, mk(prop_always_fails, NULL), &(pbt_options){1, 10, PBT_CI_OFF}, NULL, &vd) == 0);
    ISO_CHECK(strcmp(vd.failure_id, PBT_FAIL_COUNTEREXAMPLE) == 0);
    ISO_CHECK(strstr(vd.message, "...") != NULL);
    pbt_verdict_free(&vd);
    pbt_gen_release(g);
}

/* ---- standalone entry points -------------------------------------------------------------------------- */

static int prop_bytes_has_no_zero(void *user, pbt_budget *scope, const pbt_val *v)
{
    size_t i;
    (void)user;
    (void)scope;
    for (i = 0; i < v->u.bytes.n; i++) {
        if (v->u.bytes.p[i] == 0) {
            return 0;
        }
    }
    return 1;
}

static int prop_list_empty(void *user, pbt_budget *scope, const pbt_val *v)
{
    (void)user;
    (void)scope;
    return v->u.list.n == 0;
}

static void test_standalone_shrinkers(void)
{
    pbt_val b;
    pbt_val l;
    pbt_big zero = I(0);
    int steps = 0;
    uint8_t raw[4] = {5, 0, 7, 9};
    pbt_big a;
    pbt_big m;

    ISO_CHECK(pbt_val_set_bytes(&b, raw, 4) == 0);
    ISO_CHECK(pbt_shrink_bytes(NULL, &b, mk(prop_bytes_has_no_zero, NULL), 1000, &steps) == 0);
    ISO_CHECK(b.u.bytes.n == 1 && b.u.bytes.p[0] == 0); /* the zero byte is isolated */
    pbt_val_free(&b);
    /* A value that holds is a caller error. */
    {
        uint8_t ok[2] = {1, 2};
        ISO_CHECK(pbt_val_set_bytes(&b, ok, 2) == 0);
        ISO_CHECK(pbt_shrink_bytes(NULL, &b, mk(prop_bytes_has_no_zero, NULL), 1000, &steps) == PBT_E_NOT_FALSIFYING);
        ISO_CHECK(pbt_shrink_bytes(NULL, &b, mk(prop_errors, NULL), 1000, &steps) == PBT_E_GEN);
        ISO_CHECK(pbt_shrink_bytes(NULL, &b, mk(prop_bytes_has_no_zero, NULL), -1, &steps) == PBT_E_ARG);
        pbt_val_free(&b);
        pbt_val_set_i64(&b, 1);
        ISO_CHECK(pbt_shrink_bytes(NULL, &b, mk(prop_bytes_has_no_zero, NULL), 10, &steps) == PBT_E_ARG); /* not bytes */
    }

    /* Lists must hold ints. */
    {
        static const long xs[] = {4, 5};
        l = list_of(xs, 2);
        ISO_CHECK(pbt_shrink_list_ints(NULL, &l, &zero, mk(prop_list_empty, NULL), 1000, &steps) == 0);
        /* [4,5] fails (non-empty); the empty list holds, so: left half [4] (1 step), then 4 -> 2 -> 1 -> 0. */
        ISO_CHECK(l.u.list.n == 1 && as_long(&l.u.list.items[0]) == 0 && steps == 4);
        pbt_val_free(&l);
        l = list_of(xs, 2);
        pbt_val_set_bool(&l.u.list.items[1], 1);
        ISO_CHECK(pbt_shrink_list_ints(NULL, &l, &zero, mk(prop_list_empty, NULL), 1000, &steps) == PBT_E_ARG);
        pbt_val_free(&l);
        pbt_val_set_i64(&b, 3);
        ISO_CHECK(pbt_shrink_list_ints(NULL, &b, &zero, mk(prop_list_empty, NULL), 1000, &steps) == PBT_E_ARG);
    }

    /* Replay: a property no draw falsifies is refused. */
    ISO_CHECK(pbt_simulate_replay(NULL, 1, mk(prop_always_holds, NULL), &a, &m) == PBT_E_ARG);
    {
        pbt_prop none;
        none.fn = NULL;
        none.user = NULL;
        ISO_CHECK(pbt_simulate_replay(NULL, 1, none, &a, &m) == PBT_E_ARG);
    }
    /* A replay is a pure function of the seed. */
    {
        counter c;
        pbt_big a2;
        pbt_big m2;
        c.calls = 0;
        c.threshold = 100;
        ISO_CHECK(pbt_simulate_replay(NULL, 42, mk(prop_int_lt, &c), &a, &m) == 0);
        ISO_CHECK(pbt_simulate_replay(NULL, 42, mk(prop_int_lt, &c), &a2, &m2) == 0);
        ISO_CHECK(pbt_big_cmp(&a, &a2) == 0 && pbt_big_cmp(&m, &m2) == 0);
        ISO_CHECK(pbt_big_to_i64(&a) == 413 + 0 || pbt_big_to_i64(&a) >= 100);
        ISO_CHECK(pbt_big_to_i64(&m) == 100);
    }
}

int main(void)
{
    pbt_hooks_reset();
    test_primitive_ranges();
    test_bytes_and_list_lengths();
    test_filter();
    test_map_flat_map_constant();
    test_choice();
    test_ownership();
    test_tuple();
    test_tuple_shrinks_coordinatewise();
    test_monotone_thresholds_exact();
    test_arbitrary_failing_set_is_a_local_minimum();
    test_extremes_and_errors();
    test_list_one_minimality();
    test_both_bisection_halves();
    test_rejected_candidates_leave_the_value_intact();
    test_min_len_and_bounds_respected();
    test_bool_and_unshrinkable();
    test_passes_and_basic_verdicts();
    test_zero_cases_and_invalid_options();
    test_generator_error_and_property_error();
    test_errors_while_shrinking_are_not_falsifying();
    test_a_lying_shrinker_cannot_change_the_verdict();
    test_ci_helper();
    test_redaction_leaks_nothing();
    test_hostile_witnesses_still_get_a_verdict();
    test_standalone_shrinkers();
    ISO_CHECK_EQ_INT(pbt_hooks.live_allocs, 0);
    return ISO_TEST_RESULT();
}
