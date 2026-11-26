/*
 * test_fuzz_engine.c -- campaign contracts (VER06 section 9.1, VER06j sections 5-7).
 *
 * Every test is bounded: campaigns run a fixed number of iterations, budgets are small, so a regression fails
 * (a wrong count, a missing outcome) instead of hanging.
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "fuzz.h"
#include "fuzz_hooks.h"
#include "fuzz_internal.h"
#include "iso_test.h"
#include "pbt_hooks.h"
#include "pbt_internal.h"

static fz_options opts(size_t iters)
{
    fz_options o = fz_options_default();
    o.max_iterations = iters;
    return o;
}

static const fz_crash *first_crash(const fz_report *r)
{
    return r->ncrashes > 0 ? &r->crashes[0].crash : NULL;
}

/* ---- simple targets --------------------------------------------------------------------------------------- */

static int ok_target(const uint8_t *d, size_t n, fz_handle *h, fz_crash *out)
{
    (void)d;
    (void)n;
    (void)h;
    (void)out;
    return 0;
}

static size_t g_calls;
static size_t g_maxlen;

static int counting_target(const uint8_t *d, size_t n, fz_handle *h, fz_crash *out)
{
    (void)d;
    (void)h;
    (void)out;
    g_calls++;
    if (n > g_maxlen) {
        g_maxlen = n;
    }
    return 0;
}

static int always_crash(const uint8_t *d, size_t n, fz_handle *h, fz_crash *out)
{
    (void)d;
    (void)n;
    (void)h;
    fz_crash_set(out, "boom");
    fz_crash_push(out, "inner");
    fz_crash_push(out, "outer");
    return 1;
}

/* A three-stage staircase (bytes ff 80 7f, all boundary values the mutators insert directly): each correct byte reports a new edge; the third crashes. */
static int staircase(const uint8_t *d, size_t n, fz_handle *h, fz_crash *out)
{
    fz_edge(h, 1);
    if (n > 0 && d[0] == 0xFF) {
        fz_edge(h, 2);
        if (n > 1 && d[1] == 0x80) {
            fz_edge(h, 3);
            if (n > 2 && d[2] == 0x7F) {
                fz_crash_set(out, "staircase");
                fz_crash_push(out, "stage3");
                return 1;
            }
        }
    }
    return 0;
}

/* The same target without any edge report: blind mutation. */
static int staircase_blind(const uint8_t *d, size_t n, fz_handle *h, fz_crash *out)
{
    (void)h;
    if (n > 2 && d[0] == 0xFF && d[1] == 0x80 && d[2] == 0x7F) {
        fz_crash_set(out, "staircase");
        fz_crash_push(out, "stage3");
        return 1;
    }
    return 0;
}

static void test_basic_contracts(void)
{
    fz_report r;
    fz_report r2;
    fz_options o = opts(300);
    fz_input seed;
    const uint8_t s[] = {'A', 'B', 'C'};
    seed.p = s;
    seed.n = 3;
    /* iterations == max_iterations exactly; no candidate longer than max_input_len; seeds run as given */
    g_calls = 0;
    g_maxlen = 0;
    o.max_input_len = 5;
    ISO_CHECK_EQ_INT(fz_run(&seed, 1, &o, counting_target, &r), FZ_OK);
    ISO_CHECK_EQ_INT(r.iterations, 300);
    ISO_CHECK_EQ_INT(g_calls, 301); /* triage + 300 mutations */
    ISO_CHECK(g_maxlen <= 5);
    ISO_CHECK(fz_report_is_clean(&r));
    ISO_CHECK_EQ_INT(strcmp(r.tier, FZ_TIER_BLACKBOX), 0);
    ISO_CHECK_EQ_INT(r.total_edges, 0);
    ISO_CHECK_EQ_INT(r.admitted_seeds, 1); /* the first passing seed is admitted even without coverage */
    fz_report_free(&r);
    /* an empty corpus is replaced by one 00 seed */
    ISO_CHECK_EQ_INT(fz_run(NULL, 0, &o, ok_target, &r), FZ_OK);
    ISO_CHECK_EQ_INT(r.ncorpus, 1);
    ISO_CHECK(r.corpus[0].n == 1 && r.corpus[0].p[0] == 0);
    fz_report_free(&r);
    /* determinism: same inputs, same report; a different seed, a different corpus */
    o = opts(500);
    ISO_CHECK_EQ_INT(fz_run(&seed, 1, &o, staircase, &r), FZ_OK);
    ISO_CHECK_EQ_INT(fz_run(&seed, 1, &o, staircase, &r2), FZ_OK);
    ISO_CHECK(r.iterations == r2.iterations && r.admitted_seeds == r2.admitted_seeds &&
              r.total_edges == r2.total_edges && r.ncrashes == r2.ncrashes && r.ncorpus == r2.ncorpus);
    {
        size_t i;
        int same = 1;
        for (i = 0; i < r.ncorpus && i < r2.ncorpus; i++) {
            same = same && r.corpus[i].n == r2.corpus[i].n && memcmp(r.corpus[i].p, r2.corpus[i].p, r.corpus[i].n) == 0;
        }
        ISO_CHECK(same);
    }
    fz_report_free(&r2);
    o.seed = 43;
    ISO_CHECK_EQ_INT(fz_run(&seed, 1, &o, staircase, &r2), FZ_OK);
    {
        int differs = r.ncorpus != r2.ncorpus || r.iterations != r2.iterations;
        size_t i;
        for (i = 0; !differs && i < r.ncorpus; i++) {
            differs = r.corpus[i].n != r2.corpus[i].n || memcmp(r.corpus[i].p, r2.corpus[i].p, r.corpus[i].n) != 0;
        }
        ISO_CHECK_MSG(differs, "a different seed must give a different campaign");
    }
    fz_report_free(&r);
    fz_report_free(&r2);
    ISO_CHECK_EQ_INT(fz_handles_live(), 0);
}

