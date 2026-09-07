# Changelog

## Unreleased

- Adopted the complete typed language and repository-boundary source-input
  registries, including deterministic generation, canonical digest checks,
  direct Starlark declared sources, exact pruning, and reverse boundary-diff
  selection.
- Replaced ambiguous source concatenation with repository-relative Hashing-v1
  path/content frames and exact raw-byte SHA-256 package digests while leaving
  dependency hashing to its separately owned contract.
- Added bounded native no-follow source reads, hardlink/non-regular rejection,
  constant-descriptor file and directory identity rechecks, scrubbed and
  bounded Git-index capture, complete mode/OID/stage/path stability comparison,
  redacted failures, all 13 source fixtures, all three package-digest fixtures,
  and live security tests. Removed unused public discovery helpers that bypassed
  the secure reader and made .NET changes select the native Windows CI leg.
- Consumed the complete shared discovery-language registry, restricted language
  inference to the exact bucket immediately below `packages` or `programs`,
  classified Mosaic and Twig, ignored BUILD roots outside those containers,
  and pruned exact case-sensitive `_build` and `dist-newstyle` components.
- Added direct shared-fixture and exact/case/near-name discovery regressions.
- Consumed the language-neutral extra-CI-toolchain declaration contract with a
  bounded process-free snapshot evaluator, exact selected-front parsing,
  canonical `cpp` and `ocaml` keys, C/C++ language normalization, stable
  unsupported-language diagnostics, and production affected-package scheduling.
- Added direct xUnit coverage for all ten neutral toolchain fixtures, real
  platform-front discovery, canonical language mappings, and per-file plus
  aggregate resource ceilings, including strict CRLF acceptance and lone-CR
  rejection.
- Kept forced-full neutral snapshots strict for unsupported selected languages
  while production forced-full scheduling provisions the complete registry
  without misclassifying repository-only special/fixture buckets; affected
  Starlark BUILD packages use the existing Go bootstrap.
- Added process-free orphan-crate snapshot validation with exact artifact
  exclusion, direct and ancestor BUILD coverage, invalid and stale exemption
  diagnostics, pending-debt accounting, hostile-path redaction, and direct
  coverage of all four shared language-neutral fixtures.
- Pinned tracked-artifact NFC, NFKC, full default folding, and root full
  uppercase to generated source-embedded Unicode 17.0.0 tables, with explicit
  snapshot-version validation and version-delta fixture coverage. Build,
  publish, and package outputs include the Unicode License v3 notice and
  declare the mixed MIT and Unicode-3.0 licensing.
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
