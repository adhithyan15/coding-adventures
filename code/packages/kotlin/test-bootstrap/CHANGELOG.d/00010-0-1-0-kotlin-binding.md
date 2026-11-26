## 0.1.0

- Initial Kotlin `test-bootstrap` package (spec VER05f, PT-KT-PBT): an idiomatic
  binding over the Java engine of VER05e. No second engine (WD-901).
- `Gen<T>` wrapper with `map`, `filter`, `bind`/`flatMap`, `oneOf`, `frequency`,
  `list`, `tuple`; primitives with `UByte`/`ULong`; `Options`, `CiMode`, sealed
  `Verdict`, `checkProperty`/`forAll`, `Shrunk`, `shrinkInt/List/Bytes`,
  `simulateReplay`, `gen*` samplers, `FailureId`, `subSeed`.
- Tests replay all 25 pbt-v1 cases through a Kotlin corpus runner that calls only
  the Kotlin API (exact outputs, minimals and step counts), per-case negative
  controls, dropped/truncated-document controls, and a Kotlin-layer sabotage seam
  proving the corpus exercises the Kotlin wrapper. JaCoCo gate: 95% lines.