static void test_crashers_not_mutated(void)
{
    fz_report r;
    fz_options o = opts(100);
    fz_input seed;
    const uint8_t s[] = {1};
    seed.p = s;
    seed.n = 1;
    /* seeds that crash are never mutated: a corpus of crashers fuzzes nothing, and is never clean */
    ISO_CHECK_EQ_INT(fz_run(&seed, 1, &o, always_crash, &r), FZ_OK);
    ISO_CHECK_EQ_INT(r.iterations, 0);
    ISO_CHECK_EQ_INT(r.ncrashes, 1);
    ISO_CHECK_EQ_INT(r.crashes[0].hits, 1);
    ISO_CHECK(!fz_report_is_clean(&r));
    ISO_CHECK_EQ_INT(strcmp(first_crash(&r)->kind, "boom"), 0);
    ISO_CHECK_EQ_INT(first_crash(&r)->nframes, 2);
    ISO_CHECK_EQ_INT(strcmp(first_crash(&r)->frames[0], "inner"), 0);
    fz_report_free(&r);
    /* max_iterations 0: nothing ran, so not clean */
    o = opts(0);
    ISO_CHECK_EQ_INT(fz_run(&seed, 1, &o, ok_target, &r), FZ_OK);
    ISO_CHECK(!fz_report_is_clean(&r));
    fz_report_free(&r);
}

static void test_feedback(void)
{
    const uint8_t s[] = {0, 0, 0};
    fz_input seed;
    uint64_t sd;
    int guided = 0;
    int blind = 0;
    seed.p = s;
    seed.n = 3;
    for (sd = 1; sd <= 6; sd++) {
        fz_report r;
        fz_options o = opts(6000);
        o.seed = sd;
        o.max_input_len = 16;
        ISO_CHECK_EQ_INT(fz_run(&seed, 1, &o, staircase, &r), FZ_OK);
        guided += r.ncrashes > 0;
        ISO_CHECK_EQ_INT(strcmp(r.tier, FZ_TIER_CALLER), 0);
        fz_report_free(&r);
        ISO_CHECK_EQ_INT(fz_run(&seed, 1, &o, staircase_blind, &r), FZ_OK);
        blind += r.ncrashes > 0;
        ISO_CHECK_EQ_INT(strcmp(r.tier, FZ_TIER_BLACKBOX), 0);
        fz_report_free(&r);
    }
    printf("  staircase: guided %d/6, blind %d/6\n", guided, blind);
    ISO_CHECK_MSG(guided >= 5, "per-stage coverage must crack the staircase for at least 5 of 6 seeds");
    ISO_CHECK_MSG(blind <= 3 && guided > blind, "blind mutation must crack it less often than guided");
}

/* ---- dedup and minimisation ---------------------------------------------------------------------------------- */

static int two_bugs(const uint8_t *d, size_t n, fz_handle *h, fz_crash *out)
{
    size_t i;
    (void)h;
    for (i = 0; i < n; i++) {
        if (d[i] == 'X') {
            fz_crash_set(out, "x-bug");
            fz_crash_push(out, "scan");
            return 1;
        }
    }
    for (i = 0; i < n; i++) {
        if (d[i] == 'Y') {
            fz_crash_set(out, "y-bug");
            fz_crash_push(out, "scan");
            return 1;
        }
    }
    return 0;
}

static void test_dedup_and_minimize(void)
{
    fz_report r;
    fz_options o = opts(400);
    fz_input seeds[3];
    const uint8_t a[] = {'q', 'q', 'X', 'q', 'q', 'q', 'q'};
    const uint8_t b[] = {'z', 'Y', 'z', 'z', 'z'};
    const uint8_t c[] = {'X', 'X', 'X'};
    size_t i;
    size_t total_hits = 0;
    seeds[0].p = a;
    seeds[0].n = sizeof a;
    seeds[1].p = b;
    seeds[1].n = sizeof b;
    seeds[2].p = c;
    seeds[2].n = sizeof c;
    ISO_CHECK_EQ_INT(fz_run(seeds, 3, &o, two_bugs, &r), FZ_OK);
    ISO_CHECK_EQ_INT(r.ncrashes, 2); /* one record per signature, initial seeds included */
    ISO_CHECK_EQ_INT(r.iterations, 0); /* every seed crashed: nothing to mutate */
    for (i = 0; i < r.ncrashes; i++) {
        total_hits += r.crashes[i].hits;
        ISO_CHECK(r.crashes[i].minimized);
        ISO_CHECK_EQ_INT(r.crashes[i].minimal_len, 1);
        ISO_CHECK(r.crashes[i].minimal[0] == (strcmp(r.crashes[i].crash.kind, "x-bug") == 0 ? 'X' : 'Y'));
    }
    ISO_CHECK_EQ_INT(total_hits, 3); /* hits counts every campaign execution with that signature */
    ISO_CHECK_EQ_INT(r.crashes[0].hits + r.crashes[1].hits, 3);
    /* the minimal input of x-bug was found from the 7-byte seed, then the 3-byte seed hit it too */
    fz_report_free(&r);
}

static int crash_if_len3(const uint8_t *d, size_t n, fz_handle *h, fz_crash *out)
{
    (void)d;
    (void)h;
    if (n >= 3) {
        fz_crash_set(out, "len3");
        return 1;
    }
    return 0;
}

static void test_minimize_shapes(void)
{
    fz_report r;
    fz_options o = opts(1000);
    fz_input seed;
    uint8_t big[40];
    size_t i;
    for (i = 0; i < sizeof big; i++) {
        big[i] = (uint8_t)(i + 1);
    }
    seed.p = big;
    seed.n = sizeof big;
    /* a length-3 crasher minimises to three ZERO bytes (block deletion, then byte zeroing) */
    ISO_CHECK_EQ_INT(fz_run(&seed, 1, &o, crash_if_len3, &r), FZ_OK);
    ISO_CHECK_EQ_INT(r.ncrashes, 1);
    ISO_CHECK_EQ_INT(r.crashes[0].minimal_len, 3);
    ISO_CHECK(r.crashes[0].minimal[0] == 0 && r.crashes[0].minimal[1] == 0 && r.crashes[0].minimal[2] == 0);
    ISO_CHECK(r.crashes[0].minimized);
    fz_report_free(&r);
    /* an always-crashing 1-byte input reaches the EMPTY input (first chunk is max(1, len/2)) */
    seed.n = 1;
    ISO_CHECK_EQ_INT(fz_run(&seed, 1, &o, always_crash, &r), FZ_OK);
    ISO_CHECK_EQ_INT(r.crashes[0].minimal_len, 0);
    ISO_CHECK(r.crashes[0].minimal != NULL);
    ISO_CHECK(r.crashes[0].minimized);
    fz_report_free(&r);
}

