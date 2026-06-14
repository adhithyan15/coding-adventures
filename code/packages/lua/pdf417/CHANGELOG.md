# Changelog

## 0.1.0 — 2026-04-27

### Added
- Initial implementation of PDF417 encoder (ISO/IEC 15438:2015).
- Byte compaction mode (codeword 924 latch + 6-bytes-to-5-codewords base-900).
- GF(929) Reed-Solomon ECC (b=3 convention, α=3, levels 0–8 + auto-select).
- Auto dimension selection (roughly square symbol, rows 3–90, cols 1–30).
- Row indicator encoding (LRI + RRI per row).
- Cluster table lookup (codeword → 17-module bar/space pattern).
- Start/stop patterns per row.
- `M.encode(data, opts?)` — encodes string to ModuleGrid table.
