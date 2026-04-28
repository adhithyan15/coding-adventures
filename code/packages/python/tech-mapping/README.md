# tech-mapping

Generic HNL -> Sky130-style standard-cell HNL. Rule-based cell-type rename + pin remap, plus optional bubble-pushing optimization (eliminates adjacent INV-INV pairs).

See [`code/specs/tech-mapping.md`](../../../specs/tech-mapping.md).

## Quick start

```python
from gate_netlist_format import Netlist  # generic HNL from synthesis
from tech_mapping import map_to_stdcell, push_bubbles

# Generic HNL with built-in cell types: AND2, OR2, NOT, etc.
mapped, report = map_to_stdcell(generic_netlist)
# mapped.level == Level.STDCELL
# Cells now: and2_1, or2_1, inv_1, ... (Sky130-style names)

print(report.cells_before, report.cells_after)
print(report.unmapped)  # [] if every cell type was recognized

# Optional: push bubbles to cancel INV-INV pairs
optimized, cancelled = push_bubbles(mapped)
print(f"Cancelled {cancelled} INV-INV pairs")
```

## v0.1.0 scope

- `map_to_stdcell(netlist)`: rename + pin-remap built-in HNL cell types to Sky130-style stdcells via `DEFAULT_MAP`.
- `TechMapper(cell_map=...)`: customizable mapping table.
- Pin remapping: e.g., HNL `Y` pin → stdcell `X` pin for combinational outputs.
- `push_bubbles(netlist)`: walks each module and cancels adjacent inv_1/INV pairs (driver `Y` -> reader `A` rewires through).
- `MappingReport`: cells_before/after, list of unmapped cell types (passed through unchanged).

## Out of scope (v0.2.0)

- AOI/OAI folding: needs DAG covering / pattern matching across larger sub-trees.
- Drive-strength selection: needs load estimation (post-floorplan).
- Multi-target mapping (different cells for different design constraints).
- Sequential cell variants (clock-gating, scan, etc.).

MIT.
