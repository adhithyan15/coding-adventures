# Changelog

## [0.1.0] - 2026-04-12

### Added

- `BlobStore` interface: `Put`, `Get`, `Exists`, `KeysWithPrefix` — the single
  abstraction over any byte-blob backend.
- `ContentAddressableStore[S BlobStore]` — generic CAS wrapper that automatically
  keys content by SHA-1, verifies integrity on every read, and resolves
  abbreviated hex prefixes to full 20-byte keys.
- `LocalDiskStore` — filesystem backend using Git's 2/38 fanout layout
  (`<root>/<xx>/<38-hex-chars>`). Atomic writes via PID+timestamp temp file and
  `os.Rename`.
- `MemStore` — in-memory backend backed by a `map[[20]byte][]byte`, useful for
  tests and as a reference implementation of `BlobStore`.
- `CasError` typed error with `ErrKind` discriminator covering: `ErrKindStore`,
  `ErrKindNotFound`, `ErrKindCorrupted`, `ErrKindAmbiguous`,
  `ErrKindPrefixMissing`, `ErrKindInvalidPrefix`.
- `KeyToHex` / `HexToKey` hex utility functions.
- `ErrNotFound` sentinel error for `BlobStore.Get`.
- Comprehensive test suite: round-trip (empty, small, 1 MiB), idempotent put,
  not-found, corrupted file, exists before/after, FindByPrefix (unique,
  ambiguous, not-found, invalid, empty, odd-length), 2/38 path layout
  verification, interface boxing, deduplication, known SHA-1 vectors, error
  unwrapping.
- Port of the Rust `coding-adventures-content-addressable-storage` crate, following the shared
  `code/specs/cas.md` specification.
