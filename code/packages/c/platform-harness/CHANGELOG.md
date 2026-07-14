# Changelog

All notable changes to this package will be documented in this file.

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
