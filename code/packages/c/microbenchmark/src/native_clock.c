#if !defined(_WIN32) && !defined(_POSIX_C_SOURCE)
#define _POSIX_C_SOURCE 200809L
#endif
#include "microbenchmark.h"
#include <limits.h>
#include <stdint.h>
#if defined(_WIN32)
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
int mb_native_clock(void *unused, int64_t *nanoseconds) {
  LARGE_INTEGER now, freq;
  uint64_t ticks, hz, whole, rem, base, frac;
  (void)unused;
  if (!nanoseconds || !QueryPerformanceFrequency(&freq) ||
      !QueryPerformanceCounter(&now) || freq.QuadPart <= 0 || now.QuadPart < 0)
    return 0;
  ticks = (uint64_t)now.QuadPart;
  hz = (uint64_t)freq.QuadPart;
  whole = ticks / hz;
  rem = ticks % hz;
  if (whole > (uint64_t)INT64_MAX / UINT64_C(1000000000))
    return 0;
  base = whole * UINT64_C(1000000000);
  if (rem > UINT64_MAX / UINT64_C(1000000000))
    return 0;
  frac = (rem * UINT64_C(1000000000)) / hz;
  if (base > (uint64_t)INT64_MAX - frac)
    return 0;
  *nanoseconds = (int64_t)(base + frac);
  return 1;
}
#elif defined(__unix__) || defined(__APPLE__)
#include <time.h>
int mb_native_clock(void *unused, int64_t *nanoseconds) {
  struct timespec t;
  (void)unused;
  if (!nanoseconds || clock_gettime(CLOCK_MONOTONIC, &t) != 0 || t.tv_sec < 0 ||
      t.tv_nsec < 0 || t.tv_nsec >= 1000000000L)
    return 0;
  if ((uintmax_t)t.tv_sec > (uintmax_t)INT64_MAX / UINT64_C(1000000000))
    return 0;
  {
    int64_t base = (int64_t)t.tv_sec * INT64_C(1000000000);
    if (base > INT64_MAX - (int64_t)t.tv_nsec)
      return 0;
    *nanoseconds = base + (int64_t)t.tv_nsec;
  }
  return 1;
}
#else
int mb_native_clock(void *unused, int64_t *nanoseconds) {
  (void)unused;
  (void)nanoseconds;
  return 0;
}
#endif
