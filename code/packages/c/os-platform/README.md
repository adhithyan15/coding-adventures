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

| Module   | Header                  | Status        |
|----------|-------------------------|---------------|
| `clock`  | `os_platform/clock.h`   | ✅ implemented |
| `thread` | `os_platform/thread.h`  | ✅ implemented |
| `fs`     | `os_platform/fs.h`      | ✅ implemented |
| process  | —                       | planned       |
| dynlib   | —                       | planned       |
| mmap     | —                       | planned       |

All primitives share the `osp_status` return convention from
`os_platform/status.h` (`OSP_OK == 0`; negative on error).

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

### `thread` — threads, mutexes, condition variables

Concurrency is bucket B (C11 `<threads.h>` is optional and MSVC-absent). Opaque
handles hide the OS types; create with `_init`/`_spawn`, release with
`_destroy`/`_join`:

```c
#include "os_platform/thread.h"

static void *worker(void *arg) { /* … */ return arg; }

osp_thread *t;
osp_thread_spawn(&t, worker, ctx);
void *result;
osp_thread_join(t, &result);        /* waits, delivers worker's return, frees t */

osp_mutex *m; osp_mutex_init(&m);
osp_mutex_lock(m); /* … */ osp_mutex_unlock(m);

osp_cond *c; osp_cond_init(&c);
osp_mutex_lock(m);
while (!ready) osp_cond_wait(c, m); /* loop guards spurious wake-ups */
osp_mutex_unlock(m);
```

Backends:

| OS          | thread                             | mutex              | cond                 |
|-------------|------------------------------------|--------------------|----------------------|
| macOS/Linux | `pthread_create` / `_join`         | `pthread_mutex_t`  | `pthread_cond_t`     |
| Windows     | `_beginthreadex` / `WaitForSingleObject` | `CRITICAL_SECTION` | `CONDITION_VARIABLE` |

The POSIX backend links the OS thread library (`-pthread`); the Windows backend
uses only the CRT + kernel32. Mutexes are non-recursive; a condition variable is
always waited on while holding its paired mutex.

### `fs` — metadata, whole-file I/O, directory listing

ISO `<stdio.h>` opens a file by name but cannot list a directory or report a
file's type, size, or modification time — that needs the OS.

```c
#include "os_platform/fs.h"

osp_fs_write_file("out.bin", bytes, n);   /* create/truncate, binary-safe */

unsigned char *data; size_t len;
osp_fs_read_file("out.bin", &data, &len); /* malloc'd, NUL-terminated; you free */
free(data);

osp_file_info info;
osp_fs_stat("out.bin", &info);            /* is_dir / is_regular / size / mtime */
if (osp_fs_exists("out.bin")) { /* … */ }

static void on_entry(const char *name, void *user) { /* … */ }
osp_fs_list_dir(".", on_entry, NULL);     /* callback per entry, skips . and .. */
```

Backends:

| OS          | metadata              | read / write               | list                        |
|-------------|-----------------------|----------------------------|-----------------------------|
| macOS/Linux | `stat`                | `open`/`read` · `write`    | `opendir`/`readdir`         |
| Windows     | `GetFileAttributesEx` | `CreateFile`+`ReadFile`/`WriteFile` | `FindFirstFile`/`FindNextFile` |

`osp_fs_read_file` is binary-safe (length-based; embedded NULs preserved) and the
allocation is guarded against `size_t` overflow. `mtime_unix_ns` is
second-resolution on POSIX (portable `st_mtime`).

## Build & test

```sh
cd code/packages/c/os-platform
sh tools/run.sh        # macOS / Linux (Windows: tools\run.ps1 via BUILD_windows)
```

This compiles each primitive's test with the OS's backend under every present
compiler and runs it, printing `N checks, 0 failed`. Output goes to `_build/`
(never `build/`, which collides with the `BUILD` file on case-insensitive
filesystems). Remove it with `rm -rf _build`.

## Layout

```
os-platform/
├── include/os_platform/
│   ├── status.h                  # shared osp_status enum (all primitives)
│   ├── clock.h                   # clock API
│   ├── thread.h                  # thread API
│   └── fs.h                      # fs API
├── src/
│   ├── clock_posix.c   · clock_windows.c    # per-OS clock backends
│   ├── thread_posix.c  · thread_windows.c   # per-OS thread backends
│   └── fs_posix.c      · fs_windows.c       # per-OS fs backends
├── tests/clock_test.c · thread_test.c · fs_test.c   # per-primitive tests
├── tools/run.sh  · run.ps1       # per-OS build drivers
├── BUILD  · BUILD_windows        # per-OS source selection
└── required_capabilities.json    # CI needs gcc, clang, cl
```
