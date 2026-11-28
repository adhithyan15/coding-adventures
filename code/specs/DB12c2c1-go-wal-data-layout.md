# DB12c2c1 — Validate the log and data layout on one device

Admit a fixed log reservation and a disjoint data reservation on the
same logical block device. Every full data-page image must fit in one
log slot before a caller can open the first redo-only transaction
profile.
[Fixed log slot](DB12b1-go-wal-page-slot.md) · [Shared capture](DB12c2b1-go-wal-reservation.md)

Add the pure Go package `storage-wal-data-layout`.

```go
func Validate(log, data storagepagelayout.Layout) error
```

Validate the log layout, then the data layout, and preserve the first
refusal. Require identical device geometries using the pager's
`ErrGeometryMismatch`. Require at least three log slots for a begin,
one page image and its commit, using `ErrLogTooSmall`. Preserve the
shared reservation's capacity error above its maximum.

The two half-open block intervals must be disjoint. An interval may end
exactly where the next one starts. Use `ErrLayoutOverlap` for a shared
block. Compute their endpoints from the validated layouts. Then require
an admitted log slot width using its page-size error. Use
`ErrImageDoesNotFit` if the log slot cannot store the data page plus
the fixed 48-byte WAL overhead. This overhead is the four-byte slot
length, the 16-byte DBRF overhead and the 28-byte WAL record header.
Use the slot contract's minimum width as the fixed overhead constant.

The 32-sector HDD profile with eight bytes per sector can fit four
56-byte log slots followed by four 8-byte data pages. A NAND profile
with 53-byte logical blocks needs at least two such blocks per log
slot to encode one full 53-byte data block. A log slot at the maximum
65,536-byte width can admit at most a 65,488-byte data image.
Admission performs no device calls or allocations.

Compare a finite two-interval model, crossed faults, adjacent and
overlapping reservations, and portable signed bounds. For data widths
around each boundary, compare the decision with the actual fixed-slot
encoder over valid page images. Exercise the largest admitted image
and its next width. Run both command lists, race, vet, guided inputs
and controls for accepting one overlapping block or dropping one
byte from the framing overhead.

## Encoder boundary audit

An independent caller checked 1,119 actual encoding decisions at 225
log slot widths under race and vet. It tested data images on both
sides of the 48-byte overhead boundary, including the maximum log
slot and its largest admitted image. The writer and layout admission
will use this existing encoder contract.

## Local verification

The implementation is in `a3410161deb`. An independent model marked
the occupied block addresses and matched 142,560 layout decisions,
including 633 admitted pairs. It verified the last legal device
address, one shared block, one extra image byte, the reservation
size boundary and zero allocations at the portable ceiling.
Race and vet passed.

Both command lists passed on Linux at 100% statement coverage
with race and vet. Module tidy, 100,000 guided inputs and both
mutation controls passed. The native interval model checked
15,711,268 decisions, and actual encoding matched the image-fit
boundary. The graph gained only this package and its nine
expected edges per platform.
