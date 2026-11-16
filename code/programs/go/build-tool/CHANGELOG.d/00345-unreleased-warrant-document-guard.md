## Unreleased — Any .warrant document selects the grammar guard

- `rust/warrant-grammar` is now selected by any changed path matching
  `**/?*.warrant`, anywhere in the repository (`isWarrantDocument`, declared as
  `warrantDocumentGlob` so the Kiln export's `repo_input_globs` keys it on the
  same files). Its `v0_compat` test parses and checks every `.warrant` file, but
  it only ran when a PR happened to touch the package, so about twenty PRs
  (#19417-#19474) landed invalid documents unchecked and later PRs failed on
  files they never touched. `*.warrant.draft`, `.warrant.bak`, a bare
  `.warrant` and non-Git (backslash) paths do not select it.
- The selection uses the existing repository input consumer table: the guard is
  added with its prerequisites, not its dependents, and no other Warrant
  adopter is selected.
- `TestAnyWarrantDocumentSelectsOnlyTheGrammarGuard` pins the selection (a
  document under `code/specs/` and under a package directory), the draft and
  lookalike exclusions, the language filter and the missing-guard failure;
  `TestWarrantDocumentGlobAgreesWithPredicate` pins glob and predicate together.
  `TestWarrantLaneArtifactsSelectGrammarAndAssuranceConsumers` now expects a
  nested `ct-compare.warrant` lookalike path to select the grammar guard.
