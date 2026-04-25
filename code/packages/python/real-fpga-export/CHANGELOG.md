# Changelog

## [0.1.0] — Unreleased

### Added
- `write_verilog(hir, path)`: emit a Verilog `.v` file from an HIR document.
- `write_verilog_str(hir)`: same but returns the source as a string.
- HIR -> Verilog backwriter:
  - Modules, ports (input/output/inout), nets, continuous assignments, instances.
  - Expression nodes: Lit, NetRef, PortRef, VarRef, Slice, Concat, Replication, UnaryOp, BinaryOp, Ternary.
  - Verilog reserved-word escaping (`\name ` syntax).
- `to_ice40(hir, top, pcf, ...)`: shell-out driver running `yosys` (synth_ice40) -> `nextpnr-ice40` -> `icepack`. Returns `ToolchainResult` with `verilog_path`, `json_path`, `asc_path`, `bin_path`, `log_lines`.
- `program_ice40(bin_path)`: shell out to `iceprog` to flash a real board.
- `skip_missing` option: bail after Verilog emission if external tools aren't on PATH (testing).

### Out of scope (v0.2.0)
- Behavioral process emission (`always @(...)`).
- ECP5 / Xilinx 7-series flows.
- EDIF backwriter.
- Auto-generated PCF from port lists.
