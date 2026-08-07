# platform-harness

A portable, multi-compiler build harness for **OS-dependent** C/C++ — the
non-pure-ISO sibling of [`iso-harness`](../iso-harness). Part of the
[CCPP02](../../../specs/CCPP02-os-platform-lane.md) OS-platform lane.

## Why it exists

`iso-harness` proves code is **pure ISO** by compiling it under
`-pedantic-errors` / `/permissive-` — perfect for computation (math, crypto,
codecs, data structures). But some things fundamentally need the operating
system — threads, real clocks, filesystem enumeration, sockets, event loops,
dynamic loading — and there is no pure-ISO way to express them. POSIX and Win32
headers, and idioms like `dlsym`'s `void*`→function-pointer cast, are
legitimately *not* strict ISO.

`platform-harness` is for that code. It still:

- compiles with **every** compiler present (gcc + clang on Linux, Apple clang on
  macOS, `cl` / `clang-cl` on Windows) and **runs** the result, so portability
  is proven across the CI matrix;
- keeps `-Wall -Wextra -Werror` (`/W4 /WX`) so real bugs stay fatal;

but **drops** `-pedantic-errors` / `/permissive-`, because the code deliberately
talks to the OS. It also links OS-provided libraries (`PLATFORM_LIBS`) and
per-OS source selection is done by the build-tool via `BUILD_mac` /
`BUILD_linux` / `BUILD_windows`.

## Using it from a package

A platform package's `tools/run.sh` sources the library and calls the workhorse:

```sh
. /path/to/code/packages/c/platform-harness/lib/platform-lib.sh
PLATFORM_INCLUDE="include /path/to/iso-harness/include"   # your headers + iso_test.h
PLATFORM_LIBS="-pthread -ldl"                             # OS-provided libs only
PLATFORM_DEFINES="_POSIX_C_SOURCE=200809L"
export PLATFORM_INCLUDE PLATFORM_LIBS PLATFORM_DEFINES
platform_build_and_run c mylib-tests tests/mylib_test.c src/mylib_posix.c
```

The Windows half (`lib/platform-lib.ps1`, called from `tools/run.ps1`) mirrors
this over `cl.exe`/`clang-cl.exe` with `PLATFORM_LIBS="ws2_32.lib"`-style tokens.

### Environment knobs

| knob | meaning |
| --- | --- |
| `PLATFORM_REQUIRE` | compilers that MUST be present (else hard fail) |
| `PLATFORM_INCLUDE` | include dirs → `-I` each (point one at iso-harness/include for `iso_test.h`) |
| `PLATFORM_LIBS` | OS-provided link tokens appended after sources (`-pthread`, `ws2_32.lib`, …) |
| `PLATFORM_DEFINES` | preprocessor defines → `-D` each |
| `PLATFORM_CSTD` / `PLATFORM_CXXSTD` | standard overrides (default `c17` / `c++17`) |
| `PLATFORM_BUILD_DIR` | output dir (default `_build`; never `build`) |

## Self-test

```sh
sh BUILD          # POSIX: builds+runs a pthread test on gcc and/or clang
```

The self-test spawns a POSIX thread (C) and a `std::thread` (C++), linking
`-pthread`; on Windows (`BUILD_windows`) it initialises Winsock, linking
`ws2_32.lib`. Each prints `N checks, 0 failed`, proving the harness compiles and
runs OS code under strict warnings without the pure-ISO pedantic gate.
