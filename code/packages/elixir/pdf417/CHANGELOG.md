# Changelog

All notable changes to `coding_adventures_pdf417` are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
This project uses [Semantic Versioning](https://semver.org/).

## [0.1.0] - 2026-04-26

### Added

- Initial release of the Elixir PDF417 encoder.
- `CodingAdventures.PDF417` module with `encode/1` and `encode/2` public API.
- `CodingAdventures.PDF417.ModuleGrid` struct — `%ModuleGrid{rows, cols, modules}`.
- GF(929) arithmetic — exp/log tables for α=3 built at compile time as module attributes.
- Reed-Solomon ECC — b=3 convention; `build_generator/1` and `rs_encode/2` over GF(929).
- Byte compaction — `byte_compact/1` emitting codeword 924 latch then 6-bytes→5-codewords base-900 groups.
- Auto ECC level selection — thresholds from ISO/IEC 15438 recommendation table.
- Dimension selection — `choose_dimensions/1` targeting a roughly square symbol.
- Row indicator computation — `compute_lri/4` and `compute_rri/4` using the three-cluster formulas.
- Pattern expansion — `expand_widths/1` converting width lists to boolean module sequences.
- Rasterization — `rasterize/5` producing the final `%ModuleGrid{}`.
- No runtime dependencies — returns a plain struct, no barcode_2d dependency needed.
- Test suite with >80% coverage covering all ECC levels, dimension selection, determinism, and error cases.
