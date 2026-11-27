# Windows mixed-toolchain CI

Windows CI establishes the MSVC developer environment before installing Swift.
The selected MSVC `cl.exe` and `link.exe` and the compatible `clang-cl.exe`
must retain their PATH precedence when Swift is also required. Swift setup
must append newly discovered directories to the existing process PATH and
persist that same ordering through `GITHUB_ENV`. It must preserve the Swift
executable discovery, pinned toolchain version, and SDK setup.

The ISO C/C++ harness continues to exercise both MSVC and Clang with its
existing strict conformance flags. A mixed-toolchain job must verify the
path-ordering contract in addition to running the package builds.

The Kiln working-tree verifier's exhausted-budget regression test must use an
already-expired deadline so the failure assertion is independent of platform
timer resolution. The production working-tree verification budget and error
handling are unchanged.
