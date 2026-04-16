# @coding-adventures/content-addressable-storage

Generic Content-Addressable Storage (CAS) for Node.js/TypeScript.

## What Is Content-Addressable Storage?

Ordinary storage maps a *name* to content: you ask for `photo.jpg`, you get that photo.
CAS flips the relationship — you ask for the *hash of the content*, and you get that
content back. The hash is both the address and the integrity check.

```
Traditional:   name  ──►  content           (name can be reused, content can change)
CAS:           hash  ──►  content           (hash is derived from content, cannot lie)
```

The defining property: if you know the hash, you know the content. If the stored bytes
don't hash to the address you asked for, the store is corrupt. This makes CAS
self-authenticating — trust the hash, trust the data.

Git's entire object model is built on CAS. Every file snapshot (blob), directory listing
(tree), commit, and tag is stored by the SHA-1 hash of its serialized bytes. Two
identical files → one stored object. A renamed file → zero new storage.

## Architecture

```
┌──────────────────────────────────────────────────────────┐
│  ContentAddressableStore<S extends BlobStore>            │
│                                                          │
│  put(data)         → SHA-1 key, delegate to S           │
│  get(key)          → fetch from S, verify hash          │
│  exists(key)       → check presence without reading     │
│  findByPrefix(hex) → resolve abbreviated hex to key     │
│  inner()           → access the raw BlobStore           │
└─────────────────────────┬────────────────────────────────┘
                          │ interface BlobStore
            ┌─────────────┴──────────────────────────────┐
            │                                            │
     LocalDiskStore                            (your own: S3Store, MemStore …)
     root/<xx>/<38-hex-chars>
     atomic rename writes
```

## Installation

```bash
npm install @coding-adventures/content-addressable-storage
```

## Usage

### Basic round-trip

```ts
import { ContentAddressableStore, LocalDiskStore, keyToHex } from "@coding-adventures/content-addressable-storage";

const store = new LocalDiskStore("/var/lib/myapp/objects");
const cas = new ContentAddressableStore(store);

// Store a blob — returns its SHA-1 key.
const key = cas.put(Buffer.from("hello, world"));
console.log(keyToHex(key)); // → 40-char hex

// Retrieve it — verifies integrity automatically.
const data = cas.get(key);
console.log(data.toString()); // → "hello, world"
```

### Existence check

```ts
console.log(cas.exists(key)); // → true (after put)

const unknown = hexToKey("0000000000000000000000000000000000000000");
console.log(cas.exists(unknown)); // → false
```

### Abbreviated prefix lookup (like `git show a3f4`)

```ts
const key = cas.put(Buffer.from("some data"));
const hex = keyToHex(key); // "a9993e364706816aba3e25717850c26c9cd0d89d"

// Resolve by 7-char prefix — throws if ambiguous or not found.
const resolved = cas.findByPrefix(hex.slice(0, 7));
console.log(resolved.equals(key)); // → true
```

### Custom backend

```ts
import { BlobStore, ContentAddressableStore } from "@coding-adventures/content-addressable-storage";

class MemStore implements BlobStore {
  private map = new Map<string, Buffer>();

  put(key: Buffer, data: Buffer): void {
    this.map.set(key.toString("hex"), data);
  }
  get(key: Buffer): Buffer {
    const v = this.map.get(key.toString("hex"));
    if (!v) throw new Error("not found");
    return v;
  }
  exists(key: Buffer): boolean {
    return this.map.has(key.toString("hex"));
  }
  keysWithPrefix(prefix: Buffer): Buffer[] {
    const hexPfx = prefix.toString("hex");
    return [...this.map.keys()]
      .filter(k => k.startsWith(hexPfx))
      .map(k => Buffer.from(k, "hex"));
  }
}

const cas = new ContentAddressableStore(new MemStore());
```

## Error Types

| Class | When thrown |
|---|---|
| `CasNotFoundError` | `get` on a key that was never stored |
| `CasCorruptedError` | stored bytes don't hash to the key |
| `CasAmbiguousPrefixError` | `findByPrefix` matched 2+ objects |
| `CasPrefixNotFoundError` | `findByPrefix` matched 0 objects |
| `CasInvalidPrefixError` | `findByPrefix` called with empty string or non-hex chars |

All extend `CasError` which extends `Error`.

## LocalDiskStore Path Layout

Objects are stored using Git's 2/38 fanout layout:

```
root/
  a9/
    993e364706816aba3e25717850c26c9cd0d89d   ← SHA-1 of "abc"
  da/
    39a3ee5e6b4b0d3255bfef95601890afd80709   ← SHA-1 of ""
```

The first byte of the hex hash becomes a two-character directory name. This
keeps individual directories small even in large repositories — 100 000 objects
create at most ~390 files per directory rather than 100 000 files in one place.

Writes are atomic: data is written to a temp file (`<base>.<pid>.<ns>.tmp`)
in the same directory as the final file, then renamed into place. On POSIX,
`rename(2)` is guaranteed atomic. On Windows, a racing rename is treated as a
successful idempotent write.

## How It Fits in the Stack

This package is the CAS layer. It does NOT implement:

- Git object headers (`"blob N\0content"`) — a git-object layer above
- Zlib/deflate compression — the `BlobStore` implementation's choice
- Pack file format — a future pack-store `BlobStore` implementation
- Ref database (branches, HEAD) — a separate refs package
- SHA-256 support — future: parameterize the hash algorithm

## Hashing

SHA-1 is used, matching Git's default. Delegates to `@coding-adventures/sha1`.

## Related Packages

- `@coding-adventures/sha1` — SHA-1 hash function (dependency)
- `coding-adventures-content-addressable-storage` — Rust implementation (same spec, same design)
