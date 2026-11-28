# DB12d4 — Commit only under an admitted Ready checkpoint

Use the durable epoch to gate the existing sequential WAL
commit. A reset keeps its previous log reserved until Ready
has been published.
[Checkpoint selection](DB12d2d-go-wal-checkpoint-capture.md) · [WAL commit](DB12c2c3c-go-wal-commit.md)

Add the Go package `storage-wal-ready-commit`.

```go
type Result struct {
    Checkpoint storagewalcheckpointrecord.Record
    Commit     storagewalcommit.Result
}

func Commit(device blockdevicecontract.Device, log, data, checkpoint storagepagelayout.Layout, epoch, txID uint64, images []storagewalpreparecommit.PageImage) (Result, error)
```

Validate the requested epoch from one through
9,007,199,254,740,991, using `ErrInvalidEpoch`. Capture the
two checkpoint pages and preserve its first error. If the
selected checkpoint is Resetting, return `ErrCheckpointResetting`.
For a Ready checkpoint with a different epoch, return
`ErrCheckpointEpoch`. These refusals return zero Result and
issue no log or data writes or flushes.

For the requested Ready epoch, invoke the existing WAL commit
once with the unchanged transaction ID and image sequence.
Return the selected checkpoint and exact underlying commit
receipt. Preserve its error, including partial or ambiguously
acknowledged progress. The caller supplies a sequential interval
with no concurrent writers or unrelated pending writes.

Check fixed observation order, stored phases, epoch boundaries,
no effects on refusal, exact request ownership, and unchanged
commit/restart receipts on the actual memory and HDD backends.
Run both build lists, coverage, race, vet, guided inputs and
clean controls for admitting Resetting and admitting a mismatched
epoch.

## Local verification

The implementation is in `6c90cdd65ac`. Independent checks
matched 396 phase, epoch and checkpoint read decisions and
828 actual HDD restart histories. The caller compared exact
ordered effects, entire media and receipts from direct WAL
commit. Race, vet and emulated Linux/386 passed. Both command
lists passed at 100% statement coverage with 100,000 guided
inputs, module tidy and both clean mutation controls.
