# DB05c3b1 — Execute one planned NAND page copy

Extend the Go relocation device after DB05c3a. A copy moves one live page
to a specified free physical destination. The two locations are already
present in an evacuation plan. The device verifies the copy against its
current map because a caller can retain plans across intervening writes.

## Public operation

```go
func (device Device) CopyLivePage(block int64, move ftlevacuationplan.Copy,
    cut Cut) (Device, Receipt, error)
```

Use the existing physical destination, publication and acknowledgement
receipt. The operation gets its payload from the currently visible
source page. Keep the caller's earlier device and the source page bytes
available as a predecessor value across every transition.

Perform admission in this order.

1. Validate the device and the erase-enabled block profile. Use the
   existing geometry and erase-disabled errors.
2. Validate the block index. Refuse an incomplete target block before
   checking the copy fields.
3. Validate the logical page, then the physical source index and its
   membership in the target block. Check that the current live map
   still points the logical page to that source.
4. Validate that the physical destination fits the geometry, lies
   outside the target and every incompletely erased block, and is free.
5. Reserve exactly that destination and apply the page program cut.
   Existing program-cut errors preserve the original device.

The page-map mask can exclude every physical slot except the desired
destination. Every refusal returns the original device and a zero receipt. Use
`ErrEvacuationBlockIncomplete` when the selected block is already
incomplete, `ErrInvalidCopySource` for a source outside the selected
extent, and `ErrCopySourceChanged` for a source that lost ownership.
Use `ErrInvalidCopyDestination` for an out-of-range or same-block target
and `ErrCopyDestinationUnavailable` when a destination is occupied,
spent or in an incomplete block. The caller can replan on the returned
value.

## Media and map outcomes

| Cut | Destination state | Current logical mapping | Receipt |
|---|---|---|---|
| BeforeAccept | Free with clear marker | Keep the old source | Unpublished and unacknowledged |
| AfterPrefix | Spent with exact prefix and set marker | Keep the old source | Unpublished and unacknowledged |
| AfterMedia | Live with complete source bytes and set marker | Publish the destination and mark the source stale | Published and unacknowledged |
| Complete | Live with complete source bytes and set marker | Publish the destination and mark the source stale | Published and acknowledged |

Keep old source bytes and its marker in the physical block until erase.
Use explicit protocol progress for all-ones bytes and a zero prefix.
A repeated plan after publication is refused by its original source
identity. BeforeAccept keeps the planned destination eligible for retry.
AfterPrefix spends the destination and a later retry needs a fresh one.

Use the admitted Go DB05c2b device at `28699172e6f9`.
The `relocation.go` SHA-256 is
`3c6578b2b36429b62cccb6e1b5c4414e48c98ac67d20e0803bd43003f1716028`.
Share its existing reserve-to-publication transition and preserve
the host write's public behavior and refusal order. Use the Go planner
at `a9a6616bbb9f`, with `plan.go` SHA-256
`6fd1d62b08f34c2c4359d78d70e8e0580000801ed70ae6cfbc415b8f8e7206ab`.

## Manual comparisons

- Prepare a mixed-live victim and obtain its copy from the admitted
  planner. Compare every cut and prefix with an independent ownership
  and media model. Cover programmed all-ones bytes and reads after
  each outcome. Verify exact source and destination snapshot bytes.
- Exercise a stale logical-to-source pair, a destination spent by a
  previous prefix, an unrelated write after planning, and a logically
  complete copy with a lost acknowledgement.
- Validate `ftl-live-page-copy.json` with schema
  `coding-adventures.database-media.ftl-live-page-copy.v1` before replay.
  Cap documents at 64 KiB and depth 32 with exact keys, token kinds
  and portable integers. Run closed graphs and seeded histories.
  Fuzz the fixture adapter and stateful command
  path. Require distinct deliberate failures for copying the wrong
  page, early publication, and retry into the spent destination.
- Re-run DB05b and DB05c2b comparisons including the old fixed
  erase-vector and all valid host cuts. Verify the original read
  buffers, snapshots and predecessors remain detached.

The next coordinator slice will keep the entire victim allocation
fenced during the sequence and carry the progress needed to replan
subsequent copies. Treat block release and durable restart evidence
through their own existing transitions.
