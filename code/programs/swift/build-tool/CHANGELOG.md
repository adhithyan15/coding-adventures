# Changelog

## Unreleased

- Add a pure orphan-crate snapshot validator consuming all four shared neutral
  fixtures. It enforces direct and ancestor BUILD coverage, independent empty
  BUILD reporting, exact artifact exclusions, portable and reasoned exemption
  records, NFC full-fold duplicate identities, stale-entry precedence,
  hostile-path redaction, pending counts, and deterministic Unicode-scalar plus
  Python-compatible ASCII JSON diagnostic ordering without filesystem,
  process, environment, credential, link, or network authority.
- Add a pure tracked-artifact snapshot validator consuming all five shared
  neutral fixtures, with hostile-path redaction, exact problem precedence,
  Unicode-scalar length and ordering, inert entry metadata, and exact
  Unicode 17 alias plus Windows-reserved-basename behavior.
- Add generated, source-embedded Unicode 17.0.0 NFC, NFKC, full-fold,
  NFKC-fold, and full-uppercase tables, the Unicode License v3 notice, and a
  required isolated Swift 6.3.3 official-vector CI check that preserves the
  driver-sensitive `swift` and `swiftc` entrypoint names and limits POSIX
  linker discovery to root-owned system tool directories.
- Exclude Cabal's exact, case-sensitive `dist-newstyle` generated directory
  from package discovery and source hashing while preserving near-names such
  as `dist-newstyle-example`. Shared-fixture and focused Swift tests cover the
  boundary.
- Classify package and program buckets with the canonical discovery language
  registry, exclude specification fixture trees, preserve program identities,
  and fail closed on duplicate qualified names with
  `DUPLICATE_PACKAGE_IDENTITY`, sorted repository-relative paths, and CLI exit
  code `2`. Shared registry and duplicate-identity fixtures now cover the API
  and real executable.
- Decode Lua `.rockspec` metadata as strict UTF-8 before dependency parsing.
  Invalid bytes now fail closed with `METADATA_INVALID_UTF8`, stable package
  and repository-relative manifest identity, CLI exit code `2`, and no
  checkout-path disclosure. Resolver and CLI coverage consume the shared
  positive and invalid-byte conformance fixtures and require the exact
  expected edge set.

## 0.1.0

- Added a full Swift port of the monorepo build tool.
- Implemented package discovery, dependency graph resolution, hashing, cache persistence, parallel execution, plan IO, and CI validation.
- Added lightweight Starlark BUILD parsing for the repo's declarative BUILD rules.
- Added focused Swift tests covering discovery, Swift dependency parsing, plan round-tripping, Starlark parsing, and CI validator behavior.
