# DB12d2b — Select a published WAL checkpoint

Choose which of two independently persisted slots governs restart.
Keep the decision pure so the same input bytes can determine the
result on either block device.
[Checkpoint slot](DB12d2a-go-wal-checkpoint-slot.md)

Add the Go package `storage-wal-checkpoint-selection`.

```go
type Selection struct {
    Slot   uint8
    Record storagewalcheckpointrecord.Record
}

func Select(slots [2][]byte) (Selection, error)
```

Inspect each slot in physical ordinal order. Preserve the first
inspection error, including a wrong-width slot, an unknown guard
or an invalid published frame. Perform these checks before deciding
whether the admitted records conflict. An empty or staged slot is
ineligible. With neither slot published, return
`ErrNoPublishedCheckpoint`.

With one published record, return its ordinal and record. With two,
the higher epoch wins. If both records name the same epoch, they
must agree on `PreviousLSN`. A disagreement returns
`ErrCheckpointConflict`. With both fields equal, Ready supersedes
Resetting. Equal records select the lower physical ordinal. Every
refusal returns zero selection. Inputs remain caller-owned and
unchanged.

This decision allows restart to finish a Resetting epoch while its
successor frame is staged. It also allows a fully published Ready
frame to govern restart after a lost acknowledgement. A zero guard
with malformed contents is evidence of a published-slot fault and
must be surfaced, even when another slot could be decoded. Initial
formatting establishes the first Ready record in a later operation.

Compare the crossed state and record combinations with an
independent oracle, including both physical orders, empty or staged
peers, mixed faults, duplicate frames, boundary epochs and ranks.
Check exact slot identity as well as phase and record. Run both
build lists, coverage, race, vet, guided inputs and clean mutation
controls for choosing a lower epoch and accepting conflicting
prior ranks in the same epoch.

## Local verification

The implementation is in `37009667851`. An independent caller
constructed actual checkpoint bytes with a bitwise checksum and
matched 7,921 crossed slot observations. It checked exact winning
physical ordinal, same-epoch conflicts, error identity and
unchanged inputs. Race and vet passed.

Both command lists passed at 100% statement coverage with race,
vet and tidy. The package passed 100,000 guided inputs and two
clean mutation controls. The build graph gained only this selector
and its four expected edges.
