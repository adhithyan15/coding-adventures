# Changelog

All notable changes to this package are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] — 2026-05-04

### Added

- Initial implementation of VLT15
  (`code/specs/VLT15-vault-import-export.md`).
- `PortableBundle` — top-level container with
  `version: u32` + `records: Vec<PortableRecord>`.
- `PortableRecord` — `kind / title / username / password /
  url / notes / totp_seed / tags / custom_fields`. Sensitive
  fields (`password`, `totp_seed`, every `custom_fields`
  value) held under `Zeroizing<String>`. Hand-rolled `Debug`
  redacts each sensitive field. Hand-rolled `Clone` since
  `Zeroizing<String>` doesn't derive it.
- `PortableRecordKind` — `#[non_exhaustive]` enum with five
  known variants (`Login`, `SecureNote`, `Card`, `SshKey`,
  `Totp`) + `Custom(String)` escape hatch.
- `Importer` trait — `Send + Sync`, object-safe, with
  `name()` + `import(&[u8]) -> Result<Vec<PortableRecord>>`.
- `PassthroughImporter` — reference adapter that round-trips
  the canonical bundle. Used by upper-tier integration tests.
- `export_to_bundle(records)` — hand-rolled deterministic JSON
  writer. `BTreeMap`-iteration sorted custom-field keys,
  fixed key order at every level, U+2028 / U+2029 escaped so
  output embedded in `<script>` is JS-safe.
- `import_from_bundle(bytes)` — hand-rolled strict JSON
  reader. Rejects unknown root keys, unknown record keys,
  unknown `kind` strings, trailing input, raw control chars
  inside strings, oversize fields, oversize record / tag /
  custom-field counts, and unsupported bundle versions.
- `ImportError` — `Decode` / `TooLarge` / `InvalidParameter`
  / `UnsupportedVersion(u32)` / `Adapter(String)`.
- 29 unit tests covering: round-trip of one login,
  round-trip of all five known kinds + a custom kind,
  deterministic export, passthrough importer round-trip,
  empty title rejection, oversize field / records / tags /
  custom-fields rejection, empty Custom kind label
  rejection, decoder rejection of unknown root key, unknown
  record key, unsupported version, trailing bytes, unknown
  kind value, oversize bundle bytes, raw control chars in
  strings, escape-character round-trip, U+2028 / U+2029
  round-trip, password / TOTP / custom-field redaction in
  Debug, Send + Sync compile-time check, empty bundle
  round-trip.
- `#![forbid(unsafe_code)]` + `#![deny(missing_docs)]`.

### Security hardening (pre-merge review)

Three findings flagged before push, all fixed inline:

- **HIGH** — `read_string` mis-decoded multi-byte UTF-8: a
  previous byte-by-byte `b as char` push would silently
  rewrite `é` (UTF-8 `0xC3 0xA9`) as two Latin-1 chars
  `U+00C3 U+00A9` instead of one `U+00E9`. For a password
  manager importing a Bitwarden/1Password export with
  non-ASCII passwords, this would silently corrupt the
  password and lock the user out of accounts. Fix:
  accumulate raw bytes through the loop and run a single
  `String::from_utf8` at the closing quote, rejecting
  malformed UTF-8 with `Decode("invalid UTF-8 in string")`.
  New regression test
  `round_trip_preserves_unicode_passwords` exercises
  passwords containing German, Japanese, Cyrillic, Greek,
  emoji, and Vietnamese diacritics.
- **LOW** — Permissive comma handling: missing or trailing
  commas at the bundle and record levels were silently
  accepted ("JSON parsing differences" attack surface).
  Tightened: between fields requires a comma, no trailing
  comma before `}`. Four new regression tests.
- **LOW** — Leading zeros in `read_u32` (`0123` → 123). RFC
  8259 forbids leading zeros on integer literals; rejected
  with `Decode("leading zero in number")`. New regression
  test `reject_leading_zero_in_version`.

### Bounds

`MAX_RECORDS = 100_000`, `MAX_FIELD_LEN = 64 KiB`,
`MAX_BUNDLE_LEN = 256 MiB`, `MAX_TAGS_PER_RECORD = 64`,
`MAX_CUSTOM_FIELDS_PER_RECORD = 64`, `BUNDLE_VERSION = 1`.

### Out of scope (future PRs)

- **Format adapters** — sibling crates per external format
  (`vault-import-bitwarden`, `vault-import-keepass`, etc).
- **Recipient list at export** — VLT04 wraps DEKs at export
  time; this crate hands plaintext records to the host which
  applies VLT04 before persistence.
- **Streaming reader** — current API is `&[u8] -> Bundle`;
  for very large bundles (~hundreds of MiB) a streaming
  reader / writer is future work.
- **Schema validation** — VLT02 typed-record validation runs
  on the host's import path after `import_from_bundle`
  returns. This crate enforces only structural bounds.
- **Compression** — host's responsibility before / after
  serialisation.
