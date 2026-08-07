# Changelog

All notable changes to `iso-harness` are documented here.

## [Unreleased]

### Added

- **Extended `iso_test.h` assertions** for the Rust→C/C++ port campaign:
  `ISO_CHECK_EQ_UINT` (unsigned/`size_t`), `ISO_CHECK_STR_EQ` (C strings /
  `std::string::c_str()`), `ISO_CHECK_MEM_EQ` (byte buffers — hashes, cipher
  output; prints the first differing byte), and `ISO_CHECK_EQ_DBL` (float
  equality within a tolerance, no `<math.h>`/`-lm`). The self-test fixtures now
  exercise all of them under GCC, Clang, and MSVC.
- Initial release of the pure-ISO C/C++ multi-compiler build harness (CCPP01 PR3).
- `lib/iso-lib.sh` (POSIX shell) and `lib/iso-lib.ps1` (PowerShell/MSVC): compile
  sources with every present compiler under strict conformance flags
  (`-std=c17`/`-std=c++17 -pedantic-errors -Wall -Wextra -Werror` for GCC/Clang;
  `/std:c17`/`/std:c++17 /permissive- /W4 /WX` for MSVC), run the result, and a
  negative-test helper asserting non-ISO code is rejected. Honors `ISO_REQUIRE`
  to make specific compilers mandatory, plus `ISO_INCLUDE` / `ISO_CSTD` /
  `ISO_CXXSTD` / `ISO_BUILD_DIR`.
- `include/iso_test.h`: a header-only, dependency-free unit-test harness that is
  the intersection of ISO C17 and C++17 (`ISO_CHECK`, `ISO_CHECK_MSG`,
  `ISO_CHECK_EQ_INT`, `ISO_TEST_RESULT`).
- Self-testing `BUILD` / `BUILD_windows`: conforming fixtures must build and run
  on every compiler; non-conforming fixtures (GNU statement expressions) must be
  rejected — the harness verifying it enforces conformance.