typedef struct counter {
    size_t calls;
    int reproduce;
} counter;

static int counting_test(void *user, const uint8_t *d, size_t n, fz_outcome *out)
{
    counter *c = (counter *)user;
    (void)d;
    (void)n;
    c->calls++;
    out->kind = c->reproduce ? FZ_CRASH : FZ_PASS;
    memset(&out->crash, 0, sizeof out->crash);
    if (c->reproduce) {
        fz_crash_set(&out->crash, "k");
    }
    return FZ_OK;
}

static void test_minimize_budget(void)
{
    uint8_t data[100];
    fz_bytes out;
    counter c;
    char sig[9];
    size_t i;
    static const char *const none[1] = {0};
    for (i = 0; i < sizeof data; i++) {
        data[i] = (uint8_t)(i + 1);
    }
    fz_crash_signature(sig, "k", none, 0);
    /* every REJECTED candidate counts: a test that never reproduces is called exactly `budget` times */
    c.calls = 0;
    c.reproduce = 0;
    ISO_CHECK_EQ_INT(fz_minimize_crashing_input(&out, data, sizeof data, sig, counting_test, &c, 10), FZ_OK);
    ISO_CHECK_EQ_INT(c.calls, 10);
    ISO_CHECK_EQ_INT(out.n, 100);
    ISO_CHECK(memcmp(out.p, data, 100) == 0);
    fz_bytes_free(&out);
    /* budget 0: no execution at all */
    c.calls = 0;
    ISO_CHECK_EQ_INT(fz_minimize_crashing_input(&out, data, sizeof data, sig, counting_test, &c, 0), FZ_OK);
    ISO_CHECK_EQ_INT(c.calls, 0);
    ISO_CHECK_EQ_INT(out.n, 100);
    fz_bytes_free(&out);
    /* an always-reproducing test shrinks to empty and stops */
    c.calls = 0;
    c.reproduce = 1;
    ISO_CHECK_EQ_INT(fz_minimize_crashing_input(&out, data, sizeof data, sig, counting_test, &c, 100000), FZ_OK);
    ISO_CHECK_EQ_INT(out.n, 0);
    ISO_CHECK(c.calls < 200);
    fz_bytes_free(&out);
    /* a DIFFERENT signature is not "the same crash": minimisation never trades one bug for another */
    c.calls = 0;
    c.reproduce = 1;
    fz_crash_signature(sig, "other", none, 0);
    ISO_CHECK_EQ_INT(fz_minimize_crashing_input(&out, data, sizeof data, sig, counting_test, &c, 50), FZ_OK);
    ISO_CHECK_EQ_INT(out.n, 100);
    fz_bytes_free(&out);
    /* argument validation */
    ISO_CHECK_EQ_INT(fz_minimize_crashing_input(&out, data, 4, sig, counting_test, &c, FZ_MAX_MINIMIZE_BUDGET + 1),
                     FZ_E_ARG);
    ISO_CHECK_EQ_INT(fz_minimize_crashing_input(&out, data, 4, sig, NULL, &c, 1), FZ_E_ARG);
    ISO_CHECK_EQ_INT(fz_minimize_crashing_input(&out, data, 4, "ZZZZZZZZ", counting_test, &c, 1), FZ_E_ARG);
    ISO_CHECK_EQ_INT(fz_minimize_crashing_input(&out, NULL, 4, sig, counting_test, &c, 1), FZ_E_ARG);
}

/* The campaign-wide cap: minimisation spends at most max_iterations executions in total, rejected candidates
 * included, however many crashes there are; per crash it is min(minimize_budget, what is left). */
static size_t g_execs;

static int crash_by_first_byte(const uint8_t *d, size_t n, fz_handle *h, fz_crash *out)
{
    char kind[16];
    (void)h;
    g_execs++;
    if (n == 0) {
        return 0;
    }
    (void)snprintf(kind, sizeof kind, "k%u", (unsigned)d[0]);
    fz_crash_set(out, kind);
    return 1;
}

static void test_campaign_minimize_cap(void)
{
    fz_report r;
    fz_options o = opts(0);
    fz_input seeds[10];
    uint8_t data[10][64];
    size_t i;
    size_t j;
    /* ten distinct crashes, each from a 64-byte seed; max_iterations 0 means NOTHING is minimised */
    for (i = 0; i < 10; i++) {
        for (j = 0; j < 64; j++) {
            data[i][j] = (uint8_t)(i + 1);
        }
        seeds[i].p = data[i];
        seeds[i].n = 64;
    }
    g_execs = 0;
    ISO_CHECK_EQ_INT(fz_run(seeds, 10, &o, crash_by_first_byte, &r), FZ_OK);
    ISO_CHECK_EQ_INT(r.ncrashes, 10);
    ISO_CHECK_EQ_INT(g_execs, 10);
    for (i = 0; i < r.ncrashes; i++) {
        ISO_CHECK(!r.crashes[i].minimized);
        ISO_CHECK_EQ_INT(r.crashes[i].minimal_len, 64);
    }
    fz_report_free(&r);
    /* max_iterations 100: the whole campaign minimises at most 100 executions, over all ten crashes */
    o = opts(100);
    g_execs = 0;
    ISO_CHECK_EQ_INT(fz_run(seeds, 10, &o, crash_by_first_byte, &r), FZ_OK);
    ISO_CHECK_EQ_INT(r.ncrashes, 10);
    ISO_CHECK(g_execs <= 10 + 100);
    {
        size_t unfinished = 0;
        for (i = 0; i < r.ncrashes; i++) {
            unfinished += !r.crashes[i].minimized;
        }
        ISO_CHECK_MSG(unfinished > 0, "the campaign-wide minimisation cap must leave later crashes unminimised");
    }
    fz_report_free(&r);
    /* per-crash budget: minimize_budget 5 stops each crash after 5 candidates */
    o = opts(1000);
    o.minimize_budget = 5;
    g_execs = 0;
    ISO_CHECK_EQ_INT(fz_run(seeds, 10, &o, crash_by_first_byte, &r), FZ_OK);
    ISO_CHECK(g_execs <= 10 + 10 * 5);
    for (i = 0; i < r.ncrashes; i++) {
        ISO_CHECK(!r.crashes[i].minimized);
    }
    fz_report_free(&r);
}

