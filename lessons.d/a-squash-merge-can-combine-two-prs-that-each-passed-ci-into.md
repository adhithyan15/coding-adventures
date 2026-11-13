---
category: CI & GitHub Actions
---

# A squash-merge can combine two PRs that each passed CI into a main that fails a coverage test

**What went wrong.** #19249 added `-emit-kiln-graph` to the Go build tool, plus
a test that every file a repository-input predicate accepts is covered by a
declared path or glob in that row. #19236 had just extended the
`rust/warrant-grammar` and `rust/ca-assurance-report` predicates to the shared
Warrant lane files (`code/specs/ct-compare.warrant`, the
`ct-compare-warrant-v1` fixtures) without declaring paths. That was fine at the
time, because no test checked declarations yet. Each PR was green on its own
branch. After both merged, `main` failed
`TestRepositoryInputPredicatesAreCoveredByDeclaredInputs`, and nobody noticed
until a later build-tool PR ran the suite locally. The gap was not cosmetic:
the Kiln export omitted inputs the selector uses, which is a false-cache-hit
vector for legacy keys.

**Fix.** Declare the Warrant lane paths on both rows (`warrantGrammarPaths`,
new `warrantAssurancePaths`).

**Do differently.**
- When a PR adds a test that constrains a *table* other PRs also edit
  (`repositoryInputConsumers`, `external_inputs.go`), merge `origin/main` and
  re-run that test immediately before the merge goes in, not only when the
  branch was cut.
- When a test fails locally on an unrelated change, check it on a clean
  `origin/main` right away. A failure already on `main` is a bug to fix now,
  not noise to skip.
