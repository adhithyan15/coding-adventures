# DB05c2a — Flash page allocation fences and erased extents

An interrupted erase can clear part of a NAND block while leaving the
whole block unavailable for reuse. The controller needs a way to skip
every slot in that block, including slots the page map already calls
free. After the media completes the erase, the controller needs a single
metadata transition that releases the whole extent. Specify those two
transitions on the [DB05a page map](DB05a-ftl-page-map.md) before
composing them with the [DB04d erase model](DB04d-nand-block-erase-cuts.md).

## Allocation fence

Add the following Go method to the existing immutable table.

```go
func (table Table) BeginExcluding(logical int64, excluded []bool) (Table, Reservation, error)
```

A zero-length mask allows every free slot and gives the same results as
`Begin`. Otherwise, the mask has one entry per physical slot. A true
entry excludes the slot from this call. Choose the lowest free index
with a false entry and reserve it with the existing DB05a transition.
Return `ErrNoFreePage` when there is no eligible slot. The caller keeps
the input stable during the call. The table retains only the reservation
and its normal state. Subsequent commands use their own supplied masks.

Validate configuration, logical address, absence of a reservation, mask
length, then capacity. Add stable `ErrInvalidFence` for an incompatible
nonempty mask. Each refusal returns the original table and a zero
reservation. The existing `Begin` API and all its outcomes remain
compatible.

For physical statuses `stale, free, live, free`, a fence covering
indices 0 and 1 sends a reservation to index 3. Lookups through the
returned table continue to see index 2 until a finish publishes a new
mapping. The predecessor table still reports index 3 as free.

## Erased extent release

Add a metadata operation for callers that have established full-media
completion.

```go
func (table Table) ReleaseErased(firstPhysical, pageCount int64) (Table, error)
```

Check these conditions in order.

1. The table is configured. Refuse with `ErrInvalidGeometry` otherwise.
2. The first physical index is in bounds, the count is strictly
   positive, and the half-open extent fits in the table. Refuse with
   `ErrInvalidExtent` otherwise. Check before converting to host-sized
   integers or adding the endpoints.
3. Every page in the extent is free or stale. Refuse with
   `ErrIneligibleExtent` if a live or reserved page occurs anywhere in
   it.

Success gives a new table with every slot of the extent free. Every
logical mapping, outstanding reservation outside the extent, and slot
outside the extent retains its prior value. Releasing an all-free
extent succeeds with the same logical state. Refusals return the
original table, including when eligible slots precede an ineligible
one. Releasing an extent while a reservation is pending elsewhere
preserves the ability to finish that reservation.

The operation takes physical index and count. The media adapter
establishes full-media completion and whole-block alignment before it
calls release. DB05c2b will track incomplete erases, apply the fence
during writes, and call release even when a completed media erase lost
its acknowledgement. DB05e owns restart reconstruction.

## Validation

- Build the bounded independent model and a shared
  `ftl-page-reclamation.json` vector before the Go transitions. Include
  fenced allocation, all-fenced capacity, malformed masks, refusals
  with earlier stale slots, release with a pending reservation
  elsewhere, and reuse of the lowest released page.
- Close the combined state spaces for geometries 1/2 and 2/3. Evaluate
  all masks and all in-bounds extents at every state, plus representative
  invalid masks, logical addresses and signed extent bounds. Check
  command ordering on overlapping refusals. Check owner uniqueness,
  predecessor immutability, snapshot detachment and typed refusals.
- Check geometry 64/128, including a final-slot extent and maximal
  signed bounds. Replay stateful seeded histories, plus
  coverage-guided fixture and history campaigns. Bound fixture parsing
  to 64 KiB, nesting depth 32, unique fields and exact schema integers.
- Prove named failures for ignoring a fenced free page, partial release
  before encountering a live page, and discarding a reservation outside
  the extent. Require native coverage, race, vet and Linux/386 execution.
- Re-run the admitted DB05a tests and downstream DB05b relocation and
  DB05c1 selector tests against the extended page map. Record the
  runtime and fixture revisions and route the new fixture exactly in
  DB-E45.
