/*
 * clock_test.c — behavioural tests for os_platform/clock, run on each OS.
 * ===========================================================================
 *
 * There is no golden vector for "what time is it" — the values are live. So we
 * test the *properties* that must hold on every platform, chosen to be robust on
 * loaded CI machines (no tight upper bounds, generous lower bounds):
 *
 *   - the calls succeed and reject NULL out-pointers;
 *   - the monotonic clock never goes backwards;
 *   - the wall clock lands in a sane calendar window (proving each backend's
 *     epoch conversion is correct — this is what would break if the Windows
 *     1601→1970 shift or the ×100 tick scaling were wrong);
 *   - sleeping actually advances the monotonic clock by at least a floor.
 *
 * 64-bit note: we compare nanosecond values with plain ISO_CHECK(expr), never
 * ISO_CHECK_EQ_INT — that macro widens to `long`, which is only 32 bits on
 * Windows and would truncate a nanosecond timestamp.
 */
#include "iso_test.h"

#include "os_platform/clock.h"

int main(void) {
    uint64_t m1 = 0, m2 = 0;
    int64_t wall = 0;
    uint64_t before = 0, after = 0;

    /* Calendar window, in nanoseconds since the UNIX epoch:
     *   lower = 2020-01-01 UTC (1577836800 s), upper = 2100-01-01 UTC. Any
     * correct wall clock reading today falls strictly inside. */
    const int64_t YEAR_2020_NS = 1577836800LL * 1000000000LL;
    const int64_t YEAR_2100_NS = 4102444800LL * 1000000000LL;

    /* ── monotonic: succeeds, rejects NULL, never decreases ─────────────── */
    ISO_CHECK(osp_monotonic_ns(&m1) == OSP_OK);
    ISO_CHECK(osp_monotonic_ns(&m2) == OSP_OK);
    ISO_CHECK_MSG(m2 >= m1, "monotonic clock must not run backwards");
    ISO_CHECK(osp_monotonic_ns(NULL) == OSP_ERR_INVAL);

    /* ── wall clock: succeeds, rejects NULL, lands in [2020, 2100) ──────── */
    ISO_CHECK(osp_wall_unix_ns(&wall) == OSP_OK);
    ISO_CHECK_MSG(wall > YEAR_2020_NS, "wall clock should be after 2020");
    ISO_CHECK_MSG(wall < YEAR_2100_NS, "wall clock should be before 2100");
    ISO_CHECK(osp_wall_unix_ns(NULL) == OSP_ERR_INVAL);

    /* ── sleep: zero is instant-OK; a real sleep advances the monotonic clock */
    ISO_CHECK(osp_sleep_ns(0) == OSP_OK);

    ISO_CHECK(osp_monotonic_ns(&before) == OSP_OK);
    ISO_CHECK(osp_sleep_ns(50ULL * 1000000ULL) == OSP_OK); /* request 50 ms */
    ISO_CHECK(osp_monotonic_ns(&after) == OSP_OK);
    /* Floor of 5 ms: far below the 50 ms requested, yet above any plausible
     * clock granularity, so this is not flaky even on a busy runner. */
    ISO_CHECK_MSG(after - before >= 5ULL * 1000000ULL,
                  "50ms sleep must advance the monotonic clock by >= 5ms");

    return ISO_TEST_RESULT();
}
