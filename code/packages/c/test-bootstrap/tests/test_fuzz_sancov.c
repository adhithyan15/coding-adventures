/*
 * test_fuzz_sancov.c -- the optional sanitizer-coverage source (VER06j section 4.3).
 *
 * Built ONLY by the dedicated step of tools/run.sh: sancov_target.c carries -fsanitize-coverage=trace-pc-guard, the
 * engine does not, and src/fuzz_sancov.c supplies the two callbacks.  Checks:
 *
 *   - the report is labelled "sancov-trace-pc-guard" and the map holds real edges although the target never calls
 *     fz_edge;
 *   - the engine is NOT instrumented: only the target's few guards exist;
 *   - guided mutation cracks the staircase from compiler edges alone, deterministically;
 *   - an uninstrumented target in the same binary is still labelled black-box;
 *   - the instrumented code is harmless outside a target run (no active handle).
 */
#include <stdio.h>
#include <string.h>

#include "fuzz.h"
#include "iso_test.h"
#include "pbt_hooks.h"

uint32_t fz_guard_count(void);
int sancov_target(const uint8_t *d, size_t n, fz_handle *h, fz_crash *out);

static int plain_target(const uint8_t *d, size_t n, fz_handle *h, fz_crash *out)
{
    (void)d;
    (void)n;
    (void)h;
    (void)out;
    return 0;
}

int main(void)
{
    const uint8_t zeros[3] = {0, 0, 0};
    fz_input seed;
    fz_report r;
    fz_report r2;
    fz_options o = fz_options_default();
    uint64_t sd;
    int cracked = 0;
    uint32_t guards = fz_guard_count();
    seed.p = zeros;
    seed.n = 3;
    printf("  sancov guards handed over: %lu\n", (unsigned long)guards);
    ISO_CHECK_MSG(guards > 0, "the instrumented target must have guards");
    ISO_CHECK_MSG(guards < 64, "the engine itself must NOT be instrumented");

    /* outside a target run the callbacks are inert */
    ISO_CHECK_EQ_INT(sancov_target(zeros, 3, NULL, NULL), 0);
    ISO_CHECK(fz_active_handle() == NULL);

    o.max_iterations = 6000;
    o.max_input_len = 16;
    for (sd = 1; sd <= 6; sd++) {
        o.seed = sd;
        ISO_CHECK_EQ_INT(fz_run(&seed, 1, &o, sancov_target, &r), FZ_OK);
        ISO_CHECK_EQ_INT(strcmp(r.tier, FZ_TIER_SANCOV), 0);
        ISO_CHECK(r.total_edges >= 2);
        ISO_CHECK(r.total_edges <= guards);
        cracked += r.ncrashes > 0;
        fz_report_free(&r);
    }
    printf("  sancov staircase cracked for %d of 6 seeds\n", cracked);
    ISO_CHECK_MSG(cracked >= 5, "compiler edges must guide the staircase");

    /* deterministic: the same campaign twice gives the same report */
    o.seed = 3;
    ISO_CHECK_EQ_INT(fz_run(&seed, 1, &o, sancov_target, &r), FZ_OK);
    ISO_CHECK_EQ_INT(fz_run(&seed, 1, &o, sancov_target, &r2), FZ_OK);
    ISO_CHECK(r.total_edges == r2.total_edges && r.ncorpus == r2.ncorpus && r.ncrashes == r2.ncrashes &&
              r.iterations == r2.iterations);
    fz_report_free(&r);
    fz_report_free(&r2);

    /* a target with no edges, in the same instrumented binary, is still honest */
    ISO_CHECK_EQ_INT(fz_run(&seed, 1, &o, plain_target, &r), FZ_OK);
    ISO_CHECK_EQ_INT(strcmp(r.tier, FZ_TIER_BLACKBOX), 0);
    ISO_CHECK_EQ_INT(r.total_edges, 0);
    fz_report_free(&r);
    ISO_CHECK_EQ_INT(pbt_hooks.live_allocs, 0);
    return ISO_TEST_RESULT();
}
