# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-12

### Added

- Initial pure-ISO C17 port of the Rust `fpga-bitstream` crate: an iCE40
  IceStorm record-stream emitter (structurally correct, stub-zero CRAM).
- `fpga_part_specs`, `FpgaClbConfig` / `fpga_clb_config_default`, `FpgaConfig`
  (`new` / `free` / `insert_clb` overwriting a duplicate key / `clb_count`),
  `fpga_emit_bitstream` (malloc'd bytes + report), `fpga_cmd` (record builder),
  and `fpga_write_bin` (file output via `<stdio.h>`).
- `emit` sorts the CLBs by `(row, col)`, so the byte stream is deterministic
  regardless of insertion order; growable buffers guard against `size_t`
  overflow; the Rust `cmd` payload>253 panic becomes a NULL / status return.
- Tests via the shared `iso-harness` (GCC, Clang, MSVC) assert the exact bytes
  captured from the real Rust crate (empty Hx1k stream, one-CLB framing, the CRC
  and end marker), plus determinism, overwrite semantics, and the cmd builder.
