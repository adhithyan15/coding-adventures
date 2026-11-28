# DB12d3a2 — Publish the first Ready checkpoint on an empty reservation

Initialize a fresh pair of checkpoint pages so later recovery
can select epoch one. Use the existing guarded publication
steps for complete and interrupted initialization.
[Raw checkpoint pages](DB12d3a1-go-wal-checkpoint-raw-pages.md) · [Three-barrier publication](DB12d2e-go-wal-checkpoint-publication.md)

Extend the Go `storage-wal-checkpoint-publication` package.

```go
func Initialize(device blockdevicecontract.Device, log, data, checkpoint storagepagelayout.Layout) (Receipt, error)
```

Read the two physical checkpoint pages through ReadPages and
preserve its error. Inspect both frame prefixes in ordinal
order before determining eligibility. Preserve exact slot and
frame errors. Require a vacant slot one. Slot zero can be
vacant, staged, or published with the exact initial record
`Ready, epoch 1, previous LSN 0`. Use `ErrCheckpointOccupied`
for any other admitted existing state.

Then read the log and data reservations in that order, with
ascending blocks within each. Preserve lower read and width
errors. Require all bytes in both reservations to be zero.
Use `ErrInitialLogNonempty` and `ErrInitialDataNonempty` for the
first nonzero byte in each reservation. A refusal at any
preflight check returns zero receipt and issues no writes or
flushes.

If slot zero already contains the initial record, return its
confirmed and already-current receipt with no writes or
flushes. Otherwise use the publisher's three ordered guard,
body and final-guard steps on slot zero. Share their
implementation with Publish while preserving both existing
Publish preflight and effect semantics. Preserve receipt
steps, partial writes, acknowledgement and lower errors.
Keep slot one and every other reservation byte untouched.

The caller opens the durable device image after a failed
write or barrier before retrying. Every admitted cut has an
empty log and data reservation with the other slot vacant.
A retry may see a vacant, staged or published initial slot
and uses the same eligibility checks. The caller supplies a
sequential interval with no concurrent writers or unrelated
pending writes.

Verify both admitted physical alignments, every mixed preflight
error, both reservations' nonzero guards and exact written
bytes. Cross all three HDD barrier cuts with reopening and
initialization retries. Run both build lists, coverage, race,
vet, guided inputs and clean mutation controls for admitting
nonempty data or overwriting an occupied alternate slot.

## Local verification

The implementation is in `13e4ead0fb9`. An independent HDD caller
matched 138 initialization cuts across two physical arrangements,
14 write-free lost-ack observations, and 44 nonempty reservation
refusals. The existing 552 publication cuts also passed through
the shared effect code.

Both command lists passed at 96.6% coverage with race, vet and
tidy. Each target passed 100,000 guided inputs. All four mutation
controls passed and the build graph and recipes matched exactly.
