# DB12d3b — Clear the preceding WAL under a Resetting checkpoint

Erase the previous log's bytes only after restart can see an
admitted Resetting epoch. The marker remains durable while any
prefix of the log is cleared.
[Physical checkpoint capture](DB12d2d-go-wal-checkpoint-capture.md) · [Checkpoint publication](DB12d2e-go-wal-checkpoint-publication.md)

Add the Go package `storage-wal-checkpoint-clear`.

```go
type Receipt struct {
    Checkpoint   storagewalcheckpointrecord.Record
    BlocksWritten int64
    Acknowledged bool
}

func Clear(device blockdevicecontract.Device, log, data, checkpoint storagepagelayout.Layout) (Receipt, error)
```

Capture the two checkpoint pages through the admitted reader and
preserve its error. Require the selected record to be Resetting,
using `ErrCheckpointNotResetting` for a Ready record. Both
refusals return zero receipt and perform no writes or flushes.

Write one all-zero block to each block in the validated log
reservation, in increasing physical order. Reuse one zero
buffer of the device's block width. Count each successful write
and stop at the first refusal. After the last write, issue one
barrier. Only its acknowledgement sets `Acknowledged`. Return
the exact selected checkpoint record and completed write count
on the write path. Preserve every lower error.

The admitted Resetting record asserts that the preceding data
replay is durable. Clearing uses the validated block reservation
even when the old log has damaged bytes. It changes only log blocks and leaves the
checkpoint and data reservations available for restart. After
any interrupted clear, reopen from durable bytes before retrying.
Every durable prefix still selects the same Resetting record.
Clearing the complete reservation again is idempotent. A later
operation publishes Ready before accepting a new transaction.
The caller supplies one sequential interval with no concurrent
writers or unrelated pending writes.

Check every scripted read, write and barrier refusal, exact
reserved ranges, previously empty or damaged log blocks,
and exact non-log bytes. Compare repeated HDD clear cuts and
restart to independent durable prefixes. Run both build lists,
coverage, race, vet, guided inputs and clean mutation controls
for clearing under Ready or acknowledging before the barrier.

## Local verification

The implementation is in `9ff5525da24`. An independent caller
matched 4,584 two-cut actual HDD histories and final acknowledged
clears across both physical arrangements and checkpoint ordinals.
The full durable image retained every data and control byte.
It matched 84 lower write refusals and four Ready refusals.
Both command lists passed at 100% coverage with race, vet and
tidy. Guided inputs and both clean mutation controls passed.
Build discovery added one package and its 18 expected edges.
