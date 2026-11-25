/*
 * test_oom.c -- out-of-memory discipline (VER05i section 9).
 *
 * Every allocation in the engine goes through pbt_alloc/pbt_realloc, which the
 * test seam can make FAIL at the N-th call.  Each scenario is run with the
 * failure injected at allocation 1, 2, 3 ... until a run completes without the
 * injection firing (the sweep is exhaustive for that scenario), and at EVERY
 * point the test asserts:
 *
 *   - no crash (the sanitizer build also proves no out-of-bounds or use-after-free);
 *   - no leak: live allocations return to the baseline after cleanup;
 *   - no false success: a run that lost an allocation never claims success.
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "iso_test.h"
#include "pbt.h"
#include "pbt_hooks.h"
#include "pbt_internal.h"
#include "test_util.h"

static long sweep_count;

/*
 * A scenario returns 1 when it CLAIMS success (a pass, a clean parse...).  A STRICT scenario may never
 * claim success once an allocation failed; a non-strict one (a falsified check, a corpus run) may degrade
 * gracefully (a lost message text does not change the verdict) but must still leak nothing and crash nowhere.
 */
typedef int (*scenario_fn)(void *ctx);

static void sweep(const char *name, scenario_fn fn, void *ctx, int strict)
{
    long n;
    int clean = fn(ctx);
    ISO_CHECK_MSG(clean == 1, name); /* without injection the scenario succeeds */
    for (n = 1; n < 20000; n++) {
        long baseline = pbt_hooks.live_allocs;
        long before = pbt_hooks.injected;
        int claims;
        int fired;
        pbt_hooks.fail_after = n;
        claims = fn(ctx);
        pbt_hooks.fail_after = 0;
        fired = pbt_hooks.injected != before;
        sweep_count++;
        if (strict && fired && claims) {
            printf("  %s: claimed success although allocation %ld failed\n", name, n);
        }
        ISO_CHECK(!(strict && fired && claims));
        if (pbt_hooks.live_allocs != baseline) {
            printf("  %s: leak when allocation %ld failed (%ld live, baseline %ld)\n", name, n,
                   pbt_hooks.live_allocs, baseline);
        }
        ISO_CHECK_EQ_INT(pbt_hooks.live_allocs, baseline);
        if (!fired) {
            return; /* n exceeded the scenario's allocations: the sweep is complete */
        }
    }
    ISO_CHECK_MSG(0, "the sweep did not terminate");
}

static int always_holds(void *user, pbt_budget *scope, const pbt_val *v)
{
    (void)user;
    (void)scope;
    (void)v;
    return 1;
}

static long list_sum(const pbt_val *v)
{
    long s = 0;
    size_t i;
    for (i = 0; i < v->u.list.n; i++) {
        s += (long)pbt_big_to_i64(&v->u.list.items[i].u.i);
    }
    return s;
}

static int sum_lt_10(void *user, pbt_budget *scope, const pbt_val *v)
{
    (void)user;
    (void)scope;
    return list_sum(v) < 10;
}

/* ---- scenario 1: a complete passing check over a composite generator ---------------------------- */

static int scenario_pass(void *ctx)
{
    pbt_gen *u8 = pbt_gen_u8();
    pbt_gen *bytes = pbt_gen_bytes(0, 6);
    pbt_gen *pair[2];
    pbt_gen *tuple;
    pbt_gen *list;
    pbt_verdict vd;
    pbt_options o;
    int claims;
    (void)ctx;
    pair[0] = u8;
    pair[1] = bytes;
    tuple = pbt_gen_tuple(pair, 2);
    list = pbt_gen_list(tuple, 0, 4);
    o.num_tests = 5;
    o.max_shrink_steps = 50;
    o.ci = PBT_CI_OFF;
    claims = 0;
    if (list != NULL) {
        pbt_prop p;
        p.fn = always_holds;
        p.user = NULL;
        if (pbt_check(NULL, 7, list, p, &o, NULL, &vd) == 0) {
            claims = vd.passed;
        } else {
            claims = vd.passed; /* PBT_E_NOMEM/ARG: the verdict is a failure */
        }
        pbt_verdict_free(&vd);
    }
    pbt_gen_release(list);
    pbt_gen_release(tuple);
    pbt_gen_release(bytes);
    pbt_gen_release(u8);
    return claims;
}

/* ---- scenario 2: a falsified check (clone, shrink, render, verdict) -------------------------------- */

