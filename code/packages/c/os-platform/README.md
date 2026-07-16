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
| `fs`      | `os_platform/fs.h`      | ✅ implemented |
| `process` | `os_platform/process.h` | ✅ implemented |
| `dynlib`  | `os_platform/dynlib.h`  | ✅ implemented |
| `mmap`    | `os_platform/mmap.h`    | ✅ implemented |
| `jit`     | `os_platform/jit.h`     | ✅ implemented |

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

### `process` — spawn a child, wait, read its exit code

ISO `system()` blocks, needs a shell, and can't reliably report the exit code —
process control is bucket B.

```c
#include "os_platform/process.h"

const char *argv[] = { "cat", "notes.txt", NULL };
osp_process *p;
osp_process_spawn(&p, "/bin/cat", argv);   /* explicit path — no shell, no PATH */
int code;
osp_process_wait(p, &code);                /* blocks, reports exit code, frees p */
```

Backends:

| OS          | spawn              | wait                   | exit code            |
|-------------|--------------------|------------------------|----------------------|
| macOS/Linux | `fork` + `execv`   | `waitpid`              | `WIFEXITED`/`WEXITSTATUS` (128+signo if signalled) |
| Windows     | `CreateProcess`    | `WaitForSingleObject`  | `GetExitCodeProcess` |

**No shell, no PATH search:** `path` is handed to `execv`/`CreateProcess`
directly, so there is no shell word-splitting, globbing, or injection surface.
On Windows the backend re-quotes `argv` into a command line using the exact
`CommandLineToArgvW` rules, so the child reconstructs the caller's `argv` intact.

### `dynlib` — load a shared library, resolve a symbol, close

Loading code at run time (plugins, FFI) is bucket B — ISO C has no notion of it.

```c
#include "os_platform/dynlib.h"
#include <string.h>

osp_dynlib *lib;
osp_dynlib_open(&lib, "libm.so.6");     /* dlopen / LoadLibrary */

void *addr;
osp_dynlib_symbol(lib, "cos", &addr);   /* dlsym / GetProcAddress */

double (*cosfn)(double);
memcpy(&cosfn, &addr, sizeof cosfn);    /* void* -> fn ptr, warning-free */
double c = cosfn(0.0);                   /* == 1.0 */

osp_dynlib_close(lib);                   /* dlclose / FreeLibrary */
```

Backends:

| OS          | load          | resolve          | unload        |
|-------------|---------------|------------------|---------------|
| macOS/Linux | `dlopen`      | `dlsym`          | `dlclose`     |
| Windows     | `LoadLibrary` | `GetProcAddress` | `FreeLibrary` |

The POSIX BUILD links `-ldl` on **Linux only** (macOS has `dlopen` in libc). A
resolved symbol is a `void *`; convert it to a function pointer with `memcpy`
(the direct object↔function cast is non-ISO — which is exactly why `dynlib` lives
on `platform-harness`, not `iso-harness`).

### `mmap` — anonymous memory with protection control

`malloc` gives bytes; only the OS gives a page range with a chosen protection
(read-only data, guard pages, or executable JIT memory).

```c
#include "os_platform/mmap.h"

osp_mapping *m;
osp_map_anon(&m, 4096, OSP_PROT_READ | OSP_PROT_WRITE);  /* zero-filled pages */
unsigned char *p = osp_map_base(m);
p[0] = 0x42;                                             /* real, committed memory */
osp_map_protect(m, OSP_PROT_READ);                      /* now read-only */
osp_map_unmap(m);
```

Backends:

| OS          | reserve+commit | protect         | release      |
|-------------|----------------|-----------------|--------------|
| macOS/Linux | `mmap`         | `mprotect`      | `munmap`     |
| Windows     | `VirtualAlloc` | `VirtualProtect`| `VirtualFree`|

`prot` is a bitmask of `OSP_PROT_NONE/READ/WRITE/EXEC`. The `EXEC` bit is plumbed
through for JIT consumers; the full JIT protocol lives in `jit` below.

### `jit` — emit machine code at run time and call it

`mmap` gives you an executable page, but W^X means you cannot just scribble
instructions into it and jump — each OS has a protocol for the RW→RX transition.
`jit` encapsulates it: allocate, write bytes, commit, call.

```c
#include "os_platform/jit.h"

/* machine code for: int f(void){ return 42; } (x86_64) */
static const unsigned char code[] = {0xB8, 0x2A, 0x00, 0x00, 0x00, 0xC3};

osp_jit *j;
osp_jit_alloc(&j, sizeof code);       /* JIT-capable, writable */
osp_jit_write(j, code, sizeof code);  /* append machine code   */
osp_jit_commit(j);                    /* flip to R+X, flush i-cache */

void *entry = osp_jit_entry(j);
int (*fn)(void);
memcpy(&fn, &entry, sizeof fn);        /* void* -> function pointer */
int r = fn();                          /* -> 42 */
osp_jit_free(j);
```

Backends:

| OS                    | allocate           | write              | commit                                |
|-----------------------|--------------------|--------------------|---------------------------------------|
| macOS (Apple Silicon) | `mmap` `MAP_JIT`   | write-protect toggle + `memcpy` | `sys_icache_invalidate`  |
| Linux                 | `mmap` RW          | `memcpy`           | `mprotect` RX + `__builtin___clear_cache` |
| Windows               | `VirtualAlloc` RW  | `memcpy`           | `VirtualProtect` RX + `FlushInstructionCache` |

The instruction-cache flush is a no-op on x86 (coherent i-cache) but a real flush
on arm64 (both Linux and macOS) — omitting it runs stale bytes. The
`tests/jit_test.c` emit-and-call test carries x86_64 **and** arm64 machine code so
it proves the whole path on every runner in the 3-OS matrix.

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
│   ├── fs.h                      # fs API
│   ├── process.h                 # process API
│   ├── dynlib.h                  # dynlib API
│   ├── mmap.h                    # mmap API
│   └── jit.h                     # jit API
├── src/
│   ├── clock_posix.c   · clock_windows.c     # per-OS clock backends
│   ├── thread_posix.c  · thread_windows.c    # per-OS thread backends
│   ├── fs_posix.c      · fs_windows.c        # per-OS fs backends
│   ├── process_posix.c · process_windows.c   # per-OS process backends
│   ├── dynlib_posix.c  · dynlib_windows.c    # per-OS dynlib backends
│   ├── mmap_posix.c    · mmap_windows.c      # per-OS mmap backends
│   └── jit_posix.c     · jit_windows.c       # per-OS jit backends
├── tests/  clock · thread · fs · process · dynlib · mmap · jit  (one _test.c each)
├── tools/run.sh  · run.ps1       # per-OS build drivers
├── BUILD  · BUILD_windows        # per-OS source selection
└── required_capabilities.json    # CI needs gcc, clang, cl
```
