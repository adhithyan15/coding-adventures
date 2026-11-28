# DB05c3b2 — Coordinate evacuation of one NAND block

Add the controller state that keeps one victim fenced between commands.
DB05c3a plans one current map, DB05c3b1 executes one verified move, and
DB05c2b applies an erase. Carry their outcomes across successive immutable
device values. Volatile device state models interruption between individual
operations. Durable reconstruction belongs to DB05e.

Use the admitted DB05c3b1 relocation package at `eac57c06fba0`.
Its `relocation.go` SHA-256 is
`30a06337b54ba2576d205ed6bdeb986be63e48c0b92c010be79ef4a48669e4ce`.
That revision includes the DB05c2b erase and DB-E57 fixture verdict.
Use the DB05c3a planner at `a9a6616bbb9f`, with `plan.go` SHA-256
`6fd1d62b08f34c2c4359d78d70e8e0580000801ed70ae6cfbc415b8f8e7206ab`.

## Public operations

```go
func (device Device) BeginEvacuation(block int64) (Device, error)
func (device Device) EvacuationProgress() (EvacuationProgress, bool)
func (device Device) CopyNextEvacuationPage(cut Cut) (Device, EvacuationCopyReceipt, error)
func (device Device) FinishEvacuation(cut EraseCut) (Device, EraseReceipt, error)
func (device Device) CancelEvacuation() (Device, error)

type EvacuationProgress struct {
    Block int64
    RemainingLivePages int64
    IncompleteErase bool
}

type EvacuationCopyReceipt struct {
    Move ftlevacuationplan.Copy
    Receipt Receipt
}
```

A device owns at most one active target. Progress reports the number of
current live pages in that extent, including changes from ordinary logical
writes that published elsewhere. There is no cached copy assignment or
progress countdown. Reads use the same current logical map. With no active target,
including the zero and legacy device, progress returns its zero value
and false.

### Begin and allocate

Validate the device, block profile and index in their admitted order.
Refuse an already active evacuation, an incomplete target and an erase
count at its ceiling, in that order. Report
`nandblockerasecuts.ErrEraseLimit` at the ceiling. Ask the admitted planner to establish
the initial stale-page and full remaining-spare preconditions, excluding
every incomplete block. Return the planner's typed refusal if it fails.
Successful begin changes only the active target. A target that is already
stale-only may begin and go directly to erase.

Ordinary writes exclude the entire active target, including its free slots,
alongside the existing incomplete extents. A direct CopyLivePage targeting
an active source block or a direct EraseBlock on the active target reports
ErrEvacuationActive after validation of the block index. A move from a
different source block to the active target reports
ErrCopyDestinationUnavailable at the destination availability check.
Other direct operations retain their admitted behavior. In particular a
completed erase on a different eligible extent can supply fresh spares.

### Copy and replan

Check the configured device, block profile and active target. With no
active target return ErrNoActiveEvacuation. With zero remaining live
pages return ErrEvacuationNoLivePages. Otherwise replan the current
map, excluding all incomplete extents. The planner supplies its first
assignment, ordered by source physical page. Reject insufficient
remaining spares before applying any program cut.

Execute that assignment through the admitted single-page transition.
Return the selected assignment even when the media cut did not publish.
BeforeAccept leaves current capacity and progress intact. AfterPrefix
spends that destination while retaining the old logical source. The next
copy is planned again and can choose a different spare. AfterMedia and
Complete move ownership and reduce the remaining live count once.
Publication and acknowledgement retain their distinct meanings.

### Erase, retry and cancel

Check the configured device, block profile and active target. If the
target still has live pages, return ErrEvacuationStillLive before
validating the erase cut. Otherwise run the admitted block erase.
BeforeAccept retains the active fence. An accepted partial erase retains
the active fence and sets incomplete state. A completed erase releases
the map extent and clears the active fence even when acknowledgement
was lost. A new finish after release reports ErrNoActiveEvacuation.

Cancel drops the active fence before any accepted erase prefix while
preserving media, ownership and completed copies. Cancel after an
incomplete erase reports ErrEvacuationBlockIncomplete so the target
remains fenced until erase finishes. Cancel also uses the configured,
block profile and active validation order. Every refusal returns the
original value and a zero command receipt.

## Manual comparisons

- Compare closed state graphs from an independently modelled reverse
  ownership map. Include the active target, progress, exact media,
  dirty markers, erase counts and reads while switching among copy,
  write, direct operations, finish and cancel.
- Include a target that contains stale, live and free pages together.
  Check writes before and after a successful finish or cancel. Cover a
  host write that moves a remaining victim owner, direct copy back into
  the fence, zero-prefix spending, a new lowest spare after another
  block erase and an all-fenced capacity refusal.
- Exercise exact capacity and one-short capacity, a spent spare making
  a later copy stop, cancellation with already-published moves, early
  erase versus malformed cuts, failed cancel after partial erase, lost
  acknowledgement on both program and erase and count ceiling zero.
- Use a strict self-contained shared vector `ftl-evacuation.json` with
  schema `coding-adventures.database-media.ftl-evacuation.v1`, a 64 KiB
  byte cap, a depth limit of 32 and portable integers. Use closed models,
  seeded histories, fixture and stateful fuzz. Require model acceptance
  before runtime replay. Demonstrate deliberate failures
  for allocating inside the target, reusing the previous destination
  after a prefix, erasing before zero live pages and clearing the
  target only when an erase acknowledgement arrives.
- Re-run all configured profiles and original fixture vectors from
  DB05b, DB05c2b and DB05c3b1. Pin exact admitted source before work
  begins. Record native, race, vet, cross-target and ownership evidence.
