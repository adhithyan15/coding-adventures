# DB12d2c — Reserve two checkpoint slots apart from the WAL and data

Give checkpoint frames their own device blocks before introducing an
operation that persists them. Keep layout admission free of device calls.
[Log and data layout](DB12c2c1-go-wal-data-layout.md) · [Checkpoint frames](DB12d1-go-wal-checkpoint-record.md)

Add the pure Go package `storage-wal-checkpoint-layout`.

```go
func Validate(log, data, checkpoint storagepagelayout.Layout) error
```

First apply the admitted log and data validation and preserve its
exact refusal. Then validate the checkpoint page layout and preserve
its error. Checkpoint geometry must match both existing reservations,
using the pager's `ErrGeometryMismatch`.

The checkpoint reservation contains exactly two adjacent pages. Each
page has the smallest number of whole blocks that fits the 40-byte
checkpoint frame. Compute it as
`1 + (FrameBytes - 1) / BytesPerBlock` on the validated geometry.
Use `ErrCheckpointShape` for any other page count or block count.
The first 40 bytes of each checkpoint page store its frame. The
remaining bytes, if any, are reserved zero padding in the later
physical reader and writer.

The checkpoint reservation's half-open block interval must be
disjoint from the log and data intervals. Use `ErrCheckpointOverlap`
if either interval shares a block with it. Compute each end from
the validated layout. Exact adjacency is valid regardless of
physical ordering. Admission returns no allocated buffers.

With eight-byte sectors, three seven-sector WAL slots, one data
sector and two five-sector checkpoint pages fill a 32-sector HDD.
Two one-block checkpoint pages also fit a device with 53-byte
logical blocks. The latter profile leaves 13 bytes of padding
in each checkpoint page.

Compare admitted layouts with independent block occupancy, all
physical orderings, exact widths, one-block-short reservations,
mixed errors and the largest portable address. Check allocation
counts. Run both build lists, race, vet, coverage, guided inputs
and clean controls for permitting overlap or rounding required
checkpoint blocks down.

## Local verification

The implementation is in `fbfbeb5c193`. A block occupancy model
matched 105,264 placement and shape cases across 17 sector widths,
including 8,972 admitted layouts. It checked first-error identity,
zero allocations and the last portable device address. Race, vet
and emulated Linux/386 passed.

Both command lists passed at 100% statement coverage with race,
vet and tidy. The package passed 100,000 guided inputs and two
clean mutation controls. Build discovery added only this layout
package and its eleven expected edges.
