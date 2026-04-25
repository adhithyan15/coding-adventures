# Changelog

## [0.1.0] — Unreleased

### Added
- `Iice40Part` enum: HX1K, HX8K, UP5K, LP1K.
- `PART_SPECS`: per-part rows/cols/cram_bits dimensions.
- `FpgaConfig(part, clbs)` and `ClbConfig(lut_a/b_truth_table, ff_a/b_enabled)`.
- `emit_bitstream(config) -> (bytes, BitstreamReport)`.
- `write_bin(path, config) -> BitstreamReport`.
- Record-stream format: preamble (0xff 0x00), CRAM commands (RESET/BANK/OFFSET/DATA), CRC placeholder, end marker (0xffff).
- BitstreamReport: part, bytes_written, clb_count, cram_size.

### Important caveat
- CRAM bit positions are stubbed (zero-padded) in v0.1.0. To program real silicon, integrate Project IceStorm's chipdb. The real-fpga-export package's yosys/nextpnr/icepack path produces working bitstreams today.

### Out of scope (v0.2.0)
- Project IceStorm chipdb integration (real CRAM bit mapping).
- ECP5 (Project Trellis), Xilinx 7-series (Project X-Ray).
- Bitstream encryption / authentication.
- Partial reconfiguration.
