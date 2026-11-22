## Unreleased - an edit to the conformance fixture selects go/warrant-corpus (CV2-S5-1, R23)

- The `go/warrant-corpus` row of `repositoryInputConsumers` now also matches
  `code/specs/fixtures/warrant-corpus-conformance-v1/` (new
  `isWarrantGoRunnerInput`, and `warrantConformanceFixture` added to
  `warrantGoLaneCorpusPaths`), exactly as the Python and TypeScript runner rows
  already did. The Go runner's `conformance_test.go` reads that fixture, so
  before this an edit to it did not run the Go lane.
- `TestWarrantConformanceFixtureSelectsEveryRunnerPackage` now expects all three
  runners (and only them), and a missing Go runner package fails closed.
- Number 00376 is skipped: open PR #19718 reserves it.
