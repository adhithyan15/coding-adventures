---
category: CI & GitHub Actions
---

# Run go test with -count=1 after editing a data file the tests read, a cached ok can hide a real failure

I added a gate to `code/specs/data/ci-gates.json` and then ran
`go test ./internal/cigates ./internal/plan`. It printed `ok (cached)` and I
reported the gate tests as passing. They were not: `cigates` loads the real
registry by a path outside the files Go tracks for its test cache, so editing the
registry did not invalidate the cached result. The next merge from `main`
triggered a fresh run, and `TestPathShardedLanguageDiffsStaySelective` failed:
a corpus path glob on my gate fired for every corpus-only diff, and at about
10,000 changed paths it exceeded the matcher's work limit
(`CI_GATE_MATCH_LIMIT_EXCEEDED`). The test had been in the repo all along; I had
simply never actually run it against my change.

Fix: drop the path glob. The gate only has to answer "is a package that reads
the corpus being built", which its `packages` list already does, and a
corpus-only change affects no package. A guard test now records why the gate
has no corpus glob.

Do differently:

- After editing any data file a Go test reads (registries, fixtures, golden
  files), run `go test -count=1`. Treat `(cached)` as no evidence.
- When a claim in a PR description rests on a test result, re-run it fresh
  immediately before writing the claim.
- Read the existing tests around a registry before adding an entry: a test
  named for selectivity or budget limits is a design constraint, not trivia.
