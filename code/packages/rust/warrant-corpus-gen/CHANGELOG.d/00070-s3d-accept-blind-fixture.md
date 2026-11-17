## 0.4.0 - S3d: `accept_blind`, whitespace-only profile lines, the committed ct-compare v2 fixture

### Added

- `corpus.profile` line `accept_blind <op> <fault>` (owner decision [D15], VER04r 16.8). A fault
  every candidate claim of which is blind passes generation when the profile names it; its control
  row is emitted with an empty `required_claims`, every candidate in `blind_claims` and no
  `slim_witness`, and its control id is recorded in `accepted_blind_faults` (sorted by bytes;
  until now always `[]`). `Profile::accept_blind` holds the lines.
- Refusals, all fail-closed (author refinements recorded in 16.8): an `accept_blind` line for a
  fault that has no `fault` line, a repeated line, or a fault that is not all-blind (a claim can
  see it) is a `profile` error; for a fault no claim calls the `fault` error stays.
- Tests: a synthetic all-blind fault accepted and refused, sorted output whatever the line order,
  and the committed `python/warrant-oracle/tests/fixtures/accept-blind-v2/` corpus regenerated
  byte for byte (the oracle's `verify-v2` accepts the same bytes: generator-oracle agreement).
- BUILD and BUILD_windows: `--check` against the committed
  `code/specs/fixtures/ct-compare-warrant-v2/` with that directory's `corpus.profile` and
  `evidence.bindings` (CV2-S3d). `corpus.json` records `generator` `{name, version}`, so a version
  bump of this crate fails the line until the fixture is regenerated and reviewed ([D13]); an edit
  of the hand-written profile fails through `profile_sha256` ([D14]).

### Fixed

- The profile reader skipped whitespace-only lines (`" "`, `"\t"`, `"\x0b"`); it now refuses them as
  the oracle's reader does (the S4 part 2 erratum, VER04r 16.7 item 3). Only an empty line is
  skipped.

### Changed

- The authored ct-compare profile moved from `tests/fixtures/ct-compare.corpus.profile` to
  `code/specs/fixtures/ct-compare-warrant-v2/corpus.profile`; the crate's tests read it there.
- Version 0.4.0: a new profile line is a feature, and the recorded generator version changes the
  committed fixture's bytes.
