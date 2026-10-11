---
category: Cross-platform & Windows BUILD_windows
---

# A Windows Qt MSVC kit cannot be linked with Strawberry MinGW CMake defaults

Qt installations on Windows are compiler-specific. A Qt kit under an `msvc*`
directory can compile generated QML with another compiler far enough to create
object files, but linking it with Strawberry Perl's MinGW toolchain produces a
large wall of unresolved MSVC-runtime and Qt symbols. Inspect the Qt kit name
and available compiler first. Use an MSVC developer environment for an MSVC Qt
kit, or an explicitly matching MinGW Qt kit; do not treat an ABI-mismatch link
failure as an application source failure.
