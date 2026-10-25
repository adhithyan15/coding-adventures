---
category: Cross-platform & Windows BUILD_windows
---

# clang-cl needs explicit pedantic diagnostics to reject GNU extensions in ISO harness tests

Current-head CI40242573039 Windows job121319644429 reached the new NN47
MSVC lane. MSVC and clang-cl compiled and ran the positive C/C++ fixtures;
MSVC rejected the negative GNU statement-expression fixtures, but clang-cl
accepted both under /permissive- /W4 /WX. The shared harness correctly failed
before parser execution. /permissive- alone is not Clang's pedantic mode.
Select strict flags per compiler: retain all MSVC flags and additionally pass
/clang:-pedantic-errors to clang-cl, in both positive and negative paths.
Pin flag selection and rerun both actual compiler families. Never remove the
negative fixture or skip an installed compiler to make conformance green.

Actual local two-driver validation then exposed Windows PowerShell 5 converting
clang-cl's expected stderr into NativeCommandError under Stop, even with output
redirected. Only within the expected-rejection compiler call, capture the exit
status under Continue and restore the original preference in finally. The
accept/reject verdict still uses the real compiler status; positive compilation
keeps Stop and unchanged fatal diagnostics.
