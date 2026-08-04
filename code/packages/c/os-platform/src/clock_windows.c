/*
 * clock_windows.c — the Windows backend of os_platform/clock.
 * ===========================================================================
 *
 * Compiled ONLY on Windows (named by the package's `BUILD_windows`; the POSIX
 * file clock_posix.c is named by the shared `BUILD`). As on the POSIX side there
 * are no OS `#if`s here — the build already selected this file.
 *
 * Three Win32 calls carry the load, all in kernel32 (linked by default, so no
 * extra .lib):
 *
 *   QueryPerformanceCounter / QueryPerformanceFrequency
 *        a high-resolution monotonic tick counter and its ticks-per-second rate.
 *   GetSystemTimePreciseAsFileTime
 *        the wall clock as a FILETIME (100-ns ticks since 1601-01-01 UTC).
 *   Sleep
 *        millisecond-granularity suspension of the current thread.
 */
#include "os_platform/clock.h"

#define WIN32_LEAN_AND_MEAN
#include <windows.h>

#define OSP_NS_PER_SEC 1000000000ULL

/*
 * The number of 100-nanosecond ticks between the Windows FILETIME epoch
 * (1601-01-01) and the UNIX epoch (1970-01-01). Windows counts time from 1601;
 * the UNIX world counts from 1970. The gap is 11644473600 seconds, and a
 * FILETIME tick is 100 ns, so 11644473600 * 10,000,000 = this constant. We
 * subtract it to convert a FILETIME into "ticks since 1970".
 */
#define OSP_FILETIME_UNIX_EPOCH_DELTA 116444736000000000ULL

osp_status osp_monotonic_ns(uint64_t *out_ns) {
    LARGE_INTEGER counter;
    LARGE_INTEGER freq;
    uint64_t c;
    uint64_t f;

    if (out_ns == NULL) {
        return OSP_ERR_INVAL;
    }
    /* On Windows XP and later both calls always succeed (they return non-zero),
     * but we still check: a 0 return is documented as failure. */
    if (!QueryPerformanceCounter(&counter) || !QueryPerformanceFrequency(&freq)) {
        return OSP_ERR_OS;
    }
    if (freq.QuadPart <= 0) {
        return OSP_ERR_OS;
    }

    c = (uint64_t)counter.QuadPart;
    f = (uint64_t)freq.QuadPart;

    /* We want counter * 1e9 / freq, but counter * 1e9 would overflow 64 bits
     * after a few seconds of uptime. Split the division to keep every
     * intermediate small: the whole-seconds part (c / f) is multiplied by 1e9,
     * and only the sub-second remainder (c % f, which is < f) is scaled and
     * divided. This yields exact nanoseconds with no overflow and no floating
     * point. */
    *out_ns = (c / f) * OSP_NS_PER_SEC + ((c % f) * OSP_NS_PER_SEC) / f;
    return OSP_OK;
}

osp_status osp_wall_unix_ns(int64_t *out_ns) {
    FILETIME ft;
    uint64_t ticks; /* 100-ns intervals since 1601 */

    if (out_ns == NULL) {
        return OSP_ERR_INVAL;
    }
    /* "Precise" gives the full resolution of the system clock (Windows 8+);
     * it returns void — it cannot fail. */
    GetSystemTimePreciseAsFileTime(&ft);

    /* Reassemble the split 64-bit value from its high and low 32-bit halves. */
    ticks = ((uint64_t)ft.dwHighDateTime << 32) | (uint64_t)ft.dwLowDateTime;

    /* Shift the epoch from 1601 to 1970, then convert 100-ns ticks to ns (×100).
     * The subtraction is safe for any real system time (post-1601), and the
     * result in signed int64 holds until the year ~2262. */
    *out_ns = (int64_t)(ticks - OSP_FILETIME_UNIX_EPOCH_DELTA) * 100;
    return OSP_OK;
}

osp_status osp_sleep_ns(uint64_t ns) {
    /* Sleep() takes whole milliseconds, so round the request UP: any positive
     * sub-millisecond request sleeps ~1 ms rather than not at all, honouring the
     * "at least this long" contract. ns == 0 yields 0 ms — Sleep(0) simply
     * yields the rest of the timeslice and returns. */
    uint64_t ms = (ns + 999999ULL) / 1000000ULL;

    /* DWORD is 32-bit, so a very large request would not fit. Sleep in chunks of
     * at most (MAXDWORD-1) ms until the whole duration has elapsed. INFINITE
     * (0xFFFFFFFF) would block forever, so we deliberately cap a chunk below it. */
    while (ms > 0) {
        DWORD chunk = (ms > 0xFFFFFFFEULL) ? 0xFFFFFFFEUL : (DWORD)ms;
        Sleep(chunk);
        ms -= chunk;
    }
    return OSP_OK;
}
