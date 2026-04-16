# coding_adventures_content_addressable_storage

Generic Content-Addressable Storage (CAS) for Ruby, ported from the
[Rust implementation](../../rust/content_addressable_storage). This package implements the CAS
architecture layer: hash content → store it → retrieve it by hash → verify it.

## What Is Content-Addressable Storage?

Ordinary storage maps a **name** to content: ask for `photo.jpg`, get the photo.
CAS flips the relationship — you ask for the **hash of the content** and get the
content back. The hash is simultaneously the address and the integrity check.

```
Traditional:  name  ──►  content   (name can lie; content can change)
CAS:          hash  ──►  content   (hash derived from content — cannot lie)
```

Git's entire object model is CAS. Every blob, tree, commit, and tag is stored
by the SHA-1 hash of its bytes. Two identical files → one stored object. Rename
a file → zero new storage.

## Architecture

```
┌──────────────────────────────────────────────────────────┐
│  ContentAddressableStore                                  │
│  put(data)         → 20-byte SHA-1 key                   │
│  get(key)          → bytes  (integrity verified)         │
│  find_by_prefix(hex) → key  (abbreviated hash lookup)    │
└────────────────────────┬─────────────────────────────────┘
                         │ BlobStore module (interface)
           ┌─────────────┴──────────────────────────────┐
           │                                            │
    LocalDiskStore                         (your own backend)
    root/XX/38-hex-chars
    atomic rename writes
```

## Installation

Add to your `Gemfile`:

```ruby
gem "coding_adventures_content_addressable_storage"
```

Or install directly:

```sh
gem install coding_adventures_content_addressable_storage
```

## Usage

```ruby
require "coding_adventures_content_addressable_storage"
require "tmpdir"

Dir.mktmpdir do |root|
  store = CodingAdventures::ContentAddressableStorage::LocalDiskStore.new(root)
  cas   = CodingAdventures::ContentAddressableStorage::ContentAddressableStore.new(store)

  # Store a blob — returns the 20-byte SHA-1 key
  key = cas.put("hello, world")

  # Retrieve and verify integrity in one call
  data = cas.get(key)
  puts data   # → "hello, world"

  # Check existence without fetching
  cas.exists?(key)  # → true

  # Abbreviated hash lookup (like `git show a3f4b2`)
  hex = CodingAdventures::ContentAddressableStorage.key_to_hex(key)
  found = cas.find_by_prefix(hex[0, 7])   # first 7 hex chars
  found == key   # → true
end
```

## API

### Module-level helpers

| Method | Description |
|---|---|
| `CodingAdventures::ContentAddressableStorage.key_to_hex(key)` | 20-byte binary String → 40-char hex String |
| `CodingAdventures::ContentAddressableStorage.hex_to_key(hex)` | 40-char hex String → 20-byte binary String |

### ContentAddressableStore

| Method | Description |
|---|---|
| `new(store)` | Wrap any `BlobStore`-compatible backend |
| `put(data)` | Hash with SHA-1, store, return 20-byte key |
| `get(key)` | Fetch and integrity-verify; raises on missing/corrupt |
| `exists?(key)` | Check presence without fetching |
| `find_by_prefix(hex)` | Resolve 1–40 char hex prefix to a full key |
| `inner` | Access the underlying `BlobStore` directly |

### BlobStore module (interface)

Include this module and implement the four methods to create a custom backend:

```ruby
class MyStore
  include CodingAdventures::ContentAddressableStorage::BlobStore

  def put(key, data)    ... end
  def get(key)          ... end
  def exists?(key)      ... end
  def keys_with_prefix(prefix) ... end
end
```

### LocalDiskStore

```ruby
store = CodingAdventures::ContentAddressableStorage::LocalDiskStore.new("/path/to/objects")
```

Stores objects at `<root>/XX/YYYYYY…` (2/38 fanout layout, matching Git's
`.git/objects/` structure). Writes are atomic: temp file + rename.

### Errors

All errors inherit from `CodingAdventures::ContentAddressableStorage::CasError < StandardError`.

| Class | Raised when |
|---|---|
| `CasNotFoundError` | `get` on a key that doesn't exist |
| `CasCorruptedError` | stored bytes don't hash to the requested key |
| `CasAmbiguousPrefixError` | prefix matches ≥2 objects |
| `CasPrefixNotFoundError` | prefix matches 0 objects |
| `CasInvalidPrefixError` | prefix is empty or not valid hex |

## How It Fits in the Stack

This package sits at the storage layer. Layers above it (not in this package):

- Git object format — `"blob N\0content"` headers
- Compression — zlib deflation at rest
- Pack files — delta-compressed object packs
- Ref database — branches, tags, HEAD

## Tests

```sh
bundle install
bundle exec rake test
```

Tests cover: round-trips (empty, small, 1 MiB blobs), idempotent puts,
not-found, corruption detection, exists? lifecycle, prefix resolution (unique,
ambiguous, not-found, invalid, odd-length), path layout verification, and
BlobStore as a mixin.

## License

MIT
