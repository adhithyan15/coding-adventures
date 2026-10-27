---
category: Cross-platform & Windows BUILD_windows
---

# POSIX native build fronts on Windows must account for the executable suffix

NN51's shared POSIX harness correctly compiled and ran MinGW binaries on
Windows, but Python's strict path resolver could not find the suffixless path
passed by the front. If that same trusted path plus .exe exists, prefer the
actual .exe path. MSYS `test -f` may itself resolve the suffixless executable,
so testing the suffixless path for absence first does not work. Keep native
compiler discovery and scheduling in the shared harness. Preserve its verbose
XOR receipt in a build log while the validator emits a compact proof summary.

A later manual coverage invocation guessed clang++-native-xor.exe, while this
LLVM driver had actually emitted clang++-native-xor without a suffix. Inspect
the produced artifacts or reuse the front's existence-based resolution, rather
than assuming all Windows compiler drivers choose the same filename suffix.
