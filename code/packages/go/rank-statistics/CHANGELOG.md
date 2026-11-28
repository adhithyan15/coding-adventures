# Changelog

## 0.1.0

- Add reusable sample median and exact distribution-free median interval.
- Validate against the shared VER07b fixture, exhaustive bounded oracle, and Go fuzzing.
- Embed the shared fixture in the test binary; both BUILD fronts compare its package-local copy with the canonical repository fixture before tests, keeping test-time filesystem access out of the pure package. Direct `go test` uses the embedded snapshot and does not check canonical-fixture drift; use BUILD for that repository-level check.
