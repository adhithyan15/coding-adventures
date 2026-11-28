# DB11a — Address fixed-size pages over logical blocks

A page is a contiguous, fixed-width group of logical blocks. Eight
8-byte sectors can hold one 64-byte page in the first HDD model.
Reserve a block interval for database pages and use the same address
calculation on the memory device and HDD.

Add the Go package `storage-page-layout`. Its only local dependency is
the verified [logical block contract](DB06b2a-block-device-contract.md).

```go
type Layout struct {
    Geometry      blockdevicecontract.Geometry
    FirstBlock    int64
    PageCount     int64
    BlocksPerPage int64
}

type BlockSpan struct {
    FirstBlock   int64
    BlockCount   int64
    BytesPerPage int64
}

const MaxPageBytes int64 = 65_536

func (layout Layout) Validate() error
func (layout Layout) Span(page int64) (BlockSpan, error)
```

First validate the underlying geometry and preserve its error identity.
Then require the first block to be in that geometry, positive page
count and blocks per page, and a page at most `MaxPageBytes` bytes.
Require every reserved page to fit in the geometry. All layout failures
return `ErrInvalidLayout`.

Pages use zero-based IDs relative to the reservation. After validating
the layout, require `0 <= page < PageCount`. An invalid ID returns
`ErrInvalidPage`. On success the first block is
`FirstBlock + page * BlocksPerPage`, the block count is
`BlocksPerPage`, and the width is `BlocksPerPage * BytesPerBlock`.
Return a zero span on refusal. A device may have blocks before or
after the reservation. Adjacent valid pages occupy disjoint intervals
with no gap.

Compare blocks per page with `MaxPageBytes / BytesPerBlock` and page
count with `(BlockCount - FirstBlock) / BlocksPerPage` before
multiplication. Keep the input and result signed and at most the
portable bounds of the block contract. The span calculation accesses
no device and allocates no buffers. Those bounds let ports check the
same arithmetic with 64-bit integers or an exact 53-bit integer
range.

Use finite small geometries and a separate oracle that explicitly
enumerates occupied blocks. Compare signed edge cases with wide
integer arithmetic, cover exact-fit and one-block-short layouts, and
exercise the upper safe block count and width without allocating a
device. Property checks cover containment, disjointness and adjacency.
Run both build lists, race, vet, a 100,000-input guided campaign and
controlled failures for accepting an out-of-range final page and
dropping the reserved offset. Verify the public operations allocate
zero bytes on both success and typed refusal. Retain portable exact
input/result vectors for later ports.

The following page I/O slice assembles and splits one page over the
validated span and preserves lower-layer failures. A crash can cut a
multi-block update between blocks. The WAL and recovery work will
address that boundary with the backing device's explicit guarantees.
