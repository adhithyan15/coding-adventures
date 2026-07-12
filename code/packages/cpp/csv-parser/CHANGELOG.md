# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Pure ISO C++17, header-only port of the Rust `csv-parser` crate, in namespace
  `ca::csv`: an RFC 4180 CSV parser built on the same four-state machine.
- `parse(source, delimiter=',')` → `std::vector<ca::csv::Record>` (each `Record`
  a `std::map<std::string,std::string>` keyed by header; missing columns read as
  `""`, a repeated header keeps the last column's value). `parse_records` →
  `ca::csv::Grid` (`std::vector<std::vector<std::string>>`), header included.
- Handles quoted fields, escaped `""`, embedded delimiters/newlines, ragged rows
  (pad short with `""`, drop extra), `\n` / `\r` / `\r\n`, and an optional
  trailing newline; throws `ca::csv::UnclosedQuote` on an unterminated quote.
- Byte-oriented state machine; multibyte UTF-8 content preserved verbatim.
- Tests use the crate's own cases (quoting, escaping, ragged rows, empty and
  header-only files, CR/LF/CRLF, TSV/`;`/`|`, unclosed-quote, UTF-8) under GCC
  and Clang via `iso-harness`.
