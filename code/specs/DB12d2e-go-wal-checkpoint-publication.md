# DB12d2e — Publish a successor checkpoint in three durable steps

Keep the selected checkpoint legible while constructing its
successor in the other page. Each acknowledged barrier permits
one step in the publication protocol.
[Checkpoint capture](DB12d2d-go-wal-checkpoint-capture.md) · [Guard and crash cuts](DB12d2a-go-wal-checkpoint-slot.md)

Add the Go package `storage-wal-checkpoint-publication`.

```go
type Step uint8

const (
    NoWrite Step = iota
    InvalidateGuard
    StageBody
    PublishGuard
)

type Receipt struct {
    Slot           uint8
    Attempted      Step
    BodyBlocks     int64
    Confirmed      bool
    AlreadyCurrent bool
}

func Publish(device blockdevicecontract.Device, log, data, checkpoint storagepagelayout.Layout, desired storagewalcheckpointrecord.Record) (Receipt, error)
```

First admit and encode the desired record with the checkpoint slot
codec. Preserve any encoding refusal. Capture the current pages
and preserve any capture error. An exact match with the selected
record returns its slot, `Confirmed` and `AlreadyCurrent`, with
zero blocks and `NoWrite`. A caller that sees a write or barrier
failure must reopen on the durable image before using this
idempotent observation.

A Ready record can advance to Resetting at the next epoch. A
Resetting record can advance to Ready in the same epoch with the
same previous rank. Any other change returns
`ErrCheckpointTransition`. These preflight refusals return a zero
receipt and issue no writes or flushes. The reset orchestrator will
establish durable data before requesting Resetting and clear the old
WAL before requesting Ready.

The destination is the other physical slot. Use the validated
checkpoint span and guard offset to resolve its guard block.
Perform these steps in order.

1. Copy the captured destination guard block, change only its guard
   byte to the staged marker, write that block and flush.
2. Build the destination page with the staged frame and zero
   padding. Write all its blocks in ascending order and flush.
3. Copy the staged page's guard block, change only the guard to
   zero, write that block and flush.

Set `Attempted` to the current step immediately before its first
write. Preserve the first lower error and stop there. Count
successful step-two writes in `BodyBlocks`, including an entirely
written body whose barrier failed. Only an acknowledged final
barrier sets `Confirmed` on the write path. Return the destination
slot in every receipt that records a write attempt. Capture and
publication occupy one sequential device interval with no concurrent
writers or unrelated pending writes.

Before the first barrier, a crash can leave the older destination
frame published or staged. The original selection still governs restart.
After that barrier, any interrupted body leaves the destination
staged. The final guard write changes exactly one byte. Its
admitted byte-prefix cuts either preserve staging or expose the
complete desired frame. The previous selected frame survives every
step.

Use a scripted device to check the exact effect sequence and the
refusal from every write and barrier. Independently compare
whole-device HDD bytes at every modeled cut, including a durable
final guard with a lost acknowledgement and idempotent restart.
Exercise padded pages through a second block geometry. Run both
build lists, coverage, race, vet, guided inputs and clean mutation
controls for a missing body barrier and a premature confirmation.

## Local verification

The implementation is in `f49ddef76ec`. On the actual HDD
model, an independent caller matched 552 barrier cuts across both
directions and ordinals and 56 lower write refusals. After 56
durable but ambiguously acknowledged publications, restart and
retry observed the selected record with zero writes or flushes.
It compared the whole device and retained prior marker at each
boundary.

Both command lists passed at 96.0% statement coverage with race,
vet and tidy. The package passed 100,000 guided inputs and both
clean mutation controls. Build discovery added only the publisher
and its 18 expected edges.
