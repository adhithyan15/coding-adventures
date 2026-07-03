# Changelog

All notable changes to the `xls` crate are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/), and this
project adheres to Semantic Versioning.

## [0.1.0] — 2026-07-03

### Added

- Initial release (milestone **B1** of the OOXML effort): a from-scratch,
  zero-third-party-dependency reader for legacy `.xls` (BIFF8 / [MS-XLS])
  workbooks, layered on the `cfb` OLE2 container reader.
  `#![forbid(unsafe_code)]`.
- **Container layering**: opens the OLE2 compound file via `cfb`, reads the
  `Workbook` stream (falling back to `Book` for very old files).
- **BIFF record framing**: walks back-to-back `u16 type` / `u16 size` / body
  records, grouped into `BOF`/`EOF` substreams; classifies the workbook-globals
  (`0x0005`) and worksheet (`0x0010`) substreams.
- **Globals parsing**: the shared string table (`SST`) and the sheet directory
  (`BOUNDSHEET`, matching each sheet's `lbPlyPos` to its worksheet BOF offset).
- **Cell records**: `LABELSST`, `RK` (all four flag combinations, incl. signed
  30-bit integers and ÷100), `MULRK` (one cell per column), `NUMBER`, `LABEL`
  (inline strings, 8- and 16-bit), `BOOLERR` (boolean / error), `BLANK`, and
  `FORMULA` (cached result: numeric, boolean, error, empty, or a following
  `STRING` record).
- **CONTINUE handling**: a record-spanning SST reader that correctly resumes a
  string's character data across a `CONTINUE` boundary and re-reads the fresh
  `fHighByte` flag byte for the remainder (proven by a synthetic split-string
  unit test that flips 16-bit → 8-bit at the boundary).
- Public API: `open_xls`, `Workbook` (`sheets`, `sheet`), `Sheet` (`cells`,
  `cell`), `Cell`, `CellValue`, `XlsError` (`Display` + `Error` +
  `From<cfb::CfbError>`).
- **Security hardening** for untrusted input: bounds-checked reads, checked
  arithmetic, per-count safety caps (SST strings, CONTINUE chain length, MULRK
  span), no allocation driven by an unchecked declared count. Hostile inputs
  (non-CFB bytes, missing Workbook stream, truncated records, lying counts,
  MULRK underflow) yield clean typed errors — never a panic, hang, or huge
  allocation.
- End-to-end test against a real 5632-byte `xlwt` fixture (sheet "Revenue" with
  `LABELSST`, `RK`, and `FORMULA` cells) plus extensive unit tests.
- Spec: `code/specs/XLS01-biff-reader.md` — literate walkthrough of BIFF8, the
  CFB layering, and this crate's model.
