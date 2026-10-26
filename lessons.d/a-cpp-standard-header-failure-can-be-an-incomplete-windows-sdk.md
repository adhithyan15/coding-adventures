---
category: Cross-platform & Windows BUILD_windows
---

# A Cpp standard-header failure can be an incomplete Windows SDK

NN50's actual MSVC build reached the installed compiler, then failed in MSVC
`yvals.h` because `crtdbg.h` was absent. Inspection of the installed Windows Kit
10.0.26100.0 found a UCRT directory but no such header. A previous C build does
not prove the larger C++ standard-library SDK surface is installed.

Classify this as local toolchain/bootstrap incompleteness until a complete SDK
can compile the package. Do not create a fake system header, weaken diagnostics
or claim MSVC validation from GCC/Clang results. Preserve the actual Windows
BUILD front so complete CI runners still exercise the real compiler.
