# DB-E163 — Publish Go checkpoint advancement and restart

Complete the sequential WAL lifecycle with three packages. Advance
and resume a checkpoint reset, admit a commit only in the selected
Ready epoch, and reopen according to the durable checkpoint phase.
[Advance](DB12d3c-go-wal-checkpoint-advance.md) · [Ready commit](DB12d4-go-wal-ready-commit.md) · [Reopen](DB12d5-go-wal-reopen.md)

Reconcile the release with the merged raw capture, initialization and
clearing packages. Use the admitted package trees at `ea49a8b0d54`,
`6c90cdd65ac` and `482a9601305`, with their own specifications first.
Retain the exact lower receipts and compare the package dependency
closure against the actual fetched main. The initial archive exposed
a compile-time dependency on `RecoverWithBoundary`, which the merged
recovery package does not yet export. Apply the admitted DB-E164
change from `6bddc3ff7dd` and its boundary spec before these three
operations. Keep the original `Recover` API and exact receipt tests.
[Recovery boundary](DB12c2c2a-go-wal-recovery-boundary.md)

Run both build lists, race, vet, tidy, guided inputs, clean mutation
controls, and the exact metadata checks from the release tree. Recheck
the independent physical HDD histories through both durable phases,
including commit eligibility, repeated cuts and write-free retries.
Verify selected native jobs and both required named gates before
conditional auto-merge. After merge compare the reviewed paths and
confirm the changed package bytes on the actual remote main.
