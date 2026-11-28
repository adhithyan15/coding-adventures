# DB12b2 — Admit a WAL prefix and classify its suffix

Read fixed WAL pages in physical order. A stream carries every
sequence from 1 through its last accepted record. A vacant or
damaged position marks a possible end. A later occupied page
invalidates that boundary.
[Fixed WAL slot](DB12b1-go-wal-page-slot.md)

Add the pure Go package `storage-wal-scan`.

```go
type TailKind uint8

const (
    Full TailKind = iota + 1
    Vacant
    Damaged
)

type Scan struct {
    Records  []storagewalrecord.Record
    Tail     TailKind
    TailPage int64
    Cause    error
}

func ScanPages(pages [][]byte) (Scan, error)
```

The caller supplies an ordered, fixed-width reservation. Check the
rows in scan order. A later row of different width returns
`ErrPageWidthMismatch`. Preserve the slot decoder's `ErrPageSize`
as an invalid reservation shape. Return a zero scan for either
refusal. An empty reservation returns a full scan at page zero.

For each page, use the admitted slot decoder. Before the first
tail position, accept occupied pages only when each LSN equals
the preceding accepted LSN plus one, starting at one. Use
`ErrSequence` on a wrong rank. A vacant page records the first
vacant position. The first malformed slot records a damaged tail
and its exact decoder error. Inspect every later page. Use
`ErrPageAfterTail` if a later page is occupied or malformed.

Return the accepted records in their original order with owned images.
A full scan has `TailPage == len(pages)` and `Cause == nil`.
A vacant scan has the first vacant ordinal and a nil cause.
A damaged scan has the first damaged ordinal and its typed cause.
A recovery policy will decide when the modeled device evidence
permits discarding that damaged tail.

Compare with an independent stream model across blank pages,
checksummed records, missing and repeated ranks, torn tails and
later nonblank pages. Check shape failure order, full and vacant
reservations, detached image results, race and vet. Run both build
lists, 100,000 guided inputs and controls for stopping at the first
vacant page or admitting a skipped rank.
