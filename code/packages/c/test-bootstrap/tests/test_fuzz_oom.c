/*
 * test_fuzz_oom.c -- out-of-memory discipline of the fuzz engine (VER06j sections 1 and 11).
 *
 * Every allocation goes through pbt_alloc/pbt_realloc, which the shared test seam can make FAIL at the N-th call.
 * Each scenario runs with the failure injected at allocation 1, 2, 3 ... until a run completes without the
 * injection firing (the sweep is exhaustive for that scenario), and at EVERY point the test asserts:
 *
 *   - no crash (the sanitizer build also proves no out-of-bounds or use-after-free);
 *   - no leak: live allocations return to the baseline;
 *   - no false success: a run that lost an allocation never claims success.
 *
 * Scenarios: the campaign (with crashes, so the minimiser runs inside it), the standalone minimiser, every
 * mutator in both forms, mutate_sequence with growth, corpus admission, and the fuzz-v1 corpus runner.
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
#include "test_util.h"

static long sweep_count;

/* A scenario returns 1 when it CLAIMS success. */
typedef int (*scenario_fn)(void *ctx);

static void sweep(const char *name, scenario_fn fn, void *ctx)
{
    long n;
    int clean = fn(ctx);
    ISO_CHECK_MSG(clean == 1, name); /* without injection the scenario succeeds */
    for (n = 1; n < 100000; n++) {
        long baseline = pbt_hooks.live_allocs;
        long before = pbt_hooks.injected;
        int claims;
        int fired;
        pbt_hooks.fail_after = n;
        claims = fn(ctx);
        pbt_hooks.fail_after = 0;
        fired = pbt_hooks.injected != before;
        sweep_count++;
        if (fired && claims) {
            printf("  %s: claimed success although allocation %ld failed\n", name, n);
        }
        ISO_CHECK(!(fired && claims));
        if (pbt_hooks.live_allocs != baseline) {
            printf("  %s: leak when allocation %ld failed (%ld live, baseline %ld)\n", name, n, pbt_hooks.live_allocs,
                   baseline);
        }
        ISO_CHECK_EQ_INT(pbt_hooks.live_allocs, baseline);
        ISO_CHECK_EQ_INT(fz_handles_live(), 0);
        if (!fired) {
            printf("  %s: swept %ld allocation points\n", name, n - 1);
            return; /* n exceeded the scenario's allocations: the sweep is complete */
        }
    }
    ISO_CHECK_MSG(0, "the sweep did not terminate");
}

/* ---- the campaign ------------------------------------------------------------------------------------------- */

static int crashy(const uint8_t *d, size_t n, fz_handle *h, fz_crash *out)
{
    size_t i;
    fz_edge(h, n);
    for (i = 0; i < n; i++) {
        if (d[i] == 0xFF) {
            fz_crash_set(out, "ff");
            fz_crash_push(out, "scan");
            return 1;
        }
    }
    if (n > 6) {
        fz_edge(h, 100 + n);
    }
    return 0;
}

static int scenario_campaign(void *ctx)
{
    fz_report r;
    fz_options o = fz_options_default();
    fz_input seeds[2];
    static const uint8_t a[] = {1, 2, 3, 4, 5, 6, 7, 8};
    static const uint8_t b[] = {9, 0xFF, 9};
    int rc;
    (void)ctx;
    seeds[0].p = a;
    seeds[0].n = sizeof a;
    seeds[1].p = b;
    seeds[1].n = sizeof b;
    o.max_iterations = 12;
    o.minimize_budget = 40;
    rc = fz_run(seeds, 2, &o, crashy, &r);
    if (rc == FZ_OK) {
        int good = r.ncrashes >= 1 && r.ncorpus >= 1 && r.crashes[0].minimal_len == 1;
        fz_report_free(&r);
        return good;
    }
    /* a failed run hands back nothing */
    ISO_CHECK(r.corpus == NULL && r.crashes == NULL && r.ncrashes == 0);
    return 0;
}

