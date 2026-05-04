# Changelog

All notable changes to `coding_adventures_pdf417` will be documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] — 2026-04-26

### Added

- Initial release of the pure-Ruby PDF417 encoder.
- **Byte compaction** — codeword 924 latch with 6-bytes-to-5-codewords
  base-900 packing (`byte_compact`).
- **GF(929) finite field arithmetic** — exp/log tables built at module load
  time for O(1) multiplication (`gf_mul`, `gf_add`, `GF_EXP`, `GF_LOG`).
- **Reed-Solomon ECC** — b=3 convention generator polynomial (`build_generator`)
  and LFSR encoder (`rs_encode`) over GF(929).
- **Auto ECC level selection** — ISO/IEC 15438:2015 recommendation table
  (`auto_ecc_level`): levels 2–6 based on data codeword count.
- **Dimension auto-selection** — roughly square symbol heuristic
  (`choose_dimensions`): rows ∈ [3, 90], cols ∈ [1, 30].
- **Row indicators** — left (LRI) and right (RRI) row indicator computation
  (`compute_lri`, `compute_rri`) encoding R/C/ECC metadata per row.
- **Cluster table lookup** — three 929-entry cluster tables included
  (`CLUSTER_TABLES`) for codeword-to-17-module expansion.
- **Pattern expansion** — packed 32-bit codeword → flat boolean module array
  (`expand_pattern`, `expand_widths`).
- **Rasterisation** — full row layout with start/stop guards, LRI/RRI,
  data codewords, and `row_height` repetition (`rasterize`).
- **Public API** — `CodingAdventures::PDF417.encode(data, opts = {})` returns
  a `ModuleGrid` struct with `.rows`, `.cols`, `.modules` fields.
- **Options** — `ecc_level:` (0–8), `columns:` (1–30), `row_height:` (≥ 1).
- **Error hierarchy** — `PDF417Error`, `InputTooLongError`,
  `InvalidDimensionsError`, `InvalidECCLevelError` (all in `errors.rb`).
- **Comprehensive RSpec test suite** — covers constants, GF arithmetic,
  RS encoding, byte compaction, dimension selection, row indicators, pattern
  expansion, and the full `encode` API with 50+ test cases.
- **Zero runtime dependencies** — fully self-contained gem.
- **standardrb compatible** — frozen string literals throughout; all files
  pass `standardrb --no-fix`.

### Implementation notes

- Ported from the Lua reference implementation
  (`code/packages/lua/pdf417/src/coding_adventures/pdf417/init.lua`) and
  verified against the Go implementation
  (`code/packages/go/pdf417/pdf417.go`).
- Cluster tables were pre-generated from the TypeScript reference and are
  shared with the Go and Lua packages.
- Ruby integers are arbitrary-precision so 48-bit byte-compaction arithmetic
  requires no special handling.
- v0.2.0 will add text and numeric compaction modes for shorter codeword
  sequences on ASCII and digit inputs.
