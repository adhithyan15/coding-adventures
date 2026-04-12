# Changelog

## [0.1.0] - 2026-04-12

### Added
- `BlobStore` trait: pluggable backend interface with `put`, `get`, `exists`, `keys_with_prefix`
- `ContentAddressableStore<S>`: generic CAS wrapper that hashes with SHA-1, verifies integrity on read, and resolves abbreviated hex prefixes
- `LocalDiskStore`: filesystem backend using Git-style 2/38 fanout layout with atomic rename writes
- `CasError<E>`: typed error enum covering `NotFound`, `Corrupted`, `AmbiguousPrefix`, `PrefixNotFound`, `InvalidPrefix`, and `Store(E)`
- Utility functions `key_to_hex` and `hex_to_key` for converting between 20-byte keys and hex strings
