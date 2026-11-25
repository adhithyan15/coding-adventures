#define CAML_NAME_SPACE
#include <caml/fail.h>
#include <caml/memory.h>
#include <caml/mlvalues.h>
#include <caml/alloc.h>
#include <stdint.h>
#include <limits.h>

#ifdef _WIN32
#include <windows.h>

static int64_t mb_qpc_nanoseconds(void) {
  LARGE_INTEGER frequency;
  LARGE_INTEGER counter;
  uint64_t ticks, hz, seconds, remainder, whole, fraction;
  if (!QueryPerformanceFrequency(&frequency) || frequency.QuadPart <= 0)
    caml_failwith("QueryPerformanceFrequency failed");
  if (!QueryPerformanceCounter(&counter) || counter.QuadPart < 0)
    caml_failwith("QueryPerformanceCounter failed or returned a negative value");
  ticks = (uint64_t) counter.QuadPart;
  hz = (uint64_t) frequency.QuadPart;
  if (hz > (uint64_t) INT64_MAX / UINT64_C(1000000000))
    caml_failwith("QPC frequency is too high for checked integer conversion");
  seconds = ticks / hz;
  remainder = ticks % hz;
  if (seconds > (uint64_t) INT64_MAX / UINT64_C(1000000000))
    caml_failwith("QPC time exceeds signed nanosecond range");
  whole = seconds * UINT64_C(1000000000);
  fraction = (remainder * UINT64_C(1000000000)) / hz;
  if (whole > (uint64_t) INT64_MAX - fraction)
    caml_failwith("QPC time exceeds signed nanosecond range");
  return (int64_t) (whole + fraction);
}
#else
#include <time.h>

static int64_t mb_qpc_nanoseconds(void) {
  struct timespec now;
  uint64_t seconds, nanos;
  if (clock_gettime(CLOCK_MONOTONIC, &now) != 0)
    caml_failwith("clock_gettime(CLOCK_MONOTONIC) failed");
  if (now.tv_sec < 0 || now.tv_nsec < 0 || now.tv_nsec >= 1000000000L)
    caml_failwith("CLOCK_MONOTONIC returned an invalid timespec");
  seconds = (uint64_t) now.tv_sec;
  nanos = (uint64_t) now.tv_nsec;
  if (seconds > ((uint64_t) INT64_MAX - nanos) / UINT64_C(1000000000))
    caml_failwith("monotonic time exceeds signed nanosecond range");
  return (int64_t) (seconds * UINT64_C(1000000000) + nanos);
}
#endif

CAMLprim value caml_mb_monotonic_ns(value unit) {
  CAMLparam1(unit);
  CAMLreturn(caml_copy_int64(mb_qpc_nanoseconds()));
}

CAMLprim value caml_mb_os_name(value unit) {
  CAMLparam1(unit);
#if defined(_WIN32)
  CAMLreturn(caml_copy_string("windows"));
#elif defined(__APPLE__)
  CAMLreturn(caml_copy_string("macos"));
#elif defined(__linux__)
  CAMLreturn(caml_copy_string("linux"));
#else
  CAMLreturn(caml_copy_string("unknown"));
#endif
}

CAMLprim value caml_mb_architecture(value unit) {
  CAMLparam1(unit);
#if defined(__x86_64__) || defined(_M_X64)
  CAMLreturn(caml_copy_string("x86_64"));
#elif defined(__aarch64__) || defined(_M_ARM64)
  CAMLreturn(caml_copy_string("aarch64"));
#elif defined(__i386__) || defined(_M_IX86)
  CAMLreturn(caml_copy_string("x86"));
#else
  CAMLreturn(caml_copy_string("unknown"));
#endif
}
