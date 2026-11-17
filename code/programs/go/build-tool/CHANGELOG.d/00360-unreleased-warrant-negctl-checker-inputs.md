## Unreleased — shared negative-control checker inputs (WD-HARN1c)

- `rust/warrant-grammar` gains two repository inputs: `code/scripts/warrant_negctl.py` (the shared
  sabotage checker that adopter BUILD lines call) and `code/scripts/tests/test_warrant_negctl.py`.
  Its BUILD and BUILD_windows now run those tests, so editing either file reruns them on Linux,
  macOS and Windows. The script is deliberately *not* an input of any adopter: a rule tweak selects
  `warrant-grammar` only, never lzw, wasm-leb128 or hash-functions.
- The `isWarrantGrammarInput` predicate and `warrantGrammarPaths` list agree (the Kiln coverage test
  `TestRepositoryInputPredicatesAreCoveredByDeclaredInputs` gets both paths in its corpus).
- `TestNegativeControlCheckerSelectsOnlyItsTestHome` proves both files select exactly one package;
  `TestWarrantGrammarInputsSelectOnlyTheirConsumer` gains the two paths and lookalikes (`.pyc`, `.bak`,
  a sibling name, a nested directory).
- `TestWarrantGrammarDeclaresPythonOnBothBuildFronts` pins `# needs-toolchain: python` on both of
  warrant-grammar's BUILD fronts, the same directive and test shape as `java-to-semantic-ir`.
- The `repository-source-input-boundary.json` fixture and its digest are unchanged. The fixture is a
  curated registry, not derived from `repository_inputs.go`: the sibling `warrant_corpus_oracle.py`
  is in it (boundary `warrant-ct-compare-corpus-inputs`), while `warrant_syntax_corpus.py` is an
  input here and is left out of it. The new script follows that `warrant_syntax_corpus.py`
  precedent and is left out too; the digest is unchanged because the fixture file is not edited.
