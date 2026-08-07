# CCPP02 — OS-dependent C/C++ lane (from-scratch, no CMake, no libm)

Status: in progress (Phase 0 of ~5)

## Motivation

The C/C++ port campaign (CCPP01) established a strict **pure-ISO** lane: every C
and C++ package builds through the shared `iso-harness`
(`code/packages/c/iso-harness/`), which compiles each translation unit with every
available compiler under `-std=c17/c++17 -pedantic-errors -Wall -Wextra -Werror`
(`/permissive- /W4 /WX` on MSVC). That lane rejects anything non-ISO by design —
which is exactly why a large class of Rust crates has been **skipped**: anything
that needs the operating system or a language extension cannot pass.

A census of the 774 Rust crates lacking a C port shows the backlog (categories
overlap): math ~83, `u128`/`i128` ~48, threads/atomics ~112, filesystem ~72,
time ~76, process ~62, networking ~24, event-loop ~15, FFI/dlopen ~77,
GPU/audio/GUI ~73.

**Goal:** give the repo a from-scratch, cross-platform *systems* substrate so
these crates stop being permanently deferred, then continue the port campaign
into them — while preserving the pure-ISO lane unchanged for everything that
does not need the OS.

## The dividing line (the key idea)

Every deferred crate falls into one of two buckets, and the fix differs:

### Bucket A — computable from scratch → stays pure & portable (touches no OS)

Runs on the existing `iso-harness`, unchanged. This includes **math**
(`sqrt`/`sin`/`cos`/`exp`/`log`/`pow` via Newton / CORDIC / Taylor series with
argument reduction — the repo already does this in the `c/trig` crate, and
`rust/math-core` is the from-scratch elementary-functions library to port),
`u128`/`i128`, crypto, compression, codecs, parsers, PRNGs, fixed-point
arithmetic, and data structures. **No libm, no OS** — just write the algorithm.

### Bucket B — fundamentally the OS → must call the OS C API

Real clock/time, sleep, threads / mutex / condvar / atomic fences, filesystem
enumeration & metadata, process spawn/wait, dynamic loading, sockets, event
loops (epoll/kqueue/iocp), memory-mapping & executable memory. There is **no**
pure-ISO way to do these:

- C11 `<threads.h>` is an *optional* feature and MSVC does not ship it, so C code
  must call **pthreads** (POSIX) or **Win32** (Windows) directly.
- C++17 `<thread>`, `<mutex>`, `<atomic>`, `<chrono>`, `<filesystem>` *are*
  portable standard library — but they are thin wrappers over those same OS
  calls (and on Linux `<thread>` still needs `-pthread` at link).

Bucket B builds on a new, from-scratch abstraction library, `os-platform`, with
per-OS `#if` backends, compiled by a new `platform-harness`.

## Governing principles

1. **No CMake.** The Go build-tool already selects a per-OS BUILD file
   (`GetBuildFileForPlatform`: `BUILD_mac` / `BUILD_linux` / `BUILD_windows` /
   `BUILD_mac_and_linux` → `BUILD`), which is all the platform selection we need.
   (`cpp/mosaic-flux-qt/CMakeLists.txt` remains end-user documentation only — it
   is already not on the build path, and is untouched.)
2. **Only OS-provided libraries + the portable standard library; everything else
   from scratch.** Prefer *from scratch* wherever a thing is computable
   (bucket A) — including **math** (no libm). Link an OS-provided library only
   for bucket-B primitives that have no portable form: `pthread`/`-ldl`, Winsock
   (`ws2_32`), `kernel32`, epoll/kqueue/iocp, mmap/`VirtualAlloc`,
   dlopen/`LoadLibrary`. The standard library is allowed **iff** it is in the
   core language *and* supported by GCC + Clang + MSVC (→ C++17
   `<thread>`/`<mutex>`/`<atomic>`/`<chrono>`/`<filesystem>` yes; C11
   `<threads.h>` no).
3. **Infra + 1–2 representative ports per category**, then the campaign grinds
   the rest tier-by-tier using the established patterns.

## Design

### Two harnesses

**`iso-harness` (unchanged)** — the strict pure-ISO multi-compiler lane. Because
math is written from scratch, no pure-ISO package needs to link a library, so no
change is required here.

**`platform-harness` (new, `code/packages/c/platform-harness/`)** — a sibling
harness, same shape as iso-harness (`lib/platform-lib.sh`, `lib/platform-lib.ps1`,
a self-test `BUILD`/`BUILD_windows`, README, CHANGELOG). It reuses
`iso-harness/include/iso_test.h` (packages add it via `PLATFORM_INCLUDE`).
Contract:

- Detect the OS (`platform_os` → `mac` | `linux` | `windows` | `other`) and every
  available compiler; compile + run with each; a `PLATFORM_REQUIRE` knob makes
  "compiler X must be present" a hard failure (mirrors `ISO_REQUIRE`).
