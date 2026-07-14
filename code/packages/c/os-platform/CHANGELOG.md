# Changelog

All notable changes to the `os-platform` (C) package are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to semantic versioning.

## [Unreleased]

### Added

- **`thread` primitive** (CCPP02 Phase 2, PR 2): threads, mutexes, and condition
  variables — concurrency is bucket B (C11 `<threads.h>` is optional and
  MSVC-absent). Opaque heap handles keep OS types out of the shared header; each
  `_init`/`_spawn` allocates and each `_destroy`/`_join` frees, so nothing leaks.
  - `osp_thread_spawn` / `osp_thread_join` (worker is `void *(*)(void *)`; join
    delivers the worker's result and frees the handle).
  - `osp_mutex_init` / `_lock` / `_unlock` / `_destroy` (non-recursive).
  - `osp_cond_init` / `_wait` / `_signal` / `_broadcast` / `_destroy`.
  - Backends: `thread_posix.c` (pthreads; links `-pthread` via PLATFORM_LIBS) and
    `thread_windows.c` (`_beginthreadex` + `CRITICAL_SECTION` +
    `CONDITION_VARIABLE`; CRT + kernel32, no extra lib).
  - Integration tests (`tests/thread_test.c`): four-thread locked-counter mutual
    exclusion (deterministic, not flaky), condition-variable handoff + return
    value, and NULL-argument rejection. Verified under ASan+UBSan **and
    ThreadSanitizer** (no data races) with 0 leaks.
- **`os_platform/status.h`**: the shared `osp_status` enum, extracted so multiple
  primitive headers can be included together without a duplicate `enum`
  definition; adds `OSP_ERR_NOMEM`. `clock.h` now includes it (no API change).

### Added — PR 1

- **Initial package + `clock` primitive** (CCPP02 Phase 2, PR 1). The first
  bucket-B library: OS-provided capabilities that pure-ISO C cannot compute.
  Built by `platform-harness` (warnings-as-errors, but not `-pedantic-errors`).
  - `osp_monotonic_ns` — steady, never-backward elapsed-time clock in ns.
  - `osp_wall_unix_ns` — calendar time as ns since the UNIX epoch.
  - `osp_sleep_ns` — suspend the current thread for at least N ns (EINTR-safe on
    POSIX; millisecond granularity on Windows).
  - `osp_status` error enum (`OSP_OK`, `OSP_ERR_OS`, `OSP_ERR_INVAL`).
  - Per-OS backends selected by BUILD, never `#ifdef`: `clock_posix.c`
    (`clock_gettime` / `nanosleep`, macOS + Linux) and `clock_windows.c`
    (`QueryPerformanceCounter` / `GetSystemTimePreciseAsFileTime` / `Sleep`).
  - Property tests (`tests/clock_test.c`) that run on each OS in the 3-OS CI
    matrix: monotonicity, NULL rejection, a sane wall-clock calendar window
    (which validates each backend's epoch conversion), and sleep-advances-clock.
