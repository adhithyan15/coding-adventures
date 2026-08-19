# Changelog

All notable changes to this package are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] — 2026-08-19

### Added

- Initial implementation of the browser/LastPass-style CSV format adapter
  named by VLT15's reuse map and VLT-PM00 §23 item 13
  (`code/specs/VLT-PM49-cli-external-import.md`).
- `CsvLoginImporter` — implements `vault-import-export`'s `Importer`
  trait (`name()` returns `"browser-csv"`).
- `decode(&[u8]) -> Result<Vec<PortableRecord>, ImportError>` — the free
  function the trait impl calls, exposed directly for callers that don't
  need a trait object.
- Case-insensitive header-alias resolution covering Chrome/Edge/Brave,
  Firefox, LastPass, and Bitwarden CSV column names in one adapter
  instead of one crate per vendor.
- Title fallback chain (title column -> url -> username -> generated
  `"Imported login N"`) so exports with no title column (Firefox) still
  produce a usable item.
- Delegates all CSV structural parsing (quoting, embedded commas/
  newlines, `""` escaping, ragged rows) to this repository's existing
  RFC 4180 `csv-parser` crate rather than writing new CSV-syntax parsing.
- Bounded: `MAX_SOURCE_BYTES` (32 MiB), `MAX_ROWS` (200,000),
  `MAX_COLUMNS` (256), `MAX_FIELD_LEN` (64 KiB).
- 20 unit tests: happy-path decode for each vendor's column shape,
  case-insensitive/whitespace-trimmed header matching, embedded comma/
  newline preservation, multi-row decoding, an explicit CSV
  formula-injection test proving `=`/`+`/`-`/`@`-prefixed payloads round-
  trip as inert literal text, and an adversarial matrix — empty input,
  oversize input, invalid UTF-8, unclosed quote, header-only file,
  ragged short/long rows, over-bound rows/columns/field-length, and
  `Send + Sync`.
- `#![forbid(unsafe_code)]` + `#![deny(missing_docs)]`.

### Out of scope (documented, not silently dropped)

- Non-login record kinds (secure notes, cards) — no CSV shape in the
  supported vendor set carries them; use `vault-import-bitwarden` for
  Bitwarden's JSON export instead.
- CSV export / writing, and therefore the formula-injection
  *neutralization* an export path would need — there is no writer yet
  for that mitigation to belong to.
