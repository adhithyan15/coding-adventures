# HOS-003d: OCaml and Swift library ports

This spec reconstructs the HOS-003d language lane from its queued contract and
the merged HOS00/HOS-003c implementations. The earlier local implementation
objects were lost when the execution workspace was replaced, so this document
does not claim to reproduce that unpublished source byte for byte.

## Purpose and boundary

Add reusable OCaml and Swift ports of the HOS00 First Edition Unix teaching
model. Each package provides in-memory filesystem, descriptor-table, and kernel
components. A caller can use those components without loading fixtures, reading
host files, printing output, or launching a command-line program. The package
profiles therefore declare no host capabilities.

The packages are semantic teaching models. They do not execute historical
machine code and do not claim a historical syscall ABI, binary compatibility,
or behavior beyond the shared model contract.

## Shared behavior

The two implementations follow HOS00 path lookup and byte-range read rules,
including `ENOENT`, `ENOTDIR`, `EISDIR`, invalid NUL paths, repeated slashes,
`.` and `..`, trailing-slash validation, ordered directory entries, and
short/empty reads. The kernel opens the lowest free descriptor, tracks an
independent byte offset per open descriptor, advances offsets only by bytes
returned, and frees only the closed slot. Failed operations leave component
state unchanged.

Path traversal checks that the current node is a directory before processing
each component, including `.` and `..`. A dot component leaves a directory
unchanged, but `/file/.` fails with `ENOTDIR` because traversal has already
reached a regular file. Dot-dot from root remains at root.

Descriptor-table capacity is explicit and validated: negative capacities are
rejected as invalid input; zero capacity is valid and simply cannot allocate a
descriptor. Constructors must not silently clamp a negative capacity to zero.
Tests pin both cases and a positive-capacity allocation/close/reuse transition.

The canonical parity contract is the nine cases in
`fixtures/historical-unix-v1/file-read.json`. That file remains the only source
of expected commands, output, and ordered events. Package component tests use
in-memory values and do not open the fixture at runtime. They should assert
failures rather than print a package-level success message.

## Fixture replay boundary

Fixture loading and JSON serialization belong to test-only adapters under
`fixtures/historical-unix-v1/test-adapters/{ocaml,swift}`. The adapters call the
reusable package APIs and are built only by the package BUILD recipes. They do
not become package executables, public CLI entry points, or capability-bearing
runtime code. The shared `replay_cli.py` drives one canonical command at a time
and compares each adapter's JSON output and event sequence with the fixture.

OCaml uses Dune for the library and component tests. The fixture adapter is a
separate Dune project or build target whose files remain outside the package
root; any adapter-only project and source files are explicit build-tool inputs.
Swift uses a SwiftPM library product and package-local XCTest component tests.
Its fixture adapter is a separate fixture-adjacent SwiftPM test/build project
that imports the library through a local package dependency. Swift `.build/`
and `.swiftpm/` directories are ignored before the first Swift command. Build
artifacts and adapter state stay in temporary build paths, not in the package.

## Repository routing and builds

Each package has Unix `BUILD` and Windows `BUILD_windows` fronts. Every recipe
line is an independent shell command launched from that package's working
directory. Paths in the recipes must be relative to that directory. Windows
commands use `cmd /C` syntax; no POSIX redirection, shell tests, or path
assumptions are allowed. The same component checks and nine-case replay run on
both fronts when the corresponding runtime is available.

The current Linux/macOS workflow pins OCaml 5.2.1, opam 2.5.2 and Dune 3.17.2,
and Swift 6.3.3. Windows CI uses its reviewed Swift 6.4.2 toolchain. Local
validation must report actual compiler versions and must not treat a different
local runtime as proof of the CI-pinned run.

The Go build-tool external-input registry selects OCaml and Swift only for the
canonical fixture, shared replay helper, shared build/replay driver where
used, and the exact adapter project/source files consumed by that language.
Selectors and package hash inputs use the same explicit path list. Unrelated
fixtures, directory prefixes, and other language consumers do not select these
packages. Rename and deletion behavior remains covered by the registry tests.

## Evidence and limits

Record exact runtime/compiler versions, package-CWD BUILD commands, component
checks, and nine-case replay results. Local Linux results must be distinguished
from CI-pinned compiler versions and from unexecuted Windows recipes. A static
Windows review is not a Windows test. Do not mark this lane complete until the
ordered hosted checks pass and both adapters have demonstrated all nine cases.
