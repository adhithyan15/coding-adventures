# zip

**CMP09 — ZIP archive format**: read and write `.zip` files in pure Rust.

This crate is part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures)
compression series:

```
CMP00 (LZ77,     1977) — Sliding-window backreferences.
CMP01 (LZ78,     1978) — Explicit dictionary (trie).
CMP02 (LZSS,     1982) — LZ77 + flag bits; no wasted literals.
CMP03 (LZW,      1984) — LZ78 + pre-initialised alphabet; GIF.
CMP04 (Huffman,  1952) — Entropy coding; prerequisite for DEFLATE.
CMP05 (DEFLATE,  1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib.
CMP06 (Brotli,   2013) — DEFLATE successor; HTTP/2 standard.
CMP07 (ZStd,     2016) — FSE + LZ77; Linux kernel / npm / macOS.
CMP08 (LZMA,     2001) — Range coding + LZ77; 7-Zip / XZ.
CMP09 (ZIP,      1989) — DEFLATE container; .zip files.    ← YOU ARE HERE
```

---

## What it does

The ZIP format (PKWARE, 1989) wraps one or more files in a self-describing
archive. Each entry carries its own compression metadata, and a **Central
Directory** at the end of the file is the authoritative index. This crate
implements:

- **Write**: `ZipWriter` / `zip()` — create a multi-file `.zip` archive in
  memory, choosing DEFLATE (method 8) or Stored (method 0) per file.
- **Read**: `ZipReader` / `unzip()` — parse the Central Directory, enumerate
  entries, and decompress individual files.
- **Raw DEFLATE**: ZIP-owned `raw_deflate`, `raw_inflate`, and
  `raw_inflate_counted` APIs for stored, fixed, dynamic, multi-block, and
  full-window RFC 1951 streams without a zlib or gzip wrapper.
- **CRC-32**: CRC-32/ISO-HDLC polynomial (0xEDB88320), precomputed table.
- **MS-DOS date/time**: packing and `DOS_EPOCH` constant (1980-01-01 00:00:00).

---

## Usage

### Write an archive

```rust
use zip::{ZipWriter, zip};

// High-level: one call for a simple archive
let bytes = zip(&[
    ("hello.txt", b"Hello, world!\n"),
    ("data/info.txt", b"Some nested file"),
]);

// Low-level: full control over compression
let mut w = ZipWriter::new();
w.add_file("readme.txt", b"compressed", true);   // DEFLATE
w.add_file("raw.bin", &random_bytes, false);      // Stored
w.add_directory("assets/");
let bytes = w.finish();
```

### Read an archive

```rust
use zip::{ZipReader, unzip};

// High-level: extract all files
let files = unzip(&bytes)?;
for (name, data) in &files {
    println!("{}: {} bytes", name, data.len());
}

// Low-level: random access
let reader = ZipReader::new(&bytes)?;
for entry in reader.entries() {
    println!("{} ({} bytes compressed)", entry.name, entry.compressed_size);
}
let data = reader.read_by_name("hello.txt")?;
```

### CRC-32

```rust
use zip::crc32;

let checksum = crc32(b"hello world", 0);
// Incremental: combine two buffers
let c1 = crc32(b"hello ", 0);
let c2 = crc32(b"world", c1);
assert_eq!(c2, crc32(b"hello world", 0));
```

### Raw RFC 1951

```rust
use zip::{raw_deflate, raw_inflate_counted, RAW_INFLATE_MAX_OUTPUT};

let encoded = raw_deflate(b"hello hello hello");
let decoded = raw_inflate_counted(&encoded, RAW_INFLATE_MAX_OUTPUT)?;
assert_eq!(decoded.output, b"hello hello hello");
assert_eq!(decoded.bytes_consumed, encoded.len());
# Ok::<(), zip::RawInflateError>(())
```

`RawInflateErrorCode::as_str()` yields one of the 14 stable
`zip-owned-raw-rfc1951-v1` identifiers. Errors carry no input bytes, offsets,
sizes, or partial output.

---

## Architecture

```
zip/src/lib.rs
├── crc32()                  — CRC-32/ISO-HDLC
├── dos_datetime()           — MS-DOS timestamp packing
├── raw_deflate()            — RFC 1951 fixed-Huffman encoder
│   ├── BitWriter            — LSB-first bit stream
│   ├── fixed_ll_encode()    — literal/length code → (bits, nbits)
│   └── fixed_dist_encode()  — distance code → (bits, nbits)
├── raw_inflate_counted()    — strict shared RFC 1951 decoder
├── ZipWriter                — builds Local File Headers + Central Directory
└── ZipReader                — EOCD-first parsing; per-entry decompression
```

### Compression policy

`add_file(..., compress: true)` will only use DEFLATE if the compressed size
is strictly smaller than the original. If DEFLATE expands the data (e.g.
random/encrypted bytes), it falls back to Stored automatically.

### Wire format compatibility

Archives produced by this crate are compatible with:
- `unzip` (Info-ZIP), `7-Zip`, macOS Archive Utility, Windows Explorer
- Any ZIP reader conforming to PKWARE Application Note §4.3

---

## Security

| Threat | Mitigation |
|--------|-----------|
| Decompression bombs | Caller-lowerable cap with a 256 MiB hard ceiling; ZIP entries use their declared size |
| Compressed suffix cavities | `ZipReader` requires counted consumption to equal the declared compressed payload |
| Zip-slip path traversal | Callers should validate `entry.name` for `..` and absolute paths |
| Corrupt EOCD | Validate signature and comment_len on every candidate |
| CRC mismatch | `read()` verifies CRC-32 after decompression; returns `Err` on mismatch |

---

## Spec

See [`specs/CMP09-zip.md`](../../../../specs/CMP09-zip.md) for the full
wire-format specification, test vectors, and educational notes.

---

## License

MIT
