# cas — Generic Content-Addressable Storage (Go)

A Go port of the `coding-adventures-content-addressable-storage` Rust crate. Provides generic
content-addressable storage (CAS) where every blob is keyed by the SHA-1 hash
of its bytes — the same model Git uses for its object store.

## What Is Content-Addressable Storage?

Traditional storage maps a *name* to content. CAS maps the *hash of content*
to content. The hash is simultaneously the address and the integrity check.

```
Traditional:  name  ──► content   (name can lie; content can change)
CAS:          hash  ──► content   (hash is derived from content, cannot lie)
```

If the bytes stored under a key don't hash to that key, the store is corrupt.
CAS is self-authenticating.

## Architecture

```
┌──────────────────────────────────────────────────────────┐
│  ContentAddressableStore[S BlobStore]                     │
│  Put(data)         → SHA-1 key, delegates to BlobStore   │
│  Get(key)          → fetch + verify SHA-1 integrity      │
│  FindByPrefix(hex) → abbreviated-hash lookup             │
└─────────────────────────┬────────────────────────────────┘
                          │ BlobStore interface
             ┌────────────┴──────────────────────────────┐
             │                                           │
      LocalDiskStore                        MemStore (in-package)
      root/<xx>/<38-hex>                    map[[20]byte][]byte
      atomic rename writes
```

## Usage

```go
import "github.com/adhithyan15/coding-adventures/code/packages/go/content_addressable_storage"

// Create a filesystem-backed store.
store, err := cas.NewLocalDiskStore("/path/to/objects")
if err != nil { /* handle */ }

c := cas.NewContentAddressableStore(store)

// Store a blob — returns its SHA-1 key.
key, err := c.Put([]byte("hello, world"))

// Retrieve and verify a blob.
data, err := c.Get(key)

// Check existence without reading.
ok, err := c.Exists(key)

// Resolve an abbreviated hex prefix (like git short-hash).
fullKey, err := c.FindByPrefix("a3f4b2c")
```

## Hex Utilities

```go
// Convert key to 40-char lowercase hex.
hex := cas.KeyToHex(key)

// Parse 40-char hex to [20]byte key.
key, err := cas.HexToKey("a3f4b2c1...")
```

## Custom Backends

Implement the `BlobStore` interface to add your own backend:

```go
type BlobStore interface {
    Put(key [20]byte, data []byte) error
    Get(key [20]byte) ([]byte, error)
    Exists(key [20]byte) (bool, error)
    KeysWithPrefix(prefix []byte) ([][20]byte, error)
}
```

The package includes `MemStore` (in-memory) and `LocalDiskStore` (filesystem).

## Error Handling

All errors from `ContentAddressableStore` are `*cas.CasError`, which carries a
`Kind` field:

| Kind                  | Meaning                                     |
|-----------------------|---------------------------------------------|
| `ErrKindStore`        | Underlying BlobStore I/O failure            |
| `ErrKindNotFound`     | Key not present in store                    |
| `ErrKindCorrupted`    | Stored bytes don't hash to the expected key |
| `ErrKindAmbiguous`    | Hex prefix matches two or more objects      |
| `ErrKindPrefixMissing`| Hex prefix matches zero objects             |
| `ErrKindInvalidPrefix`| Prefix is not valid hexadecimal or is empty |

```go
var ce *cas.CasError
if errors.As(err, &ce) {
    switch ce.Kind {
    case cas.ErrKindNotFound:
        // ...
    case cas.ErrKindCorrupted:
        // alert operator
    }
}
```

## LocalDiskStore Layout

Objects are stored at `<root>/<xx>/<38-hex-chars>` where `xx` is the first byte
of the SHA-1 key encoded as two lowercase hex digits — the same 2/38 fanout Git
has used since 2005. This keeps each bucket to ~400 entries even for 100k-object
repositories.

Writes are atomic: data goes to a temp file with a `<pid>.<nanos>.tmp` suffix,
then `os.Rename` moves it into place.

## How It Fits in the Stack

This package sits below a hypothetical `git-object` package that adds the Git
object header (`"blob N\0content"`), compression, and pack-file support. The CAS
layer is intentionally minimal — it only hashes, stores, retrieves, and verifies.

## Related Packages

- `code/packages/go/sha1` — SHA-1 implementation used for hashing
- `code/packages/rust/content_addressable_storage` — Reference Rust implementation (same spec)
- `code/specs/cas.md` — Shared specification document
