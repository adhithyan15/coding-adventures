# Changelog — cas (Swift)

## [0.1.0] - 2026-04-12

### Added

- `BlobStore` protocol: `put(key:data:)`, `get(key:)`, `exists(key:)`,
  `keysWithPrefix(_:)` — the pluggable backend abstraction.
- `ContentAddressableStore<S: BlobStore>` generic struct:
  - `put(data:)` — SHA-1 hashes content, delegates write to backend.
  - `get(key:)` — fetches from backend, re-hashes for integrity check.
  - `exists(key:)` — delegated to backend.
  - `findByPrefix(_:)` — decodes hex prefix, scans backend, enforces uniqueness.
  - `inner` property — direct access to the underlying `BlobStore`.
- `LocalDiskStore` struct: filesystem backend with Git-style 2/38 fanout layout,
  atomic writes via temp-file rename (PID + nanosecond suffix for unguessable
  temp names), idempotent put with short-circuit on existing file.
- `CasError` enum: `.notFound`, `.corrupted`, `.ambiguousPrefix`,
  `.prefixNotFound`, `.invalidPrefix`, `.storeError`.
- `keyToHex(_:)` and `hexToKey(_:)` hex utility functions.
- `InMemoryStore` test helper implementing `BlobStore` with a dictionary.
- 30+ XCTest unit tests covering:
  - Hex utilities (round-trips, edge cases, upper/lowercase, wrong length).
  - `CasError` description strings.
  - Protocol conformance via `InMemoryStore`.
  - Round-trips for empty, small, and large (1 MiB) blobs.
  - Idempotent put.
  - `notFound` and `corrupted` error cases.
  - `exists` before and after put.
  - `findByPrefix`: unique, ambiguous, not-found, invalid hex, empty string.
  - `LocalDiskStore` 2/38 path layout verification.
  - Atomic write: `LocalDiskStore.init` creates the root directory.
