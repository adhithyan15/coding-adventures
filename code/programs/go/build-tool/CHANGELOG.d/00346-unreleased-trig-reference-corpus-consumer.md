## Unreleased — TRIG-REF-1 conformance corpus inputs (PHY00a 6, TC-1)

- `python/trig` is selected by any file beneath
  `code/specs/fixtures/trig-reference-v1/` **and** by the files that generate and
  validate it: `code/scripts/generate_trig_reference_fixture.py`,
  `trig_ref_exact.py`, `trig_ref_softfloat.py`, `trig_reference_xcheck/` and
  `tests/test_trig_reference_fixture.py` (`isTrigReferenceInput`). The package's
  `BUILD` runs the generator's `--check` and the validator tests, so an edit to the
  generator or the fixture is checked by CI. (The package does not read the corpus
  itself until slice TC-2; until then this registration is what makes the
  generator's `--check` run at all.) Matches are slash-delimited prefixes or exact
  filenames, so a lookalike such as `trig-reference-v10` or `generate_...py.bak`
  selects nothing, and a selected consumer missing from discovery is a hard error.
- `TestTrigReferenceCorpusSelectsConvertedPorts` pins the selection, the lookalike
  paths, the generator files, the `-language` filter and the missing-consumer
  error. Each later slice (TC-2 to TC-17) adds its port's row to
  `repositoryInputConsumers` when that port adopts the corpus.
