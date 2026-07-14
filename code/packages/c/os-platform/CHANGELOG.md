# Changelog

All notable changes to the `os-platform` (C) package are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to semantic versioning.

## [Unreleased]

### Added

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
