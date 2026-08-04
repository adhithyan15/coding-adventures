/*
 * os_platform/clock.h — real time from the operating system, in portable C.
 * ===========================================================================
 *
 * WHAT THIS IS
 * ------------
 * The `os-platform` library is the repo's *bucket B* substrate: the small set of
 * things a program fundamentally cannot compute on its own and must ask the
 * operating system for. `clock` is the first of them. It answers three
 * questions that pure-ISO C17 cannot answer portably and precisely:
 *
 *   1. "How much time has elapsed?"      → osp_monotonic_ns()
 *   2. "What time is it, on the wall?"    → osp_wall_unix_ns()
 *   3. "Pause me for a while."            → osp_sleep_ns()
 *
 * WHY NOT JUST USE <time.h>?
 * --------------------------
 * ISO C gives you `time()` (whole seconds only — too coarse for timing) and
 * `clock()` (CPU time, not elapsed wall time — it stops counting while you
 * sleep). Neither gives a high-resolution *monotonic* clock, and `<threads.h>`'s
 * `thrd_sleep` is an *optional* feature MSVC does not ship. So sub-second timing
 * and sleeping are genuinely OS territory. This header hides the per-OS calls
 * behind one interface:
 *
 *      OS        monotonic            wall-clock                     sleep
 *      ────────  ───────────────────  ─────────────────────────────  ──────────
 *      mac/linux clock_gettime        clock_gettime(CLOCK_REALTIME)  nanosleep
 *                (CLOCK_MONOTONIC)
 *      windows   QueryPerformance     GetSystemTimePreciseAsFileTime Sleep
 *                Counter/Frequency
 *
 * MONOTONIC vs WALL — the crucial distinction
 * -------------------------------------------
 * A *monotonic* clock only ever moves forward at a steady rate. Its zero point
 * is arbitrary (often system boot), so a single reading is meaningless — but the
 * *difference* between two readings is a trustworthy elapsed duration. Use it for
 * "how long did this take?" and timeouts. It is immune to the user changing the
 * system clock or to NTP/daylight-saving adjustments.
 *
 * A *wall-clock* reading is calendar time: nanoseconds since the UNIX epoch
 * (1970-01-01 00:00:00 UTC). Use it for timestamps a human will read. It CAN
 * jump — backwards, even — if the clock is corrected, so never measure a
 * duration by subtracting two wall-clock readings.
 *
 * Analogy: a stopwatch (monotonic) vs a wristwatch (wall). You time a race with
 * the stopwatch; you write "finished at 3:42pm" from the wristwatch.
 *
 * ERROR MODEL
 * -----------
 * Every function returns an `osp_status`: OSP_OK (0) on success, a negative code
 * otherwise. The result is written through an out-parameter, so the return value
 * is always the thing you branch on. No OS handles are opened, so nothing leaks.
 *
 * PORTABILITY / BUILD
 * -------------------
 * Compiled by the sibling `platform-harness` (not the strict `iso-harness`): the
 * POSIX backend needs `_POSIX_C_SOURCE >= 199309L` for clock_gettime/nanosleep
 * (supplied by the BUILD), and the Windows backend includes <windows.h>. Each OS
 * compiles exactly one backend (BUILD selects the source file); this header is
 * shared by all of them.
 */
#ifndef OS_PLATFORM_CLOCK_H
#define OS_PLATFORM_CLOCK_H

#include <stdint.h>

#include "os_platform/status.h" /* osp_status: OSP_OK / OSP_ERR_* */

#ifdef __cplusplus
extern "C" {
#endif

/*
 * osp_monotonic_ns — elapsed-time clock, in nanoseconds.
 *
 * Writes through *out_ns a value from a steadily-increasing clock whose epoch is
 * unspecified. Only *differences* between two readings are meaningful; that
 * difference is an elapsed duration in nanoseconds, unaffected by wall-clock
 * adjustments. Guaranteed non-decreasing within a single process.
 *
 * Returns OSP_OK on success, OSP_ERR_INVAL if out_ns is NULL, OSP_ERR_OS if the
 * OS clock call fails.
 */
osp_status osp_monotonic_ns(uint64_t *out_ns);

/*
 * osp_wall_unix_ns — calendar clock, in nanoseconds since the UNIX epoch.
 *
 * Writes through *out_ns the current time as nanoseconds since
 * 1970-01-01 00:00:00 UTC. Signed, because times before the epoch are negative
 * in principle (in practice the value is a large positive number). This clock
 * can jump when the system time is corrected — do not use it to measure
 * durations; use osp_monotonic_ns for that.
 *
 * Returns OSP_OK on success, OSP_ERR_INVAL if out_ns is NULL, OSP_ERR_OS if the
 * OS clock call fails.
 */
osp_status osp_wall_unix_ns(int64_t *out_ns);

/*
 * osp_sleep_ns — suspend the calling thread for at least `ns` nanoseconds.
 *
 * Sleeping 0 returns immediately with OSP_OK. The OS may sleep slightly longer
 * than requested (scheduling granularity); it will not return early on this
 * path — the POSIX backend retries across signal interruptions (EINTR) so the
 * full duration elapses. NOTE: the Windows backend's granularity is the
 * millisecond (Sleep rounds the request up to whole milliseconds).
 *
 * Returns OSP_OK on success, OSP_ERR_OS on an unexpected OS failure.
 */
osp_status osp_sleep_ns(uint64_t ns);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* OS_PLATFORM_CLOCK_H */
