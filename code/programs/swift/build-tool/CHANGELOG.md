# Changelog

## Unreleased

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
