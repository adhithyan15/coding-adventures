# Changelog

## Unreleased

- Skip `package-metadata-selection-*.json` cases, whose options carry
  `metadata_policy: "package_local"`. That variant keeps edits to a
  package's root changelog, capability manifest and assurance evidence from
  reaching its dependents; this core does not model the policy, so it skips
  the variant exactly as it skips `trigger_relations` and still replays the
  unchanged twelve-case legacy diff roster. The skip covers all seven
  variant cases, including the platform BUILD-front and disallowed-type cases.

## 0.1.0 - 2026-09-19

- Add a process-free Dart dependency-graph and diff-selection core.
- Consume all eight graph and eleven diff-selection conformance fixtures.
- Add deterministic Unicode 17 normalization, full-fold, and uppercase tables
  generated from pinned Unicode Consortium sources.
