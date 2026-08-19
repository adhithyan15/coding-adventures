# Changelog

All notable changes to this package are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] — 2026-08-19

### Added

- Initial implementation of the Bitwarden JSON format adapter named by
  VLT15's reuse map and VLT-PM00 §23 item 13
  (`code/specs/VLT-PM49-cli-external-import.md`).
- `BitwardenJsonImporter` — implements `vault-import-export`'s `Importer`
  trait (`name()` returns `"bitwarden-json"`).
- `decode(&[u8]) -> Result<Vec<PortableRecord>, ImportError>` — the free
  function the trait impl calls, exposed directly for callers that don't
  need a trait object.
- Maps Bitwarden `type: 1` (login) to `PortableRecordKind::Login`, `2`
  (secure note) to `SecureNote`, `3` (card) to `Card` (fields carried in
  `custom_fields`), and `4` (identity) or any unrecognized type to
  `Custom("bitwarden-type-N")` rather than dropping the record.
- A login's TOTP seed becomes a second, separate `Totp` record, because
  vault-pm's own `Login` record has no TOTP slot.
- A login's extra `uris[]` entries beyond the first are kept as
  `custom_fields["uri_2"]`, `["uri_3"]`, … instead of silently dropped.
- Bounded parsing built on this repo's existing depth-capped
  `json-lexer`/`json-parser`/`json-value` pipeline rather than a new
  hand-rolled JSON decoder: `MAX_SOURCE_BYTES` (64 MiB), `MAX_ITEMS`
  (50,000), `MAX_URIS_PER_LOGIN` (32), `MAX_CUSTOM_FIELDS_PER_ITEM` (64),
  `MAX_FIELD_LEN` (64 KiB).
- 27 unit tests: happy-path decode of each mapped kind, TOTP-splits-into-
  two-records, extra-URIs-preserved, card-field mapping, identity/unknown-
  type preservation as `Custom`, custom-field decode and override
  precedence, and a broad adversarial matrix — empty input, oversize
  input, invalid UTF-8, malformed JSON, non-object root, missing/wrong-
  typed `items`, non-object item, missing/empty `name`, missing/wrong-
  typed `type`, non-object `login`, over-bound items/URIs/custom-fields/
  field-length, duplicate-key last-write-wins (both nested and top-level),
  a 10,000-deep nested array proving the inherited depth cap prevents a
  stack overflow instead of only being cited, null-vs-absent field
  handling, and `Send + Sync`.
- `#![forbid(unsafe_code)]` + `#![deny(missing_docs)]`.

### Out of scope (documented, not silently dropped)

- The Bitwarden *encrypted* JSON export variant.
- Folder/collection assignment and the `favorite` flag.
- Attachment bytes (the export carries only metadata).
