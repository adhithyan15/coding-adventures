# DB06b2d1 — Translate logical blocks in a larger HDD geometry

A cylinder groups the tracks at the same seek position. A
surface contributes one track per cylinder. An address selects
one sector on one surface in a cylinder. The logical block
interface still uses a single linear block number.
[Fixed HDD model](DB02-hdd-simulator.md) · [Block contract](DB06b2a-block-device-contract.md)

Add the pure Go package `hdd-geometry`.

```go
type Geometry struct {
    Cylinders       int64
    Surfaces        int64
    SectorsPerTrack int64
}

type Address struct {
    Cylinder int64
    Surface  int64
    Sector   int64
}

func (geometry Geometry) BlockGeometry() (blockdevicecontract.Geometry, error)
func (geometry Geometry) Locate(block int64) (Address, error)
func (geometry Geometry) Block(address Address) (int64, error)
```

Require positive dimensions and a sector count of at most
9,007,199,254,740,991. BlockGeometry returns that count and
eight bytes per block. Use the block contract's
`ErrInvalidGeometry` and a zero result for invalid geometry.
Bound intermediate products by division before multiplication.

Locate checks the geometry first, then uses the block contract's
`ErrInvalidBlock` for an out-of-range relative block number.
Blocks increase by sector, then surface, then cylinder. For
`track = block / SectorsPerTrack`, the cylinder is
`track / Surfaces`, the surface is `track % Surfaces`, and
the sector is `block % SectorsPerTrack`. All are zero based.
Return zero Address on refusal.

Block checks the geometry first, then checks each coordinate
against its dimension. An invalid coordinate returns
`ErrInvalidAddress` and zero. Construct the block from
`(Cylinder * Surfaces + Surface) * SectorsPerTrack + Sector`.
Every valid address maps to one block and both operations are
inverses. The operations allocate zero bytes and touch no media.

The original HDD profile uses four cylinders, two surfaces and
four sectors per track. A larger profile uses 16 cylinders, two
surfaces and 16 sectors per track, yielding 512 blocks. Geometry
validation and address translation use the same arithmetic for
both profiles. Timing, media bytes and durability are separate
layers above this contract.

Compare every address for bounded small geometries with a
separate cylinder/surface/sector enumerator. Include all 32 old
addresses, all 512 larger addresses, the final portable block,
invalid dimensions and adjacent out-of-range coordinates.
Cross the product bounds with wide-integer arithmetic. Check
zero allocations and retain portable exact vectors. Run both
build lists, race, vet, guided inputs and clean controlled
failures for reversing surface and cylinder order and accepting
the first block beyond the geometry.

## Local verification

The implementation is in `5febaad47dd`. An independent
enumerator matched 43,176 mapped addresses and their inverses.
A wide-integer oracle compared 4,913 products and checked
366 valid final-block round trips, including the portable
ceiling, with zero allocations. Both command lists passed
at 100% statement coverage with race, vet, tidy, 100,000
guided inputs and both clean mutation controls. Emulated
Linux/386 passed. Build discovery added one package and one edge.
