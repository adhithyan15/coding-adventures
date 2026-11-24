## Unreleased — the fuzz-v1 corpus selects go/test-bootstrap (VER06e, PT-GO-FUZZ)

- `repositoryInputConsumers` gains a second `go/test-bootstrap` row, matched by `isFuzzV1Corpus` over
  `fuzzV1CorpusPaths` (`code/specs/fixtures/fuzz-v1/`). The Go fuzz engine's `fuzz/corpus_test.go` walks up to the
  repository root and reads `cases.json`, so an edit to any corpus file (including the generated `cases.tfv`) reruns
  its tests.
- `TestFuzzV1CorpusSelectsTheGoEngine` proves the selection, the `go` language filter, fail-closed behaviour with the
  package missing, that neighbouring directories (`fuzz-v10`, `fuzz-v2`) and the Go package's own test files are not
  matched by this row, and that the pbt-v1 and fuzz-v1 predicates stay distinct.
