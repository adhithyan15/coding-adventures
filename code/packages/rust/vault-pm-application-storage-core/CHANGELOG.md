# Changelog

## Unreleased

- Added `StorageCoreApplicationStore::reclaim_orphaned_preparations`, for
  `VLT-PM05-application.md` §7.3 and backlog item #16 (VLT-PM41 §8). `init`
  and `vault create` both install a `PreparedInit` journal under a freshly
  drawn random locator *before* the caller writes the configuration record
  that makes that locator discoverable again, so a crash strictly between
  those two writes strands the journal forever under a locator nothing
  durable will ever name again — a storage leak, not a correctness or
  availability defect. Every later state a locator's record can be found in
  requires that configuration write to have already succeeded, so a record
  decoding as `PreparedInit` whose locator is absent from the caller's live
  set is provably this leak's orphan and every other state is provably never
  one; `reclaim_orphaned_preparations` deletes exactly and only those,
  compare-and-delete against the exact observed revision so a record that
  changes between the list and the delete is left alone. `init`'s fresh-vault
  path and `vault create`'s fresh-target path in `vault-pm-cli` both call it,
  under the same writer lock that already serializes them, immediately before
  installing their own new locator's journal.
- Implemented `BootstrapStore::supersede_generation` over the backend's
  idempotent `delete`, for `VLT-PM43-cli-passphrase-rotation.md` §5.4.
  Generations are installed under their own immutable per-ID keys, so a
  passphrase rotation that only advanced the latest pointer would leave the
  retired record — a wrapping of the *same, unchanged* vault root key under the
  *old* passphrase-derived key — readable on disk indefinitely. The delete is
  the part of a rotation that makes the rotation mean something.

  The generation the latest pointer names is refused outright with `Conflict`
  rather than trusted to caller discipline: that record is the only way into
  the vault. An already-absent record is success, because the rotation's
  recovery replays the call after a crash and must be able to reach the end.
  The removal is read back, so a delete that silently did not happen is a
  `Corruption`, not a success.

  **The wrap is destroyed by a write, not only by the unlink.** Every other
  durable step of a rotation is a write, and a lost write is merely lost work
  the journal replays; this one is a removal, and a lost removal resurrects key
  material into a vault whose owner state has already moved on and will never
  revisit it. `remove_file` returning success proves the entry is gone from the
  page cache, not that its removal is committed — on a journalling filesystem
  it can still be uncommitted while a later `fsync`ed write elsewhere lands
  ahead of it. So the retired record's body is first replaced with nothing
  through the same write-`fsync`-`rename` path every other step uses, and only
  then unlinked. After that write returns the wrap is gone whether or not the
  unlink survives a power cut.

## 0.1.0

- Added provider-neutral `storage-core` implementations of the VLT-PM05
  bootstrap and local owner-state store contracts.
- Preserved immutable bootstrap generations behind an atomic latest pointer.
- Added exact, bounded, read-back-verified local-state compare-and-exchange.
- Serialized application-store writes within the supported backend instance so
  restart-local revision tokens cannot admit an exact stale value.
- Added in-memory race coverage and filesystem restart coverage.
