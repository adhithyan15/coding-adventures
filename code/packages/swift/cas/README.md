# cas (Swift)

Content-Addressable Storage — a generic CAS layer that stores blobs by their
SHA-1 hash and verifies integrity on every read.

## What is CAS?

Ordinary storage maps a *name* to content. CAS flips the relationship: you ask
for the *hash of the content*, and you get that content back. The hash is both
the address and an integrity check — if the stored bytes don't hash to the key
you requested, the store is corrupt.

```
Traditional:  name ──► content     (name can lie, content can change)
CAS:          hash ──► content     (hash is derived from content, cannot lie)
```

Git's entire object model is built on this principle.

## Architecture

```
┌──────────────────────────────────────────────────────────┐
│  ContentAddressableStore<S: BlobStore>                    │
│  put(data:)        → [UInt8] key (20-byte SHA-1)         │
│  get(key:)         → [UInt8], verifies SHA-1 on read     │
│  exists(key:)      → Bool                                │
│  findByPrefix(_:)  → resolves abbreviated hex prefix     │
└─────────────────────────┬────────────────────────────────┘
                          │ BlobStore protocol
            ┌─────────────┴──────────────────────────┐
            │                                        │
     LocalDiskStore                      (InMemoryStore, S3, …)
     root/<xx>/<38-hex-chars>
```

## Usage

```swift
import Cas

// Open (or create) a store on disk.
let store = try LocalDiskStore(root: URL(fileURLWithPath: "/tmp/my-cas"))
var cas   = ContentAddressableStore(store: store)

// Store some content — the key is the SHA-1 hash of the bytes.
let key = try cas.put(data: Array("hello, world".utf8))
print(keyToHex(key))  // "430ce34d020724ed75a196dfc2ad67c77772d169"

// Retrieve by key — integrity is checked automatically.
let data = try cas.get(key: key)
print(String(bytes: data, encoding: .utf8)!)  // "hello, world"

// Abbreviated prefix lookup (like `git show a3f4b2`).
let found = try cas.findByPrefix("430ce")
assert(found == key)
```

## Protocols

### `BlobStore`

Implement `BlobStore` to plug in a custom backend:

```swift
public protocol BlobStore {
    mutating func put(key: [UInt8], data: [UInt8]) throws
    func get(key: [UInt8]) throws -> [UInt8]
    func exists(key: [UInt8]) throws -> Bool
    func keysWithPrefix(_ prefix: [UInt8]) throws -> [[UInt8]]
}
```

### `CasError`

```swift
public enum CasError: Error {
    case notFound([UInt8])          // key not in store
    case corrupted([UInt8])         // stored bytes don't hash to key
    case ambiguousPrefix(String)    // prefix matches 2+ objects
    case prefixNotFound(String)     // prefix matches 0 objects
    case invalidPrefix(String)      // malformed hex string
    case storeError(Error)          // backend I/O failure
}
```

## LocalDiskStore Layout

Objects are stored using the same 2/38 fanout layout as Git:

```
<root>/
  da/
    39a3ee5e6b4b0d3255bfef95601890afd80709   ← SHA-1("") minus first byte
  43/
    0ce34d020724ed75a196dfc2ad67c77772d169
```

Writes are atomic: content goes to a uniquely-named temp file in the same
bucket directory, then `FileManager.moveItem` renames it into place.

## Dependencies

- `sha1` — the coding-adventures SHA-1 implementation (no CryptoKit)

## Layer

`CAS01` — depends on `SHA1` (no other package dependencies).
