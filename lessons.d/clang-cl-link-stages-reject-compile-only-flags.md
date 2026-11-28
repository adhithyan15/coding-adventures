---
category: Cross-platform & Windows BUILD_windows
---

# Clang-cl link stages reject compile-only flags

When invoking clang-cl to link precompiled objects under `/WX`, do not pass
compile-only C++ flags such as `/permissive-` or `/EHsc` on the link-only
command. Clang 19 reports them as unused arguments and treats those warnings as
errors. Apply strict language and warning flags while compiling each source
into a separate object, then link the objects with `/nologo`, `/Fe`, and linker
options only. Keep the MSVC/clang-cl link stage isolated from its compile
flags, and verify the driver command rather than assuming cl.exe and clang-cl
accept the same unused-argument behavior.
