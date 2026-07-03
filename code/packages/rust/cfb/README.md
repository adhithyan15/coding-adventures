# cfb (Rust)

**CFB01 — Compound File Binary Format (OLE2) reader**

A from-scratch, zero-dependency reader for the **Compound File Binary Format**
([MS-CFB]), also known as **OLE2 storage** or **Structured Storage**. This is
the container format that lives *inside* legacy Microsoft Office files: a
pre-2007 `.xls`, `.doc`, or `.ppt` is, on disk, a CFB file holding named
byte-streams (`Workbook`, `WordDocument`, `PowerPoint Document`, …).

This crate lets you open such a file and pull those streams out by name — the
bottom layer that the BIFF8 (`.xls`), Word-binary (`.doc`), and PowerPoint-binary
(`.ppt`) parsers build on top of.

- `#![forbid(unsafe_code)]`, no third-party dependencies (pure `std`).
- Hardened against hostile input: every sector-chain walk is cycle-guarded,
  every offset is bounds-checked, and total output is capped.
- Handles both **512-byte** and **4096-byte** sectors, and both storage paths
  (the regular FAT **and** the mini-stream / mini-FAT).

See `code/specs/CFB01-compound-file.md` for a full literate walkthrough of the
format — sectors, the FAT, the directory red-black tree, and the mini-stream.

## The mental model in one paragraph

A CFB file is a **FAT filesystem crammed into a single file**. The file is
chopped into fixed-size **sectors** (usually 512 bytes). A **File Allocation
Table (FAT)** is an array of "next sector" pointers: to read a multi-sector
stream you follow `FAT[n]` like a linked list until you hit `ENDOFCHAIN`. A
**directory** (itself a FAT-stored stream) lists the named objects — CFB calls
a file a *stream* and a folder a *storage*. Tiny streams (smaller than the
mini-stream cutoff, usually 4096 bytes) are packed inside a **mini-stream**,
chained by a parallel **mini-FAT**, to avoid wasting a whole sector each.

## Usage

```rust
use cfb::{CompoundFile, EntryKind};

fn dump(bytes: &[u8]) -> Result<(), cfb::CfbError> {
    let cf = CompoundFile::open(bytes)?;

    // Enumerate every stream and storage.
    for e in cf.entries() {
        println!("{} — {} bytes ({:?})", e.name, e.size, e.kind);
    }

    // Read a stream's decoded bytes by name (case-insensitive).
    if let Some(workbook) = cf.read_stream("Workbook") {
        // The first two little-endian bytes of an Excel workbook stream are a
        // BIFF8 BOF record: type 0x0809.
        assert_eq!(u16::from_le_bytes([workbook[0], workbook[1]]), 0x0809);
    }
    Ok(())
}
```

## API

| Item | Purpose |
|------|---------|
| `CompoundFile::open(&[u8]) -> Result<CompoundFile, CfbError>` | Parse header, FAT, directory, mini-stream. |
| `entries() -> &[Entry]` | All objects: name, size, kind, directory id. |
| `stream_names() -> Vec<String>` | Convenience: just the stream names. |
| `read_stream(name) -> Option<Vec<u8>>` | Top-level stream by name (case-insensitive). |
| `read_stream_by_id(id) -> Result<Vec<u8>, CfbError>` | Precise stream access. |

`EntryKind` is `Stream` / `Storage` / `RootStorage`.

`CfbError`: `BadSignature`, `Truncated`, `UnsupportedSectorSize`,
`BadSectorChain`, `CycleDetected`, `OutputTooLarge`, `BadDirectory`,
`NotAStream`.

## Security notes

CFB files arrive as email attachments, so the reader treats every input as
hostile:

- **Cycle guards** — every FAT / mini-FAT / directory walk uses a visited-set
  *and* a hard iteration cap (= total sector count), so a self-referential
  chain returns `Err(CycleDetected)` instead of hanging.
- **Bounds checks** — every sector→offset computation is checked against the
  slice length with checked arithmetic; nothing panics or reads out of bounds.
- **Output cap** — a directory that lies about a stream's size cannot force an
  unbounded allocation; assembled output is capped (256 MiB) and stream sizes
  are validated against the file length.
- No `unwrap` / `expect` / `panic!` on input-derived data.

## Build & test

```sh
cargo test -p cfb -- --nocapture
```
