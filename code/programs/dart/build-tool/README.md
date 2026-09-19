# Build Tool Core (Dart)

This package is the first bounded Dart tranche of the language-neutral
[build-tool conformance contract](../../../specs/build-tool-conformance.md). It
implements only the pure dependency-graph and diff-selection domains.

The production API accepts caller-provided values and returns deterministic
values. It has no filesystem, Git, process, environment, clock, randomness,
credential, or network authority. JSON and repository fixture discovery are
test-only.

## Contract surface

- `BuildToolCore.evaluateGraph` canonicalizes prerequisite-to-dependent edges,
  computes deterministic prerequisite-first levels, and reports `GRAPH_CYCLE`
  without leaking partial results.
- `BuildToolCore.evaluateDiffSelection` performs package-prefix, portable-glob,
  exact BUILD-front, forced-package, and repository-boundary selection before
  computing affected and prerequisite closures.
- Match work is completely preflighted over Unicode scalar values with the
  shared 50,000,000-unit ceiling. Stable domain failures include
  `DIFF_UNKNOWN_PATH` and `DIFF_MATCH_LIMIT_EXCEEDED`.
- Repository-boundary pins use the shared domain-separated canonical digest.
- Path normalization, package-root identity, and reserved-name checks use a
  source-embedded, generator-owned Unicode 17 runtime rather than host SDK
  tables. Ordering compares Dart runes as numeric Unicode scalar values.

The native Dart suite discovers and evaluates exactly all eight graph and
eleven diff-selection fixtures. This package is not a CLI, conformance adapter,
executor, or complete Dart build-tool implementation.

## Validation

```sh
dart pub get
dart format --output=none --set-exit-if-changed lib test
dart analyze --fatal-infos
dart run coverage:test_with_coverage --branch-coverage --function-coverage --fail-under=90
```
