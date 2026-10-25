# Changelog

All notable changes to `iso-harness` are documented here.

## [Unreleased]

- Fix actual NN47 Windows CI negative-fixture failures under clang-cl: add its
  explicit `/clang:-pedantic-errors` in both positive and negative compiler paths,
  preserving `/permissive- /W4 /WX` and standards. Pin all four language/compiler
  flag policies; retain all installed compilers and the negative fixtures.
  Capture expected negative-compile stderr under a locally restored PowerShell 5
  error preference and decide rejection from the actual compiler exit status.

- NN47 Windows prerequisite repair: capture compiler discovery as an array in
  both build/negative guards, including zero/one/multiple-compiler regressions.
  Keep the executable diagnostic ASCII so PowerShell 5 can load the BOM-free
  shared source without interpreting a mojibake smart quote as syntax.
- Express the conforming C fixture's integer-width invariant with C17
  `_Static_assert` and exercise its runtime message macro with a function call,
  avoiding MSVC C4127 without suppressing warnings.

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
