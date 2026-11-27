# HOS-003b: Dart and Kotlin library ports

This item ports the HOS00 First Edition Unix teaching model to Dart and Kotlin.
Each package exposes reusable in-memory filesystem, descriptor-table, kernel,
and command-semantics components. It has no command-line entrypoint, fixture
file access, filesystem output, or other host I/O in its runtime library.

## Component contract

The two ports preserve the HOS00 path resolution, byte-range reads, descriptor
allocation and offsets, kernel error codes, and `cat`/`ls` trace semantics.
The canonical nine scenarios remain solely in
`fixtures/historical-unix-v1/file-read.json`. Package unit tests cover component
behavior without reading repository fixtures. An external adapter in the
fixture tree imports the package API and compares each canonical scenario with
the expected output and event sequence.

Dart builds and tests through the cached Dart SDK and its package-local
`.dart_tool` metadata. The external Dart adapter imports the library source by
relative path, so it adds no runtime dependency or duplicate fixture. The
Kotlin package is a Gradle Kotlin/JVM library. Its external fixture adapter is
compiled as test-only code by Gradle and exercises the same library API; JSON
parsing stays in the test adapter. Neither package ships a CLI executable.

## Validation boundary

The Linux `BUILD` and Windows `BUILD_windows` recipes run component tests and
the external nine-case fixture adapter. Repository-input routing names the
canonical fixture and each external adapter/project file actually read or
compiled by those recipes, plus the shared replay helper only for a language
that invokes it. It does not register `build_native.py` for Dart or Kotlin.

The model is a teaching contract, not a historical executable or syscall ABI.
No claim is made about historical binary compatibility or unimplemented Unix
features.
