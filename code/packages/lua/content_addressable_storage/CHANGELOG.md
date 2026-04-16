# Changelog — coding-adventures-content-addressable-storage (Lua)

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-04-12

### Added

- `BlobStore` abstract base class with four abstract methods:
  `put`, `get`, `exists`, `keys_with_prefix`.  Direct calls on the base class
  raise a Lua error to signal unimplemented subclass methods.

- `ContentAddressableStore` wrapping any `BlobStore`:
  - `put(data)` — SHA-1-hashes the data and delegates to the backend.
    Idempotent: the same content always yields the same key.
  - `get(key)` — fetches from the backend and re-hashes for integrity.
    Returns a `"corrupted"` error table if the stored bytes don't match.
  - `exists(key)` — forwards to the backend.
  - `find_by_prefix(hex)` — resolves 1–40 char abbreviated hex prefixes to
    full 20-byte keys.  Returns `"invalid_prefix"`, `"prefix_not_found"`, or
    `"ambiguous_prefix"` error tables as appropriate.
  - `inner()` — access the underlying BlobStore directly.

- `LocalDiskStore` filesystem backend:
  - Git-style 2/38 fanout layout (`root/XX/38-hex-chars`).
  - Atomic writes: data written to a temp file then `os.rename`d into place.
  - Temp file names use `os.time() .. "_" .. math.random(999999)` to avoid
    collisions without depending on platform-specific process-id APIs.
  - `keys_with_prefix` scans the appropriate fanout directory using
    `io.popen("ls")` / `io.popen("dir /b")` for POSIX / Windows portability.

- `key_to_hex` / `hex_to_key` utility functions for 20-byte binary ↔
  40-char lowercase hex string conversion.

- SHA-1 keying delegates to `coding_adventures.sha1.digest()` from the
  repo's own `coding-adventures-sha1` package — no external C libraries.

- Busted test suite (`tests/test_cas.lua`) covering:
  - Round-trip put/get for empty and small blobs
  - Idempotent put
  - `get` on unknown key → `"not_found"` error
  - Corrupted file on disk → `"corrupted"` error
  - `exists` before and after put
  - `find_by_prefix`: unique match, ambiguous, not-found, invalid hex,
    empty string, odd-length nibble prefix, full 40-char prefix
  - `LocalDiskStore` 2/38 path layout verified
  - `BlobStore` abstract methods raise errors when called directly
