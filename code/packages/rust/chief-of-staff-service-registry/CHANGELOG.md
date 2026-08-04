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
