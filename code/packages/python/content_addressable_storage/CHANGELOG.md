# Changelog

All notable changes to `coding-adventures-content-addressable-storage` will be documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
This project adheres to [Semantic Versioning](https://semver.org/).

## [0.1.0] — 2026-04-12

### Added

- `BlobStore` abstract base class with four abstract methods: `put`, `get`,
  `exists`, `keys_with_prefix`. Mirrors the Rust `BlobStore` trait.
- `ContentAddressableStore` class wrapping any `BlobStore`:
  - `put(data)` — SHA-1 hashes `data` and delegates to the backend.
  - `get(key)` — fetches from the backend and re-hashes for integrity.
  - `exists(key)` — thin delegation to the backend.
  - `find_by_prefix(hex_prefix)` — decodes abbreviated hex, scans via the
    backend, raises on ambiguity or no match.
  - `store` property — exposes the underlying `BlobStore`.
- `LocalDiskStore` filesystem backend using Git-style 2/38 fanout layout:
  - Atomic writes via temp file (PID + nanosecond timestamp suffix) +
    `os.replace()`.
  - Idempotent `put`: short-circuits if the object file already exists.
  - `keys_with_prefix` scans the relevant 2-char bucket directory, skips
    non-38-char files (temp files, artifacts).
- Typed exception hierarchy rooted at `CasError`:
  - `CasStoreError` — backend I/O failure.
  - `CasNotFoundError` — key absent from store.
  - `CasCorruptedError` — stored bytes hash mismatch.
  - `CasAmbiguousPrefixError` — prefix matches two or more keys.
  - `CasPrefixNotFoundError` — prefix matches zero keys.
  - `CasInvalidPrefixError` — empty string or non-hex characters.
- `key_to_hex` / `hex_to_key` utility functions.
- `_decode_hex_prefix` internal utility: right-pads odd-length hex strings
  before byte-decoding (nibble-prefix semantics, same as Git).
- Full type annotations; `mypy --strict` clean.
- Ruff lint passing (`E`, `W`, `F`, `I`, `UP`, `B`, `SIM`, `ANN`).
- Test suite in `tests/test_cas.py` covering:
  - Empty, small, and 1 MB round-trips.
  - Idempotent put.
  - `CasNotFoundError` on missing key.
  - `CasCorruptedError` on tampered file.
  - `exists()` before/after put.
  - `find_by_prefix` with unique, ambiguous, not-found, invalid, and empty
    prefix inputs.
  - 2/38 fanout directory creation and 38-char filename verification.
  - `BlobStore` abstractness (cannot instantiate directly or partially).
  - Error hierarchy subclass relationships and attribute presence.
  - `keys_with_prefix` edge cases (empty prefix, missing bucket, temp files).

### Port notes

Ported from the Rust `coding-adventures-content-addressable-storage` implementation. Key divergences:

- Python uses `hashlib.sha1()` directly (stdlib) rather than depending on
  `coding-adventures-sha1`. The Rust version needs its own SHA-1 because
  Rust's standard library has no built-in hash functions.
- Rust's generic `BlobStore::Error` associated type maps to plain Python
  exceptions (no parameterized generics needed).
- Rust's `[u8; 20]` fixed-size array becomes `bytes` (always 20 bytes).
- `match` statement used for `find_by_prefix` branching (Python 3.10+).
