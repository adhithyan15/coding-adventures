## Unreleased - the conformance kit, its v1 checker and the checker's tests select python/warrant-oracle (CV2-S5b, R23)

- `isWarrantOracleInput` now also matches `code/specs/fixtures/warrant-corpus-conformance-v1/` (slash-delimited
  prefix, through `isWarrantConformanceInput`), `code/scripts/warrant_corpus_conformance.py` and
  `code/scripts/tests/test_warrant_corpus_conformance.py` (exact files, `warrantOracleConformanceScripts`); the
  `python/warrant-oracle` row's `paths` list them. The oracle's BUILD runs `make_conformance.py --check` against the
  fixture, the v1 checker and its tests, none of which a CI line ran before. The fixture still selects the three lane
  runners; the script and its test select only the oracle.
- `TestWarrantConformanceFixtureSelectsEveryRunnerPackage` expects the oracle beside the three runners (and checks a
  `v2-*` file); new `TestWarrantConformanceKitInputsSelectTheOracle` pins the script and test file, with lookalike
  negatives (`.pyc`, `.py.bak`, `~`, a longer name, a path under `docs/`, `-v10`, `-v2`, the bare directory).
- Number 00378 is skipped: open PR #19733 reserves it.
