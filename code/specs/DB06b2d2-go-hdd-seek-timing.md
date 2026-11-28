# DB06b2d2 — Plan seek and rotation for a configurable HDD

Map a block to cylinder, surface and sector. Move the seek
coordinate to the target cylinder and advance the rotational
phase during seek, wait and transfer.
[HDD geometry](DB06b2d1-go-hdd-geometry.md) · [Fixed timing profile](DB02-hdd-simulator.md)

Add the pure Go package `hdd-seek-timing`.

```go
type State struct {
    HeadCylinder int64
    Tick         uint64
}

type Result struct {
    Target        hddgeometry.Address
    End           State
    SeekTicks     uint64
    RotationTicks uint64
    TransferTicks uint64
    TotalTicks    uint64
}

func Plan(geometry hddgeometry.Geometry, current State, block int64) (Result, error)
```

Use the existing geometry locator and preserve its first error.
Then require the head cylinder inside the geometry and use
`ErrHeadCylinder` on refusal. A current tick or proposed end
tick above 9,007,199,254,740,991 returns `ErrClockOverflow`.
Every refusal returns zero Result.

Seek costs three ticks per cylinder crossed. Rotation advances
by one sector phase per tick. Compute the phase after seek
modulo sectors per track and the forward wait from that phase
to the target sector. The target surface appears in the
address. The seek coordinate remains the cylinder. Every
accepted transfer costs one additional tick.

Compare the seek distance with the remaining clock budget
divided by three before multiplication. Compute the phase
from reduced terms. If adding the remainders would reach one
full rotation, subtract the phase from the period first and
subtract that difference from the advance. Compute the forward
wait without exceeding one rotation. Require room for both
the wait and the transfer before summing. Keep every admitted
intermediate in the portable exact-integer range.

TotalTicks is the sum of seek, wait and transfer. End.Tick is
the start tick plus that sum, and End.HeadCylinder names the
target cylinder. Planning allocates zero bytes and observes
only its inputs. The medium layer can apply the returned state
at its own commit and power-loss boundaries.

Check all fixed-profile head, phase and block combinations
against DB02, then compare both 512-sector geometries with an
independent tick-by-tick model over bounded operation streams.
Check portable ceilings with wide arithmetic. Cross invalid
geometry, block, head and tick values to pin refusal order.
Run both build lists, race, vet, guided inputs and clean
controls for skipping rotation during seek and accepting a
one-tick overflow.

## Local verification

The implementation is in `cbba5e43dea`. An independent tick
model matched 262,656 head, phase and block combinations across
the fixed and both 512-block profiles. Another 4,096 operations
matched the original fixed simulator and 5,280 wide arithmetic
cases checked exact bounds and error precedence. Both command
lists passed at 100% coverage with race, vet, tidy, guided
inputs and two clean mutation controls. Emulated Linux/386 passed.
