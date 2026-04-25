# asic-placement

Simulated-annealing placement on the half-perimeter wirelength (HPWL) cost function, followed by greedy row-by-row legalization. Output is a `lef_def.Def` with cells placed at legal coordinates.

See [`code/specs/asic-placement.md`](../../../specs/asic-placement.md).

## Quick start

```python
from asic_placement import place, PlacementOptions, CellSize
from asic_floorplan import compute_floorplan, CellInstanceEstimate, IoSpec
from lef_def import write_def, Direction

cells = [CellInstanceEstimate(f"u{i}", "nand2_1", area=3.75) for i in range(16)]
fp = compute_floorplan(cells=cells, site_height=2.72, site_width=0.46, site_name="unithd")

# Cell sizes for the SA cost function
sizes = {"nand2_1": CellSize("nand2_1", width=1.4, height=2.72)}

# Nets connecting cells (for HPWL minimization)
nets = [["u0", "u1", "u2"], ["u3", "u4", "u5"], ...]

placed_def, report = place(
    fp=fp, cell_sizes=sizes, nets=nets,
    options=PlacementOptions(iterations=10000, seed=42),
)

print(f"Final HPWL = {report.final_hpwl:.2f} µm; {report.cells_placed} cells in {report.runtime_sec:.2f}s")
write_def(placed_def, "adder4_placed.def")
```

## v0.1.0 scope

- Initial placement: random row + left-to-right within row
- Simulated annealing on HPWL with exponential cooling
- Greedy left-to-right legalization
- `PlacementReport`: final HPWL, accepted/rejected swap counts, runtime

## Out of scope (v0.2.0)

- Analytical placement (quadratic / RC-tree solver)
- Detailed placement (Abacus / tetris / RowLeg)
- Timing-driven placement (slack-weighted nets)
- Region constraints (cells with `(* group = "x" *)`)
- Pre-placed cells

MIT.
