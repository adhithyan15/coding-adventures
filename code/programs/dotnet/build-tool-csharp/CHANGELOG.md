# Changelog

## Unreleased

- Added the first C# implementation of the monorepo build tool.
- Implemented package discovery, dependency resolution, git-diff change
  detection, cache fallback, plan emission, CI toolchain detection, and
  parallel execution.
- Added xUnit coverage for discovery, resolver, hashing, cache, executor, and
  plan behavior.
- Added pure tracked-artifact snapshot validation with portable-path safety,
  redacted invalid-path diagnostics, Unicode-aware `node_modules` alias
  detection, Unicode-scalar length and ordering, full-uppercase Windows
  reserved-basename matching, and direct coverage of all five shared
  language-neutral conformance fixtures.
- Fixed trailing slash and backslash paths to report redacted `EMPTY_SEGMENT`
  diagnostics after separator normalization.
