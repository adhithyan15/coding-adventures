## Unreleased — Kiln export covers the Warrant lane inputs

- **Fix: the Kiln graph export missed the shared Warrant lane artifacts.**
  `rust/warrant-grammar` and `rust/ca-assurance-report` select on
  `code/specs/ct-compare.warrant` and the `ct-compare-warrant-v1` fixtures
  (cases, corpus, consumer registry and schema), but neither row declared
  those paths, and `rust/ca-assurance-report` declared no paths at all. The
  export therefore omitted inputs the selector uses: a false-hit vector for
  Kiln legacy keys. Both rows now declare every path their predicate accepts
  (`warrantGrammarPaths` gains the lane files; new `warrantAssurancePaths`).
  `TestRepositoryInputPathsAgreeWithPredicates` and
  `TestRepositoryInputPredicatesAreCoveredByDeclaredInputs`, which were
  failing on `main`, pass again.
- **Changelog** converted from the monolithic `CHANGELOG.md` to
  `CHANGELOG.d/`, one file per former section, verbatim.
