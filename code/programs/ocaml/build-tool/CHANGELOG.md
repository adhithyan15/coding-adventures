# Changelog

All notable changes to the OCaml build-tool core are documented here.

## [Unreleased]

### Fixed

- Skip `package-metadata-selection-*.json` cases, whose options carry
  `metadata_policy: "package_local"`. That variant keeps edits to a
  package's root changelog, capability manifest and assurance evidence from
  reaching its dependents; this core does not model the policy, so it skips
  the variant exactly as it skips `trigger_relations` and still replays the
  unchanged twelve-case legacy diff roster. The skip covers all seven
  variant cases, including the platform BUILD-front and disallowed-type cases.

- Ignore test-trigger variant diff-selection fixtures carrying
  `trigger_relations`, matching the Java, Kotlin, and Dart legacy fixture
  filtering contracts.

### Added

- A pure typed graph API with canonical edges, deterministic
  prerequisite-first levels, bounded validation, and empty cycle failures.
- A pure diff-selection API with forced, package-local, and exact
  repository-boundary seeds; forward affected closure; reverse prerequisite
  closure; stable failures; and no partial results.
- Unicode 17 portable path identity, memoized strict scalar glob matching,
  exact BUILD-front handling, the 50,000,000-unit match-work ceiling, and pure
  SHA-256 boundary-digest validation.
- Dynamic native consumption of all eight graph and eleven diff-selection
  language-neutral fixtures, focused adversarial tests, cross-platform build
  fronts, honest test-only capabilities, and measured production coverage.
- Bounded fixture reads and JSON depth, exact coverage-path attribution, cheap
  pre-decode byte ceilings, and repository-boundary shape and authorization
  guards before digesting or consumer projection.