static int exec_ok_edge(void *user, const uint8_t *d, size_t n, fz_bitmap *cov, fz_outcome *out)
{
    (void)user;
    (void)d;
    fz_bitmap_record(cov, n);
    out->kind = n > 2 ? FZ_TIMEOUT : FZ_PASS;
    return FZ_OK;
}

static int scenario_outcomes(void *ctx)
{
    fz_report r;
    fz_options o = fz_options_default();
    int rc;
    (void)ctx;
    o.max_iterations = 8;
    rc = fz_run_outcomes(NULL, 0, &o, exec_ok_edge, NULL, &r);
    if (rc == FZ_OK) {
        fz_report_free(&r);
        return 1;
    }
    ISO_CHECK(r.corpus == NULL);
    return 0;
}

/* Enough seeds and distinct crashes to force the corpus array (16) and the crash array (8) to grow. */
static int exec_grow(void *user, const uint8_t *d, size_t n, fz_bitmap *cov, fz_outcome *out)
{
    (void)user;
    fz_bitmap_record(cov, n);
    memset(&out->crash, 0, sizeof out->crash);
    if (n > 0 && d[0] >= 100) {
        char kind[16];
        out->kind = FZ_CRASH;
        (void)snprintf(kind, sizeof kind, "k%u", (unsigned)d[0]);
        fz_crash_set(&out->crash, kind);
    } else {
        out->kind = FZ_PASS;
    }
    return FZ_OK;
}

static int scenario_growth(void *ctx)
{
    fz_report r;
    fz_options o = fz_options_default();
    fz_input seeds[40];
    static uint8_t data[40][24];
    size_t i;
    int rc;
    (void)ctx;
    for (i = 0; i < 40; i++) {
        memset(data[i], i < 20 ? (int)(100 + i) : 1, sizeof data[i]);
        seeds[i].p = data[i];
        seeds[i].n = (i % 20) + 1;
    }
    o.max_iterations = 3;
    rc = fz_run_outcomes(seeds, 40, &o, exec_grow, NULL, &r);
    if (rc == FZ_OK) {
        int good = r.ncrashes >= 20 && r.ncorpus >= 17;
        fz_report_free(&r);
        return good;
    }
    ISO_CHECK(r.corpus == NULL && r.crashes == NULL);
    return 0;
}

/* ---- the minimiser -------------------------------------------------------------------------------------------- */

static int test_len3(void *user, const uint8_t *d, size_t n, fz_outcome *out)
{
    (void)user;
    (void)d;
    out->kind = n >= 3 ? FZ_CRASH : FZ_PASS;
    memset(&out->crash, 0, sizeof out->crash);
    fz_crash_set(&out->crash, "k");
    return FZ_OK;
}

static int scenario_minimize(void *ctx)
{
    uint8_t data[12] = {1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12};
    fz_bytes out;
    char sig[9];
    int rc;
    static const char *const none[1] = {0};
    (void)ctx;
    fz_crash_signature(sig, "k", none, 0);
    rc = fz_minimize_crashing_input(&out, data, sizeof data, sig, test_len3, NULL, 500);
    if (rc == FZ_OK) {
        int good = out.n == 3;
        fz_bytes_free(&out);
        return good;
    }
    ISO_CHECK(out.p == NULL);
    return 0;
}

/* ---- the mutators ---------------------------------------------------------------------------------------------- */

