# Changelog

## Unreleased

- Added `evaluateToolchainSnapshot`, a no-inline pure F# facade over the shared
  .NET toolchain-decision engine, and independently consumed all 11 neutral
  toolchain-detection fixtures through that symbol. The cases pin canonical
  keys, platform-front precedence, affected/null/full scheduling, declarations,
  CRLF-only carriage-return stripping, diagnostics, and resource ceilings
  without adding host or execution authority.
- Added a pure F# orphan-crate validation facade and independently consumed all
  four language-neutral orphan coverage and exemption-ledger fixtures through
  that F# boundary without adding filesystem, Git, process, environment, or
  network authority.
- Required the explicit Unicode 17.0.0 snapshot version at the F# facade and
  independently exercised the shared generated .NET normalization/casing
  substrate through the version-delta fixture. Build, publish, and package
  outputs include the Unicode License v3 notice and declare the mixed MIT and
  Unicode-3.0 licensing.
- Added the first F# build-tool program for the repo.
- Wired the F# entry point and smoke tests to the shared .NET build engine so
  the C# and F# programs stay behaviorally aligned.
- Added a pure F# tracked-artifact validation facade with independent coverage
  of all five shared language-neutral conformance fixtures.
- Extended the shared-fixture coverage to require redacted `EMPTY_SEGMENT`
  diagnostics for trailing slash and backslash paths.
