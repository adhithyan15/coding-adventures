# DB-E107 — Preserve the selected Windows Clang driver for ISO tests

The Windows runner's MSVC bootstrap selected Visual Studio Clang
22.1.3 for Microsoft STL 14.52, whose headers require Clang 22 or
newer. Both compilers passed the harness early in the run. After
Swift 6.4.2 joined the job's PATH, the affected-package build
selected its Clang 21 driver. The ISO C++ positive control failed at
`STL1000` and the build skipped C NAND and other ISO dependents.
[C NAND Windows job](https://github.com/adhithyan15/coding-adventures/actions/runs/45931050492/job/140285098816)

Export the compiler selected by the current MSVC bootstrap as an
absolute executable path. Use that identity consistently for
discovery, required-compiler validation, compilation, and negative
controls in the PowerShell ISO harness. When the selection is
absent, use ordinary PATH discovery. A supplied but missing or
invalid executable receives a diagnostic and a failed test front.
Keep the existing C17 and C++17 strict flags and preserve Swift's
toolchain PATH ordering.

Exercise both a selected and a PATH-only compiler, a shadow candidate
first on PATH, a selected path containing spaces, and explicit
invalid and missing candidates. Check the positive and negative
compile paths and require the unchanged strict switches. Rerun the
Python bootstrap tests and the PowerShell and POSIX harness controls.
Close the native Windows evidence after the repaired PR runs both C
compilers and reaches the C NAND build.

## Local verification

The independent bootstrap call exported the selected executable path
with spaces. A separate PowerShell caller put an older stand-in first
on PATH and verified the chosen executable, exact C++17 argument
tokens and the positive and negative calls. It verified PATH
fallback and the relative, wrong-name and missing-path refusals.

Ten Python tests, six compiler cardinality checks, four flag
policies, 13 resolver checks and the Windows front syntax check
passed. The native C/C++ ISO self-test passed with GCC and Clang.
Both C compilers passed 1,595 bounded JSON checks and 113,713 C NAND
checks. The NAND source-input projection remains synchronized.
Native Windows verification follows in the repaired PR.
