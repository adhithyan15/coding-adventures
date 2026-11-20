## [0.1.0] - UNRELEASED

### Changed (host-roots and env policy: two /proc reads and GOWORK, KILN04 D38)

- `code/kiln/legacy/host-roots.json`: `ephemeral` gains the exact names
  `/proc/sys/vm/overcommit_memory` and `/proc/version_signature`.
- `code/kiln/legacy/env.json`: `GOWORK` is a keyed variable. The census
  recording step runs `kiln record prepare` with `GOWORK=off`, so Go's upward
  go.work search no longer probes `/go.work` and `/home/go.work`.
- Unchanged by design: sibling `/proc/sys/vm/*` files, `/proc/version` and
  names beneath the two files stay `proc-path`; a write open of either file
  leaves the write set unknown (`trace-incomplete:writes-unknown`).
- No `RecorderVersion` bump: both policy files are consumed inputs, as D27 and D37.

### Added

- `internal/traced/procsys_test.go` (real policy file) and
  `internal/legacy/realenv_test.go` (real `env.json` keys `GOWORK`).
