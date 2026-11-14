# Changelog

All notable changes to this package will be documented in this file.

## [Unreleased]

### Fixed

- The MSVC-family flags now include `/D_CRT_SECURE_NO_WARNINGS=` (defined empty, as
  `iso-harness` does). Under `/W4 /WX` the C4996 "unsafe function" deprecation of
  `getenv` failed `c/os-platform` on Windows (`getenv("ComSpec")` in its process
  test; the first scheduled cross-OS build, on both `cl` and `clang-cl`). The macro
  silences only that one warning; every other warning stays fatal. No package that
  uses this harness defines the macro itself, so the empty value cannot collide.

## [0.1.0] - 2026-07-13

### Added

- Initial `platform-harness` — the OS-dependent, multi-compiler C/C++ build
  harness that is the non-pure-ISO sibling of `iso-harness` (CCPP02 Phase 0).
- `lib/platform-lib.sh` (POSIX) and `lib/platform-lib.ps1` (Windows/MSVC):
  `platform_os`, `platform_compilers`, and `platform_build_and_run` compile with
  every present compiler and run the result, keeping `-Wall -Wextra -Werror`
  (`/W4 /WX`) but **dropping** `-pedantic-errors` / `/permissive-` so OS headers
  and idioms are allowed.
- Environment knobs `PLATFORM_REQUIRE`, `PLATFORM_INCLUDE`, `PLATFORM_LIBS`
  (OS-provided link tokens), `PLATFORM_DEFINES`, `PLATFORM_CSTD`/`PLATFORM_CXXSTD`,
  and `PLATFORM_BUILD_DIR`. Reuses `iso-harness`'s `iso_test.h` via
  `PLATFORM_INCLUDE`.
- Self-test: a POSIX thread test (C `pthread` + C++ `std::thread`, linking
  `-pthread`) on mac/linux and a Winsock-init test (linking `ws2_32.lib`) on
  Windows, proving the harness builds and runs OS-dependent code under strict
  warnings without the pure-ISO pedantic gate.