static int scenario_falsified(void *ctx)
{
    pbt_gen *u8 = pbt_gen_u8();
    pbt_gen *list = pbt_gen_list(u8, 0, 12);
    pbt_verdict vd;
    pbt_options o;
    int claims = 0;
    int ok_shape = 1;
    (void)ctx;
    o.num_tests = 30;
    o.max_shrink_steps = 200;
    o.ci = PBT_CI_OFF;
    if (list != NULL) {
        pbt_prop p;
        long before = pbt_hooks.injected;
        p.fn = sum_lt_10;
        p.user = NULL;
        (void)pbt_check(NULL, 5, list, p, &o, NULL, &vd);
        claims = vd.passed;
        /* A lost allocation may degrade the verdict, but it must still be a FAILURE with a closed id. */
        if (!vd.passed) {
            ok_shape = vd.failure_id != NULL && pbt_is_failure_id(vd.failure_id);
        }
        if (pbt_hooks.injected == before && vd.minimal.kind == PBT_V_LIST) {
            ok_shape = ok_shape && list_sum(&vd.minimal) >= 10; /* an unperturbed run reports a real counterexample */
        }
        pbt_verdict_free(&vd);
    }
    ISO_CHECK(ok_shape);
    pbt_gen_release(list);
    pbt_gen_release(u8);
    return !claims && ok_shape; /* success here means: a failing verdict of the right shape */
}

/* ---- scenario 3: redacted verdict (canon + digest) -------------------------------------------------- */

static int scenario_redacted(void *ctx)
{
    pbt_gen *g = pbt_gen_bytes(8, 8);
    pbt_verdict vd;
    pbt_options o;
    int claims = 0;
    (void)ctx;
    o.num_tests = 3;
    o.max_shrink_steps = 20;
    o.ci = PBT_CI_ON;
    if (g != NULL) {
        pbt_prop p;
        p.fn = always_holds;
        p.user = NULL;
        (void)pbt_check(NULL, 3, g, p, &o, NULL, &vd);
        claims = vd.passed;
        pbt_verdict_free(&vd);
    }
    pbt_gen_release(g);
    return claims;
}

/* ---- scenario 4: combinator construction and sampling, then release ---------------------------------- */

static int map_id(void *self, const pbt_val *in, pbt_val *out)
{
    (void)self;
    return pbt_val_clone(out, in);
}

static int keep_all(void *self, pbt_budget *b, const pbt_val *v)
{
    (void)self;
    (void)b;
    (void)v;
    return 1;
}

static int to_list(void *self, const pbt_val *in, pbt_gen **out)
{
    pbt_gen *u8 = pbt_gen_u8();
    (void)self;
    (void)in;
    *out = pbt_gen_list(u8, 2, 2);
    pbt_gen_release(u8);
    return *out == NULL ? PBT_E_NOMEM : 0;
}

static int scenario_combinators(void *ctx)
{
    pbt_gen *u8 = pbt_gen_u8();
    pbt_gen *mapped = pbt_gen_map(u8, map_id, NULL);
    pbt_gen *filtered = pbt_gen_filter(mapped, keep_all, NULL, 5);
    pbt_gen *flat = pbt_gen_flat_map(filtered, to_list, NULL);
    pbt_gen *kids[2];
    pbt_gen *one;
    pbt_arm arms[2];
    pbt_gen *freq;
    pbt_val c;
    pbt_gen *konst;
    int ok = 1;
    int i;
    pbt_run run;
    (void)ctx;
    pbt_rng_init(&run.rng, 1);
    run.budget = NULL;
    run.depth = 0;
    kids[0] = flat;
    kids[1] = u8;
    one = pbt_gen_one_of(kids, 2);
    arms[0].weight = 2;
    arms[0].gen = flat;
    arms[1].weight = 1;
    arms[1].gen = u8;
    freq = pbt_gen_frequency(arms, 2);
    pbt_val_nil(&c);
    if (pbt_val_set_bytes(&c, (const uint8_t *)"abc", 3) != 0) {
        ok = 0;
    }
    konst = pbt_gen_constant(&c);
    pbt_val_free(&c);
    if (u8 == NULL || mapped == NULL || filtered == NULL || flat == NULL || one == NULL || freq == NULL || konst == NULL) {
        ok = 0;
    }
    for (i = 0; ok && i < 6; i++) {
        pbt_val v;
        const pbt_gen *g = i % 3 == 0 ? one : (i % 3 == 1 ? freq : konst);
        int rc = pbt_gen_sample(&run, g, &v);
        if (rc != 0) {
            ok = 0;
        }
        pbt_val_free(&v);
    }
    pbt_gen_release(konst);
    pbt_gen_release(freq);
    pbt_gen_release(one);
    pbt_gen_release(flat);
    pbt_gen_release(filtered);
    pbt_gen_release(mapped);
    pbt_gen_release(u8);
    return ok;
}

/* ---- scenario 5: value clone / list building ---------------------------------------------------------- */

static int scenario_values(void *ctx)
{
    pbt_val a;
    pbt_val c;
    int ok;
    (void)ctx;
    ok = pbt_val_set_list(&a, PBT_V_LIST, 3) == 0;
    if (ok) {
        ok = pbt_val_set_bytes(&a.u.list.items[0], (const uint8_t *)"xyz", 3) == 0 &&
             pbt_val_set_list(&a.u.list.items[1], PBT_V_LIST, 2) == 0 &&
             pbt_val_set_str(&a.u.list.items[2], "hello", 5) == 0;
    }
    if (ok) {
        ok = pbt_val_clone(&c, &a) == 0;
        if (ok) {
            ok = pbt_val_equal(&a, &c);
            pbt_val_free(&c);
        }
    }
    pbt_val_free(&a);
    return ok;
}

