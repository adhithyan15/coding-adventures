# cas

Generic content-addressable storage (CAS) with a pluggable backend trait.

Content-addressable storage maps *the hash of content* to the content itself. The hash
is both the address and an integrity check: if the stored bytes don't hash to the
address you requested, the store is corrupt.

## Quick Start

```rust
use coding_adventures_content_addressable_storage::{ContentAddressableStore, LocalDiskStore};

// Open (or create) a store rooted at ./objects
let store = LocalDiskStore::new("./objects").unwrap();
let cas = ContentAddressableStore::new(store);

// Store some bytes — SHA-1 is computed automatically
let key = cas.put(b"hello, world").unwrap();
println!("{}", coding_adventures_content_addressable_storage::key_to_hex(&key)); // e.g. "8ddd8be4b179..."

// Retrieve by key — hash is verified on the way back out
let data = cas.get(&key).unwrap();
assert_eq!(data, b"hello, world");

// Abbreviated prefix lookup (like `git show a3f4b2`)
let full_key = cas.find_by_prefix("8ddd8b").unwrap();
assert_eq!(full_key, key);
```

## Custom Backend

Implement `BlobStore` to use any storage backend:

```rust
use coding_adventures_content_addressable_storage::{BlobStore, ContentAddressableStore};

struct MyS3Store { /* ... */ }

impl BlobStore for MyS3Store {
    type Error = MyError;

    fn put(&self, key: &[u8; 20], data: &[u8]) -> Result<(), MyError> { /* ... */ }
    fn get(&self, key: &[u8; 20]) -> Result<Vec<u8>, MyError> { /* ... */ }
    fn exists(&self, key: &[u8; 20]) -> Result<bool, MyError> { /* ... */ }
    fn keys_with_prefix(&self, prefix: &[u8]) -> Result<Vec<[u8; 20]>, MyError> { /* ... */ }
}

let cas = ContentAddressableStore::new(MyS3Store { /* ... */ });
```

## How It Fits in the Stack

```
git-object   ← adds type headers ("blob N\0content"), tree/commit/tag parsing
    │
   cas        ← this package: hash → store/retrieve raw bytes
    │
BlobStore     ← LocalDiskStore (here) or S3Store / custom (elsewhere)
```

The CAS layer is intentionally unaware of Git object types, compression, or pack
files. Those concerns belong in layers above and below.

## Storage Layout (LocalDiskStore)

Objects are stored at `<root>/<xx>/<38-hex-chars>`, where `xx` is the first byte of
the SHA-1 hash expressed as two hex digits. This matches Git's `.git/objects/` layout.

```
objects/
  a3/
    f4b2c1d0e9f8a7b6c5d4e3f2a1b0c9d8e7f6a5   ← 38-char remainder
  fe/
    9a3b…
```