/* ---- budgets: sticky exhaustion --------------------------------------------------------------------------- */

static int g_mode;
static int g_second_tick;
static int g_second_reserve;
static int g_third_tick;

static int exhaust_target(const uint8_t *d, size_t n, fz_handle *h, fz_crash *out)
{
    (void)d;
    (void)n;
    switch (g_mode) {
    case 0: /* exhaust, then call again: both budgets answer "exhausted" at once */
        while (fz_tick(h) == 0) {
        }
        g_second_tick = fz_tick(h);
        g_second_reserve = fz_reserve(h, 1);
        g_third_tick = fz_tick(h);
        return 0;
    case 1: /* exhaust the steps, then return 0 (a pass!): still a TIMEOUT */
        while (fz_tick(h) == 0) {
        }
        return 0;
    case 2: /* exhaust the steps, then crash: still a TIMEOUT, never a crash */
        while (fz_tick(h) == 0) {
        }
        fz_crash_set(out, "after-timeout");
        return 1;
    case 3: /* exhaust memory, then tick (shared stickiness): the OOM stays the verdict */
        g_second_reserve = fz_reserve(h, (size_t)1 << 40);
        g_second_tick = fz_tick(h);
        g_third_tick = fz_reserve(h, 0);
        return 0;
    case 4: /* exhaust the steps first, then reserve: TIMEOUT wins (the budget that broke first) */
        while (fz_tick(h) == 0) {
        }
        (void)fz_reserve(h, (size_t)1 << 40);
        return 0;
    case 5: /* memory exhausted, then return nonzero: OOM, not a crash */
        (void)fz_reserve(h, (size_t)1 << 40);
        fz_crash_set(out, "after-oom");
        return 1;
    default:
        return 0;
    }
}

static void test_sticky_exhaustion(void)
{
    fz_report r;
    fz_options o = opts(3);
    fz_input seed;
    const uint8_t s[] = {1};
    seed.p = s;
    seed.n = 1;
    o.step_budget = 10;
    o.mem_budget = 1000;
    g_mode = 0;
    g_second_tick = g_second_reserve = g_third_tick = 0;
    ISO_CHECK_EQ_INT(fz_run(&seed, 1, &o, exhaust_target, &r), FZ_OK);
    ISO_CHECK(g_second_tick != 0 && g_second_reserve != 0 && g_third_tick != 0); /* exhaust-then-call */
    ISO_CHECK_EQ_INT(r.timeouts, 1); /* a timeout seed is never mutated: just the triage run */
    fz_report_free(&r);
}

static void test_sticky_each_mode(void)
{
    fz_report r;
    fz_options o = opts(3);
    fz_input seed;
    const uint8_t s[] = {1};
    seed.p = s;
    seed.n = 1;
    o.step_budget = 10;
    o.mem_budget = 1000;
    /* mode 1: exhaust-then-return-0 */
    g_mode = 1;
    ISO_CHECK_EQ_INT(fz_run(&seed, 1, &o, exhaust_target, &r), FZ_OK);
    ISO_CHECK_EQ_INT(r.timeouts, 1);
    ISO_CHECK_EQ_INT(r.ncrashes, 0);
    ISO_CHECK_EQ_INT(r.ncorpus, 0); /* never a pass: not admitted */
    ISO_CHECK(!fz_report_is_clean(&r));
    fz_report_free(&r);
    /* mode 2: exhaust-then-crash is a timeout, not a crash */
    g_mode = 2;
    ISO_CHECK_EQ_INT(fz_run(&seed, 1, &o, exhaust_target, &r), FZ_OK);
    ISO_CHECK_EQ_INT(r.timeouts, 1);
    ISO_CHECK_EQ_INT(r.ncrashes, 0);
    fz_report_free(&r);
    /* mode 3: OOM then tick */
    g_mode = 3;
    g_second_tick = g_second_reserve = g_third_tick = 0;
    ISO_CHECK_EQ_INT(fz_run(&seed, 1, &o, exhaust_target, &r), FZ_OK);
    ISO_CHECK_EQ_INT(r.ooms, 1);
    ISO_CHECK_EQ_INT(r.timeouts, 0);
    ISO_CHECK(g_second_reserve != 0 && g_second_tick != 0 && g_third_tick != 0);
    fz_report_free(&r);
    /* mode 4: the budget that broke first wins */
    g_mode = 4;
    ISO_CHECK_EQ_INT(fz_run(&seed, 1, &o, exhaust_target, &r), FZ_OK);
    ISO_CHECK_EQ_INT(r.timeouts, 1);
    ISO_CHECK_EQ_INT(r.ooms, 0);
    fz_report_free(&r);
    /* mode 5: OOM then crash */
    g_mode = 5;
    ISO_CHECK_EQ_INT(fz_run(&seed, 1, &o, exhaust_target, &r), FZ_OK);
    ISO_CHECK_EQ_INT(r.ooms, 1);
    ISO_CHECK_EQ_INT(r.ncrashes, 0);
    fz_report_free(&r);
    ISO_CHECK_EQ_INT(fz_handles_live(), 0);
}

static int budget_edges(const uint8_t *d, size_t n, fz_handle *h, fz_crash *out)
{
    size_t i;
    (void)d;
    (void)n;
    (void)out;
    /* exactly step_budget ticks are fine; the next one is exhausted.  Memory: reserve up to the budget. */
    for (i = 0; i < 10; i++) {
        if (fz_tick(h) != 0) {
            return 1;
        }
    }
    if (fz_reserve(h, 600) != 0 || fz_reserve(h, 400) != 0) {
        return 1;
    }
    return fz_tick(h) == 0 ? 1 : (fz_reserve(h, 0) == 0 ? 1 : 0); /* both must now say exhausted */
}

