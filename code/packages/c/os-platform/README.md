# os-platform (C)

The repo's **bucket B** substrate: the small set of primitives a program cannot
compute on its own and must obtain from the operating system. Everything that
*is* computable — math, crypto, codecs, parsers, 128-bit integers — stays on the
pure-ISO `iso-harness` and never comes here. `os-platform` is only for the things
that fundamentally require the OS.

It is built by the sibling **`platform-harness`** (not `iso-harness`): every
translation unit still compiles under every available compiler with
`-Wall -Wextra -Werror` (`/W4 /WX`), but **without** `-pedantic-errors` /
`/permissive-`, because POSIX and Win32 headers are legitimately not strict-ISO.

Per-OS **source selection** is done by the build-tool via `BUILD` (macOS + Linux,
the POSIX backend) and `BUILD_windows` (the Win32 backend) — so each OS compiles
exactly one backend and the code contains no `#if defined(__linux__)` mazes.

## Primitives

| Module  | Header                     | Status        |
|---------|----------------------------|---------------|
| `clock` | `os_platform/clock.h`      | ✅ implemented |
| thread  | —                          | planned       |
| fs      | —                          | planned       |
| process | —                          | planned       |
| dynlib  | —                          | planned       |
| mmap    | —                          | planned       |

See [`CCPP02-os-platform-lane.md`](../../../specs/CCPP02-os-platform-lane.md) for
the full plan.

### `clock` — real time from the OS

Three things ISO C17 cannot do portably and precisely:

```c
#include "os_platform/clock.h"

uint64_t elapsed_start, elapsed_end;
osp_monotonic_ns(&elapsed_start);        /* steady, never-backward clock  */
/* … do work … */
osp_monotonic_ns(&elapsed_end);
uint64_t took_ns = elapsed_end - elapsed_start;

int64_t now;
osp_wall_unix_ns(&now);                   /* ns since 1970-01-01 UTC       */

osp_sleep_ns(50ULL * 1000000ULL);         /* sleep ~50 ms                  */
```

**Monotonic vs wall** — a stopwatch vs a wristwatch. `osp_monotonic_ns` measures
*durations* (immune to clock changes; only differences are meaningful).
`osp_wall_unix_ns` is *calendar* time for human-readable timestamps (it can jump
when the system clock is corrected — never subtract two wall readings to time
something). Every call returns an `osp_status` (`OSP_OK == 0`; negative on error)
and writes its result through an out-parameter.

Backends:

| OS          | monotonic                              | wall                             | sleep       |
|-------------|----------------------------------------|----------------------------------|-------------|
| macOS/Linux | `clock_gettime(CLOCK_MONOTONIC)`       | `clock_gettime(CLOCK_REALTIME)`  | `nanosleep` |
| Windows     | `QueryPerformanceCounter`/`Frequency`  | `GetSystemTimePreciseAsFileTime` | `Sleep`     |

The POSIX backend needs `_POSIX_C_SOURCE=200809L` (supplied by the BUILD) and
links no extra library (both calls live in libc on modern glibc/macOS). The
Windows backend uses only kernel32, linked by MSVC by default. On Windows,
`osp_sleep_ns` has millisecond granularity (`Sleep` rounds up to whole ms).

## Build & test

```sh
cd code/packages/c/os-platform
sh tools/run.sh        # macOS / Linux (Windows: tools\run.ps1 via BUILD_windows)
```

This compiles `tests/clock_test.c` with the OS's clock backend under every
present compiler and runs it, printing `N checks, 0 failed`. Output goes to
`_build/` (never `build/`, which collides with the `BUILD` file on
case-insensitive filesystems). Remove it with `rm -rf _build`.

## Layout

```
os-platform/
├── include/os_platform/clock.h   # shared public API (one header per primitive)
├── src/clock_posix.c             # macOS + Linux backend
├── src/clock_windows.c           # Windows backend
├── tests/clock_test.c            # property tests, run on each OS
├── tools/run.sh  · run.ps1       # per-OS build drivers
├── BUILD  · BUILD_windows        # per-OS source selection
└── required_capabilities.json    # CI needs gcc, clang, cl
```
