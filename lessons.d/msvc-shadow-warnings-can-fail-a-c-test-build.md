---
category: Cross-platform & Windows BUILD_windows
---

# MSVC shadow warnings can fail a C++ test build

MSVC warning C4456 reports a local variable that hides a same-named variable
from an enclosing scope. Under the repository's `/W4 /WX` flags that warning
is fatal (C2220). Use a distinct name for nested test-loop variables and
preflight C++ test translation units with both GCC/Clang shadow warnings and
the strict MSVC-family build.
