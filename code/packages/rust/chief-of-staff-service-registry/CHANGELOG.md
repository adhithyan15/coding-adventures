# Changelog

## Unreleased

- Add revision-CAS package replacement for stopped host registrations.
- Reset cached process observation when package identity changes.

## 0.1.0

- Add bounded host names, portable package paths, restart policy, desired state,
  and last-observed process state.
- Add a strict versioned binary host-record codec and stable storage keys.
- Add create-if-absent registration, revision-CAS updates and deregistration,
  deterministic listings, and local-folder restart coverage.
- Add `RestartLedger` and `RestartWindow`, and take the ledger whole in
  `HostObservation::new` in place of the loose `restart_count` /
  `last_restart_ns` pair. Restart bookkeeping now travels as one value, so an
  observation cannot carry part of it and drop the rest.
- Record which daemon run opened a restart window, so a monotonic timestamp from
  a previous run is not compared against this run's clock.
- Bump the on-disk observation format to version 2. Version 1 records still
  decode, with the window closed.
