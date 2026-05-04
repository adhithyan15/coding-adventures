# `coding_adventures_storage_fs` — STR-FILE

Filesystem-backed implementation of `storage_core::StorageBackend`.
Hand a vault a path on disk and it Just Works — atomic write +
rename + fsync, single-file-per-record format, restart-safe
revision counter.

The backend is **opaque to record content**. The Vault stack
encrypts above it (VLT01 sealed-store), so this layer only ever
sees ciphertext + non-secret metadata. That's the
storage-agnosticism property: anyone with access to a vault file
on disk learns nothing about the vault's contents.

## Quick example

```rust
use coding_adventures_storage_fs::FsStorageBackend;
use storage_core::{StorageBackend, StoragePutInput};
use coding_adventures_json_value::JsonValue;

let be = FsStorageBackend::new("/var/lib/myvault/data");
be.initialize()?;

let rec = be.put(StoragePutInput {
    namespace: "vault/login".into(),
    key:       "github-personal".into(),
    content_type: "vault/login/v1".into(),
    metadata:  JsonValue::Object(Vec::new()),
    body:      ciphertext,             // encrypted by VLT01 above
    if_revision: None,
})?;
```

## Wire format

```text
   <root>/
     <hex(namespace)>/
       <hex(key)>          ← single file per record
       <hex(key)>.tmp      ← only mid-write; cleaned on init

   record_file =
       magic(4) "STRF" || version(1)=1 ||
       meta_len(4 BE) || meta_json(N) || body(rest)
```

`meta_json` is a small JSON object: `revision`, `content_type`,
`created_at`, `updated_at`, caller-supplied `metadata`.

## Crash safety

Writes follow the standard "write-tmp / fsync / rename" pattern.
POSIX `rename(2)` is atomic with respect to concurrent readers.
A best-effort `fsync` of the parent directory is performed for
durability on filesystems that need it.

`initialize()` cleans up any stranded `.tmp` files and seeds the
in-memory revision counter from the highest revision on disk so
monotonic numbering survives process restart.

## Where it fits

```text
                 ┌──────────────────────────────────────┐
                 │  application                         │
                 └──────────────┬───────────────────────┘
                                │
                 ┌──────────────▼───────────────────────┐
                 │  vault-sealed-store (VLT01)          │
                 │  envelope encryption — produces      │
                 │  opaque ciphertext bytes             │
                 └──────────────┬───────────────────────┘
                                │  ciphertext only
                 ┌──────────────▼───────────────────────┐
                 │  storage-core::StorageBackend trait  │
                 │  ────────────────────────────────────│
                 │  InMemory   ◄── default for tests    │
                 │  Fs        ◄── THIS CRATE            │
                 │  S3 / GDrive / WebDAV / git ◄── later│
                 └──────────────────────────────────────┘
```

## Future backends

Each of these is a sibling crate that implements the same
`StorageBackend` trait — VLT01 sits above all of them and the
ciphertext-only invariant is preserved:

- AWS S3 / S3-compatible object stores
- Google Drive / Dropbox / OneDrive (consumer cloud)
- WebDAV
- Git
- IPFS / content-addressable
- SQLite (single-file)

See [`VLT00-vault-roadmap.md`](../../../specs/VLT00-vault-roadmap.md)
and [`STR01-storage-fs-backend.md`](../../../specs/STR01-storage-fs-backend.md).
