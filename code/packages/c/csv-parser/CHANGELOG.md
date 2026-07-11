# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Pure ISO C17 port of the Rust `csv-parser` crate: an RFC 4180 CSV parser built
  on the same four-state machine (FieldStart / InUnquoted / InQuoted /
  InQuotedMaybeEnd).
- Two views: `csv_parse` / `csv_parse_with_delimiter` produce a header-mapped
  `CsvTable` (with `csv_table_get` — missing columns read as `""`, a repeated
  header keeps the last column), and `csv_parse_records` produces the raw
  `CsvGrid`.
- Handles quoted fields, escaped `""`, embedded delimiters/newlines, ragged rows
  (pad short with `""`, drop extra), `\n` / `\r` / `\r\n`, and an optional
  trailing newline. `CSV_ERR_UNCLOSED_QUOTE` on an unterminated quote;
  `CSV_ERR_ALLOC` on out-of-memory. `csv_grid_free` / `csv_table_free` release
  the results.
- Byte-oriented state machine (multibyte UTF-8 preserved verbatim); all growable
  buffers use overflow-guarded doubling / checked multiplies.
- Tests use the crate's own cases (quoting, escaping, ragged rows, empty and
  header-only files, CR/LF/CRLF, TSV/`;`/`|`, unclosed-quote, UTF-8) under GCC
  and Clang via `iso-harness`.
