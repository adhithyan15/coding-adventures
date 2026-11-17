## Unreleased — TRIG-REF-1 corpus consumers for every converged trig port (TRIG-CONV close-out)

- All seventeen `trig` ports have converged on TRIG-REF-1, so the corpus now selects
  every one of them. Added consumers of `code/specs/fixtures/trig-reference-v1/` for
  `ruby`, `haskell`, `java`, `kotlin`, `swift`, `csharp`, `fsharp`, `lua`, `perl`,
  `elixir` and `go` (each `<lang>/trig`; their conformance tests decode the TSVs), joining
  python, rust, typescript, c and cpp.
- `dart/trig` is also a corpus consumer, but its test imports the generated
  `test/generated/trig_reference_vectors.dart`. `code/scripts/generate_trig_reference_dart_fixtures.py`
  and `code/scripts/tests/test_trig_reference_dart_fixtures.py` join the python/trig
  inputs (`isTrigReferenceInput`): python/trig's BUILD runs them, and its `--check`
  catches a stale generated file, after which the Dart tests re-run against it.
- `TestTrigReferenceCorpusSelectsConvertedPorts` now covers all seventeen packages;
  the "go/trig has not adopted the corpus" assertion is removed.
- The `repository-source-input-boundary.json` fixture only lists package directories and
  does not mirror these consumers, so it and its digests are unchanged.
