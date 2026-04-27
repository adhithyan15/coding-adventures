# Changelog

## [0.1.0] — Unreleased

### Added
- `LookupTable`: 2-D NLDM table indexed by (input_slew, output_load) with bilinear interpolation and out-of-range clamping.
- `TimingArc`: per-arc rise/fall delays and transitions plus unate sense.
- `CellTiming`: per-cell aggregate (area, leakage, pin capacitance, timing arcs).
- `Library`: collection of cells at a given voltage/temperature/process corner.
- `build_default_library()`: in-memory Sky130 teaching-subset library with hand-tuned values targeting Sky130 reference within ~10%. Covers ~33 cells from sky130_pdk.TEACHING_CELLS including INV/BUF (4 drives), NAND2/3, NOR2/3, AND2, OR2, XOR2, XNOR2, MUX2, AOI21, OAI21, DFXTP, DFRTP, DFSTP, DLXTP, CLKBUF.
- `select_drive(lib, base_name, target_load_ff, target_delay_ns)`: picks the smallest drive strength meeting timing; falls back to largest if no drive is fast enough.
- 5x5 standard slew/load grid (0.01-0.5 ns slew × 0.5-10 fF load).

### Out of scope (v0.2.0)
- SPICE-driven characterization (mosfet-models + spice-engine across PVT).
- CCS (current-source) model.
- Liberty `.lib` text-format reader/writer.
- Variation-aware models (statistical / Monte Carlo).
- Multiple process/voltage/temperature corners populated.
