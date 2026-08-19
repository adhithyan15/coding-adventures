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
- Bounded: `MAX_SOURCE_BYTES` (16 MiB), `MAX_ROWS` (200,000),
  `MAX_COLUMNS` (256), `MAX_FIELD_LEN` (64 KiB).
- 22 unit tests: happy-path decode for each vendor's column shape,
  case-insensitive/whitespace-trimmed header matching, embedded comma/
  newline preservation, multi-row decoding, an explicit CSV
  formula-injection test proving `=`/`+`/`-`/`@`-prefixed payloads round-
  trip as inert literal text, and an adversarial matrix — empty input,
  oversize input, invalid UTF-8, unclosed quote, header-only file,
  ragged short/long rows, over-bound rows/columns/field-length, and
  `Send + Sync`.
- `#![forbid(unsafe_code)]` + `#![deny(missing_docs)]`.

### Security hardening (pre-merge review)

Three findings flagged across four review rounds before push, all fixed
inline:

- **MEDIUM** — `MAX_COLUMNS` could only reject a wide row after
  `csv-parser` had already fully materialized it into a `HashMap`, so a
  crafted file within the byte cap could amplify past its raw size before
  that check ran. Mitigated by lowering `MAX_SOURCE_BYTES` from an
  earlier 32 MiB draft to 16 MiB, directly shrinking the worst case (real
  exports are low single-digit megabytes).
- **LOW/MEDIUM** — every parsed CSV cell value lived in an ordinary,
  non-zeroizing `String` inside `rows: Vec<HashMap<String, String>>`
  until the function returned, so a password or TOTP seed already
  extracted into a `Zeroizing` `PortableRecord` field left an unwiped
  copy in freed heap. Fixed by zeroizing every cell value in `rows` in
  place before it drops.
- **MEDIUM** — that zeroize pass initially ran only after every row
  decoded successfully, so a malformed row partway through the file (an
  over-`MAX_COLUMNS` row, an over-`MAX_FIELD_LEN` cell) skipped the wipe
  for every row already decoded before it — exactly the adversarial
  input this crate's threat model exists to survive. Fixed by capturing
  the decode loop's result and running the zeroize pass unconditionally,
  on both its `Ok` and `Err` paths, before propagating it.

### Out of scope (documented, not silently dropped)

- Non-login record kinds (secure notes, cards) — no CSV shape in the
  supported vendor set carries them; use `vault-import-bitwarden` for
  Bitwarden's JSON export instead.
- CSV export / writing, and therefore the formula-injection
  *neutralization* an export path would need — there is no writer yet
  for that mitigation to belong to.
