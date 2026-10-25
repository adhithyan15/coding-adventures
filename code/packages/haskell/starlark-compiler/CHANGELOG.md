# Changelog

## Unreleased — 2026-10-25

### Fixed

- Scoped Darwin Cabal execution to Command Line Tools so the linker matches
  GHC's hardcoded SDK on macos-26-arm64 image 20261013.0424.1. Added executable
  routing, environment-isolation and failure-propagation tests. Native macOS
  qualification remains pending fresh hosted CI.
- Enabled coverage in the named project-package stanza and removed the global
  BUILD flag that makes Cabal 3.10.3.0 reject internal-library dependencies.
  Kept library/test instrumentation, dependency bounds and provider APIs.
- Added independent byte-range/type, ordering, null-pair and depth-boundary
  rejection witnesses and strengthened retained-map/direct-lookup assertions.
- Reproduced the original planner failure, then passed 13 native examples,
  actual BUILD, package checks and warning-as-error compilation with GHC 9.4.8
  and Cabal 3.10.3.0. Production HPC remains 447/447 expressions and 61/61
  alternatives. Windows CI's explicit skip is not a native test result.

## 0.1.0 — 2026-10-24

### Added

- Added all 61 STP01 opcode constructors with exact bytes, names, and
  categories, including absence for all 195 unassigned bytes.
- Added immutable binary, comparison, augmented-assignment, and unary operator
  maps plus exact direct lookup helpers.
- Added a bounded dynamic reader for the 359-record neutral corpus, independent
  corruption witnesses, complete public-API tests, and production coverage.
- Declared an empty production capability set: the package is pure inert data
  and performs no compiler, VM, module, output, process, or network action.
