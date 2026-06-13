# Changelog — real-fpga-export

## [0.1.0] — 2026-06-13

### Added

- `write_verilog_str(hir)` — convert an `Hir` to an IEEE 1364-2005 structural Verilog string.
- `write_verilog(hir, path)` — write Verilog to a file, creating parent directories.
- `to_ice40(hir, top, pcf, out_dir, part, package, opts, skip_missing)` — drive the full
  yosys → nextpnr-ice40 → icepack pipeline for iCE40 FPGAs.
- `program_ice40(bin_path, opts)` — flash a `.bin` bitstream with `iceprog`.
- `ToolchainOptions` / `ToolchainResult` — typed wrappers for tool paths, timeouts, and
  intermediate artefact paths.
- Identifier escaping: all IEEE 1364-2005 reserved words are emitted as `\name ` escaped
  identifiers so output is always legal Verilog.
- `skip_missing` flag to gracefully short-circuit the flow when tools are absent (CI-friendly).
- 17 integration tests + 1 doctest; 100% of public API exercised.
