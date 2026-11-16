## Unreleased — TRIG-REF-1 conformance corpus inputs (PHY00a 6, TC-1)

- `python/trig` is now also selected by any file beneath
  `code/specs/fixtures/trig-reference-v1/`, because its tests will decode every row of (from TC-2; registered now so the corpus never lands unwatched)
  that bit-exact corpus. The match is a slash-delimited prefix, so a lookalike
  directory such as `trig-reference-v10` selects nothing, and a selected
  consumer missing from discovery is a hard error.
- `TestTrigReferenceCorpusSelectsConvertedPorts` pins the selection, the
  lookalike paths, the `-language` filter and the missing-consumer error. Each
  later slice (TC-2 to TC-17) adds its port's row to `repositoryInputConsumers`
  when that port adopts the corpus.
