# Changelog

All notable changes to `xls-writer` are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/), and this project adheres to
[Semantic Versioning](https://semver.org/).

## [0.1.0] — 2026-07-03

### Added

- Initial release (**XLSW01**): a from-scratch, zero-third-party-dependency
  **writer** for the legacy `.xls` / BIFF8 format ([MS-XLS]). Emits BIFF records
  and wraps them in an OLE2 Compound File via the `cfb-writer` crate. Milestone
  **C4** (legacy write).
- Public API:
  - `Workbook::new()`, `Workbook::add_sheet(name) -> &mut Sheet`.
  - `Sheet::set_string(row, col, s)`, `Sheet::set_number(row, col, n)`.
  - `write_xls(&Workbook) -> Vec<u8>` producing `.xls` bytes.
- **BIFF records** emitted: `BOF`/`EOF` (substream framing), `BOUNDSHEET`,
  `SST`, `LABELSST` (string cells), `NUMBER` (numeric cells).
- **Substream layout**: one globals substream (BOUNDSHEET per sheet + SST) and
  one worksheet substream per sheet.
- **Shared strings**: identical string values are de-duplicated into one SST
  entry; `cstTotal`/`cstUnique` tracked correctly; `LABELSST` references by
  index.
- **`BOUNDSHEET.lbPlyPos` two-pass**: worksheet BOF offsets are backfilled into
  each BOUNDSHEET after all substreams are sized, so `lbPlyPos` is exact.
- **String encoding**: per-string `fHighByte` choice — compact 8-bit for
  all-Latin-1 strings, 16-bit UTF-16LE otherwise. Applies to SST strings
  (`XLUnicodeRichExtendedString`) and sheet names (`ShortXLUnicodeString`).
- **Robustness**: `#![forbid(unsafe_code)]`; no `unwrap`/`expect`/`panic!` on the
  public path; `checked`/`try_from` guards on every `u16`/`u32` size or address
  field; deterministic output. Cells beyond the `u16` grid and overflowing SST
  bodies are clamped/skipped (documented) rather than corrupting a record.

### Tests

- **Round-trip proof**: build a workbook, `write_xls`, re-open with the `cfb`
  reader, extract the `Workbook` stream, and walk BIFF records to assert the
  BOUNDSHEET name + `lbPlyPos` (→ worksheet BOF), the SST contents, and every
  cell's address/type/value.
- Unit tests: SST dedup (`cstTotal=2`/`cstUnique=1`), non-ASCII forces the
  16-bit path, Latin-1 stays compressed, multiple sheets get distinct
  `lbPlyPos`, empty workbook / empty sheet don't panic, out-of-range cells are
  skipped, exact `f64` preservation, deterministic output.

### Known limitations

- No `CONTINUE`-record splitting of a huge SST (kept small; clamped otherwise).
- Numeric cells always use `NUMBER`, never the space-saving `RK` record.
