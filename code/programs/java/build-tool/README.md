# Build Tool Core (Java)

This package is the first bounded Java tranche of the language-neutral
[build-tool conformance contract](../../../specs/build-tool-conformance.md). It
implements only the pure dependency-graph and diff-selection domains.

The production API accepts immutable values and returns deterministic values.
It has no filesystem, Git, process, environment, clock, randomness, credential,
or network authority. JSON and repository fixture discovery are test-only.

## Contract surface

- `BuildToolCore.evaluateGraph` canonicalizes prerequisite-to-dependent edges,
  computes deterministic prerequisite-first levels, and reports `GRAPH_CYCLE`
  without leaking partial results.
- `BuildToolCore.evaluateDiffSelection` performs package-prefix, portable-glob,
  exact BUILD-front, forced-package, and repository-boundary selection before
  computing affected and prerequisite closures.
- Match work is preflighted over Unicode scalar values with the shared
  50,000,000-unit ceiling. Stable domain failures are
  `DIFF_UNKNOWN_PATH` and `DIFF_MATCH_LIMIT_EXCEEDED`.
- Repository-boundary pins use the shared domain-separated canonical digest.

The native JUnit suite discovers and evaluates all eight graph and eleven
diff-selection fixtures directly. This package is not yet a CLI, conformance
adapter, executor, or complete Java build-tool implementation.

## Validation

```sh
gradle --no-daemon --no-build-cache --max-workers=1 clean check
```

The build treats compiler warnings as errors and enforces at least 95% line
coverage with JaCoCo.
