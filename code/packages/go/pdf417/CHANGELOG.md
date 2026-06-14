# Changelog

## 0.1.0 — 2026-04-27

### Added
- Initial implementation of PDF417 encoder (ISO/IEC 15438:2015).
- Byte compaction mode (codeword 924 latch + 6-bytes-to-5-codewords base-900).
- Reed-Solomon ECC over GF(929) with b=3 convention (levels 0–8 + auto-select).
- Auto dimension selection (roughly square symbol).
- Row indicator encoding (LRI + RRI per row carrying R/C/ECC metadata).
- Cluster table lookup (codeword → 17-module bar/space pattern).
- Start/stop patterns per row.
- `Encode(string) *ModuleGrid` convenience wrapper.
- `EncodeBytes([]byte, Options) (*ModuleGrid, error)` with full options.
- `EncodeToScene([]byte, Options, *LayoutConfig) (PaintScene, error)` layout wrapper.
