# Changelog

## [0.1.0] — Unreleased

### Added
- `TRUTH_TABLES`: lookup table mapping HNL primitive cell types to (input pin order, 2^k output values). Covers BUF, NOT, AND2-4, OR2-4, NAND2-4, NOR2-4, XOR2-3, XNOR2, MUX2, CONST_0, CONST_1.
- `hnl_to_fpga_json(netlist, options=FpgaBridgeOptions()) -> (dict, FpgaBridgeReport)`.
- `FpgaBridgeOptions`: rows, cols, lut_inputs, seed.
- `FpgaBridgeReport`: cells_packed, cells_unmapped, routes_emitted.
- One LUT per cell; placed in row-major order on the fabric grid.
- Auto-generated IO pins for top-level ports.
- Truth-table expansion to lut_inputs width.
- Routes emitted as {from, to} pairs per pin connection.

### Out of scope (v0.2.0)
- Multi-cell-per-CLB packing.
- SA-based placement.
- PathFinder routing.
- Sequential cell mapping (DFFs into CLB FFs).
- Block RAM mapping.
