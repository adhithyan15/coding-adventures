# Changelog — gdsii-writer

## [0.1.0] — 2026-06-13

### Added
- `GdsWriter`, `GdsCell`, `GdsBoundary`, `GdsPath`, `GdsSref`, `GdsText` — GDSII object model.
- `GdsWriter::encode()` — serializes the full library to a GDSII binary stream.
- `stream` module — low-level record builders: `record()`, `rec_int2()`, `rec_int4()`, `rec_string()`, `rec_units()`, `rec_timestamp()`, `rec_bgnstr()`.
- `double_to_gds_real()` — 8-byte Calma base-16 floating-point encoding.
- `um_to_dbu()` — converts µm to database units (× 1000).
- Record type constants for all used GDS record types.
- 14 integration tests + 1 doc-test.
