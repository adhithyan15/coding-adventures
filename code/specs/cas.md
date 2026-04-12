# Content Addressable Storage (CAS)

## What Is Content Addressable Storage?

Ordinary storage maps a *name* to content: you ask for `photo.jpg`, you get that photo.
CAS flips the relationship — you ask for the *hash of the content*, and you get that
content back. The hash is both the address and the integrity check.

```
Traditional:  name  ──► content           (name can be reused, content can change)
CAS:          hash  ──► content           (hash is derived from content, cannot lie)
```

The defining property: if you know the hash, you know the content. If the stored bytes
don't hash to the address you asked for, the store is corrupt. This makes CAS
self-authenticating — trust the hash, trust the data.

Git's entire object model is built on CAS. Every file snapshot (blob), directory listing
(tree), commit, and tag is stored by the SHA-1 hash of its serialized bytes. Two
identical files → one object. A renamed file → zero new storage. The history graph is
an immutable DAG of hashes pointing to hashes.

## Scope of This Package

This package implements the generic CAS layer only. It does NOT define:

- What the stored bytes *mean* (blob vs tree vs commit — that's a layer above).
- How the bytes are compressed on the wire or at rest (the store decides).
- Which hash algorithm is used for future Git SHA-256 objects (SHA-1 only for now).

The single responsibility: **hash content → store it → retrieve it by hash → verify it**.

## Architecture

```
┌──────────────────────────────────────────────────────────┐
│  ContentAddressableStore<S: BlobStore>                    │
│                                                           │
│  put(data: &[u8]) → [u8; 20]                             │
│    1. key = SHA1(data)                                    │
│    2. if !store.exists(key) → store.put(key, data)        │
│    3. return key                                          │
│                                                           │
│  get(key: &[u8; 20]) → Vec<u8>                           │
│    1. data = store.get(key)                               │
│    2. verify SHA1(data) == key  (integrity check)         │
│    3. return data                                         │
│                                                           │
│  find_by_prefix(hex: &str) → [u8; 20]                    │
│    1. decode hex prefix to bytes                          │
│    2. store.keys_with_prefix(prefix_bytes)                │
│    3. error if 0 or 2+ matches, else return the one key   │
└────────────────────────┬─────────────────────────────────┘
                         │ trait BlobStore
           ┌─────────────┴──────────────────────────────┐
           │                                            │
    LocalDiskStore                            (future: S3Store, MemStore, …)
    root/<xx>/<38-hex>
    atomic rename writes
```

## The `BlobStore` Trait

Any backend that can store and retrieve byte blobs by a 20-byte key qualifies:

```
trait BlobStore {
    type Error: std::error::Error + 'static;

    put(key: &[u8; 20], data: &[u8])        → Result<(), Error>
    get(key: &[u8; 20])                      → Result<Vec<u8>, Error>
    exists(key: &[u8; 20])                   → Result<bool, Error>
    keys_with_prefix(prefix: &[u8])          → Result<Vec<[u8; 20]>, Error>
}
```

`put` is idempotent: storing the same key twice is not an error. Implementations may
short-circuit (skip writing if already present) or overwrite — the result must be the
same blob regardless.

`keys_with_prefix` returns all stored keys whose first `prefix.len()` bytes match
`prefix`. It is used for abbreviated-hash lookup (e.g., the user types `git show a3f4`
and the store finds the one object whose hash starts with those bytes).

## `ContentAddressableStore<S>`

The CAS struct owns a `BlobStore` and adds three things the store alone cannot provide:

1. **Automatic keying**: callers pass content, not a key. SHA-1 is always correct.
2. **Integrity verification on read**: after `store.get`, the CAS re-hashes the bytes
   and panics/errors if they don't match the requested key.
3. **Prefix resolution**: translates abbreviated hex (7 chars like git uses in logs) to
   a full 20-byte key, with proper "not found" / "ambiguous" error discrimination.

```
ContentAddressableStore<S> {
    store: S,
}

new(store: S) → Self
put(data: &[u8]) → Result<[u8; 20], CasError<S::Error>>
get(key: &[u8; 20]) → Result<Vec<u8>, CasError<S::Error>>
exists(key: &[u8; 20]) → Result<bool, CasError<S::Error>>
find_by_prefix(hex_prefix: &str) → Result<[u8; 20], CasError<S::Error>>
inner() → &S
```

`inner()` gives the caller access to the raw store when they need backend-specific
operations (e.g., listing all keys for GC, or getting storage statistics).

## Error Model

```
CasError<E> {
    Store(E)                      — backend I/O or network failure
    NotFound([u8; 20])            — key is not in the store
    Corrupted { key: [u8; 20] }   — bytes in store don't hash to the key
    AmbiguousPrefix(String)       — hex prefix matches two or more objects
    PrefixNotFound(String)        — hex prefix matches zero objects
    InvalidPrefix(String)         — hex prefix is not valid hexadecimal
}
```

`Corrupted` means the store lied — what came back from `store.get(key)` does not hash
to `key`. This is a data integrity violation. The CAS layer surfaces it distinctly from
`Store(E)` so callers can decide whether to attempt repair, alert an operator, or abort.

## `LocalDiskStore`

The first backend. Stores objects on the local filesystem using the Git 2/38 fanout
layout:

```
<root>/
  ab/
    cdef012345678901234567890123456789    ← the 38-char remainder of the hex hash
  fe/
    9a3b…
```

Why the 2/38 split? Git pioneered this layout to avoid performance problems with
directories containing millions of files. Most filesystems degrade when a directory
holds hundreds of thousands of entries. Splitting on the first byte creates 256
sub-directories (~`00/` through `ff/`), keeping each to a manageable size even in
large repositories.

### Write path (atomic)

To avoid a reader seeing a partial write:

1. Write data to a temp file in `<root>/tmp/` (or OS temp dir).
2. `rename(tmp, final_path)` — atomic on POSIX, best-effort on Windows.
3. If the final path already exists (duplicate `put`), no-op.

### Read path

1. Compute `root/XX/remaining` from the hex-encoded key.
2. Open and read the file. Return the raw bytes (no decompression — that's the
   caller's concern).
3. If the file does not exist, return `Err(io::ErrorKind::NotFound)`.

### Prefix lookup

1. Decode the hex prefix to a byte prefix.
2. Identify which `XX/` bucket(s) to scan (the first byte of the prefix determines
   the bucket if at least 1 byte of prefix is given, otherwise scan all 256 buckets).
3. Collect entries whose names start with the remaining prefix bytes.
4. Return the full 20-byte keys as a `Vec<[u8; 20]>`.

## Hashing

SHA-1 is used as the hash function, matching Git's default. The CAS delegates to the
`coding-adventures-sha1` crate's `sum1(data)` function which returns a `[u8; 20]`.

To convert a key to/from hex, the CAS provides two small utilities:

```
key_to_hex(key: &[u8; 20]) → String          // "a3f4b2…" (40 chars)
hex_to_key(hex: &str) → Result<[u8; 20], _>  // parse 40-char hex to bytes
```

## Encoding of Hex Prefixes

`find_by_prefix` accepts any non-empty hex string of 1–40 characters. Rules:

- Odd-length strings are padded with `0` on the right before converting to bytes.
  Example: `"a3f"` → `[0xa3, 0xf0]` → matches any key starting with `0xa3`, `0xf0`.
- Characters must be `[0-9a-fA-F]`. Anything else → `InvalidPrefix`.
- Empty string → `InvalidPrefix` (would match everything, which is never useful).

## Deduplication

`put` is content-addressed: if you put the same data twice you get the same key back.
The `LocalDiskStore` checks `exists` before writing; if the object is already present
it skips the write. This means:

- Two files with identical content → one stored object.
- Re-committing an unchanged file → no new storage.
- Idempotent: `put(put(x)) == put(x)`.

## What This Package Does NOT Cover

| Concern | Where it lives |
|---|---|
| Zlib/deflate compression of stored bytes | The `BlobStore` implementation's choice |
| Git object headers (`"blob N\0"`) | A git-object layer above CAS |
| Pack file format | A future pack-store `BlobStore` implementation |
| Ref database (branches, HEAD) | A separate refs package |
| Tree / commit / tag deserialization | A git-object package |
| SHA-256 support | Future: parameterize hash algorithm |

## Test Requirements

- Round-trip: `get(put(data)) == data` for empty, small, and large (1 MiB) blobs.
- Idempotent put: calling `put` twice with the same data returns the same key, no error.
- `NotFound`: `get` on a key that was never stored.
- Integrity check: mutate the raw file on disk, then `get` returns `Corrupted`.
- `exists` returns false before put, true after.
- `find_by_prefix`: unique match, ambiguous (two objects, shared prefix), not found,
  invalid hex, empty string.
- `LocalDiskStore` path layout: verify the 2/38 directory structure is created.
- Trait object usage: `Box<dyn BlobStore<Error = std::io::Error>>` compiles and works.
