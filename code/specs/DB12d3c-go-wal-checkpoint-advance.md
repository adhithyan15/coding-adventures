# DB12d3c — Advance or resume one explicit checkpoint epoch

Compose replay, guarded publication and clearing into a restart
operation. An explicit target epoch makes retry after a lost
final acknowledgement observe the same checkpoint.
[Recovery boundary](DB12c2c2a-go-wal-recovery-boundary.md) · [Checkpoint publication](DB12d2e-go-wal-checkpoint-publication.md) · [WAL clearing](DB12d3b-go-wal-checkpoint-clear.md)

Add the Go package `storage-wal-checkpoint-advance`.

```go
type Report struct {
    Observed  storagewalcheckpointrecord.Record
    Replay    storagewalrecovery.Result
    Resetting storagewalcheckpointpublication.Receipt
    Clear     storagewalcheckpointclear.Receipt
    Ready     storagewalcheckpointpublication.Receipt
}

func Advance(device blockdevicecontract.Device, log, data, checkpoint storagepagelayout.Layout, target uint64) (Report, error)
```

Require a target from one through 9,007,199,254,740,991,
using `ErrTargetEpoch`. Capture the current checkpoint and
preserve its error. An observed Ready target returns its record
in `Observed` and a confirmed, already-current Ready receipt
with its physical slot. It issues no writes or flushes.

A Ready epoch may advance to its immediate successor. Run
RecoverWithBoundary first and store its report. Upon success
publish a Resetting record with the target epoch and recovered
boundary. Capture this receipt in `Resetting`. The durable
Resetting record authorizes reclaiming the preceding WAL.

An observed Resetting epoch may resume the same target. Use its
record and continue directly with clearing. Any other pairing
returns `ErrCheckpointOrder` before replay or writes. Target,
capture and order refusals return a zero report.

After Resetting publication or observation, run Clear and retain
its receipt. Upon its acknowledgement, publish Ready for that
epoch with the same previous LSN. Retain its receipt. At any
effect refusal return completed and partial receipts through
that step with the exact lower error. Steps that never ran keep
zero receipts. The caller supplies a sequential device interval
with no concurrent writers or unrelated pending writes.

Reopen after any write or barrier error. An interruption before
Resetting leaves the log admissible for replay. A selected
Resetting record directs restart to the clear. A selected
Ready target gives a write-free retry. Compare actual HDD cuts
and reopen at each transition, including an interrupted second
attempt, and verify exact data and checkpoint bytes while the
log is reused. Check memory execution, lower timeouts or
refusals, epoch order, empty logs, coverage, race, vet, both
build lists, guided inputs and clean mutation controls for
clearing before Resetting or replaying a selected Resetting log.

## Local verification

The implementation is in `ea49a8b0d54`. An independent media
model matched 1,360 actual HDD cuts across eight phases, two
reservations and both slot orientations. Reopening and retrying
the same target after an already durable Ready transition took
zero writes in 28 histories. The checks compared whole media,
receipts and restart effects. Both command lists passed at
97.1% statement coverage with race, vet, tidy, 100,000 guided
inputs and both clean mutation controls.
