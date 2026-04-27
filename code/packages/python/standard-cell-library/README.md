# standard-cell-library

Liberty-style standard cell library for the Sky130 teaching subset. NLDM (Non-Linear Delay Model) timing tables indexed by (input slew, output load), plus pin capacitances, area, and leakage power per cell. Drive-strength selection helper.

See [`code/specs/standard-cell-library.md`](../../../specs/standard-cell-library.md).

## Quick start

```python
from standard_cell_library import build_default_library, select_drive

lib = build_default_library()

# Look up a cell
cell = lib.get("sky130_fd_sc_hd__nand2_1")
print(cell.area, cell.leakage_power)

# Look up a timing arc
arc = cell.timing_arcs[0]
delay_ns = arc.cell_rise.lookup(slew_ns=0.05, load_ff=2.0)
print(delay_ns)

# Pick the smallest INV that drives a 5fF load in under 30ps
chosen = select_drive(lib, "sky130_fd_sc_hd__inv", target_load_ff=5.0, target_delay_ns=0.030)
print(chosen)
```

## v0.1.0 scope

- `LookupTable`: 5x5 NLDM table with bilinear interpolation.
- `TimingArc`: related_pin -> output_pin, with cell_rise / cell_fall / rise_transition / fall_transition tables and unate sense.
- `CellTiming`: name, area, leakage_power, pin_capacitance dict, timing_arcs tuple.
- `Library`: cells dict; voltage / temperature / process corner.
- `build_default_library()`: populates ~33 Sky130 teaching cells with hand-tuned values targeting Sky130 reference within ~10%.
- `select_drive(lib, base_name, target_load_ff, target_delay_ns=None)`: picks the smallest drive strength that meets timing.

## Out of scope (v0.2.0)

- SPICE-driven characterization (run mosfet-models + spice-engine across PVT corners to populate the tables empirically).
- CCS (current-source) model alongside NLDM.
- Liberty `.lib` text-format reader/writer.
- Variation-aware models (statistical / Monte Carlo).
- Different process / voltage / temperature corners populated.

MIT.
