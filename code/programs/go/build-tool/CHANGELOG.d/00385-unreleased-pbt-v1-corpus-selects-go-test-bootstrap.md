## Unreleased — the pbt-v1 corpus selects go/test-bootstrap (VER05d, PT-GO-PBT)

- `repositoryInputConsumers` gains a `go/test-bootstrap` row beside `python/test-bootstrap`, matched by `isPbtV1Corpus`
  over `pbtV1CorpusPaths` (`code/specs/fixtures/pbt-v1/`). The Go engine's `pbt/corpus_test.go` walks up to the
  repository root and reads `cases.json`, so an edit to any corpus file reruns its tests.
- `TestPbtV1CorpusSelectsTheGoEngine` proves the corpus selects the Go package (and the Python one under `all`), that the
  `go` language filter selects only the Go package, that a corpus change with the package missing fails closed, and that
  an edit to the package's own test file is selected by the package, not by this row. The Python test now filters by
  `python` so it keeps asserting exactly its own consumer.