static int scenario_mutators(void *ctx)
{
    static const uint8_t d[] = {1, 2, 3, 4, 5, 6, 7, 8};
    static const uint8_t ins[] = {9, 9};
    pbt_big a;
    pbt_big b;
    fz_bytes o;
    int ok = 1;
    int rc;
    (void)ctx;
    pbt_big_set_i64(&a, -3);
    pbt_big_set_i64(&b, 5);
#define STEP(call)                                                                                                   \
    do {                                                                                                             \
        rc = (call);                                                                                                 \
        if (rc == FZ_OK) {                                                                                           \
            fz_bytes_free(&o);                                                                                       \
        } else {                                                                                                     \
            ok = 0;                                                                                                  \
            ISO_CHECK(o.p == NULL);                                                                                  \
        }                                                                                                            \
    } while (0)
    STEP(fz_bit_flip(&o, d, 8, 3, 9));
    STEP(fz_bit_flip_big(&o, d, 8, &a, &b));
    STEP(fz_byte_flip(&o, d, 8, 3, 9));
    STEP(fz_byte_flip_big(&o, d, 8, &a, &b));
    STEP(fz_byte_arithmetic(&o, d, 8, 1, 7, 4, FZ_BIG));
    STEP(fz_byte_arithmetic_big(&o, d, 8, &a, &b, 2, FZ_LITTLE));
    STEP(fz_boundary_insert(&o, d, 8, 1, 4, 0xFFFFFFFFu, FZ_BIG));
    STEP(fz_boundary_insert_big(&o, d, 8, &a, 8, &b, FZ_LITTLE));
    STEP(fz_block_delete(&o, d, 8, 2, 3));
    STEP(fz_block_delete_big(&o, d, 8, &a, &b));
    STEP(fz_block_insert(&o, d, 8, 2, ins, 2));
    STEP(fz_block_insert_big(&o, d, 8, &a, ins, 2));
    STEP(fz_block_duplicate(&o, d, 8, 2, 3, 1));
    STEP(fz_block_duplicate_big(&o, d, 8, &a, &b, &b));
    STEP(fz_splice(&o, d, 8, ins, 2, 3, 1));
    STEP(fz_splice_big(&o, d, 8, ins, 2, &a, &b));
    STEP(fz_mutate_sequence(&o, d, 8, 42, 40, 64)); /* grows past the initial capacity: realloc paths */
    STEP(fz_mutate_sequence(&o, NULL, 0, 7, 30, 64));
    return ok;
}

static int scenario_admit(void *ctx)
{
    static const uint64_t ex[] = {1, 2, 3};
    static const uint64_t tr[] = {2, 4, 4};
    int a;
    size_t n;
    (void)ctx;
    if (fz_corpus_admit(ex, 3, tr, 3, &a, &n) != FZ_OK) {
        return 0;
    }
    return a == 1 && n == 1;
}

/* ---- the corpus runner ------------------------------------------------------------------------------------------ */

static int scenario_corpus(void *ctx)
{
    pbt_json *doc = (pbt_json *)ctx;
    pbt_corpus_report r;
    int ok;
    pbt_corpus_report_init(&r);
    ISO_CHECK_EQ_INT(fz_corpus_evaluate(doc, &r), 0);
    /* an allocation lost anywhere must show up as a PROBLEM: never a false conformance */
    ok = r.nproblems == 0 && r.evaluated == FZ_V1_CASE_COUNT;
    pbt_corpus_report_free(&r);
    return ok;
}

int main(void)
{
    size_t len = 0;
    char *text = tu_read_repo_file("code/specs/fixtures/fuzz-v1/cases.json", &len);
    pbt_json *doc = NULL;
    char err[200];
    ISO_CHECK_MSG(text != NULL && pbt_json_parse(text, len, &doc, err, sizeof err) == 0, "fuzz-v1 must load");
    sweep("campaign", scenario_campaign, NULL);
    sweep("outcome campaign", scenario_outcomes, NULL);
    sweep("array growth", scenario_growth, NULL);
    sweep("minimiser", scenario_minimize, NULL);
    sweep("mutators", scenario_mutators, NULL);
    sweep("corpus admission", scenario_admit, NULL);
    if (doc != NULL) {
        sweep("corpus runner", scenario_corpus, doc);
    }
    printf("  OOM sweep: %ld injected runs\n", sweep_count);
    pbt_json_free(doc);
    free(text);
    fz_hooks_reset();
    ISO_CHECK_EQ_INT(pbt_hooks.live_allocs, 0);
    return ISO_TEST_RESULT();
}
