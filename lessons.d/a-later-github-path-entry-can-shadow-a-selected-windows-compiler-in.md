---
category: Cross-platform & Windows BUILD_windows
---

# A later GITHUB_PATH entry can shadow a selected Windows compiler in every later step

The MSVC bootstrap selected the newest `clang-cl` and exported the resulting
PATH. A later Swift setup wrote its directories to `GITHUB_PATH`; GitHub
prepends those entries in later steps, so Swift's LLVM 21 unexpectedly shadowed
Visual Studio's LLVM 22 during the general Windows build. Keep later toolchain
directories at the end of the existing PATH and export that ordered PATH via
`GITHUB_ENV`. When a later setup step must use `GITHUB_PATH`, rerun compiler
selection after it and preserve the verified MSVC `cl.exe` and `link.exe`
directory ahead of the selected `clang-cl` directory.
