# Changelog — engram-anki-package

## Unreleased

### Started sqlite-file reader cutover for Anki V11 imports

`parse_v11_collection_bytes` now reads the `col`, `notes`, `cards`, `revlog`,
and `graves` tables directly from raw SQLite bytes through the repo
`sqlite-file` reader instead of deserializing the collection into an in-memory
`rusqlite` connection. This removes the unsafe serialized-rusqlite import path
(`sqlite3_malloc64` + `OwnedData::from_raw_nonnull`) while keeping the existing
owned Anki V11 representation and APKG round-trip tests unchanged.

`rusqlite` remains in this crate for the SQLite writer and test fixtures until
Phase F of the zero-dependency roadmap replaces collection export as well.

### Removed third-party `prost` — Anki `meta`/`media` protobuf is now zero-dep

The Anki `.apkg` `meta` (package version) and `media` (filename/size/sha1 map)
messages were encoded/decoded via the third-party `prost` derive. They are now
hand-coded against the repo's zero-dependency `protobuf` wire crate
(`code/packages/rust/protobuf`), removing `prost` from this crate's dependencies.

The hand-coded `encode_pb`/`decode_pb` implementations follow proto3 semantics
(implicit-presence scalar fields omitted at their default; explicit-`optional`
`legacy_zip_filename` emitted when `Some`). Before the cutover, a cross-compat
gate asserted they produce **byte-for-byte identical output to `prost`** and
round-trip its bytes across edge cases (empty maps, zero-size entries, `Some(0)`
optionals, non-ASCII filenames, multi-entry maps) — guaranteeing continued
real-Anki `.anki21b` interoperability. All 41 crate tests, including the APKG
round-trip and the checked-in golden fixture, pass unchanged.

Phase A of the Engram zero-dependency plan
(`code/specs/engram-zero-dep-plan.md`). Remaining third-party deps in this crate
(`rusqlite`, `zstd_crate`, `serde`/`serde_json`) are removed in later phases.
