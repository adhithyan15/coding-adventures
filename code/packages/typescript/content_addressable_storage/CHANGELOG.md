# Changelog — @coding-adventures/content-addressable-storage

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-04-12

### Added

- `BlobStore` interface: `put`, `get`, `exists`, `keysWithPrefix` — the pluggable
  storage backend contract. Any class implementing this interface works with
  `ContentAddressableStore`.
- `ContentAddressableStore<S extends BlobStore>`: wraps a `BlobStore` and adds
  automatic SHA-1 keying, integrity verification on read, and hex prefix resolution.
- `LocalDiskStore`: filesystem backend using Git's 2/38 fanout layout
  (`root/XX/38-hex-chars`). Writes are atomic (temp file + `renameSync`).
- `CasError` class hierarchy:
  - `CasNotFoundError` — key absent from store
  - `CasCorruptedError` — stored bytes don't hash to the key
  - `CasAmbiguousPrefixError` — hex prefix matched 2+ objects
  - `CasPrefixNotFoundError` — hex prefix matched 0 objects
  - `CasInvalidPrefixError` — hex prefix string is malformed (empty or non-hex)
- Hex utilities: `keyToHex`, `hexToKey`, `decodeHexPrefix` (with nibble padding
  for odd-length prefixes, matching Git's abbreviated-hash behaviour).
- Full test suite using vitest: >95% line coverage. Tests cover round-trips for
  empty/small/1 MiB blobs, idempotent puts, not-found and corrupted errors,
  exists before/after put, all `findByPrefix` error cases, 2/38 path layout
  verification, and a minimal in-memory `BlobStore` implementation as a
  compile-time interface correctness test.
- Depends on `@coding-adventures/sha1` for the SHA-1 hash function, consistent
  with the Rust implementation that depends on `coding-adventures-sha1`.
