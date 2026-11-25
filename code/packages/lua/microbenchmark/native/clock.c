/*
 * In-process monotonic nanosecond source for Lua 5.4.
 *
 * Lua's os.clock measures process CPU time and is not suitable for wall-time
 * batches. This module uses QueryPerformanceCounter on Windows and
 * CLOCK_MONOTONIC on POSIX. The module is linked against the Lua 5.4 API.
 * A negative return value signals a platform or conversion failure.
 */

#include <stdint.h>
#include <stddef.h>
#include "lua.h"

#if defined(_WIN32)
#include <windows.h>
#else
#include <time.h>
#endif

static int lua_now_ns(lua_State *state);

#if defined(_WIN32)
static LARGE_INTEGER qpc_origin;
static LARGE_INTEGER qpc_frequency;
static int qpc_ready;

static int64_t monotonic_ns(void) {
    LARGE_INTEGER current;
    uint64_t elapsed, seconds, remainder, fractional_ns;
    if (!qpc_ready) {
        if (!QueryPerformanceFrequency(&qpc_frequency) || qpc_frequency.QuadPart <= 0 ||
            (uint64_t)qpc_frequency.QuadPart > UINT64_MAX / UINT64_C(1000000000) ||
            !QueryPerformanceCounter(&qpc_origin)) return -1;
        qpc_ready = 1;
    }
    if (!QueryPerformanceCounter(&current) || current.QuadPart < qpc_origin.QuadPart) return -1;
    elapsed = (uint64_t)current.QuadPart - (uint64_t)qpc_origin.QuadPart;
    seconds = elapsed / (uint64_t)qpc_frequency.QuadPart;
    remainder = elapsed % (uint64_t)qpc_frequency.QuadPart;
    if (remainder > UINT64_MAX / UINT64_C(1000000000)) return -1;
    fractional_ns = (remainder * UINT64_C(1000000000)) / (uint64_t)qpc_frequency.QuadPart;
    if (seconds > ((uint64_t)INT64_MAX - fractional_ns) / UINT64_C(1000000000)) return -1;
    return (int64_t)(seconds * UINT64_C(1000000000) + fractional_ns);
}
#else
static struct timespec posix_origin;
static int posix_ready;

static int64_t monotonic_ns(void) {
    struct timespec current;
    int64_t seconds, nanoseconds;
    if (clock_gettime(CLOCK_MONOTONIC, &current) != 0 || current.tv_nsec < 0 || current.tv_nsec >= 1000000000L) return -1;
    if (!posix_ready) { posix_origin = current; posix_ready = 1; }
    seconds = (int64_t)current.tv_sec - (int64_t)posix_origin.tv_sec;
    nanoseconds = (int64_t)current.tv_nsec - (int64_t)posix_origin.tv_nsec;
    if (nanoseconds < 0) { seconds--; nanoseconds += INT64_C(1000000000); }
    if (seconds < 0 || seconds > (INT64_MAX - nanoseconds) / INT64_C(1000000000)) return -1;
    return seconds * INT64_C(1000000000) + nanoseconds;
}
#endif

static int lua_now_ns(lua_State *state) {
    int64_t value = monotonic_ns();
    if (value < 0) value = -1;
    lua_pushinteger(state, (lua_Integer)value);
    return 1;
}

#if defined(_WIN32)
__declspec(dllexport)
#endif
int luaopen_coding_adventures_microbenchmark_clock(lua_State *state) {
    lua_createtable(state, 0, 1);
    lua_pushcfunction(state, lua_now_ns);
    lua_setfield(state, -2, "now_ns");
    return 1;
}
