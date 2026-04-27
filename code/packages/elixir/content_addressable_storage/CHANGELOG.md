# Changelog

## [0.1.0] — 2026-04-12

### Added

- `CodingAdventures.ContentAddressableStorage.BlobStore` — Elixir `@behaviour` defining the four
  callbacks (`put/3`, `get/2`, `exists?/2`, `keys_with_prefix/2`) that any
  storage backend must implement.

- `CodingAdventures.ContentAddressableStorage.Store` — Content-addressable store that wraps any
  `BlobStore`:
  - Computes SHA-1 key via `CodingAdventures.Sha1.sha1/1` (repo's own
    implementation, not `:crypto`).
  - Idempotent `put/2`: skips writing if the object already exists.
  - `get/2` verifies integrity on every read — returns
    `{:error, {:corrupted, key}}` if the stored bytes don't re-hash to the key.
  - `exists?/2` checks presence without fetching data.
  - `find_by_prefix/2` resolves 1–40 character abbreviated hex strings to full
    20-byte keys, with `{:ambiguous_prefix, _}` / `{:prefix_not_found, _}` /
    `{:invalid_prefix, _}` discrimination.
  - `inner/1` exposes the underlying backend for backend-specific operations.

- `CodingAdventures.ContentAddressableStorage.LocalDiskStore` — Filesystem backend using Git's 2/38
  fanout layout (`<root>/<xx>/<38-hex-chars>`):
  - Atomic writes via temp-file + `File.rename/2` (POSIX-atomic; best-effort
    on Windows).
  - Temp file names include OS PID + `:erlang.unique_integer([:monotonic])` to
    avoid TOCTOU races.
  - `keys_with_prefix/2` scans the relevant fanout bucket, skipping temp files
    and other non-object entries.

- `CodingAdventures.ContentAddressableStorage.Hex` — Hex utilities:
  - `key_to_hex/1` — 20-byte binary → 40-char lowercase hex string.
  - `hex_to_key/1` — 40-char hex string → 20-byte binary (error on bad input).
  - `decode_hex_prefix/1` — 1–40 char abbreviated hex → byte prefix binary,
    with right-padding for odd-length strings.

- `CodingAdventures.ContentAddressableStorage.Error` — Typed error reason catalogue with `format/1`
  for human-readable messages.

- Full ExUnit test suite (`test/content_addressable_storage_test.exs`) covering:
  - Hex encode/decode round-trips and edge cases.
  - Error formatting.
  - `MemStore` — in-memory `BlobStore` backed by ETS; exercises the behaviour
    contract and serves as a fast backend for unit tests.
  - Round-trip tests for empty, small, and 1 MiB blobs.
  - Idempotent put.
  - `get` on unknown key → `:not_found`.
  - Integrity violation → `{:corrupted, key}`.
  - `exists?` before and after put.
  - `find_by_prefix`: unique, ambiguous, not-found, invalid hex, empty string,
    odd-length prefix.
  - `LocalDiskStore` 2/38 path layout verification.
  - `LocalDiskStore` corruption detection via direct file mutation.