- Keep `-Wall -Wextra -Werror` (`/W4 /WX`) for real diagnostics, but **drop
  `-pedantic-errors` / `/permissive-`**: POSIX/Win32 headers and idioms (e.g.
  `dlsym`'s `void*`→function-pointer cast, `_POSIX_C_SOURCE`-gated declarations)
  are legitimately non-strict-ISO. This is the deliberate difference from
  iso-harness.
- Two extra knobs: `PLATFORM_LIBS` (OS libraries to link, e.g. `-pthread -ldl` /
  `ws2_32.lib`) and `PLATFORM_DEFINES` (e.g. `_POSIX_C_SOURCE=200809L`,
  `WIN32_LEAN_AND_MEAN`).
- Per-OS **source selection** happens at the BUILD-file level (each package ships
  `BUILD_mac`/`BUILD_linux`/`BUILD_windows`, a one-line `sh tools/run.sh`), so the
  harness never globs sources; each platform compiles only its backend.
- Output to `_build/` (never `build/`, which collides with the `BUILD` file on
  case-insensitive filesystems).

### `os-platform` API surface (Phase 2)

One library (`code/packages/{c,cpp}/os-platform/`) with submodule headers and
per-OS `#if defined(_WIN32) / __APPLE__ / __linux__` backends. Each primitive is
implemented once per OS and reused by all downstream ports:

- **clock** — monotonic nanoseconds + wall-clock UNIX time + sleep
  (`clock_gettime(CLOCK_MONOTONIC)` / `QueryPerformanceCounter`).
- **thread** — spawn/join, mutex, condvar (C: pthread / Win32;
  C++: `<thread>`/`<mutex>`/`<condition_variable>`).
- **fs** — directory listing, stat/metadata, whole-file read/write
  (C: dirent + stat / FindFirstFile + GetFileAttributesEx; C++: `<filesystem>`).
- **process** — spawn + wait + exit code (fork/exec / CreateProcess).
- **dynlib** — load, symbol lookup, close (dlopen/dlsym / LoadLibrary/GetProcAddress).
- **mmap** — anonymous + executable memory (mmap/mprotect / VirtualAlloc/VirtualProtect).

Each returns a status code (C) or throws (C++), never leaks OS handles, and is
tested on its own OS via the CI matrix.

### Build-tool & CI

No changes are expected. `c`/`cpp` already infer to the `cpp` toolchain
(CCPP01); the 3-OS PR matrix already installs GCC+Clang (ubuntu) / MSVC
(windows) / Apple Clang (macOS); the OS libraries above ship with every runner.
A `-t platform-library` template is added to `scaffold-generator` to emit the
`BUILD_*` + platform-harness wiring when the first `os-platform` package is
created.

## Phases

- **Phase 0 (this):** this spec; `platform-harness` + self-test. No ports yet.
- **Phase 1:** bucket-A quick wins on iso-harness — `wide-int` (from-scratch
  `u128`/`i128`) and `math-core` (port of `rust/math-core`, no libm).
- **Phase 2:** `os-platform` (bucket B), one primitive per sub-PR.
- **Phase 3:** `net` (sockets) and `reactor` (epoll/kqueue/iocp) on top of
  `os-platform`, each with a loopback integration test.
- **Phase 4:** one SDK-bound representative (a `dlopen`-based plugin host) with
  graceful skip when the SDK is absent.

## Out of scope / deferred (documented, not built this round)

- **TLS** — a large from-scratch effort; the crypto primitives (aes, sha256,
  x25519, chacha20-poly1305, hmac, hkdf) already exist, so it is feasible later.
- **GPU** (CUDA/Metal), **GUI** (Qt/Cocoa/X11), and full **FFI bridges** beyond
  the one `dlopen` representative — SDK/environment-bound; each will get a
  per-OS BUILD that detects the SDK and skips gracefully (the `cpp/conduit`
  `BUILD_windows` skip precedent).

## Verification

1. `platform-harness` self-test: a POSIX snippet that spawns a `pthread` builds
   and runs on mac + linux (linking `PLATFORM_LIBS="-pthread"`), and a Win32
   snippet builds and runs on windows — each under `-Wall -Wextra -Werror`
   without `-pedantic-errors`, printing `N checks, 0 failed`.
2. `math-core` / `wide-int`: tolerance / golden-vector / property tests under
   ASan + UBSan, proving from-scratch math and 128-bit arithmetic are correct
   with no libm and no `__int128` reliance.
3. `os-platform`: per-primitive tests run on their own OS across the 3-OS PR
   matrix; ASan + UBSan + macOS `leaks` on the POSIX side.
4. `net`/`reactor`: a loopback echo round-trip that opens a real socket and
   exchanges bytes on each OS.
