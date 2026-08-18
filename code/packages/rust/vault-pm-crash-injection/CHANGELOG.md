# Changelog

All notable changes to this package are documented here.

## [0.1.0] - 2026-08-18

### Added

- `DurableStep`, the closed ledger vocabulary for every kind of durable write a
  local `vault-pm` host performs: `storage.initialize`, `storage.put`,
  `storage.delete`, `storage.lease`, `config.create`, `config.replace`, and
  `export.artifact`. Ledger lines are built only from these names, so no caller
  can smuggle vault content into a trace by choosing an interesting label.
- `Phase` and `ledger_line`, fixing the `ordinal \t phase \t step` wire format
  a drill parses. Nothing else may ever appear in it — no key, namespace,
  object identifier, path, title, ciphertext, or length.
- `record`, which consumes the next process-global ordinal, appends it to the
  ledger when `VAULT_PM_CRASH_TRACE` names one, and removes the process with
  `SIGKILL` when `VAULT_PM_CRASH_AT` names that ordinal. A `VAULT_PM_CRASH_AT`
  that is not a positive decimal integer panics rather than silently disabling
  injection, so a typo cannot turn a crash drill into an ordinary run.
- `around`, which brackets one durable write in its two landing points so the
  "before" and "after" ordinals of a write are reliably adjacent and a sweep
  can read an ordinal's parity as "did this write happen".
- `CrashInjectingStorageBackend<B>`, a `storage-core` `StorageBackend`
  decorator that brackets `initialize`, `put`, `delete`, and `acquire_lease`.
  Reads pass through and consume no ordinal, because a crash during a read
  changes nothing on disk and collapses into the "before" point of the next
  write.
- An owner-only, append-only ledger file. A ledger that cannot be written
  panics rather than degrading, since a silently short ledger would make a
  sweep believe a ceremony performs fewer durable writes than it does and the
  missing landing points would never be tested.
- Untrusted-path handling for the ledger. The path must be absolute so a
  working directory cannot redirect it, is opened with `O_NOFOLLOW` so a
  symlink at the final component is refused rather than followed, is created
  `0600`, and is refused outright if it already exists as anything other than a
  regular file this user owns privately - a creation mode says nothing about a
  file that is already there.
- `O_NONBLOCK` alongside `O_NOFOLLOW`. A symlink at the final component is
  refused by the first flag; a reader-less FIFO is refused by the second.
  Without it, opening a FIFO for writing blocks forever and the "not a regular
  file" check is unreachable, because the open never returns to run it.
- A `kill(2)` fallback in both directions. A sandbox that filters the syscall
  must not turn a crash drill into a process that spins forever, so a failed
  `kill` falls back to `abort`; and a `kill` that *succeeds* but whose signal
  the kernel discards - which is what happens to a process that is PID 1 in
  its own namespace, since `SIGKILL` cannot have a handler - now falls back to
  `abort` after a bounded wait rather than parking forever.
- Twelve unit tests covering vocabulary distinctness, the ledger format,
  owner-only append, refusal of relative, symlinked, FIFO, and world-readable
  ledger paths, ordinal monotonicity, bracketing order, write wrapping versus read
  pass-through, the wrapper accessors, and the production-shaped path where no
  policy is configured.