static void test_budget_boundaries(void)
{
    fz_report r;
    fz_options o = opts(5);
    fz_input seed;
    const uint8_t s[] = {1};
    seed.p = s;
    seed.n = 1;
    o.step_budget = 10;
    o.mem_budget = 1000;
    /* 10 ticks and exactly mem_budget bytes fit; the 11th tick is the breach: the run is a TIMEOUT (sticky),
     * and the target's own checks (it returns 0 only when everything behaved) did not misfire */
    ISO_CHECK_EQ_INT(fz_run(&seed, 1, &o, budget_edges, &r), FZ_OK);
    ISO_CHECK_EQ_INT(r.ncrashes, 0);
    ISO_CHECK_EQ_INT(r.timeouts, 1);
    fz_report_free(&r);
    /* a reserve that would overflow a naive reserved + n sum is refused, not wrapped */
    g_mode = 3;
    o.mem_budget = FZ_MAX_MEM_BUDGET;
    ISO_CHECK_EQ_INT(fz_run(&seed, 1, &o, exhaust_target, &r), FZ_OK);
    ISO_CHECK_EQ_INT(r.ooms, 1);
    fz_report_free(&r);
}

/* ---- stale handles ---------------------------------------------------------------------------------------- */

static fz_handle *g_stale;
static uint64_t g_stale_epoch;
static size_t g_runs;
static int g_bad;
static int g_stale_tick_nonzero;
static int g_stale_reserve_nonzero;
static void *g_stale_user;

static int stale_target(const uint8_t *d, size_t n, fz_handle *h, fz_crash *out)
{
    size_t i;
    (void)d;
    (void)n;
    (void)out;
    g_runs++;
    if (g_stale != NULL) {
        /* the previous execution's handle: retired.  None of this may touch THIS execution. */
        fz_edge(g_stale, 777);
        g_stale_tick_nonzero += fz_tick(g_stale) != 0;
        g_stale_reserve_nonzero += fz_reserve(g_stale, 1) != 0;
        g_stale_user = fz_handle_user(g_stale);
        if (fz_handle_epoch(g_stale) == fz_handle_epoch(h)) {
            g_bad = 1; /* a fresh handle per execution */
        }
    }
    /* this execution still has its WHOLE budget (20 ticks), whatever the stale handle did */
    for (i = 0; i < 20; i++) {
        if (fz_tick(h) != 0) {
            g_bad = 1;
        }
    }
    fz_edge(h, 5);
    if (fz_active_handle() != h) {
        g_bad = 1;
    }
    g_stale = h;
    g_stale_epoch = fz_handle_epoch(h);
    return 0;
}

static void test_stale_handle_inert(void)
{
    fz_report r;
    fz_options o = opts(50);
    fz_input seed;
    const uint8_t s[] = {1, 2};
    int *marker = &g_bad;
    seed.p = s;
    seed.n = 2;
    o.step_budget = 20;
    o.user = marker;
    g_stale = NULL;
    g_runs = 0;
    g_bad = 0;
    g_stale_tick_nonzero = g_stale_reserve_nonzero = 0;
    ISO_CHECK_EQ_INT(fz_run(&seed, 1, &o, stale_target, &r), FZ_OK);
    ISO_CHECK_EQ_INT(g_bad, 0);
    ISO_CHECK_EQ_INT(g_runs, 51);
    ISO_CHECK_EQ_INT(g_stale_tick_nonzero, 50);
    ISO_CHECK_EQ_INT(g_stale_reserve_nonzero, 50);
    ISO_CHECK(g_stale_user == NULL);
    ISO_CHECK_EQ_INT(r.timeouts, 0); /* the stale ticks never counted against a later run */
    ISO_CHECK_EQ_INT(r.total_edges, 1); /* edge 777 reported through stale handles never reached the map */
    ISO_CHECK(fz_active_handle() == NULL);
    ISO_CHECK_EQ_INT(fz_handles_live(), 0);
    fz_report_free(&r);
    /* after the campaign has returned, the stale pointer is still memory-safe and inert (static storage) */
    fz_edge(g_stale, 1);
    ISO_CHECK(fz_tick(g_stale) != 0);
    ISO_CHECK(fz_reserve(g_stale, 0) != 0);
    ISO_CHECK(fz_handle_user(g_stale) == NULL);
    ISO_CHECK(g_stale_epoch != 0);
    /* NULL handles are inert too */
    fz_edge(NULL, 1);
    ISO_CHECK(fz_tick(NULL) != 0);
    ISO_CHECK(fz_reserve(NULL, 1) != 0);
    ISO_CHECK(fz_handle_user(NULL) == NULL);
}

static void *g_seen_user;

static int user_target(const uint8_t *d, size_t n, fz_handle *h, fz_crash *out)
{
    (void)d;
    (void)n;
    (void)out;
    g_seen_user = fz_handle_user(h);
    return 0;
}

static void test_user_pointer(void)
{
    fz_report r;
    fz_options o = opts(1);
    int token = 0;
    o.user = &token;
    g_seen_user = NULL;
    ISO_CHECK_EQ_INT(fz_run(NULL, 0, &o, user_target, &r), FZ_OK);
    ISO_CHECK(g_seen_user == &token);
    fz_report_free(&r);
}

