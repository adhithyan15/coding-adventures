# Changelog

## Unreleased

### Changed

- The test broker binary is `chief_of_staff_agent_broker::process::run_inherited`,
  no longer a copy of its descriptor checks (P2.6d-2c).

### Fixed

- `broker-e2e-host` writes its result beside the result path and renames it
  into place. `fs::write` creates the file before filling it, so the tests,
  which poll for the result to exist, could read it empty: the race failed
  `a_reservation_left_by_a_dead_broker_is_abandoned_at_launch` on Ubuntu CI.

### Added

- End-to-end tests of D18S P2.6d-2b on Linux: the process supervisor, the
  real agent broker's serve loop, and a scripted host, as three processes.
  - A host's publish goes through its own broker into storage, as
    ciphertext at sequence 0.
  - Stopping the host ends its broker.
  - A killed broker ends its host.
  - A host that exits takes its broker with it.
  - A key file whose public half is not the channel definition's fails the
    launch before the host starts.
  - A reservation left pending by a previous broker is abandoned at launch.
  - A binding provider without durable bindings is refused.
- `broker-e2e-host`: a scripted host built on the real host runtime.
- `broker-e2e-broker`: `chief_of_staff_agent_broker::serve_process` with
  the same descriptor adoption as the production binary.
