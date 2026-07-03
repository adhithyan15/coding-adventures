# Changelog — fpga-bitstream

## [0.1.0] — 2026-06-13

### Added

- `Ice40Part` enum — Hx1k, Hx8k, Up5k, Lp1k.
- `PART_SPECS` — compile-time table of (part, rows, cols, cram_size) for all supported parts.
- `part_specs(part)` — convenience function returning `(rows, cols, cram_size)`.
- `ClbConfig` — per-CLB configuration: two 16-entry LUT truth tables + two flip-flop enable flags.
- `FpgaConfig` — top-level config: part + `HashMap<(row, col), ClbConfig>`.
- `emit_bitstream(config)` — produce a `Vec<u8>` in Project IceStorm record-stream format.
- `write_bin(path, config)` — write the bitstream to a file.
- `cmd(command, payload)` — low-level record builder; panics if payload exceeds 253 bytes.
- `BitstreamReport` — reports `part`, `bytes_written`, `clb_count`, and `cram_size`.
- 12 integration tests + 1 doctest covering part specs, stream format, CLB count, file I/O,
  panic on oversized payload, and a 4-bit adder smoke test.
