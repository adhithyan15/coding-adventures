## Unreleased — the fuzz-v1 corpus selects java/test-bootstrap (VER06f, PT-JAVA-FUZZ)

- `repositoryInputConsumers` gains a second `java/test-bootstrap` row beside the pbt-v1 one, matched by the shared
  `isFuzzV1Corpus` over `fuzzV1CorpusPaths` (`code/specs/fixtures/fuzz-v1/`). The Java fuzz engine's corpus tests
  walk up to the repository root and read `cases.json`, so an edit to any corpus file (including the generated
  `cases.tfv`) reruns its tests.
- `TestFuzzV1CorpusSelectsTheJavaEngine` proves the selection, the `java` language filter, fail-closed behaviour
  with the package missing, that neighbouring directories and the Java package's own files are not matched by
  this row, and that the pbt-v1 row still selects the package. The existing fuzz-v1 Python and Go tests list the
  Java package too, because a corpus consumer missing from the package set now fails closed.
- A third `java/test-bootstrap` row, matched by the exact-path `isGoFuzzReferenceVectors`, selects the Java package
  when `code/packages/go/test-bootstrap/fuzz/reference_vectors_test.go` changes: the Java `ReferenceVectorsTest`
  parses that Go source file directly. `TestGoFuzzReferenceVectorsSelectTheJavaEngine` covers selection, fail-closed
  behaviour and that neighbouring files do not match.
