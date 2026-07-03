# cfb-writer

A from-scratch, **zero-third-party-dependency** *writer* for the **OLE2 /
Compound File Binary Format** ([MS-CFB]) — the container format that lives
inside legacy `.xls`, `.doc`, and `.ppt` files. It is the exact inverse of the
sibling [`cfb`](../cfb) reader crate: you hand it a set of named streams and it
produces a byte buffer that the reader (and real Office tooling) accepts.

`#![forbid(unsafe_code)]`, pure `std`, deterministic output (no timestamps or
randomness), never panics on the public API path.

## Where it fits in the stack

```
        deflate ── zip ── xml ── opc ── spreadsheetml ── xlsx-eval   (OOXML, .xlsx)
                                                                     │
   cfb (reader) ◄──────── round-trip proof ────────► cfb-writer     (this crate)
        │                                                 │
        └───────────── the OLE2 container ────────────────┘
                                                          │
                                          legacy .xls / .doc / .ppt  (milestone C4)
```

Modern Office files (`.xlsx`, `.docx`) are ZIP containers; the *legacy* binary
formats (`.xls`, `.doc`, `.ppt`) are CFB containers. This crate is the container
foundation for **writing** those legacy formats: a `.xls` writer will build a
`Workbook` stream (BIFF records) and hand it to `cfb-writer` to wrap.

## The one-paragraph mental model

A CFB file is a **FAT filesystem crammed into a single file**. A fixed 512-byte
*header*, then equal-sized 512-byte *sectors*. A **File Allocation Table (FAT)**
holds one "next sector" pointer per sector, so a multi-sector stream is a linked
list you follow until `ENDOFCHAIN`. A **directory** (itself a FAT-stored stream)
lists the named objects. Tiny streams (smaller than the 4096-byte *mini cutoff*)
would waste most of a sector, so they are packed into a **mini-stream** sliced
into 64-byte *mini-sectors* chained by a parallel **mini-FAT**.

## Usage

```rust
use cfb_writer::{CfbWriter, write_cfb};

// Convenience one-shot:
let bytes = write_cfb(&[
    ("Workbook", &vec![0xABu8; 5000][..]),   // large -> regular FAT
    ("SmallStream", b"hello mini-stream"),   // small -> mini-FAT
]);

// Or the builder:
let mut w = CfbWriter::new();
w.add_stream("Workbook", &[/* BIFF bytes */]);
w.add_stream("\u{5}SummaryInformation", b"...");
let bytes = w.finish();
```

Reopen with the reader to verify:

```rust
let cf = cfb::CompoundFile::open(&bytes).unwrap();
assert_eq!(cf.read_stream("Workbook").unwrap(), vec![0xABu8; 5000]);
```

## What it emits

Always **version 3**: 512-byte sectors, 64-byte mini-sectors, 4096-byte cutoff.
The layout is a fixed, simple sector order (directory → mini-FAT → mini-stream →
large streams → FAT sectors), and the directory is a trivially-valid all-black
red-black tree where streams are chained as right-siblings in insertion order.

## Behaviour & limits

- **Names** are stored in a 64-byte UTF-16LE field, so at most **31 UTF-16 code
  units** (plus a NUL). A longer name is **truncated** to 31 units (the API
  stays infallible).
- **Small vs large:** a stream strictly smaller than 4096 bytes goes to the
  mini-stream; a stream ≥ 4096 gets its own 512-byte sectors.
- **Empty streams** (0 bytes) and an **empty stream set** both produce valid
  files (the latter is just a Root Entry).
- **Deterministic:** CLSID and all timestamp fields are zeroed, so identical
  input always yields identical bytes.
- **Storages (folders)** are not yet emitted — this writer produces a flat set
  of top-level streams, which is all the legacy Office single-workbook/document
  case needs. See the spec for the extension path.

## Testing

```
cargo test -p cfb-writer -- --nocapture
```

The centrepiece is a **round-trip** test: write mixed small + large streams,
reopen with the `cfb` reader, and assert byte-for-byte equality.

## Specification

See [`code/specs/CFBW01-cfb-writer.md`](../../../specs/CFBW01-cfb-writer.md) for
the full literate walkthrough (header/FAT/DIFAT/directory/mini-FAT layout, the
small-vs-large decision, and the fixed-point FAT-sector count).

[MS-CFB]: https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-cfb/
