/*
 * clock_posix.c — the POSIX backend of os_platform/clock (macOS + Linux).
 * ===========================================================================
 *
 * This translation unit is compiled ONLY on Unix-like systems (the package's
 * `BUILD` file — shared by mac and linux — names this source; `BUILD_windows`
 * names clock_windows.c instead). So there is no `#if defined(__linux__)` here:
 * the build system already picked the right file. That is the CCPP02 rule —
 * per-OS *source selection* happens in the BUILD, not with a maze of #ifdefs.
 *
 * All three primitives are thin, careful wrappers over two POSIX calls:
 *
 *   clock_gettime(clockid, struct timespec*)  — read a clock to {sec, nsec}
 *   nanosleep(const struct timespec*, rem*)    — sleep with ns resolution
 *
 * clock_gettime and nanosleep require _POSIX_C_SOURCE >= 199309L to be visible;
 * the BUILD defines _POSIX_C_SOURCE=200809L. On modern macOS (10.12+) and glibc
 * (>= 2.17) both live in libc, so no extra library is linked.
 */
#include "os_platform/clock.h"

#include <errno.h>
#include <time.h>

/* One second is exactly this many nanoseconds. Naming the constant keeps the
 * unit conversions below readable and prevents a stray digit miscount. */
#define OSP_NS_PER_SEC 1000000000ULL

/*
 * Fold a {seconds, nanoseconds} timespec into a single unsigned nanosecond
 * count. `tv_sec` is time_t (signed, >= 32 bits) and `tv_nsec` is a long in
 * [0, 999999999]. We cast to uint64_t BEFORE multiplying so the arithmetic is
 * unsigned (well-defined on overflow) rather than risking signed overflow — a
 * point UBSan checks. A monotonic uptime in nanoseconds fits in uint64_t for
 * ~584 years, so there is no realistic wraparound.
 */
static uint64_t osp__timespec_to_ns(const struct timespec *ts) {
    return (uint64_t)ts->tv_sec * OSP_NS_PER_SEC + (uint64_t)ts->tv_nsec;
}

osp_status osp_monotonic_ns(uint64_t *out_ns) {
    struct timespec ts;
    if (out_ns == NULL) {
        return OSP_ERR_INVAL;
    }
    /* CLOCK_MONOTONIC: steadily increasing, arbitrary epoch, not settable by the
     * user and unaffected by NTP steps — exactly what elapsed timing needs. */
    if (clock_gettime(CLOCK_MONOTONIC, &ts) != 0) {
        return OSP_ERR_OS;
    }
    *out_ns = osp__timespec_to_ns(&ts);
    return OSP_OK;
}

osp_status osp_wall_unix_ns(int64_t *out_ns) {
    struct timespec ts;
    if (out_ns == NULL) {
        return OSP_ERR_INVAL;
    }
    /* CLOCK_REALTIME is calendar time measured from the UNIX epoch already, so no
     * epoch shift is needed (contrast the Windows backend, whose FILETIME counts
     * from 1601). We compute in signed int64 so the contract's signedness holds;
     * present-day values are ~1.7e18 ns, comfortably inside int64's ~9.2e18. */
    if (clock_gettime(CLOCK_REALTIME, &ts) != 0) {
        return OSP_ERR_OS;
    }
    *out_ns = (int64_t)ts.tv_sec * (int64_t)OSP_NS_PER_SEC + (int64_t)ts.tv_nsec;
    return OSP_OK;
}

osp_status osp_sleep_ns(uint64_t ns) {
    struct timespec req;
    struct timespec rem;

    /* Split the request into whole seconds + the nanosecond remainder, because
     * timespec.tv_nsec must stay in [0, 999999999]. */
    req.tv_sec = (time_t)(ns / OSP_NS_PER_SEC);
    req.tv_nsec = (long)(ns % OSP_NS_PER_SEC);

    /* A signal can wake nanosleep early; when it does it returns -1/EINTR and
     * writes the UNSLEPT remainder into `rem`. We loop, sleeping the remainder,
     * so the caller reliably sleeps at least the full requested duration. Any
     * other error (only EINVAL is possible for a well-formed timespec) is real. */
    while (nanosleep(&req, &rem) != 0) {
        if (errno != EINTR) {
            return OSP_ERR_OS;
        }
        req = rem;
    }
    return OSP_OK;
}
