## Unreleased — Warrant runner conformance fixture inputs (VER04i 9.1, WD-100)

- `python/warrant-corpus` and `typescript/warrant-corpus` now also select on any
  file beneath `code/specs/fixtures/warrant-corpus-conformance-v1/`, because
  their tests run every case of that shared fixture. The ct-compare pilots are
  not selected by it (their BUILD runs the corpus, not the runner's own tests).
  The match is a slash-delimited prefix, so a lookalike directory selects
  nothing, and the fixture directory is listed in the consumers' declared
  paths for the Kiln graph export.
- `TestWarrantConformanceFixtureSelectsEveryRunnerPackage` pins the selection,
  the lookalike paths and the fail-closed missing-consumer case. Future lanes
  (Go, WD-103) add their runner package here when they adopt the fixture.