/* ---- scenario 6: a small corpus run (parse, decode, generate, shrink, replay, filter, TFV) -------------- */

static const char *const SMALL_CORPUS =
    "{\"schema_version\":1,\"cases\":["
    "{\"id\":\"a/prim\",\"op\":\"gen_primitive\",\"type\":\"u8\",\"seed\":{\"kind\":\"int\",\"value\":\"0\"},"
    "\"count\":{\"kind\":\"int\",\"value\":\"2\"},\"outputs\":[{\"kind\":\"int\",\"value\":\"175\"},{\"kind\":\"int\",\"value\":\"244\"}]},"
    "{\"id\":\"a/list\",\"op\":\"shrink_list\",\"initial\":[{\"kind\":\"int\",\"value\":\"10\"},{\"kind\":\"int\",\"value\":\"5\"},{\"kind\":\"int\",\"value\":\"8\"},{\"kind\":\"int\",\"value\":\"4\"}],"
    "\"elem_target\":{\"kind\":\"int\",\"value\":\"0\"},\"property\":\"list_sum_le_20\","
    "\"expected_minimal\":[{\"kind\":\"int\",\"value\":\"8\"},{\"kind\":\"int\",\"value\":\"5\"},{\"kind\":\"int\",\"value\":\"8\"}],"
    "\"expected_steps\":{\"kind\":\"int\",\"value\":\"3\"}},"
    "{\"id\":\"a/bytes\",\"op\":\"shrink_bytes\",\"initial\":{\"kind\":\"bytes\",\"hex\":\"0102030405060708\"},"
    "\"property\":\"bytes_len_lt_5\",\"expected_minimal\":{\"kind\":\"bytes\",\"hex\":\"0000000000\"},"
    "\"expected_steps\":{\"kind\":\"int\",\"value\":\"16\"}},"
    "{\"id\":\"a/filter\",\"op\":\"filter_exhausted\",\"max_retries\":{\"kind\":\"int\",\"value\":\"7\"},"
    "\"verdict\":\"fail\",\"failure_id\":\"property-exhausted\"},"
    "{\"id\":\"a/reject\",\"op\":\"corpus_reject\",\"text\":\"bad header\\n\"}"
    "]}";

typedef struct corpus_baseline {
    size_t problems;
    size_t stored;
} corpus_baseline;

static int scenario_corpus(void *ctx)
{
    corpus_baseline *base = (corpus_baseline *)ctx;
    pbt_json *doc = NULL;
    pbt_corpus_report rep;
    char err[100];
    int same = 0;
    if (pbt_json_parse(SMALL_CORPUS, strlen(SMALL_CORPUS), &doc, err, sizeof err) == 0) {
        pbt_corpus_report_init(&rep);
        (void)pbt_corpus_evaluate(doc, &rep);
        if (base->problems == (size_t)-1) {
            base->problems = rep.nproblems; /* the unperturbed run defines the expected problem set */
            base->stored = rep.nstored;
            same = 1;
        } else {
            same = rep.nproblems == base->problems && rep.nstored == base->stored;
        }
        pbt_corpus_report_free(&rep);
    }
    pbt_json_free(doc);
    return same;
}

/* ---- scenario 7: the whole committed corpus, parsed ---------------------------------------------------------- */

static int scenario_parse_corpus(void *ctx)
{
    char *text = (char *)ctx;
    pbt_json *doc = NULL;
    char err[100];
    int ok = pbt_json_parse(text, strlen(text), &doc, err, sizeof err) == 0;
    pbt_json_free(doc);
    return ok;
}

int main(void)
{
    corpus_baseline base;
    char *text;
    pbt_hooks_reset();

    sweep("passing check", scenario_pass, NULL, 1);
    sweep("falsified check", scenario_falsified, NULL, 0);
    sweep("redacted verdict", scenario_redacted, NULL, 1);
    sweep("combinators", scenario_combinators, NULL, 1);
    sweep("values", scenario_values, NULL, 1);

    base.problems = (size_t)-1;
    base.stored = 0;
    sweep("small corpus run", scenario_corpus, &base, 0);

    text = tu_read_corpus_text(NULL);
    ISO_CHECK_MSG(text != NULL, "the committed corpus must be readable");
    if (text != NULL) {
        /* The full corpus has ~1100 allocations; sweep it all. */
        sweep("parse of the committed corpus", scenario_parse_corpus, text, 1);
        free(text);
    }
    printf("  %ld injected-failure runs\n", sweep_count);
    ISO_CHECK_EQ_INT(pbt_hooks.live_allocs, 0);
    return ISO_TEST_RESULT();
}
