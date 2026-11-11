# Changelog

## Unreleased

- Skip `package-metadata-selection-*.json` cases, whose options carry
  `metadata_policy: "package_local"`. That variant keeps edits to a
  package's root changelog, capability manifest and assurance evidence from
  reaching its dependents; this core does not model the policy, so it skips
  the variant exactly as it skips `trigger_relations` and still replays the
  unchanged twelve-case legacy diff roster. The skip covers all seven
  variant cases, including the platform BUILD-front and disallowed-type cases.

- Consume all 19 shared graph/diff fixtures, including empty and partial-cycle
  graphs, portable character-class globs, known near-BUILD paths, and exact
  recursive BUILD-front recognition.

## 0.1.0 - 2026-09-19

- Add an independent process-free native graph and diff-selection core.
- Consume all 14 shared graph/diff fixtures directly through Kotlin/JUnit.
- Enforce deterministic graph output, Unicode-scalar work limits, portable
  glob behavior, repository-boundary digest validation, and stable errors.
- Add warnings-as-errors compilation and a 95% JaCoCo line-coverage gate.
