# DB12d2d — Read two checkpoint pages from one device

Capture the two physical pages and apply the restart selection rule.
The publisher can use the exact captured inactive page for its first
guard-only update.
[Checkpoint layout](DB12d2c-go-wal-checkpoint-layout.md) · [Checkpoint selection](DB12d2b-go-wal-checkpoint-selection.md)

Add the Go package `storage-wal-checkpoint-capture`.

```go
type Snapshot struct {
    Selection storagewalcheckpointselection.Selection
    Pages     [2][]byte
}

func Capture(device blockdevicecontract.Device, log, data, checkpoint storagepagelayout.Layout) (Snapshot, error)
```

Validate the three layouts first and preserve their error. Open the
checkpoint pager with the supplied device. Preserve the pager's
error, including nil device or geometry mismatch. Read page zero,
then page one through the pager. Stop at the first read refusal and
preserve it. Each admitted read covers its reservation in ascending
block order and returns owned bytes.

After both reads succeed, check any bytes after the first 40 in
physical page order. Use `ErrCheckpointPadding` for a nonzero
padding byte, including on an empty or staged page. Then select
from the two 40-byte frame prefixes and preserve its exact error.
A successful snapshot owns both full pages and the typed decision.
A refusal returns the zero snapshot.

The function reads only checkpoint blocks. It performs no writes
or flushes. A restart supplies a freshly opened device with the
durable image. A live caller supplies a quiescent device during
capture and any later decision that relies on the observed pages.
Initial formatting will establish a Ready record before capture
is used for restart.

Compare ordered calls with a scripted device over aligned and
padded widths. Check every lower read refusal and malformed width,
layout-before-device precedence, disagreement between padding
and frame errors, and detached pages. Exercise actual HDD and
memory capture across reopening. Run both build lists, coverage,
race, vet, guided inputs and clean mutation controls for reading
the wrong physical ordinal and ignoring nonzero padding.

## Local verification

The implementation is in `2820934555a`. An independent scripted
device matched 56 ordered read and width decisions across four
block geometries. It checked exact requested blocks, lower error
identity, padding before logical selection, and detached full
pages. Race and vet passed.

Both command lists passed at 100% statement coverage. Tidy,
100,000 guided inputs and two clean mutation controls passed.
Build discovery gained only the capture package and its 18
expected edges.
