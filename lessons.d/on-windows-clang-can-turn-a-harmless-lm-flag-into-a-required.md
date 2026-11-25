---
category: Testing & coverage
---

# On Windows clang can turn a harmless -lm flag into a required m.lib

The Windows Swift Clang driver selected the MSVC linker, and the otherwise
unneeded Unix `-lm` flag became a request for `m.lib`. A version probe had
passed, but the CLI link failed before any source test could run.

For this C package the math operations are provided by the compiler/runtime
headers, so omit `-lm` on this Windows Clang invocation. Verify that the full
compile and link succeeds; do not report the earlier version response as
compiler validation. Keep GCC's native Windows build and the repository's
clang-cl harness as separate checks.
