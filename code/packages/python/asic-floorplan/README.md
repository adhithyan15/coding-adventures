# asic-floorplan

ASIC floorplanning: die area, IO ring, row generation, IO pin placement. Output is a `lef_def.Def` ready to feed into `asic-placement`.

See [`code/specs/asic-floorplan.md`](../../../specs/asic-floorplan.md).

## Quick start

```python
from asic_floorplan import (
    CellInstanceEstimate, IoSpec, compute_floorplan, floorplan_to_def
)
from lef_def import Direction, write_def

cells = [
    CellInstanceEstimate("u_fa0_xor", "xor2_1", area=4.5),
    CellInstanceEstimate("u_fa0_and", "and2_1", area=2.6),
    # ... more cells
]
io = [
    IoSpec("a[0]", Direction.INPUT),
    IoSpec("a[1]", Direction.INPUT),
    IoSpec("sum[0]", Direction.OUTPUT),
    IoSpec("cout", Direction.OUTPUT),
]

fp = compute_floorplan(
    cells=cells, site_height=2.72, site_width=0.46, site_name="unithd",
    utilization=0.7, aspect=1.0, io_ring_width=10.0, io_pins=io,
)

def_obj = floorplan_to_def(fp, design_name="adder4")
write_def(def_obj, "adder4_floorplan.def")
```

## v0.1.0 scope

- `compute_floorplan(cells, site_height, site_width, ...)`:
  - Sums cell area, divides by utilization for core area.
  - Snaps to integer rows × site count for legal row layout.
  - Adds io_ring_width margin around core to form die.
  - Generates ROWs with alternating N/FS orientation (so neighboring rows share VDD/VSS rails).
  - Distributes IO pins on edges: inputs left, outputs right, others bottom.
- `floorplan_to_def(fp, design_name)`: package as a `lef_def.Def`.

## Out of scope (v0.2.0)

- Power-grid (VDD/VSS rings + straps); requires SPECIALNETS in lef-def.
- Macro placement (RAM blocks, custom IP).
- Clock distribution layout.
- Floorplan optimization (today's algorithm is one-shot, not iterative).

MIT.
