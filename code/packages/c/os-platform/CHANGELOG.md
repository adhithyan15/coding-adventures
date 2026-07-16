# Changelog

All notable changes to the `os-platform` (C) package are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to semantic versioning.

## [Unreleased]

### Added

- **`mmap` primitive** (CCPP02 Phase 2, PR 6 — completes the six-primitive core):
  anonymous virtual memory with protection control — `malloc` gives bytes,
  but only the OS gives a page range with a chosen protection (read-only data,
  guard pages, or executable JIT memory).
  - `osp_map_anon(out, len, prot)` (zero-filled anonymous pages),
    `osp_map_protect`, `osp_map_base` / `osp_map_size`, `osp_map_unmap`. `prot`
    is an OSP_PROT_* bitmask (NONE/READ/WRITE/EXEC).
  - Backends: `mmap_posix.c` (`mmap(MAP_PRIVATE|MAP_ANONYMOUS)` / `mprotect` /
    `munmap`; the BUILD adds `_DEFAULT_SOURCE` + `_DARWIN_C_SOURCE` so
    MAP_ANONYMOUS is visible on both glibc and Darwin) and `mmap_windows.c`
    (`VirtualAlloc(MEM_RESERVE|MEM_COMMIT)` / `VirtualProtect` /
    `VirtualFree(MEM_RELEASE)`; OSP_PROT_* mapped onto the PAGE_* matrix).
  - Test (`tests/mmap_test.c`): anonymous RW map, zero-fill check, page-sized
    write/read checksum, protection change to READ-only, accessors, unmap, and
    NULL/zero-length validation. Clean under ASan+UBSan, 0 leaks.
  - The EXEC bit is plumbed to PROT_EXEC / PAGE_EXECUTE_* for JIT consumers; a
    dedicated JIT executor (per-arch machine code + the Apple-Silicon MAP_JIT
    write-protect protocol + an execute-and-call test) is a planned follow-up.
- **`dynlib` primitive** (CCPP02 Phase 2, PR 5): load a shared library, resolve a
  symbol, unload — the foundation for plugins/FFI, and pure bucket B (ISO C
  cannot load code at run time).
  - `osp_dynlib_open` / `osp_dynlib_symbol` (address into a `void *`) /
    `osp_dynlib_close`.
  - Backends: `dynlib_posix.c` (`dlopen(RTLD_NOW|RTLD_LOCAL)` / `dlsym` /
    `dlclose`; the `dlerror()`-clear dance distinguishes a missing symbol from a
    legitimately NULL-valued one) and `dynlib_windows.c` (`LoadLibraryA` /
    `GetProcAddress` / `FreeLibrary`; FARPROC→`void*` via memcpy to avoid the
    MSVC C4054 function/data-pointer cast).
  - The POSIX BUILD links `-ldl` on **Linux only** (macOS has dlopen in libc and
    ships no libdl). This is why dynlib lives on platform-harness: converting the
    resolved address to a function pointer is not strict-ISO.
  - Test (`tests/dynlib_test.c`): loads a per-OS system library (libc.so.6 /
    libSystem.dylib / kernel32.dll), resolves and *calls* a known symbol
    (getpid / GetCurrentProcessId) via a memcpy'd function pointer, and checks
    missing-symbol + NULL-arg errors. Clean under ASan+UBSan, 0 leaks.
- **`process` primitive** (CCPP02 Phase 2, PR 4): spawn a child program, wait,
  read its exit code — bucket B (ISO `system()` blocks, needs a shell, and can't
  reliably report the exit code).
  - `osp_process_spawn(out, path, argv)` (explicit executable path, **no shell,
    no PATH search** — nothing to inject into) and `osp_process_wait(p, &code)`
    (blocks, reports the exit code, frees the handle).
  - Backends: `process_posix.c` (`fork` + `execv` + `waitpid`, EINTR-safe; child
    `_exit(127)` on exec failure; signal death reported as 128+signo) and
    `process_windows.c` (`CreateProcess` + `WaitForSingleObject` +
    `GetExitCodeProcess`). Both link no extra library beyond libc / kernel32.
  - The Windows backend re-quotes argv into a command line using the exact
    `CommandLineToArgvW` rules (backslash/quote doubling), so a child sees the
    same argv the caller passed — implemented once and guarded against
    argument-injection.
  - Test (`tests/process_test.c`): spawns the system shell to exit with 42/0/7
    and asserts the code round-trips (which also proves the args arrived intact),
    plus NULL-arg rejection. Clean under ASan+UBSan, 0 leaks.
- **`fs` primitive** (CCPP02 Phase 2, PR 3): filesystem metadata, whole-file
  read/write, and directory listing — bucket B (ISO `<stdio.h>` opens files by
  name but cannot list a directory or report type/size/mtime).
  - `osp_fs_stat` → `osp_file_info` (is_dir / is_regular / size / mtime_unix_ns)
    and `osp_fs_exists`.
  - `osp_fs_read_file` (whole file into a malloc'd, NUL-terminated, binary-safe
    buffer; caller frees) and `osp_fs_write_file` (create/truncate).
  - `osp_fs_list_dir` (callback per entry, skipping "." / "..").
  - Backends: `fs_posix.c` (`stat`/`open`/`fstat`/`read`/`write`/`opendir`; libc
    only, EINTR-safe and partial-read/write loops; fstat on the open fd to avoid
    a TOCTOU size gap) and `fs_windows.c` (`GetFileAttributesEx` /
    `CreateFile`+`ReadFile`/`WriteFile` chunked to DWORD / `FindFirstFile`;
    FILETIME→UNIX-ns like the clock backend; kernel32 only).
  - Size→allocation guarded against 32-bit `size_t` truncation and `len+1`
    wraparound. Round-trip test (binary payload with an embedded NUL) verified
    under ASan+UBSan with 0 leaks; files created under gitignored `_build/`.
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
