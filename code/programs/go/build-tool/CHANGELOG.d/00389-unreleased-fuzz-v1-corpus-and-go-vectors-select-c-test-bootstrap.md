## Unreleased

- `fuzz-v1` corpus and Go's `fuzz/reference_vectors_test.go` now select `c/test-bootstrap` (VER06j): the C fuzz engine's `test_fuzz_corpus.c` reads `cases.json` and `test_fuzz_vectors.c` scans the Go vector table line by line. Two new consumer rows, a new selection test (`TestFuzzV1CorpusAndGoVectorsSelectTheCEngine`), and the existing fuzz-v1 and Go-vector selection tests gain the C package because the rows fail closed.
