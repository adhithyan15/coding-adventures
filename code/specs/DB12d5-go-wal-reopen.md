# DB12d5 — Reopen a WAL reservation after restart

Choose the startup procedure from the durable checkpoint. A Ready
epoch uses its WAL for redo. A Resetting epoch finishes the already
authorized reset and publishes Ready for the same epoch.
[Checkpoint advance](DB12d3c-go-wal-checkpoint-advance.md) · [Recovery boundary](DB12c2c2a-go-wal-recovery-boundary.md)

Add the Go package `storage-wal-reopen`.

```go
type Result struct {
    Observed storagewalcheckpointrecord.Record
    Ready    storagewalcheckpointrecord.Record
    Replay   storagewalrecovery.Result
    Resume   storagewalcheckpointadvance.Report
}

func Reopen(device blockdevicecontract.Device, log, data, checkpoint storagepagelayout.Layout) (Result, error)
```

Capture the checkpoint pages first. A capture refusal returns a zero
Result and the exact lower error, with zero writes or flushes. The
caller uses the existing initialization operation separately to
prepare vacant media before reopening.

When the selected record is Ready, use RecoverWithBoundary. Retain the
selected record and exact replay result. A successful replay sets Ready
to the observed record. Preserve the replay error and leave Ready zero
on refusal. An empty log succeeds with the observed epoch.

When the selected record is Resetting, call Advance with its own epoch
and retain its exact report and error. Success sets Ready to the Ready
record with that epoch and the observed previous-epoch rank. A failed
resume leaves Ready zero. The caller may restart and call Reopen again.
The resume path clears the previous log under the already durable reset
authorization. Its full effect and acknowledgement rules come from
Advance. Both paths use the same device and disjoint reservations.
The caller supplies an exclusive restart interval and stable geometry.

Use the actual sparse memory and HDD adapters. Cross complete and
incomplete log transactions, occupied and empty slots, both checkpoint
slot orientations, actual cuts during replay and clear, then repeat the
same reopen. Check exact typed errors, whole-media recovery and the
epoch admitted for the next commit. Run both build lists, coverage,
race, vet, guided inputs and clean controls for skipping Ready redo and
replaying a Resetting epoch's old log.

## Local verification

The implementation is in `482a9601305`. An independent model
matched 60 Ready and 1,044 Resetting HDD restart histories,
including repeated reopening after every modeled physical
cut. It compared all device bytes, preserved prior data during
Resetting, and separately checked six exact checkpoint refusals
with zero writes and flushes.

Both command lists passed at 94.1% statement coverage with race,
vet and tidy. The remaining defensive phase branch is outside
the admitted checkpoint record vocabulary. The package passed
100,000 guided inputs, both clean mutation controls and emulated
Linux/386.
