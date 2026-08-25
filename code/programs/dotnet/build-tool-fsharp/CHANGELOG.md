# Changelog

## Unreleased

- Added the first F# build-tool program for the repo.
- Wired the F# entry point and smoke tests to the shared .NET build engine so
  the C# and F# programs stay behaviorally aligned.
- Added a pure F# tracked-artifact validation facade with independent coverage
  of all five shared language-neutral conformance fixtures.
- Extended the shared-fixture coverage to require redacted `EMPTY_SEGMENT`
  diagnostics for trailing slash and backslash paths.