/* The ring: a handle is reissued only when no longer live, and two live handles never share a slot. */
static void test_ring(void)
{
    fz_bitmap *cov = (fz_bitmap *)malloc(sizeof *cov);
    fz_handle *hs[8];
    fz_handle *again;
    size_t i;
    size_t j;
    ISO_CHECK(cov != NULL);
    if (cov == NULL) {
        return;
    }
    fz_bitmap_init(cov);
    for (i = 0; i < 8; i++) {
        hs[i] = fz_handle_issue(cov, 5, 5, NULL);
        ISO_CHECK(hs[i] != NULL);
        for (j = 0; j < i; j++) {
            ISO_CHECK(hs[i] != hs[j] && fz_handle_epoch(hs[i]) != fz_handle_epoch(hs[j]));
        }
    }
    ISO_CHECK_EQ_INT(fz_handles_live(), 8);
    for (i = 0; i < 8; i++) {
        ISO_CHECK_EQ_INT(fz_handle_retire(hs[i], NULL), 0);
    }
    ISO_CHECK_EQ_INT(fz_handles_live(), 0);
    /* fill every slot: the next issue fails rather than aliasing a live handle */
    {
        fz_handle **all = (fz_handle **)malloc(sizeof(fz_handle *) * FZ_HANDLE_RING);
        for (i = 0; i < FZ_HANDLE_RING; i++) {
            all[i] = fz_handle_issue(cov, 5, 5, NULL);
            ISO_CHECK(all[i] != NULL);
        }
        again = fz_handle_issue(cov, 5, 5, NULL);
        ISO_CHECK(again == NULL);
        for (i = 0; i < FZ_HANDLE_RING; i++) {
            (void)fz_handle_retire(all[i], NULL);
        }
        free(all);
    }
    again = fz_handle_issue(cov, 5, 5, NULL);
    ISO_CHECK(again != NULL);
    (void)fz_handle_retire(again, NULL);
    ISO_CHECK_EQ_INT(fz_handles_live(), 0);
    free(cov);
}

/* A campaign nested inside a target: the outer handle stays live and untouched. */
static int g_inner_runs;
static int g_outer_ok;

static int inner_target(const uint8_t *d, size_t n, fz_handle *h, fz_crash *out)
{
    (void)d;
    (void)n;
    (void)out;
    g_inner_runs++;
    fz_edge(h, 9);
    return 0;
}

static int outer_target(const uint8_t *d, size_t n, fz_handle *h, fz_crash *out)
{
    fz_report r;
    fz_options o = opts(20);
    fz_handle *before = fz_active_handle();
    (void)d;
    (void)n;
    (void)out;
    ISO_CHECK(fz_tick(h) == 0);
    if (fz_run(NULL, 0, &o, inner_target, &r) == FZ_OK) {
        fz_report_free(&r);
    }
    g_outer_ok = fz_active_handle() == before && before == h && fz_tick(h) == 0;
    fz_edge(h, 3);
    return 0;
}

static void test_nested_campaign(void)
{
    fz_report r;
    fz_options o = opts(3);
    g_inner_runs = 0;
    g_outer_ok = 0;
    ISO_CHECK_EQ_INT(fz_run(NULL, 0, &o, outer_target, &r), FZ_OK);
    ISO_CHECK(g_outer_ok);
    ISO_CHECK(g_inner_runs >= 21);
    ISO_CHECK_EQ_INT(r.total_edges, 1); /* only the outer edge: the inner campaign's edges are its own */
    ISO_CHECK_EQ_INT(fz_handles_live(), 0);
    fz_report_free(&r);
}

/* ---- copies ----------------------------------------------------------------------------------------------- */

static int scribbler(const uint8_t *d, size_t n, fz_handle *h, fz_crash *out)
{
    uint8_t *w = (uint8_t *)d; /* the buffer is the target's own copy: writing through it is legal */
    size_t i;
    (void)h;
    (void)out;
    for (i = 0; i < n; i++) {
        w[i] = 0xEE;
    }
    return 0;
}

static void test_input_is_copied(void)
{
    fz_report r;
    fz_options o = opts(50);
    fz_input seed;
    uint8_t s[] = {1, 2, 3, 4};
    seed.p = s;
    seed.n = 4;
    ISO_CHECK_EQ_INT(fz_run(&seed, 1, &o, scribbler, &r), FZ_OK);
    ISO_CHECK(s[0] == 1 && s[3] == 4); /* the caller's seed is untouched */
    ISO_CHECK(r.ncorpus >= 1 && r.corpus[0].p[0] == 1 && r.corpus[0].p[3] == 4); /* and so is the corpus */
    fz_report_free(&r);
}

/* ---- caps ------------------------------------------------------------------------------------------------- */

static int many_kinds(const uint8_t *d, size_t n, fz_handle *h, fz_crash *out)
{
    char kind[16];
    (void)h;
    if (n == 0) {
        return 0;
    }
    (void)snprintf(kind, sizeof kind, "kind%u", (unsigned)d[0]);
    fz_crash_set(out, kind);
    return 1;
}

