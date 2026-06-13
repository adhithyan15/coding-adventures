# Changelog — fpga-place-route-bridge

## [0.1.0] — 2026-06-13

### Added

- `truth_table(cell_type)` — returns `(&[&str], &[u8])` pin names + truth table for a known cell.
- `truth_table_types()` — list of all supported cell type names.
- `hnl_to_fpga_json(netlist, options)` — convert HNL gate netlist to FPGA placement JSON.
- `FpgaBridgeOptions` — configurable grid dimensions (rows × cols), LUT input width, RNG seed.
- `FpgaBridgeReport` — reports `cells_packed`, `cells_unmapped`, and `routes_emitted`.
- Row-major CLB placement: cell `i` maps to row `i / cols`, column `i % cols`.
- LUT truth-table expansion from native cell arity to target LUT width via entry repetition.
- I/O pin entries emitted for every module port in the top module.
- Routing stubs emitted for every instance connection.
- 20 supported cell types: BUF, NOT, AND2/3/4, OR2/3/4, NAND2/3/4, NOR2/3/4, XOR2/3,
  XNOR2, MUX2, CONST_0, CONST_1.
- 15 integration tests covering all truth tables, packing, expansion, routing, and I/O emission.
