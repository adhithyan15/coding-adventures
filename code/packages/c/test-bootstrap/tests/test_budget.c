/*
 * test_budget.c -- the hierarchical shrink-budget tree (VER05e section 4.1,
 * VER05i section 6), ported from the Java ShrinkBudgetTest minus the tests that
 * are about threads (there are none here: the C engine is single-threaded and
 * has no ambient scope, so the budget is an explicit parameter).
 *
 * The property under test is always the same: whatever a property, a custom
 * shrinker or a filter predicate does, however it nests, the TOTAL work of one
 * top-level check stays inside the outermost root.  Observables come from the
 * test seams: pbt_hooks.last_attempts / last_work are the totals of the most
 * recently CLOSED owning scope, which for a top-level call is its root.
 */
#include <stdio.h>
#include <string.h>

#include "iso_test.h"
#include "pbt.h"
#include "pbt_hooks.h"
#include "pbt_internal.h"
#include "pbt_shrink_internal.h"
#include "test_util.h"

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

static pbt_prop mk(pbt_prop_fn fn, void *user)
{
    pbt_prop p;
    p.fn = fn;
    p.user = user;
    return p;
}

static pbt_pred pred(pbt_falsifies_fn fn, void *self)
{
    pbt_pred p;
    p.fn = fn;
    p.self = self;
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

static pbt_val list_of_n(size_t n, long x)
{
    pbt_val l;
    size_t i;
    ISO_CHECK(pbt_val_set_list(&l, PBT_V_LIST, n) == 0);
    for (i = 0; i < n; i++) {
        pbt_val_set_i64(&l.u.list.items[i], x);
    }
    return l;
}

static pbt_gen *u8_list(size_t lo, size_t hi)
{
    pbt_gen *u8 = pbt_gen_u8();
    pbt_gen *g = pbt_gen_list(u8, lo, hi);
    pbt_gen_release(u8);
    return g;
}

/* "Still fails" iff the list has exactly n elements; counts every evaluation. */
typedef struct size_ctx {
    size_t n;
    long calls;
} size_ctx;

static int size_is(void *self, pbt_budget *b, const pbt_val *v)
{
    size_ctx *c = (size_ctx *)self;
    (void)b;
    c->calls++;
    return v->kind == PBT_V_LIST && v->u.list.n == c->n;
}

/* ============================ the node itself ============================ */

static void test_budget_semantics(void)
{
    pbt_budget st;
    pbt_budget st2;
    pbt_budget *b;
    pbt_budget *nested;
    /* A work cap of 10: 6 + 4 fits, 6 + 6 does not (all or nothing). */
    b = pbt_budget_open_child_cap(&st, NULL, 1000, 10);
    ISO_CHECK(b == &st);
    ISO_CHECK(!pbt_budget_is_nested(b));
    ISO_CHECK(pbt_budget_charge(b, 6));
    ISO_CHECK(!pbt_budget_charge(b, 6)); /* 12 > 10 */
    ISO_CHECK(pbt_budget_charge(b, 4));
    ISO_CHECK(!pbt_budget_has_room(b));
    ISO_CHECK_EQ_UINT(pbt_budget_attempts(b), 2);
    ISO_CHECK_EQ_UINT(pbt_budget_work(b), 10);
    ISO_CHECK(pbt_budget_was_refused(b));
    /* A child of a spent budget cannot be afforded either, and the parent is unchanged by the refusal. */
    nested = pbt_budget_open_child_cap(&st2, b, 5, PBT_MAX_SHRINK_WORK);
    ISO_CHECK(nested == &st2 && pbt_budget_is_nested(nested));
    ISO_CHECK(!pbt_budget_charge(nested, 1));
    ISO_CHECK_EQ_UINT(pbt_budget_work(nested), 0);
    ISO_CHECK_EQ_UINT(pbt_budget_work(b), 10);
    pbt_budget_close(&st2);
    pbt_budget_close(&st);
    ISO_CHECK_EQ_UINT(pbt_hooks.last_work, 10);
    ISO_CHECK_EQ_UINT(pbt_hooks.last_attempts, 2);
    pbt_budget_close(&st); /* closing twice is harmless */
}

static void test_charge_copy_charges_work_not_attempts(void)
{
    pbt_budget st;
    pbt_budget *b = pbt_budget_open_child_cap(&st, NULL, 1000, 10);
    ISO_CHECK(pbt_budget_charge_copy(b, 8));
    ISO_CHECK_EQ_UINT(pbt_budget_attempts(b), 0);
    ISO_CHECK_EQ_UINT(pbt_budget_work(b), 8);
    ISO_CHECK(!pbt_budget_charge_copy(b, 3));
    ISO_CHECK(pbt_budget_charge_copy(b, 0)); /* at least one element per charge */
    ISO_CHECK_EQ_UINT(pbt_budget_work(b), 9);
    pbt_budget_close(&st);
}

static void test_a_refused_take_leaves_every_counter_unchanged(void)
{
    pbt_budget root;
    pbt_budget child;
    pbt_budget *r = pbt_budget_open_child_cap(&root, NULL, 1000, 100);
    pbt_budget *c = pbt_budget_open_child_cap(&child, r, 1000, 1000000);
    ISO_CHECK(pbt_budget_charge(c, 60));
    ISO_CHECK(!pbt_budget_charge(c, 60)); /* the ROOT has only 40 left, though the child has plenty */
    ISO_CHECK_EQ_UINT(pbt_budget_attempts(c), 1);
    ISO_CHECK_EQ_UINT(pbt_budget_work(c), 60);
    ISO_CHECK_EQ_UINT(pbt_budget_attempts(r), 1);
    ISO_CHECK_EQ_UINT(pbt_budget_work(r), 60);
    pbt_budget_close(&child);
    pbt_budget_close(&root);
}

static void test_a_huge_cost_is_refused_without_wrapping(void)
{
    pbt_budget st;
    pbt_budget *b = pbt_budget_open_child_cap(&st, NULL, 1000, 100);
    ISO_CHECK(pbt_budget_charge(b, 60));
    ISO_CHECK(!pbt_budget_charge(b, UINT64_MAX));
    ISO_CHECK(!pbt_budget_charge_copy(b, UINT64_MAX));
    ISO_CHECK(!pbt_budget_charge(b, UINT64_MAX - 59)); /* 60 + this would wrap to a small number */
    ISO_CHECK_EQ_UINT(pbt_budget_work(b), 60);
    pbt_budget_close(&st);
}

static void test_the_attempt_cap_refuses_even_work_only_charges(void)
{
    pbt_budget st;
    pbt_budget *b = pbt_budget_open_child_cap(&st, NULL, 0, PBT_MAX_SHRINK_WORK); /* 64 attempts */
    int i;
    for (i = 0; i < 64; i++) {
        ISO_CHECK(pbt_budget_charge(b, 1));
    }
    ISO_CHECK(!pbt_budget_charge(b, 1));
    ISO_CHECK(!pbt_budget_charge_copy(b, 1)); /* the same ceiling applies to a copy */
    ISO_CHECK(!pbt_budget_has_room(b));
    pbt_budget_close(&st);
}

static void test_depth_cap_joins(void)
{
    pbt_budget st[70];
    pbt_budget *prev = NULL;
    pbt_budget *cur;
    int i;
    int first_join = -1;
    for (i = 0; i < 70; i++) {
        cur = pbt_budget_open_child(&st[i], prev, 10);
        if (cur == prev && first_join < 0) {
            first_join = i;
        }
        prev = cur;
    }
    /* Owning scopes are depths 0..63; the next one joins instead of growing the chain. */
    ISO_CHECK_EQ_INT(first_join, PBT_BUDGET_MAX_DEPTH);
    ISO_CHECK(pbt_budget_charge(prev, 1)); /* a charge through the joined chain still works */
    ISO_CHECK_EQ_UINT(pbt_budget_work(&st[0]), 1);
    ISO_CHECK_EQ_UINT(pbt_budget_work(&st[63]), 1);
    for (i = 69; i >= 0; i--) {
        pbt_budget_close(&st[i]);
    }
}

/* ============================ one shrink, many sizes ============================ */

/* "Still fails" iff the BYTES value has exactly n bytes; counts evaluations and the bytes handed over. */
typedef struct bytes_ctx {
    size_t n;
    long calls;
    uint64_t touched;
} bytes_ctx;

static int bytes_len_is(void *self, pbt_budget *b, const pbt_val *v)
{
    bytes_ctx *c = (bytes_ctx *)self;
    (void)b;
    c->calls++;
    c->touched += v->u.bytes.n;
    return v->u.bytes.n == c->n;
}

/* Attempts alone do not bound the work; the elements copied are charged too. */
static void test_megabyte_witness_with_a_stalling_property(void)
{
    const size_t n = (size_t)1 << 20;
    pbt_gen *g = pbt_gen_bytes(0, PBT_MAX_INPUT_BYTES);
    pbt_val v;
    bytes_ctx c;
    int steps = 0;
    c.n = n;
    c.calls = 0;
    c.touched = 0;
    ISO_CHECK(pbt_val_set_bytes(&v, NULL, n) == 0);
    ISO_CHECK(pbt_gen_shrink(NULL, g, &v, pred(bytes_len_is, &c), 1000, &steps) == 0);
    ISO_CHECK_EQ_UINT(v.u.bytes.n, n);
    ISO_CHECK(pbt_hooks.last_work <= PBT_MAX_SHRINK_WORK);
    ISO_CHECK(c.touched <= PBT_MAX_SHRINK_WORK);
    /* Without a work budget the attempt budget (16,064) would allow ~16 billion elements. */
    ISO_CHECK(c.calls < 100);
    pbt_val_free(&v);
    pbt_gen_release(g);
}

/* A generator whose shrinker is the real sequence shrinker over bytes, so a runner shrink exercises it. */
static int zeros_sample(void *self, pbt_run *run, pbt_val *out)
{
    (void)run;
    return pbt_val_set_bytes(out, NULL, *(size_t *)self);
}

static int zeros_shrink(void *self, pbt_budget *b, pbt_val *value, pbt_pred falsifies, int max_steps, int *steps)
{
    (void)self;
    return pbt_shrink_sequence(b, value, falsifies, max_steps, 0, NULL, NULL, steps);
}

typedef struct runner_bytes_ctx {
    size_t n;
    uint64_t touched;
} runner_bytes_ctx;

static int runner_bytes_prop(void *user, pbt_budget *scope, const pbt_val *v)
{
    runner_bytes_ctx *c = (runner_bytes_ctx *)user;
    (void)scope;
    c->touched += v->u.bytes.n;
    return v->u.bytes.n != c->n; /* fails only at the original length */
}

static void test_the_runner_shrink_of_a_megabyte_witness_is_also_bounded(void)
{
    size_t n = (size_t)1 << 20;
    pbt_gen *g = pbt_gen_custom(zeros_sample, zeros_shrink, &n, NULL);
    runner_bytes_ctx c;
    pbt_verdict vd;
    pbt_options o = opts(1, 1000, PBT_CI_OFF);
    c.n = n;
    c.touched = 0;
    ISO_CHECK(pbt_check(NULL, 1, g, mk(runner_bytes_prop, &c), &o, NULL, &vd) == 0);
    ISO_CHECK(strcmp(vd.failure_id, PBT_FAIL_COUNTEREXAMPLE) == 0);
    ISO_CHECK(c.touched <= PBT_MAX_SHRINK_WORK + 3u * n);
    pbt_verdict_free(&vd);
    pbt_gen_release(g);
}

/* One budget for the whole shrink, across nested shrinkers (a list of lists). */
static int is_grid(const pbt_val *l, size_t rows, size_t cols)
{
    size_t i;
    size_t j;
    if (l->kind != PBT_V_LIST || l->u.list.n != rows) {
        return 0;
    }
    for (i = 0; i < rows; i++) {
        const pbt_val *row = &l->u.list.items[i];
        if (row->kind != PBT_V_LIST || row->u.list.n != cols) {
            return 0;
        }
        for (j = 0; j < cols; j++) {
            if (as_long(&row->u.list.items[j]) != 7) {
                return 0;
            }
        }
    }
    return 1;
}

typedef struct grid_ctx {
    long calls;
} grid_ctx;

static int grid_falsifies(void *self, pbt_budget *b, const pbt_val *v)
{
    (void)b;
    ((grid_ctx *)self)->calls++;
    return is_grid(v, 50, 20);
}

static int grid_prop(void *user, pbt_budget *scope, const pbt_val *v)
{
    (void)scope;
    ((grid_ctx *)user)->calls++;
    return !is_grid(v, 50, 20);
}

typedef struct grid_gen {
    pbt_gen *list;
} grid_gen;

static int grid_sample(void *self, pbt_run *run, pbt_val *out)
{
    size_t i;
    int rc;
    (void)self;
    (void)run;
    rc = pbt_val_set_list(out, PBT_V_LIST, 50);
    if (rc != 0) {
        return rc;
    }
    for (i = 0; i < 50; i++) {
        pbt_val row = list_of_n(20, 7);
        out->u.list.items[i] = row;
    }
    return 0;
}

static int grid_shrink(void *self, pbt_budget *b, pbt_val *value, pbt_pred falsifies, int max_steps, int *steps)
{
    return pbt_gen_shrink_nested(b, ((grid_gen *)self)->list, value, falsifies, max_steps, steps);
}

static void test_nested_lists_share_one_global_budget(void)
{
    pbt_gen *u8 = pbt_gen_u8();
    pbt_gen *row = pbt_gen_list(u8, 0, 20);
    pbt_gen *grid = pbt_gen_list(row, 0, 50);
    pbt_val start;
    grid_ctx c;
    int steps = 0;
    size_t i;
    const int max_steps = 10;
    pbt_gen_release(u8);
    pbt_gen_release(row);

    /* Falsifies only for the exact original: every candidate at every level is rejected. */
    ISO_CHECK(pbt_val_set_list(&start, PBT_V_LIST, 50) == 0);
    for (i = 0; i < 50; i++) {
        start.u.list.items[i] = list_of_n(20, 7);
    }
    c.calls = 0;
    ISO_CHECK(pbt_gen_shrink(NULL, grid, &start, pred(grid_falsifies, &c), max_steps, &steps) == 0);
    ISO_CHECK_EQ_INT(steps, 0);
    ISO_CHECK(is_grid(&start, 50, 20)); /* the working value is exactly as it was */
    ISO_CHECK((uint64_t)c.calls <= pbt_attempt_budget(max_steps));
    ISO_CHECK(pbt_hooks.last_work <= PBT_MAX_SHRINK_WORK);
    pbt_val_free(&start);

    /* The same through pbt_check with a generator whose shrinker is the nested list shrinker. */
    {
        grid_gen gg;
        pbt_gen *gen;
        pbt_verdict vd;
        pbt_options o = opts(1, max_steps, PBT_CI_OFF);
        gg.list = grid;
        gen = pbt_gen_custom(grid_sample, grid_shrink, &gg, NULL);
        c.calls = 0;
        ISO_CHECK(pbt_check(NULL, 1, gen, mk(grid_prop, &c), &o, NULL, &vd) == 0);
        ISO_CHECK(strcmp(vd.failure_id, PBT_FAIL_COUNTEREXAMPLE) == 0);
        ISO_CHECK((uint64_t)c.calls <= 1 + pbt_attempt_budget(max_steps) + 1);
        pbt_verdict_free(&vd);
        pbt_gen_release(gen);
    }
    pbt_gen_release(grid);
}

static int list_size_ge_1(void *self, pbt_budget *b, const pbt_val *v)
{
    (void)self;
    (void)b;
    return v->kind == PBT_V_LIST && v->u.list.n >= 1;
}

static void test_a_nested_shrinker_still_shrinks_when_the_budget_is_ample(void)
{
    pbt_gen *u8 = pbt_gen_u8();
    pbt_gen *row = pbt_gen_list(u8, 0, 5);
    pbt_gen *outer = pbt_gen_list(row, 0, 5);
    pbt_val start;
    int steps = 0;
    pbt_gen_release(u8);
    pbt_gen_release(row);
    ISO_CHECK(pbt_val_set_list(&start, PBT_V_LIST, 2) == 0);
    ISO_CHECK(pbt_val_set_list(&start.u.list.items[0], PBT_V_LIST, 2) == 0);
    pbt_val_set_i64(&start.u.list.items[0].u.list.items[0], 5);
    pbt_val_set_i64(&start.u.list.items[0].u.list.items[1], 6);
    ISO_CHECK(pbt_val_set_list(&start.u.list.items[1], PBT_V_LIST, 1) == 0);
    pbt_val_set_i64(&start.u.list.items[1].u.list.items[0], 7);
    ISO_CHECK(pbt_gen_shrink(NULL, outer, &start, pred(list_size_ge_1, NULL), 1000, &steps) == 0);
    ISO_CHECK_EQ_UINT(start.u.list.n, 1);
    ISO_CHECK(start.u.list.items[0].kind == PBT_V_LIST && start.u.list.items[0].u.list.n == 0);
    pbt_val_free(&start);
    pbt_gen_release(outer);
}

/* ============================ spent budgets end every loop ============================ */

static int always_fails_calls(void *self, pbt_budget *b, const pbt_val *v)
{
    (void)b;
    (void)v;
    ++*(long *)self;
    return 1;
}

static int prop_fails_calls(void *user, pbt_budget *scope, const pbt_val *v)
{
    (void)scope;
    (void)v;
    ++*(long *)user;
    return 0;
}

static void test_entry_points_charge_their_initial_copy_and_return_the_input_when_spent(void)
{
    pbt_gen *g = pbt_gen_bytes(0, 200);
    pbt_val b;
    pbt_val l;
    long calls = 0;
    int steps = 0;
    pbt_big zero = I(0);
    uint8_t data[100];
    memset(data, 5, sizeof data);

    pbt_hooks.default_max_work = 5; /* a tiny root allowance */
    ISO_CHECK(pbt_val_set_bytes(&b, data, 100) == 0);
    ISO_CHECK(pbt_gen_shrink(NULL, g, &b, pred(always_fails_calls, &calls), 1000, &steps) == 0);
    ISO_CHECK_EQ_INT(steps, 0);
    ISO_CHECK_EQ_UINT(b.u.bytes.n, 100);
    ISO_CHECK(memcmp(b.u.bytes.p, data, 100) == 0);
    ISO_CHECK_EQ_INT(calls, 0); /* spent before the first candidate: the property never ran */

    l = list_of_n(100, 1);
    ISO_CHECK(pbt_shrink_list_ints(NULL, &l, &zero, mk(prop_fails_calls, &calls), 1000, &steps) == 0);
    ISO_CHECK_EQ_UINT(l.u.list.n, 100);
    ISO_CHECK_EQ_INT(steps, 0);
    ISO_CHECK_EQ_INT(calls, 0);
    pbt_val_free(&l);

    ISO_CHECK(pbt_shrink_bytes(NULL, &b, mk(prop_fails_calls, &calls), 1000, &steps) == 0);
    ISO_CHECK_EQ_UINT(b.u.bytes.n, 100);
    ISO_CHECK_EQ_INT(calls, 0);
    pbt_val_free(&b);
    pbt_hooks_reset();
    pbt_gen_release(g);
}

/* Deletion attempts charge the candidate's size, so a stalling property runs out of WORK long before attempts. */
static void test_deletions_are_charged_by_size(void)
{
    const size_t n = 4096;
    pbt_gen *g = pbt_gen_bytes(0, 8192);
    pbt_val b;
    bytes_ctx c;
    int steps = 0;
    c.n = n;
    c.calls = 0;
    c.touched = 0;
    pbt_hooks.default_max_work = 1000000;
    ISO_CHECK(pbt_val_set_bytes(&b, NULL, n) == 0);
    ISO_CHECK(pbt_gen_shrink(NULL, g, &b, pred(bytes_len_is, &c), 1000, &steps) == 0);
    /* Deletions cost 4095 each against 1M of work: about 245 evaluations, not the 16,064 attempts. */
    ISO_CHECK(c.calls < 260);
    ISO_CHECK(pbt_hooks.last_work <= 1000000);
    pbt_val_free(&b);
    pbt_hooks_reset();
    pbt_gen_release(g);
}

/* A tuple whose second coordinate always makes progress forces many passes over a big list. */
static int one_step_sample(void *self, pbt_run *run, pbt_val *out)
{
    (void)self;
    (void)run;
    pbt_val_set_i64(out, 1000000);
    return 0;
}

static int one_step_shrink(void *self, pbt_budget *b, pbt_val *value, pbt_pred falsifies, int max_steps, int *steps)
{
    pbt_val cand;
    (void)self;
    *steps = 0;
    if (as_long(value) > 0 && max_steps > 0) {
        pbt_val_set_i64(&cand, as_long(value) - 1);
        if (falsifies.fn(falsifies.self, b, &cand)) {
            pbt_val_set_i64(value, as_long(value) - 1);
            *steps = 1;
        }
    }
    return 0;
}

static int tuple_list_len(void *self, pbt_budget *b, const pbt_val *v)
{
    (void)b;
    ++((long *)self)[0];
    return v->u.list.items[0].u.list.n == 10000;
}

static void test_tuple_passes_over_a_large_list_stay_inside_the_work_budget(void)
{
    const size_t n = 10000;
    pbt_gen *ints = pbt_gen_int_i64(0, 9);
    pbt_gen *lst = pbt_gen_list(ints, 0, n);
    pbt_gen *one = pbt_gen_custom(one_step_sample, one_step_shrink, NULL, NULL);
    pbt_gen *pair[2];
    pbt_gen *g;
    pbt_val start;
    long calls[1] = {0};
    int steps = 0;
    pbt_hooks.default_max_work = 2000000; /* a small allowance so the test is quick */
    pair[0] = lst;
    pair[1] = one;
    g = pbt_gen_tuple(pair, 2);
    ISO_CHECK(pbt_val_set_list(&start, PBT_V_TUPLE, 2) == 0);
    start.u.list.items[0] = list_of_n(n, 0);
    pbt_val_set_i64(&start.u.list.items[1], 1000000);
    ISO_CHECK(pbt_gen_shrink(NULL, g, &start, pred(tuple_list_len, calls), 1000, &steps) == 0);
    ISO_CHECK(pbt_hooks.last_work <= 2000000);
    ISO_CHECK(steps >= 1); /* the stateful component still makes progress while any room remains */
    pbt_val_free(&start);
    pbt_hooks_reset();
    pbt_gen_release(g);
    pbt_gen_release(one);
    pbt_gen_release(lst);
    pbt_gen_release(ints);
}

/* ============================ re-entrancy ============================ */

static int prop_list_sum_le_20(void *user, pbt_budget *scope, const pbt_val *v)
{
    long s = 0;
    size_t i;
    (void)user;
    (void)scope;
    for (i = 0; i < v->u.list.n; i++) {
        s += as_long(&v->u.list.items[i]);
    }
    return s <= 20;
}

static pbt_verdict inner_run(pbt_budget *parent)
{
    pbt_gen *g = u8_list(0, 50);
    pbt_verdict v;
    pbt_options o = opts(200, 1000, PBT_CI_ON);
    ISO_CHECK(pbt_check(parent, 5, g, mk(prop_list_sum_le_20, NULL), &o, NULL, &v) == 0);
    pbt_gen_release(g);
    return v;
}

typedef struct reentrant_ctx {
    long inner_runs;
    int mismatches;
    char standalone_msg[512];
    int standalone_steps;
    int plain;
} reentrant_ctx;

static int reentrant_prop(void *user, pbt_budget *scope, const pbt_val *v)
{
    reentrant_ctx *c = (reentrant_ctx *)user;
    if (!c->plain) {
        pbt_verdict inner = inner_run(scope); /* the scope we were handed IS the parent */
        c->inner_runs++;
        if (inner.message == NULL || strcmp(inner.message, c->standalone_msg) != 0 ||
            inner.shrink_steps != c->standalone_steps) {
            c->mismatches++;
        }
        pbt_verdict_free(&inner);
    }
    return as_long(v) < 500;
}

/* A property that runs its own pbt_check gets a verdict identical to a standalone run, while the outer budget has room. */
static void test_a_reentrant_check_is_not_starved_by_the_outer_shrink(void)
{
    pbt_verdict standalone = inner_run(NULL);
    reentrant_ctx c;
    pbt_big lo = I(0);
    pbt_big hi = I(1000);
    pbt_gen *g = pbt_gen_int(&lo, &hi);
    pbt_verdict expected;
    pbt_verdict actual;
    pbt_options outer = opts(100, 10000, PBT_CI_ON);

    ISO_CHECK(strcmp(standalone.failure_id, PBT_FAIL_COUNTEREXAMPLE) == 0);
    ISO_CHECK(standalone.shrink_steps > 3); /* the inner shrink must need a real budget */
    memset(&c, 0, sizeof c);
    (void)snprintf(c.standalone_msg, sizeof c.standalone_msg, "%s", standalone.message);
    c.standalone_steps = standalone.shrink_steps;

    c.plain = 1;
    ISO_CHECK(pbt_check(NULL, 9, g, mk(reentrant_prop, &c), &outer, NULL, &expected) == 0);
    c.plain = 0;
    ISO_CHECK(pbt_check(NULL, 9, g, mk(reentrant_prop, &c), &outer, NULL, &actual) == 0);

    ISO_CHECK(c.inner_runs > 0);
    ISO_CHECK_EQ_INT(c.mismatches, 0);
    ISO_CHECK_STR_EQ(expected.message, actual.message); /* the outer verdict is unaffected */
    pbt_verdict_free(&standalone);
    pbt_verdict_free(&expected);
    pbt_verdict_free(&actual);
    pbt_gen_release(g);
}

/* A property that shrinks through the public entry points gets fresh LOCAL budgets under the same root. */
typedef struct shrink_in_prop {
    long inner;
    int mismatches;
    pbt_gen *list;
} shrink_in_prop;

static int size_ge_5(void *self, pbt_budget *b, const pbt_val *v)
{
    (void)self;
    (void)b;
    return v->u.list.n >= 5;
}

static int prop_empty_list(void *user, pbt_budget *scope, const pbt_val *v)
{
    (void)user;
    (void)scope;
    return v->u.list.n == 0;
}

static int shrinks_inside_prop(void *user, pbt_budget *scope, const pbt_val *v)
{
    shrink_in_prop *c = (shrink_in_prop *)user;
    pbt_val l = list_of_n(40, 3);
    pbt_val l2 = list_of_n(3, 9);
    pbt_big zero = I(0);
    int s1 = 0;
    int s2 = 0;
    ISO_CHECK(pbt_gen_shrink(scope, c->list, &l, pred(size_ge_5, NULL), 1000, &s1) == 0);
    ISO_CHECK(pbt_shrink_list_ints(scope, &l2, &zero, mk(prop_empty_list, NULL), 1000, &s2) == 0);
    c->inner++;
    if (l.u.list.n != 5 || l2.u.list.n != 1 || as_long(&l2.u.list.items[0]) != 0) {
        c->mismatches++;
    }
    pbt_val_free(&l);
    pbt_val_free(&l2);
    return as_long(v) < 500;
}

static void test_reentrant_gen_shrink_and_shrink_list_get_fresh_local_budgets(void)
{
    shrink_in_prop c;
    pbt_big lo = I(0);
    pbt_big hi = I(1000);
    pbt_gen *g = pbt_gen_int(&lo, &hi);
    pbt_verdict vd;
    pbt_options o = opts(100, 1000, PBT_CI_OFF);
    c.inner = 0;
    c.mismatches = 0;
    c.list = u8_list(0, 50);
    ISO_CHECK(pbt_check(NULL, 3, g, mk(shrinks_inside_prop, &c), &o, NULL, &vd) == 0);
    ISO_CHECK(strcmp(vd.failure_id, PBT_FAIL_COUNTEREXAMPLE) == 0);
    ISO_CHECK(c.inner > 0);
    ISO_CHECK_EQ_INT(c.mismatches, 0); /* every inner shrink equals a standalone one */
    pbt_verdict_free(&vd);
    pbt_gen_release(c.list);
    pbt_gen_release(g);
}

/* ============================ the tree: the bound holds whatever the nesting ============================ */

/* 1. A property that calls pbt_check on every evaluation, at depths 1, 2 and 3. */
typedef struct nest_ctx {
    int depth;
    int max_steps;
    long evals;
} nest_ctx;

static int nest_prop(void *user, pbt_budget *scope, const pbt_val *v)
{
    nest_ctx *c = (nest_ctx *)user;
    c->evals++;
    if (c->depth > 1) {
        nest_ctx next;
        pbt_gen *g = u8_list(0, 40);
        pbt_verdict vd;
        pbt_options o = opts(100, c->max_steps, PBT_CI_OFF);
        next.depth = c->depth - 1;
        next.max_steps = c->max_steps;
        next.evals = 0;
        ISO_CHECK(pbt_check(scope, 1, g, mk(nest_prop, &next), &o, NULL, &vd) == 0);
        c->evals += next.evals;
        pbt_verdict_free(&vd);
        pbt_gen_release(g);
    }
    return v->u.list.n < 5;
}

static void test_a_property_calling_check_keeps_the_root_totals_within_the_root_cap(void)
{
    int depth;
    for (depth = 1; depth <= 3; depth++) {
        nest_ctx c;
        pbt_gen *g = u8_list(0, 40);
        pbt_verdict vd;
        pbt_options o = opts(100, 5, PBT_CI_OFF);
        uint64_t cap = 100 + pbt_attempt_budget(5); /* num_tests + shrink cap */
        c.depth = depth;
        c.max_steps = 5;
        c.evals = 0;
        pbt_hooks.default_max_work = 1000000;
        ISO_CHECK(pbt_check(NULL, 1, g, mk(nest_prop, &c), &o, NULL, &vd) == 0);
        /* the outermost check's scope is the root, and it closes last */
        ISO_CHECK(pbt_hooks.last_attempts <= cap);
        ISO_CHECK(pbt_hooks.last_work <= 1000000);
        ISO_CHECK(c.evals > 0);
        ISO_CHECK((uint64_t)c.evals <= 3 * cap * (uint64_t)depth);
        pbt_verdict_free(&vd);
        pbt_gen_release(g);
        pbt_hooks_reset();
    }
}

/* 2. A custom shrinker that recurses into pbt_gen_shrink / pbt_shrink_list_ints on every pass. */
typedef struct recurse_ctx {
    pbt_gen *inner;
    size_ctx *extra;
} recurse_ctx;

static int recursing_shrink(void *self, pbt_budget *b, pbt_val *value, pbt_pred falsifies, int max_steps, int *steps)
{
    recurse_ctx *r = (recurse_ctx *)self;
    int pass;
    *steps = 0;
    for (pass = 0; pass < 100; pass++) {
        pbt_val copy;
        pbt_val tiny;
        pbt_big zero = I(0);
        int used = 0;
        if (pbt_val_clone(&copy, value) == 0) {
            (void)pbt_gen_shrink(b, r->inner, &copy, falsifies, max_steps, &used);
            pbt_val_free(&copy);
        }
        if (pbt_val_clone(&copy, value) == 0) {
            /* a property of the shrinker's own: charged only through the budget chain */
            (void)pbt_gen_shrink(b, r->inner, &copy, pred(size_is, r->extra), max_steps, &used);
            pbt_val_free(&copy);
        }
        tiny = list_of_n(3, 1);
        (void)pbt_shrink_list_ints(b, &tiny, &zero, mk(prop_empty_list, NULL), max_steps, &used);
        pbt_val_free(&tiny);
    }
    return 0;
}

static int fifty_sample(void *self, pbt_run *run, pbt_val *out)
{
    (void)self;
    (void)run;
    *out = list_of_n(50, 7);
    return 0;
}

static void test_a_recursing_custom_shrinker_stays_inside_the_root_cap(void)
{
    recurse_ctx r;
    size_ctx calls;
    size_ctx extra;
    pbt_gen *outer;
    pbt_val start;
    const int max_steps = 10;
    int steps = 0;
    r.inner = u8_list(0, 50);
    extra.n = 50;
    extra.calls = 0;
    r.extra = &extra;
    calls.n = 50;
    calls.calls = 0;
    outer = pbt_gen_custom(fifty_sample, recursing_shrink, &r, NULL);
    start = list_of_n(50, 7);
    pbt_hooks.default_max_work = 1000000;
    ISO_CHECK(pbt_gen_shrink(NULL, outer, &start, pred(size_is, &calls), max_steps, &steps) == 0);
    ISO_CHECK(pbt_hooks.last_attempts <= pbt_attempt_budget(max_steps));
    ISO_CHECK(pbt_hooks.last_work <= 1000000);
    ISO_CHECK((uint64_t)calls.calls <= pbt_attempt_budget(max_steps));
    ISO_CHECK((uint64_t)(calls.calls + extra.calls) <= pbt_attempt_budget(max_steps));
    pbt_val_free(&start);
    pbt_gen_release(outer);
    pbt_gen_release(r.inner);
    pbt_hooks_reset();
}

/* Fails (returns 0) when the list has exactly 50 elements; counts evaluations. */
static int size_prop_fails_at_50(void *user, pbt_budget *scope, const pbt_val *v)
{
    size_ctx *c = (size_ctx *)user;
    (void)scope;
    c->calls++;
    return !(v->kind == PBT_V_LIST && v->u.list.n == c->n);
}

/* 2b. A custom shrinker that calls the public pbt_gen_shrink on every pass: one budget, never a fresh one each time. */
static int per_pass_shrink(void *self, pbt_budget *b, pbt_val *value, pbt_pred falsifies, int max_steps, int *steps)
{
    pbt_gen *inner = (pbt_gen *)self;
    int pass;
    *steps = 0;
    for (pass = 0; pass < 200; pass++) {
        pbt_val copy;
        int used = 0;
        if (pbt_val_clone(&copy, value) == 0) {
            (void)pbt_gen_shrink(b, inner, &copy, falsifies, max_steps, &used);
            pbt_val_free(&copy);
        }
    }
    return 0;
}

static void test_a_custom_shrinker_calling_gen_shrink_per_pass_stays_inside_one_budget(void)
{
    pbt_gen *inner = u8_list(0, 50);
    pbt_gen *outer = pbt_gen_custom(fifty_sample, per_pass_shrink, inner, NULL);
    pbt_val start = list_of_n(50, 7);
    size_ctx c;
    pbt_verdict vd;
    const int max_steps = 10;
    int steps = 0;
    c.n = 50;
    c.calls = 0;
    ISO_CHECK(pbt_gen_shrink(NULL, outer, &start, pred(size_is, &c), max_steps, &steps) == 0);
    ISO_CHECK((uint64_t)c.calls <= pbt_attempt_budget(max_steps));
    ISO_CHECK(pbt_hooks.last_work <= PBT_MAX_SHRINK_WORK);
    pbt_val_free(&start);

    /* The same through pbt_check with an explicit shrinker. */
    {
        pbt_shrinker sh;
        runner_bytes_ctx unused;
        size_ctx pc;
        pbt_options o = opts(1, max_steps, PBT_CI_OFF);
        (void)unused;
        sh.fn = per_pass_shrink;
        sh.self = inner;
        pc.n = 50;
        pc.calls = 0;
        ISO_CHECK(pbt_check(NULL, 1, outer, mk(size_prop_fails_at_50, &pc), &o, &sh, &vd) == 0);
        ISO_CHECK(strcmp(vd.failure_id, PBT_FAIL_COUNTEREXAMPLE) == 0);
        ISO_CHECK((uint64_t)pc.calls <= pbt_attempt_budget(max_steps) + 2);
        pbt_verdict_free(&vd);
    }
    pbt_gen_release(outer);
    pbt_gen_release(inner);
}

/* 3. A filter predicate that itself shrinks. */
typedef struct filter_shrink_ctx {
    pbt_gen *list;
    size_ctx inner;
} filter_shrink_ctx;

static int filter_that_shrinks(void *self, pbt_budget *b, const pbt_val *v)
{
    filter_shrink_ctx *c = (filter_shrink_ctx *)self;
    pbt_val l = list_of_n(200, 3);
    int steps = 0;
    (void)v;
    (void)pbt_gen_shrink(b, c->list, &l, pred(size_is, &c->inner), 1000, &steps);
    pbt_val_free(&l);
    return 1;
}

static int int_ge_10(void *self, pbt_budget *b, const pbt_val *v)
{
    (void)self;
    (void)b;
    return as_long(v) >= 10;
}

static void test_a_filter_predicate_that_shrinks_stays_inside_the_root_cap(void)
{
    filter_shrink_ctx c;
    pbt_big lo = I(0);
    pbt_big hi = I(1000);
    pbt_gen *ints = pbt_gen_int(&lo, &hi);
    pbt_gen *filtered;
    pbt_val v;
    int steps = 0;
    const int max_steps = 5;
    c.list = u8_list(0, 200);
    c.inner.n = 200;
    c.inner.calls = 0;
    filtered = pbt_gen_filter(ints, filter_that_shrinks, &c, 100);
    pbt_val_set_i64(&v, 900);
    pbt_hooks.default_max_work = 5000000;
    ISO_CHECK(pbt_gen_shrink(NULL, filtered, &v, pred(int_ge_10, NULL), max_steps, &steps) == 0);
    ISO_CHECK(c.inner.calls > 0); /* the predicate really did shrink */
    ISO_CHECK(pbt_hooks.last_attempts <= pbt_attempt_budget(max_steps));
    ISO_CHECK(pbt_hooks.last_work <= 5000000);
    /* Each inner shrink alone could use ~400 attempts; the root allows 144 in all. */
    ISO_CHECK((uint64_t)c.inner.calls <= pbt_attempt_budget(max_steps));
    pbt_gen_release(filtered);
    pbt_gen_release(ints);
    pbt_gen_release(c.list);
    pbt_hooks_reset();
}

/* 5. A passing inner check with a huge num_tests under an outer shrink is bounded by case charging. */
typedef struct huge_ctx {
    long inner_cases;
} huge_ctx;

static int count_cases_prop(void *user, pbt_budget *scope, const pbt_val *v)
{
    (void)scope;
    (void)v;
    ++((huge_ctx *)user)->inner_cases;
    return 1;
}

static int huge_inner_pred(void *self, pbt_budget *b, const pbt_val *v)
{
    huge_ctx *c = (huge_ctx *)self;
    pbt_gen *g = pbt_gen_int_i64(0, 9);
    pbt_verdict vd;
    pbt_options o = opts(PBT_MAX_CASES, 10, PBT_CI_OFF);
    ISO_CHECK(pbt_check(b, 1, g, mk(count_cases_prop, c), &o, NULL, &vd) == 0);
    pbt_verdict_free(&vd);
    pbt_gen_release(g);
    return as_long(v) >= 10;
}

static void test_a_passing_inner_check_with_huge_num_tests_is_bounded_under_an_outer_shrink(void)
{
    huge_ctx c;
    pbt_big lo = I(0);
    pbt_big hi = I(1000);
    pbt_gen *g = pbt_gen_int(&lo, &hi);
    pbt_val v;
    int steps = 0;
    const int max_steps = 5;
    c.inner_cases = 0;
    pbt_val_set_i64(&v, 900);
    pbt_hooks.default_max_work = 1000000;
    ISO_CHECK(pbt_gen_shrink(NULL, g, &v, pred(huge_inner_pred, &c), max_steps, &steps) == 0);
    ISO_CHECK((uint64_t)c.inner_cases <= pbt_attempt_budget(max_steps) + 64);
    ISO_CHECK(pbt_hooks.last_attempts <= pbt_attempt_budget(max_steps));
    pbt_gen_release(g);
    pbt_hooks_reset();
}

/* 6. pbt_shrink_int and pbt_simulate_replay inside a property are charged. */
typedef struct charged_ctx {
    long evals;
} charged_ctx;

static int lt_100_counting(void *user, pbt_budget *scope, const pbt_val *v)
{
    (void)scope;
    ++((charged_ctx *)user)->evals;
    return as_long(v) < 100;
}

static int shrink_int_and_replay_pred(void *self, pbt_budget *b, const pbt_val *v)
{
    charged_ctx *c = (charged_ctx *)self;
    pbt_big minimal;
    pbt_big start = I(500);
    pbt_big zero = I(0);
    pbt_big top = I(1000);
    pbt_big a;
    pbt_big m;
    int steps = 0;
    (void)pbt_shrink_int(b, &start, &zero, &zero, &top, mk(lt_100_counting, c), 1000, &minimal, &steps);
    (void)pbt_simulate_replay(b, 3, mk(lt_100_counting, c), &a, &m);
    return as_long(v) >= 10;
}

static void test_shrink_int_and_replay_inside_a_property_are_charged(void)
{
    charged_ctx c;
    pbt_big lo = I(0);
    pbt_big hi = I(1000);
    pbt_gen *g = pbt_gen_int(&lo, &hi);
    pbt_val v;
    int steps = 0;
    const int max_steps = 5;
    c.evals = 0;
    pbt_val_set_i64(&v, 900);
    pbt_hooks.default_max_work = 1000000;
    ISO_CHECK(pbt_gen_shrink(NULL, g, &v, pred(shrink_int_and_replay_pred, &c), max_steps, &steps) == 0);
    ISO_CHECK(c.evals > 0);
    ISO_CHECK(pbt_hooks.last_attempts <= pbt_attempt_budget(max_steps));
    ISO_CHECK((uint64_t)c.evals <= pbt_attempt_budget(max_steps));
    pbt_gen_release(g);
    pbt_hooks_reset();
}

/* 8. A starved nested check reports property-exhausted without evaluating anything; the TOP-LEVEL check always runs. */
static int fail_counting(void *user, pbt_budget *scope, const pbt_val *v)
{
    (void)scope;
    (void)v;
    ++*(long *)user;
    return 0;
}

static void test_a_starved_nested_check_is_exhausted_but_a_top_level_check_still_runs(void)
{
    pbt_budget st;
    pbt_budget *root = pbt_budget_open_child(&st, NULL, 0); /* 64 attempts */
    pbt_gen *g = pbt_gen_bytes(5, 5);
    long evaluated = 0;
    pbt_verdict nested;
    pbt_verdict top;
    uint64_t before;
    pbt_options o = opts(10, 1000, PBT_CI_OFF);
    while (pbt_budget_charge(root, 1)) {
        /* spend it all */
    }
    before = pbt_budget_attempts(root);
    ISO_CHECK(pbt_check(root, 1, g, mk(fail_counting, &evaluated), &o, NULL, &nested) == 0);
    ISO_CHECK(strcmp(nested.failure_id, PBT_FAIL_EXHAUSTED) == 0);
    ISO_CHECK(!nested.passed);
    ISO_CHECK_EQ_INT(evaluated, 0);
    ISO_CHECK_EQ_UINT(pbt_budget_attempts(root), before); /* all or nothing: unchanged */
    pbt_verdict_free(&nested);
    pbt_budget_close(&st);

    /* The top-level check has no parent: it always evaluates its first case. */
    ISO_CHECK(pbt_check(NULL, 1, g, mk(fail_counting, &evaluated), &o, NULL, &top) == 0);
    ISO_CHECK(strcmp(top.failure_id, PBT_FAIL_COUNTEREXAMPLE) == 0);
    ISO_CHECK(top.minimal.kind == PBT_V_BYTES);
    ISO_CHECK(evaluated > 0);
    pbt_verdict_free(&top);
    pbt_gen_release(g);
}

/* A branching property (two nested checks per evaluation, depth 4) stays inside the root cap. */
typedef struct branch_ctx {
    int level;
    long counted;
} branch_ctx;

static int branch_prop(void *user, pbt_budget *scope, const pbt_val *v);

static void run_branch(pbt_budget *scope, int level, long *counted)
{
    branch_ctx c;
    pbt_gen *g = pbt_gen_int_i64(0, 9);
    pbt_verdict vd;
    pbt_options o = opts(10, 5, PBT_CI_OFF);
    c.level = level;
    c.counted = 0;
    ISO_CHECK(pbt_check(scope, 1, g, mk(branch_prop, &c), &o, NULL, &vd) == 0);
    *counted += c.counted;
    pbt_verdict_free(&vd);
    pbt_gen_release(g);
}

static int branch_prop(void *user, pbt_budget *scope, const pbt_val *v)
{
    branch_ctx *c = (branch_ctx *)user;
    if (c->level > 1) {
        c->counted++; /* evaluations made by NESTED checks */
    }
    if (c->level < 4) {
        run_branch(scope, c->level + 1, &c->counted);
        run_branch(scope, c->level + 1, &c->counted);
    }
    return as_long(v) < 3;
}

static void test_a_branching_property_keeps_total_evaluations_within_the_root_cap(void)
{
    long counted = 0;
    uint64_t cap = 10 + pbt_attempt_budget(5);
    pbt_hooks.default_max_work = 1000000;
    run_branch(NULL, 1, &counted);
    ISO_CHECK(counted > 0);
    ISO_CHECK(pbt_hooks.last_attempts <= cap);
    /* every nested evaluation is preceded by a charge that reaches the root */
    ISO_CHECK((uint64_t)counted <= cap);
    pbt_hooks_reset();
}

/* A nested check during the OUTER case loop (generation phase, before any shrink) is charged to the outer root. */
static int inner_cases_prop(void *user, pbt_budget *scope, const pbt_val *v)
{
    (void)scope;
    (void)v;
    ++*(long *)user;
    return 1;
}

typedef struct case_loop_ctx {
    long inner_cases;
} case_loop_ctx;

static int outer_case_loop_prop(void *user, pbt_budget *scope, const pbt_val *v)
{
    case_loop_ctx *c = (case_loop_ctx *)user;
    pbt_gen *g = pbt_gen_int_i64(0, 9);
    pbt_verdict vd;
    pbt_options o = opts(200, 10, PBT_CI_OFF);
    (void)v;
    ISO_CHECK(pbt_check(scope, 2, g, mk(inner_cases_prop, &c->inner_cases), &o, NULL, &vd) == 0);
    pbt_verdict_free(&vd);
    pbt_gen_release(g);
    return 1; /* the outer property never fails: no shrink at all */
}

static void test_a_nested_check_inside_the_outer_case_loop_charges_the_outer_root(void)
{
    case_loop_ctx c;
    pbt_gen *g = pbt_gen_int_i64(0, 9);
    pbt_verdict vd;
    pbt_options o = opts(5, 5, PBT_CI_OFF);
    uint64_t cap = 5 + pbt_attempt_budget(5);
    c.inner_cases = 0;
    pbt_hooks.default_max_work = 1000000;
    ISO_CHECK(pbt_check(NULL, 1, g, mk(outer_case_loop_prop, &c), &o, NULL, &vd) == 0);
    ISO_CHECK(pbt_hooks.last_attempts > 0 && pbt_hooks.last_attempts <= cap);
    ISO_CHECK((uint64_t)c.inner_cases <= cap); /* uncharged it would be 5 x 200 */
    pbt_verdict_free(&vd);
    pbt_gen_release(g);
    pbt_hooks_reset();
}

/* ============================ starvation is never a pass ============================ */

/* The outer check passes every case, but the nested check inside is starved: the OUTER verdict must not be a pass. */
static int starving_prop(void *user, pbt_budget *scope, const pbt_val *v)
{
    pbt_gen *g = pbt_gen_int_i64(0, 9);
    pbt_verdict vd;
    pbt_options o = opts(1000, 10, PBT_CI_OFF);
    long *inner_cases = (long *)user;
    (void)v;
    ISO_CHECK(pbt_check(scope, 1, g, mk(inner_cases_prop, inner_cases), &o, NULL, &vd) == 0);
    ISO_CHECK(!vd.passed); /* the starved inner check itself is exhausted, never a pass */
    pbt_verdict_free(&vd);
    pbt_gen_release(g);
    return 1;
}

static void test_starvation_is_never_a_pass(void)
{
    pbt_gen *g = pbt_gen_int_i64(0, 9);
    pbt_verdict vd;
    long inner = 0;
    pbt_options o = opts(3, 0, PBT_CI_OFF); /* the outer check scope allows 3 + 64 attempts */
    ISO_CHECK(pbt_check(NULL, 1, g, mk(starving_prop, &inner), &o, NULL, &vd) == 0);
    ISO_CHECK(!vd.passed);
    ISO_CHECK(strcmp(vd.failure_id, PBT_FAIL_EXHAUSTED) == 0);
    ISO_CHECK(vd.message != NULL && strstr(vd.message, "starved") != NULL);
    ISO_CHECK(inner > 0 && inner < 1000);
    pbt_verdict_free(&vd);
    pbt_gen_release(g);
}

/* Stacked filters multiply their retries: each retry is charged as WORK at a top-level check, so they run out. */
static int keep_div2(void *self, pbt_budget *b, const pbt_val *v)
{
    (void)self;
    (void)b;
    return as_long(v) % 2 == 0;
}

static int keep_div6(void *self, pbt_budget *b, const pbt_val *v)
{
    (void)self;
    (void)b;
    return as_long(v) % 6 == 0;
}

static int keep_div30(void *self, pbt_budget *b, const pbt_val *v)
{
    (void)self;
    (void)b;
    return as_long(v) % 30 == 0;
}

static int holds(void *user, pbt_budget *scope, const pbt_val *v)
{
    (void)user;
    (void)scope;
    (void)v;
    return 1;
}

static void test_stacked_filter_retries_are_charged_as_work(void)
{
    pbt_big lo = I(0);
    pbt_big hi = I(1000000);
    pbt_gen *ints = pbt_gen_int(&lo, &hi);
    pbt_gen *f1 = pbt_gen_filter(ints, keep_div2, NULL, 100);
    pbt_gen *f2 = pbt_gen_filter(f1, keep_div6, NULL, 100);
    pbt_gen *f3 = pbt_gen_filter(f2, keep_div30, NULL, 100);
    pbt_verdict vd;
    pbt_options o = opts(20, 10, PBT_CI_OFF);

    /* With the default allowance the stack is satisfiable and the run passes. */
    ISO_CHECK(pbt_check(NULL, 1, f3, mk(holds, NULL), &o, NULL, &vd) == 0);
    ISO_CHECK(vd.passed);
    pbt_verdict_free(&vd);

    /* With a tiny work allowance the retries run out: exhausted, never a pass. */
    pbt_hooks.default_max_work = 50;
    ISO_CHECK(pbt_check(NULL, 1, f3, mk(holds, NULL), &o, NULL, &vd) == 0);
    ISO_CHECK(!vd.passed);
    ISO_CHECK(strcmp(vd.failure_id, PBT_FAIL_EXHAUSTED) == 0);
    pbt_verdict_free(&vd);
    pbt_hooks_reset();

    pbt_gen_release(f3);
    pbt_gen_release(f2);
    pbt_gen_release(f1);
    pbt_gen_release(ints);
}

/* ============================ the re-verification charge ============================ */

/* A shrinker that burns every remaining unit of work up the chain, then claims a shrink. */
static int burning_shrinker(void *self, pbt_budget *b, pbt_val *value, pbt_pred falsifies, int max_steps, int *steps)
{
    (void)self;
    (void)falsifies;
    (void)max_steps;
    while (pbt_budget_charge_copy(b, 1)) {
        /* spend the root's work */
    }
    pbt_val_set_i64(value, 777); /* a genuinely failing, smaller value that would normally be adopted */
    *steps = 2;
    return 0;
}

static int honest_shrinker(void *self, pbt_budget *b, pbt_val *value, pbt_pred falsifies, int max_steps, int *steps)
{
    (void)self;
    (void)b;
    (void)falsifies;
    (void)max_steps;
    pbt_val_set_i64(value, 777);
    *steps = 2;
    return 0;
}

static int lt_500(void *user, pbt_budget *scope, const pbt_val *v)
{
    (void)user;
    (void)scope;
    return as_long(v) < 500;
}

static void test_if_re_verification_is_refused_the_original_witness_is_kept(void)
{
    pbt_gen *g = pbt_gen_int_i64(900, 900);
    pbt_shrinker sh;
    pbt_verdict vd;
    pbt_options o = opts(3, 100, PBT_CI_OFF);

    /* Control: with room, the honest shrink IS adopted. */
    sh.fn = honest_shrinker;
    sh.self = NULL;
    ISO_CHECK(pbt_check(NULL, 1, g, mk(lt_500, NULL), &o, &sh, &vd) == 0);
    ISO_CHECK(as_long(&vd.minimal) == 777 && vd.shrink_steps == 2);
    pbt_verdict_free(&vd);

    /* With the work spent, the single re-verification charge is refused: the ORIGINAL witness, zero steps. */
    pbt_hooks.default_max_work = 1000;
    sh.fn = burning_shrinker;
    ISO_CHECK(pbt_check(NULL, 1, g, mk(lt_500, NULL), &o, &sh, &vd) == 0);
    ISO_CHECK(strcmp(vd.failure_id, PBT_FAIL_COUNTEREXAMPLE) == 0); /* a falsified verdict is never overridden */
    ISO_CHECK(as_long(&vd.minimal) == 900);
    ISO_CHECK_EQ_INT(vd.shrink_steps, 0);
    pbt_verdict_free(&vd);
    pbt_hooks_reset();
    pbt_gen_release(g);
}

/* Checks nested ever deeper: past the nesting limit the innermost check evaluates nothing and is exhausted, never a pass. */
typedef struct deep_ctx {
    int level;
    int deepest_evaluated;
    int refused_level;
} deep_ctx;

static int deep_prop(void *user, pbt_budget *scope, const pbt_val *v)
{
    deep_ctx *c = (deep_ctx *)user;
    pbt_gen *g = pbt_gen_int_i64(0, 9);
    pbt_verdict vd;
    pbt_options o = opts(1, 1, PBT_CI_OFF);
    deep_ctx next = *c;
    (void)v;
    if (c->level > c->deepest_evaluated) {
        c->deepest_evaluated = c->level;
    }
    next.level = c->level + 1;
    ISO_CHECK(pbt_check(scope, 1, g, mk(deep_prop, &next), &o, NULL, &vd) == 0);
    if (!vd.passed && vd.message != NULL && strstr(vd.message, "nested more than") != NULL && c->refused_level == 0) {
        c->refused_level = c->level + 1;
    }
    if (next.deepest_evaluated > c->deepest_evaluated) {
        c->deepest_evaluated = next.deepest_evaluated;
    }
    if (next.refused_level != 0) {
        c->refused_level = next.refused_level;
    }
    pbt_verdict_free(&vd);
    pbt_gen_release(g);
    return 1;
}

static void test_check_nesting_is_capped(void)
{
    deep_ctx c;
    pbt_gen *g = pbt_gen_int_i64(0, 9);
    pbt_verdict vd;
    pbt_options o = opts(1, 1, PBT_CI_OFF);
    c.level = 1;
    c.deepest_evaluated = 0;
    c.refused_level = 0;
    ISO_CHECK(pbt_check(NULL, 1, g, mk(deep_prop, &c), &o, NULL, &vd) == 0);
    ISO_CHECK(!vd.passed); /* the starved/refused inner checks never turn into a pass */
    ISO_CHECK(c.refused_level > 0);
    ISO_CHECK(c.deepest_evaluated <= PBT_BUDGET_MAX_DEPTH);
    pbt_verdict_free(&vd);
    pbt_gen_release(g);
}

int main(void)
{
    pbt_hooks_reset();
    test_budget_semantics();
    test_charge_copy_charges_work_not_attempts();
    test_a_refused_take_leaves_every_counter_unchanged();
    test_a_huge_cost_is_refused_without_wrapping();
    test_the_attempt_cap_refuses_even_work_only_charges();
    test_depth_cap_joins();
    test_megabyte_witness_with_a_stalling_property();
    test_the_runner_shrink_of_a_megabyte_witness_is_also_bounded();
    test_nested_lists_share_one_global_budget();
    test_a_nested_shrinker_still_shrinks_when_the_budget_is_ample();
    test_entry_points_charge_their_initial_copy_and_return_the_input_when_spent();
    test_deletions_are_charged_by_size();
    test_tuple_passes_over_a_large_list_stay_inside_the_work_budget();
    test_a_reentrant_check_is_not_starved_by_the_outer_shrink();
    test_reentrant_gen_shrink_and_shrink_list_get_fresh_local_budgets();
    test_a_property_calling_check_keeps_the_root_totals_within_the_root_cap();
    test_a_recursing_custom_shrinker_stays_inside_the_root_cap();
    test_a_custom_shrinker_calling_gen_shrink_per_pass_stays_inside_one_budget();
    test_a_filter_predicate_that_shrinks_stays_inside_the_root_cap();
    test_a_passing_inner_check_with_huge_num_tests_is_bounded_under_an_outer_shrink();
    test_shrink_int_and_replay_inside_a_property_are_charged();
    test_a_starved_nested_check_is_exhausted_but_a_top_level_check_still_runs();
    test_a_branching_property_keeps_total_evaluations_within_the_root_cap();
    test_a_nested_check_inside_the_outer_case_loop_charges_the_outer_root();
    test_starvation_is_never_a_pass();
    test_stacked_filter_retries_are_charged_as_work();
    test_check_nesting_is_capped();
    test_if_re_verification_is_refused_the_original_witness_is_kept();
    pbt_hooks_reset();
    ISO_CHECK_EQ_INT(pbt_hooks.live_allocs, 0);
    return ISO_TEST_RESULT();
}