static void test_caps(void)
{
    fz_report r;
    fz_options o = opts(10);
    fz_input seeds[6];
    uint8_t data[6][8];
    size_t i;
    for (i = 0; i < 6; i++) {
        memset(data[i], (int)(i + 1), 8);
        seeds[i].p = data[i];
        seeds[i].n = 8;
    }
    /* the crash cap: further novel signatures only increment crashes_dropped */
    fz_hooks.max_crashes = 2;
    ISO_CHECK_EQ_INT(fz_run(seeds, 6, &o, many_kinds, &r), FZ_OK);
    fz_hooks_reset();
    ISO_CHECK_EQ_INT(r.ncrashes, 2);
    ISO_CHECK_EQ_INT(r.crashes_dropped, 4);
    ISO_CHECK_EQ_INT(fz_report_unique_crashes(&r), 6);
    ISO_CHECK(!fz_report_is_clean(&r));
    fz_report_free(&r);
    /* the retained-bytes cap: an admission over the cap is dropped and counted */
    {
        const uint8_t big[] = {1, 2, 3, 4, 5, 6, 7, 8, 9, 10};
        fz_input seed;
        seed.p = big;
        seed.n = sizeof big;
        o = opts(300);
        fz_hooks.max_retained_bytes = 12;
        ISO_CHECK_EQ_INT(fz_run(&seed, 1, &o, staircase, &r), FZ_OK);
        fz_hooks_reset();
        {
            size_t total = 0;
            for (i = 0; i < r.ncorpus; i++) {
                total += r.corpus[i].n;
            }
            ISO_CHECK(total <= 12);
        }
        ISO_CHECK_EQ_INT(r.ncorpus, 1);
        fz_report_free(&r);
        o = opts(5);
        fz_hooks.max_retained_bytes = 12;
        ISO_CHECK_EQ_INT(fz_run(seed.p == NULL ? NULL : &seed, 1, &o, counting_target, &r), FZ_OK);
        fz_hooks_reset();
        ISO_CHECK_EQ_INT(r.ncorpus, 1);
        fz_report_free(&r);
        /* a crash whose input does not fit keeps no input and is not minimised */
        fz_hooks.max_retained_bytes = 4;
        ISO_CHECK_EQ_INT(fz_run(&seed, 1, &o, always_crash, &r), FZ_OK);
        fz_hooks_reset();
        ISO_CHECK_EQ_INT(r.ncrashes, 1);
        ISO_CHECK(r.crashes[0].input_dropped == 1 && r.crashes[0].minimal == NULL && !r.crashes[0].minimized);
        ISO_CHECK_EQ_INT(r.crashes[0].minimal_len, 0);
        fz_report_free(&r);
        /* a seed that does not fit is counted as a dropped admission */
        fz_hooks.max_retained_bytes = 4;
        ISO_CHECK_EQ_INT(fz_run(&seed, 1, &o, counting_target, &r), FZ_OK);
        fz_hooks_reset();
        ISO_CHECK_EQ_INT(r.ncorpus, 0);
        ISO_CHECK(r.admissions_dropped >= 1);
        fz_report_free(&r);
    }
    /* argument validation: every field has a ceiling, and a campaign with a bad one never runs */
    {
        fz_options bad;
        fz_report rr;
        size_t k;
        for (k = 0; k < 6; k++) {
            bad = fz_options_default();
            switch (k) {
            case 0: bad.max_iterations = FZ_MAX_ITERATIONS + 1; break;
            case 1: bad.max_input_len = FZ_MAX_INPUT_BYTES + 1; break;
            case 2: bad.base_energy = FZ_MAX_ITERATIONS + 1; break;
            case 3: bad.minimize_budget = FZ_MAX_MINIMIZE_BUDGET + 1; break;
            case 4: bad.step_budget = FZ_MAX_STEP_BUDGET + 1; break;
            default: bad.mem_budget = FZ_MAX_MEM_BUDGET + 1; break;
            }
            ISO_CHECK_EQ_INT(fz_options_validate(&bad), FZ_E_ARG);
            g_calls = 0;
            ISO_CHECK_EQ_INT(fz_run(NULL, 0, &bad, counting_target, &rr), FZ_E_ARG);
            ISO_CHECK_EQ_INT(g_calls, 0);
            ISO_CHECK(rr.corpus == NULL && rr.crashes == NULL);
        }
        ISO_CHECK_EQ_INT(fz_options_validate(NULL), FZ_E_ARG);
        o = fz_options_default();
        ISO_CHECK_EQ_INT(fz_options_validate(&o), FZ_OK);
        ISO_CHECK_EQ_INT(fz_run(NULL, 0, &o, NULL, &rr), FZ_E_ARG);
        ISO_CHECK_EQ_INT(fz_run(NULL, 0, &o, ok_target, NULL), FZ_E_ARG);
        ISO_CHECK_EQ_INT(fz_run(NULL, 1, &o, ok_target, &rr), FZ_E_ARG);
        {
            fz_input toolong;
            toolong.p = data[0];
            toolong.n = FZ_MAX_INPUT_BYTES + 1;
            ISO_CHECK_EQ_INT(fz_run(&toolong, 1, &o, ok_target, &rr), FZ_E_ARG);
            toolong.p = NULL;
            toolong.n = 1;
            ISO_CHECK_EQ_INT(fz_run(&toolong, 1, &o, ok_target, &rr), FZ_E_ARG);
        }
        {
            fz_input *many = (fz_input *)calloc(FZ_MAX_INITIAL_CORPUS + 1, sizeof(fz_input));
            ISO_CHECK(many != NULL);
            if (many != NULL) {
                ISO_CHECK_EQ_INT(fz_run(many, FZ_MAX_INITIAL_CORPUS + 1, &o, ok_target, &rr), FZ_E_ARG);
                free(many);
            }
        }
    }
}

/* ---- outcome-level executors and tiers -------------------------------------------------------------------- */

static int out_bad_kind(void *user, const uint8_t *d, size_t n, fz_bitmap *cov, fz_outcome *out)
{
    (void)user;
    (void)d;
    (void)n;
    (void)cov;
    out->kind = (fz_outcome_kind)99;
    return FZ_OK;
}

static int out_timeout_on_odd(void *user, const uint8_t *d, size_t n, fz_bitmap *cov, fz_outcome *out)
{
    (void)user;
    fz_bitmap_record(cov, 41);
    out->kind = (n > 0 && (d[0] & 1u)) ? FZ_TIMEOUT : FZ_PASS;
    return FZ_OK;
}

static int out_nomem(void *user, const uint8_t *d, size_t n, fz_bitmap *cov, fz_outcome *out)
{
    (void)user;
    (void)d;
    (void)n;
    (void)cov;
    (void)out;
    return FZ_E_NOMEM;
}

static void test_outcome_executors(void)
{
    fz_report r;
    fz_options o = opts(100);
    ISO_CHECK_EQ_INT(fz_run_outcomes(NULL, 0, &o, out_bad_kind, NULL, &r), FZ_E_ARG);
    ISO_CHECK(r.crashes == NULL && r.corpus == NULL);
    ISO_CHECK_EQ_INT(fz_run_outcomes(NULL, 0, &o, out_nomem, NULL, &r), FZ_E_NOMEM);
    ISO_CHECK(r.corpus == NULL);
    ISO_CHECK_EQ_INT(fz_run_outcomes(NULL, 0, &o, NULL, NULL, &r), FZ_E_ARG);
    {
        const uint8_t s[] = {2};
        fz_input seed;
        seed.p = s;
        seed.n = 1;
        ISO_CHECK_EQ_INT(fz_run_outcomes(&seed, 1, &o, out_timeout_on_odd, NULL, &r), FZ_OK);
        ISO_CHECK(r.timeouts > 0); /* an outcome-level executor (a process harness) reports its own timeouts */
        ISO_CHECK_EQ_INT(strcmp(r.tier, FZ_TIER_CALLER), 0);
        ISO_CHECK_EQ_INT(r.total_edges, 1);
        fz_report_free(&r);
    }
}

