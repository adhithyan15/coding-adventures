# Changelog

All notable changes to this package will be documented in this file.

## Unreleased

### Fixed

- Validate the channel definition before importing the caller's CMK in
  `createEpochChannel`. Custody slots are keyed by `(channelId, epoch)` and the
  first writer wins permanently, so importing first let a caller presenting a
  mismatched definition claim an unclaimed slot and then fail -- leaving the
  legitimate import to hit `conflicting_active_key` forever. Fail closed, but
  permanently unrecoverable. D18T only requires custody before *state*, and the
  D18S write still follows the import. A `try`/`finally` with a `consumed` flag
  keeps the CMK erased exactly once on every path.

  Found by the security review of the Go port (#11894) and fixed there and in
  Python (#11898), Ruby (#11928), and Elixir (#11935); TypeScript was the last
  carrier.

## [0.1.0] - 2026-08-14

### Added

- Exact D18S v2 and D18T v1 codecs, keys, content types, and validation.
- Atomic originator-key custody interface with redacted handles and non-durable
  deterministic test custody.
- State migration, custody-first preparation, immutable public replay,
  activation, current-epoch publication, abandonment, and destruction flows.
- Canonical fixture, crash-recovery, race, corruption, exhaustion, and bounded
  CAS conformance tests.
