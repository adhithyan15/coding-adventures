---
category: TypeScript / JavaScript
---

# Every build-script TypeScript project changes the pinned tsconfig portability census

The VER07c `rank-statistics` and VER07f `microbenchmark` packages each have a
build script and standalone emitting `tsconfig.json`. CI's strict repository
portability inventory therefore gained two projects and two isolated emitters;
`microbenchmark` also uses Node APIs in compiler inputs, so it adds one Node API
project and provider. Both packages add a compiler lock. Package-local checks
did not exercise the repository-wide pinned census, which failed against the
new tree.

When adding a build-script TypeScript package, run
`code/scripts/tests/test_typescript_tsconfig_portability.py` against the full
tracked repository and update only the exact aggregate assertions and their
reason comments for changes the audit confirms. Keep the zero-gap portability
assertions strict. In a sparse worktree, materialize the tracked manifests,
configs, locks, and TypeScript sources before trusting the aggregate counts.

The package configs and Node dependency ownership were checked against
`VER07c-rank-statistics.md` and `VER07f-typescript-microbenchmarks.md`; this
inventory repair does not change either package's specified behavior.