/* ---- privacy of every rendering ---------------------------------------------------------------------------- */

static int secret_crash(const uint8_t *d, size_t n, fz_handle *h, fz_crash *out)
{
    size_t i;
    (void)h;
    for (i = 0; i + 5 < n; i++) {
        if (memcmp(d + i, "SECRET", 6) == 0) {
            fz_crash_set(out, "leaky");
            fz_crash_push(out, "scan");
            return 1;
        }
    }
    return 0;
}

static void test_privacy(void)
{
    fz_report r;
    fz_options o = opts(50);
    fz_input seed;
    const uint8_t s[] = {'x', 'S', 'E', 'C', 'R', 'E', 'T', 'x'};
    char buf[512];
    char line[256];
    size_t i;
    size_t n;
    seed.p = s;
    seed.n = sizeof s;
    o.seed = 0xDEADBEEFCAFEF00Du;
    ISO_CHECK_EQ_INT(fz_run(&seed, 1, &o, secret_crash, &r), FZ_OK);
    ISO_CHECK_EQ_INT(r.ncrashes, 1);
    (void)fz_report_summary(&r, buf, sizeof buf);
    ISO_CHECK(strstr(buf, "SECRET") == NULL);
    ISO_CHECK(strstr(buf, "534543524554") == NULL); /* the bytes in hex */
    ISO_CHECK(strstr(buf, "deadbeef") == NULL && strstr(buf, "DEADBEEF") == NULL && strstr(buf, "3735928559") == NULL);
    ISO_CHECK(strstr(buf, "16045690984503098381") == NULL); /* the seed in decimal */
    ISO_CHECK(strstr(buf, "crashes=1") != NULL);
    ISO_CHECK_EQ_INT(fz_tap_failure_count(&r), 1);
    ISO_CHECK_EQ_INT(fz_tap_failure_line(&r, 0, "my target", 3, line, sizeof line), FZ_OK);
    ISO_CHECK(strncmp(line, "not ok 3 - my_target [fuzz-crash:", 33) == 0);
    ISO_CHECK_EQ_INT(strlen(line), strlen("not ok 3 - my_target [fuzz-crash:00000000]"));
    ISO_CHECK(strstr(line, "SECRET") == NULL && strstr(line, "5345") == NULL);
    ISO_CHECK(fz_tap_failure_line(&r, 1, "t", 1, line, sizeof line) == FZ_E_ARG);
    /* first_test + index must not overflow an int (index 0 + INT_MAX is fine; a negative base is refused) */
    ISO_CHECK_EQ_INT(fz_tap_failure_line(&r, 0, "t", 2147483647, line, sizeof line), FZ_OK);
    ISO_CHECK_EQ_INT(fz_tap_failure_line(&r, 0, "t", -1, line, sizeof line), FZ_E_ARG);
    /* the line carries the SAME signature the record holds */
    {
        char want[64];
        (void)snprintf(want, sizeof want, "[fuzz-crash:%s]", r.crashes[0].sig);
        ISO_CHECK(strstr(line, want) != NULL);
    }
    /* the stored minimal input is the caller's to read, but is exactly the secret and nothing else */
    ISO_CHECK(r.crashes[0].minimal_len >= 1);
    fz_report_free(&r);
    /* timeout and oom lines: a hash of the bare kind, one line each; crashes first */
    {
        fz_report t;
        fz_report_init(&t);
        t.timeouts = 3;
        t.ooms = 1;
        t.iterations = 1;
        ISO_CHECK_EQ_INT(fz_tap_failure_count(&t), 2);
        ISO_CHECK_EQ_INT(fz_tap_failure_line(&t, 0, "t", 1, line, sizeof line), FZ_OK);
        ISO_CHECK_EQ_INT(strcmp(line, "not ok 1 - t [fuzz-timeout:4e61ea76]"), 0);
        ISO_CHECK_EQ_INT(fz_tap_failure_line(&t, 1, "t", 1, line, sizeof line), FZ_OK);
        ISO_CHECK_EQ_INT(strcmp(line, "not ok 2 - t [fuzz-oom:7146a410]"), 0);
        ISO_CHECK_EQ_INT(fz_tap_failure_line(&t, 1, "t", 2147483647, line, sizeof line), FZ_E_ARG); /* 2^31-1 + 1 */
        t.timeouts = 0;
        ISO_CHECK_EQ_INT(fz_tap_failure_count(&t), 1);
        ISO_CHECK_EQ_INT(fz_tap_failure_line(&t, 0, "t", 1, line, sizeof line), FZ_OK);
        ISO_CHECK_EQ_INT(strcmp(line, "not ok 1 - t [fuzz-oom:7146a410]"), 0);
        t.ooms = 0;
        ISO_CHECK_EQ_INT(fz_tap_failure_count(&t), 0);
        n = fz_report_summary(&t, buf, 8); /* truncation is safe and reports the length needed */
        ISO_CHECK(n > 8 && strlen(buf) == 7);
        for (i = 0; i < 1; i++) {
            ISO_CHECK(fz_tap_failure_line(&t, 0, "t", 1, line, sizeof line) == FZ_E_ARG);
        }
    }
}

int main(void)
{
    test_basic_contracts();
    test_crashers_not_mutated();
    test_feedback();
    test_dedup_and_minimize();
    test_minimize_shapes();
    test_minimize_budget();
    test_campaign_minimize_cap();
    test_sticky_exhaustion();
    test_sticky_each_mode();
    test_budget_boundaries();
    test_stale_handle_inert();
    test_user_pointer();
    test_ring();
    test_nested_campaign();
    test_input_is_copied();
    test_caps();
    test_outcome_executors();
    test_privacy();
    fz_hooks_reset();
    ISO_CHECK_EQ_INT(pbt_hooks.live_allocs, 0);
    ISO_CHECK_EQ_INT(fz_handles_live(), 0);
    return ISO_TEST_RESULT();
}
