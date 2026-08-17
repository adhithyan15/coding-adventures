# Changelog

## Unreleased

### Fixed

- Enforce D18T invariant 3, "all grants before visibility". Replay reloaded and
  byte-compared only the activation plan from public storage and trusted the
  record the backend echoed from each grant write -- the same trust boundary as
  the write itself. Against a write-behind or eventually-consistent backend,
  activation could advance E to E+1 while a receiver's E+1 grant was not
  retrievable, locking an authorized receiver out of the epoch. Replay now
  re-reads every grant, checks its envelope, and byte-compares, matching the
  Rust reference. A body mismatch here reports `corrupt_record`, not
  `conflicting_grant`: `_put_immutable` already covers a genuine slot conflict,
  so reaching this point means the backend contradicted its own acknowledged
  write.
- Validate the channel definition before importing the caller's CMK in
  `create_epoch_channel`. Custody slots are keyed by `(channel_id, epoch)` and
  the first writer wins permanently, so importing first let a caller presenting
  a mismatched definition claim an unclaimed slot and then fail -- leaving the
  legitimate import to hit `conflicting_active_key` forever. Fail-closed, but
  permanently wedged. D18T only requires custody before *state*.

Both defects were found by the security review of the Go port (#11894) and are
covered here by tests verified to fail when either fix is reverted.

## 0.1.0

- Add exact D18S v2 and D18T v1 codecs.
- Add injected atomic originator-key custody and durable epoch orchestration.
- Consume the canonical Rust-authored fixture manifest without regeneration.
