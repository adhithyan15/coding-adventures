# DB06b2d3b — Apply sector I/O to sparse timed HDD media

Give the configurable HDD a durable sector image and an observable
seek state. Each sector read follows the timing plan. Each sector
write follows both the timing plan and the described media cut.
[Seek timing](DB06b2d2-go-hdd-seek-timing.md) · [Sector cut](DB06b2d3a-go-hdd-sector-cut-description.md)

Add the Go package `hdd-timed-media`.

```go
type ReadResult struct {
    Data   [8]byte
    Timing hddseektiming.Result
}

type WriteResult struct {
    Timing               hddseektiming.Result
    PersistedPrefixBytes int
    Acknowledged         bool
}

type Snapshot struct {
    Geometry hddgeometry.Geometry
    State    hddseektiming.State
    Sectors  map[int64][8]byte
}

func New(geometry hddgeometry.Geometry) (*Medium, error)
func (medium *Medium) ReadSector(block int64) (ReadResult, error)
func (medium *Medium) WriteSector(block int64, data []byte, cut hddwritecrashcuts.Cut) (WriteResult, error)
func (medium *Medium) Snapshot() (Snapshot, error)
func (medium *Medium) Restart() (*Medium, error)
```

New admits the geometry and initializes cylinder and tick to zero.
Absent sectors read as eight zero bytes. Store only nonzero
sectors. Reserve memory in proportion to stored sectors. A nil
or zero receiver returns the block contract's `ErrInvalidGeometry`.

For reads, plan the service and preserve its error. On success
return a value copy of the current sector and move to the planned
end state. A read leaves durable data unchanged.

For writes, check the geometry and block, then the exact eight-byte
payload through the block contract. Describe the cut and preserve
its typed refusal. Next admit the full service timing. Keep every
input and all medium state unchanged until every check passes.
Copy the described prefix from the input onto the existing sector
and preserve its suffix. Keep acknowledgement as reported by the cut
description. A before-media cut has zero transfer ticks and ends
immediately after seek and rotation. Every cut that reaches the
media consumes the model's single transfer tick. The byte prefix
describes media progress within that tick. All admitted writes move
the head to the target cylinder.

Snapshot owns a separate copy of the stored sector map. Restart
returns an independent medium with the same geometry and durable
sectors and with its clock and head reset to zero. Callers serialize
operations and keep input bytes stable.

Compare both HDD geometries and their sparse near-ceiling profile
with a separate byte image and tick model. Cross zero overwrites,
every valid cut, malformed requests, timing exhaustion, snapshots,
restart ownership and a second write after restart. Run both build
lists, race, vet, guided inputs and clean controls for replacing an
unwritten suffix and acknowledging a full write after response loss.

## Local validation

The frozen package in `dcc4113ea89` matched an independent stepped timing
and sector model over 5,440 histories. The replay covered every valid cut
at every address on both 32- and 512-sector geometries, checking the target
and neighboring images, zero elision, and independent restart state.
Near the portable clock ceiling, a before-media cut is refused when the
full service cannot fit. An admitted before-media cut omits the transfer
tick in the returned timing.

Both build command lists reached 100% statement coverage on Linux. The
race detector, vet, Linux 386 checks, and 100,000 guided inputs passed.
The clean control passed and the two exact mutations failed at their named
suffix and acknowledgement assertions. The native graph added the four
declared package dependencies and preserved every prior row and recipe.
