# DB06b1 — Flush the dirty HDD sectors in a deterministic order

Use the [write-back cache](DB06a-hdd-writeback-cache.md) to model a
barrier over all currently dirty sectors. The reference barrier visits
dirty sectors in ascending LBA order. An engine can observe an
acknowledgement only after the barrier has flushed the whole set.
A power cut can leave a durable prefix of sectors and a torn
sector at the boundary.

Add a whole-set transition to Go `hdd-writeback-cache`.

```go
type BarrierCutKind uint8

const (
    BeforeFirstSector BarrierCutKind = iota
    AtSector
    AfterAllMedia
    BarrierComplete
)

type BarrierCut struct {
    Kind        BarrierCutKind
    SectorIndex int64
    MediaCut    hddwritecrashcuts.Cut
}

type BarrierReceipt struct {
    PriorCompletedSectors int
    TerminalPrefixBytes   int
    Acknowledged          bool
}

func (state State) FlushDirty(cut BarrierCut) (State, BarrierReceipt, error)
```

Use the dirty set at the beginning of the command. Check the cut before
committing a transition. `AtSector` takes a zero-based ordinal within
the ordered dirty set. It requires a lower cut with no acknowledgement.
Reject an invalid barrier kind, signed ordinal, unused field or lower
acknowledgement tag with `ErrInvalidBarrierCut`. That tag is rejected
before inspecting its prefix. The other three barrier cuts require zero
ordinal and the zero lower cut value. Preserve the lower cut error on
other malformed `AtSector` inputs.
Return the original state and a zero receipt on any refusal.

| Cut | Media work | Pending writes after the call | Acknowledged |
|---|---|---|---|
| `BeforeFirstSector` | Zero sectors | Dropped | No |
| `AtSector` | Flush earlier sectors and apply the lower cut to this sector | Dropped | No |
| `AfterAllMedia` | Flush every dirty sector | Empty | No |
| `BarrierComplete` | Flush every dirty sector | Empty | Yes |

`PriorCompletedSectors` counts the fully flushed sectors before the
terminal cut. It counts the whole set for the final two cases.
`TerminalPrefixBytes` counts only bytes persisted by the selected
lower cut in `AtSector`. A complete lower media write without its
acknowledgement therefore contributes eight terminal bytes. An empty
dirty set admits the three cuts without a sector ordinal.

Reuse `FlushSector` for the physical transition and `PowerLoss` for
loss of pending writes. Keep retained state, durable neighbors and the
requested LBA order observable in deterministic traces. Validate
repeated crashes and retries from each cut. Compare against an
independent array model over small dirty sets and all sector ordinals,
including writes to sparse LBAs in reverse arrival order, previously
durable neighbors and the full 32-sector dirty set. Check error
precedence, zero receipts, race, vet, guided histories and mutation
controls for a reversed flush order and acknowledgement before the
entire set reaches durable media.


## Local verification

The implementation is in `2d063064747`. An independent sector oracle
matched 10,275 complete and interrupted barrier decisions and their
restart retries. It checked 12 exact refusal and rollback cases.
The independent caller passed race and vet.

The emitted recipe passed at 100% statement coverage, with race and
vet. Both command lists ran on Linux. The package passed 100,000 guided
inputs on each of its two targets and all five named mutation controls.
Linux/386 passed under QEMU, and the Windows/amd64 test binary compiled.
Module tidy passed. All 6,004 build graph rows and 6,003 emitted recipe
rows matched the previous export byte for byte.
