# cas

Generic content-addressable storage (CAS) with a pluggable backend and SHA-1
integrity verification.

Content-addressable storage maps *the hash of content* to the content itself.
The hash is both the address and an integrity check: if the stored bytes don't
hash to the address you requested, the store is corrupt — no separate checksum
needed.

## Quick Start

```python
from coding_adventures_content_addressable_storage import ContentAddressableStore, LocalDiskStore
import pathlib, tempfile

with tempfile.TemporaryDirectory() as tmp:
    store = LocalDiskStore(pathlib.Path(tmp))
    cas = ContentAddressableStore(store)

    # Store bytes — SHA-1 is computed automatically
    key = cas.put(b"hello, world")
    print(key.hex())  # e.g. "8ddd8be4b179a529afa5f2ffae4b9858..."

    # Retrieve by key — hash is verified on the way back out
    data = cas.get(key)
    assert data == b"hello, world"

    # Abbreviated prefix lookup (like `git show a3f4b2`)
    full_key = cas.find_by_prefix(key.hex()[:8])
    assert full_key == key
```

## How It Fits in the Stack

```
git-object   ← adds type headers ("blob N\0content"), tree/commit/tag parsing
    │
   cas        ← this package: hash → store/retrieve raw bytes
    │
BlobStore     ← LocalDiskStore (here) or S3Store / custom (elsewhere)
```

The CAS layer is intentionally unaware of Git object types, compression, or
pack files. Those concerns belong in layers above and below.

## Storage Layout (LocalDiskStore)

Objects are stored at `<root>/<xx>/<38-hex-chars>`, matching Git's
`.git/objects/` layout:

```
objects/
  a3/
    f4b2c1d0e9f8a7b6c5d4e3f2a1b0c9d8e7f6a5   ← 38-char remainder
  fe/
    9a3b…
```

Splitting on the first byte (up to 256 subdirectories) prevents any single
directory from growing too large — the same reason Git chose this layout.

Writes are atomic: content is written to a temp file named
`<hash>.<pid>.<time_ns>.tmp`, then `os.replace()` moves it into place.

## Custom Backend

Implement `BlobStore` to use any storage backend:

```python
from coding_adventures_content_addressable_storage import BlobStore, ContentAddressableStore

class MyS3Store(BlobStore):
    def put(self, key: bytes, data: bytes) -> None: ...
    def get(self, key: bytes) -> bytes: ...
    def exists(self, key: bytes) -> bool: ...
    def keys_with_prefix(self, prefix: bytes) -> list[bytes]: ...

cas = ContentAddressableStore(MyS3Store())
```

## Error Hierarchy

```
CasError
├── CasStoreError         — backend raised an exception
├── CasNotFoundError      — key does not exist in the store
├── CasCorruptedError     — stored bytes don't hash to the requested key
├── CasAmbiguousPrefixError — prefix matches multiple objects
├── CasPrefixNotFoundError  — prefix matches zero objects
└── CasInvalidPrefixError   — empty string or non-hex characters
```

## API

### `key_to_hex(key: bytes) -> str`
Convert a 20-byte SHA-1 key to a 40-character lowercase hex string.

### `hex_to_key(hex_str: str) -> bytes`
Parse a 40-character hex string into a 20-byte key. Raises `ValueError` on
invalid input.

### `BlobStore` (abstract base class)
Subclass to implement a storage backend. Methods: `put`, `get`, `exists`,
`keys_with_prefix`.

### `ContentAddressableStore(store: BlobStore)`
Wraps a `BlobStore` with hashing, integrity verification, and prefix lookup.

- `put(data: bytes) -> bytes` — hash, store, return 20-byte key
- `get(key: bytes) -> bytes` — fetch and verify integrity
- `exists(key: bytes) -> bool` — check presence without fetching
- `find_by_prefix(hex_prefix: str) -> bytes` — resolve abbreviated hash

### `LocalDiskStore(root: pathlib.Path)`
Filesystem backend using Git-style 2/38 fanout layout.

## Python Port Notes

This is a port of the Rust `coding-adventures-content-addressable-storage` package. Key differences:

- Python's `hashlib.sha1()` is used directly (no dependency on
  `coding-adventures-sha1`), because Python's stdlib provides SHA-1 while
  Rust's does not.
- The Rust `BlobStore` trait's associated `Error` type collapses to plain
  exceptions in Python.
- The Rust `[u8; 20]` fixed-size array becomes `bytes` (always 20 bytes).
- Match statement (Python 3.10+) mirrors Rust's `match matches.len()`.
