---
category: CI & GitHub Actions
---

# A later Windows GITHUB_PATH entry can shadow a verified compiler

Windows CI initially selected a compatible Visual Studio `clang-cl`, then a
later Swift setup step added its own compiler directory to `GITHUB_PATH`. The
subsequent C++ build resolved Swift's older `clang-cl` and the active MSVC STL
rejected it. The fix reuses the already selected Visual Studio compiler path
after Swift setup and checks its version against the active `yvals_core.h` rule.

When several setup steps contribute toolchains, verify the compiler that later
build steps will actually resolve after all path updates. Re-promote a verified
toolchain after later contributors instead of rerunning the full developer
environment bootstrap or assuming earlier PATH priority survives.
