# standard-cell-library

Liberty-style NLDM (Non-Linear Delay Model) timing library for the Sky130 HD teaching subset.

## Concepts

Each cell carries:
- **Area** (µm²) and **leakage power** (nW)
- **Pin capacitances** (pF) — input load seen by driving cell
- **Timing arcs** — one per (input, output) pair, each with 4 LUTs:
  - `cell_rise`, `cell_fall` — propagation delay (50% → 50%) in ns
  - `rise_transition`, `fall_transition` — output slew (10%→90% / 90%→10%) in ns

Each LUT is a 5×5 grid indexed by (input slew, output load) with bilinear interpolation.

## Pipeline position

The library feeds the **static timing analysis** step: given a placed-and-routed netlist, sum delays along paths to find the critical path and clock period.

## Usage

```rust
use standard_cell_library::{build_default_library, select_drive};

let lib = build_default_library();

// Look up timing.
let cell = lib.get("sky130_fd_sc_hd__inv_1").unwrap();
let arc = &cell.timing_arcs[0];
let delay = arc.cell_rise.lookup(0.05, 2.0);  // slew=50ps, load=2fF
println!("INV rise delay: {:.3} ns", delay);

// Pick smallest drive that meets a 0.10 ns budget at 2 fF load.
let drive = select_drive(&lib, "sky130_fd_sc_hd__inv", 2.0, Some(0.10));
println!("Selected: {drive}");  // sky130_fd_sc_hd__inv_1 or larger
```

## Testing

```
cargo test -p standard-cell-library -- --nocapture
```

16 tests covering: LUT bilinear interpolation, clamping, delay vs load monotonicity, arc counts, drive selection.
