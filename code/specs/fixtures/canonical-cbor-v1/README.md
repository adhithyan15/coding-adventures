# Canonical CBOR v1 conformance fixture

This directory is the language-neutral executable corpus for CBR01. The JSON
source owns the deterministic length-first profile, portable limits, exact wire
vectors, and stable error identifiers. The fixture performs no I/O beyond test
loading and grants no runtime capability to the codec.

## Files

- `schema.json` validates the closed fixture envelope.
- `cases.json` is the normative corpus.
- `canonical_cbor_vectors.h` is a generated dependency-free projection for the
  C, C++, and Rust reference consumers.
- `CHANGELOG.md` records contract changes.

The repository test `code/scripts/tests/test_canonical_cbor_fixtures.py`
validates the schema, IDs, hex, generated-case grammar, stable error set, and
byte-for-byte projection. Edit `cases.json` first, then update the projection to
the exact text the test derives.

## Operations

- `round-trip`: decode canonical `input`, re-encode it, and compare exact hex.
- `decode-error`: decode `input` and compare the stable error ID.
- `encode-map`: decode the semicolon-delimited `key=value` fragments, construct
  the map in that order, and compare the exact canonical bytes.
- `generated-round-trip`: construct the bounded value described by `input` and
  compare the exact generated wire described by `expected`.
- `encode-error`: construct the hostile value described by `input` and compare
  the checked encoder error ID.

Generated specifications are intentionally tiny and closed:

- `nested-array:<depth>`
- `bytes-repeat:<length>:<two-hex-digit-byte>`
- `wire:nested-array:<depth>`
- `wire:bytes-repeat:<length>:<two-hex-digit-byte>`

No consumer evaluates code, follows paths, or accepts an extension operation.
