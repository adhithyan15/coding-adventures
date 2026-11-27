## Unreleased — the pbt-v1 corpus selects haskell/test-bootstrap (VER05n, PT-HS-PBT)

- `repositoryInputConsumers` gains a `haskell/test-bootstrap` row beside the other pbt-v1 lanes, matched by
  `isPbtV1Corpus` over `pbtV1CorpusPaths` (`code/specs/fixtures/pbt-v1/`). The Haskell engine's conformance suite walks
  up to the repository root and reads `cases.json`, so an edit to any corpus file reruns its tests. Every existing row
  is kept.
- `TestPbtV1CorpusSelectsTheHaskellEngine` proves the corpus (JSON and TFV) selects the Haskell package, that the
  `haskell` language filter selects only it, that a corpus change with the package missing fails closed, that the Java
  filter is unaffected, and that an edit to the package's own sources is selected by the package, not by this row. The
  Java, C# and Go `all`-language selection tests now include the Haskell package in their package list and expected set,
  which they must (a missing consumer fails closed).
