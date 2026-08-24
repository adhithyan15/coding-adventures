# coding-adventures-zip

Python implementation of the **ZIP archive format** (CMP09, PKZIP 1989) — part of the
[coding-adventures](https://github.com/adhithyan15/coding-adventures) compression series.

## What Is ZIP?

ZIP is a lossless archive format that bundles one or more files into a single `.zip` file,
optionally compressing each entry independently using **DEFLATE** (method 8) or storing it
verbatim (method 0). It is the foundational format for:

- `.zip` files on every OS
- Java JARs (`.jar`, `.war`)
- Office Open XML (`.docx`, `.xlsx`, `.pptx`)
- Android packages (`.apk`, `.aab`)
- Python wheels (`.whl`)
- E-books (`.epub`)

## How It Fits the Stack

```
CMP00 (LZ77,    1977) — Sliding-window backreferences.
CMP01 (LZ78,    1978) — Explicit dictionary (trie).
CMP02 (LZSS,    1982) — LZ77 + flag bits.  ← dependency
CMP03 (LZW,     1984) — LZ78 + pre-initialized alphabet; GIF.
CMP04 (Huffman, 1952) — Entropy coding.
CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib.
CMP09 (ZIP,     1989) — DEFLATE container; universal archive.  ← YOU ARE HERE
```

This package depends on `coding-adventures-lzss` for LZ77 tokenization and inlines a
raw RFC 1951 DEFLATE codec. The encoder emits fixed-Huffman blocks while the
strict decoder accepts stored, fixed-Huffman, dynamic-Huffman, and multi-block
streams. The existing `deflate` package uses a custom wire format and is
intentionally not used here.

## Usage

### Writing

```python
from coding_adventures_zip import ZipWriter, zip_bytes

# Convenience — one call
archive: bytes = zip_bytes([
    ("hello.txt", b"Hello, world!"),
    ("data/numbers.bin", bytes(range(256))),
])

# Incremental — for directories or mixed files
w = ZipWriter()
w.add_directory("docs/")
w.add_file("docs/readme.txt", b"See README.", compress=True)
w.add_file("logo.png", png_bytes, compress=False)  # already compressed
archive = w.finish()
```

### Reading

```python
from coding_adventures_zip import ZipReader, unzip

# Convenience — extract everything
files: dict[str, bytes] = unzip(archive)
print(files["hello.txt"])  # b'Hello, world!'

# Random access — extract a single entry without reading others
reader = ZipReader(archive)
for entry in reader.entries():
    print(entry.name, entry.size, entry.method)

data = reader.read_by_name("hello.txt")
```

### Raw RFC 1951

```python
from coding_adventures_zip import raw_deflate, raw_inflate_counted

compressed = raw_deflate(b"hello" * 10)
result = raw_inflate_counted(compressed, max_output=1024)
assert result.output == b"hello" * 10
assert result.bytes_consumed == len(compressed)
```

These functions use raw RFC 1951 bytes with no ZIP, zlib, or gzip framing.
`raw_inflate_counted` stops exactly at the final block, which lets a container
reject trailing bytes. Its `max_output` may lower, but never raise, the public
256 MiB hard ceiling. Malformed streams raise `RawInflateError`; both `.code`
and the exception message are one of the 14 stable payload-blind error IDs.

For inputs larger than 4 KiB, `raw_deflate` uses a bounded-memory encoder:
incompressible chunks receive RFC 1951 stored framing, and repetitive chunks
use fixed Huffman coding with a fixed-size match table. The educational LZSS
tokenizer remains load-bearing for small examples, while large callers avoid
boxed per-byte tokens and exhaustive 32 KiB match scans. `bytes`, `bytearray`,
and byte-oriented `memoryview` inputs are accepted directly.

### CRC-32

```python
from coding_adventures_zip import crc32

checksum = crc32(b"hello world")  # 0x0D4A1185

# Incremental
c1 = crc32(b"hello ")
c2 = crc32(b"world", c1)
assert c2 == checksum
```

## API Reference

### `ZipWriter`

| Method | Description |
|--------|-------------|
| `__init__()` | Create an empty writer |
| `add_file(name, data, compress=True)` | Add a file; DEFLATE if smaller |
| `add_directory(name)` | Add a directory entry (name ends with `/`) |
| `finish() → bytes` | Write Central Directory + EOCD; return archive |

### `ZipReader`

| Method | Description |
|--------|-------------|
| `__init__(data)` | Parse archive; raises `ValueError` if no EOCD |
| `entries() → list[ZipEntry]` | All entries (files and directories) |
| `read(entry) → bytes` | Decompress + verify CRC-32 |
| `read_by_name(name) → bytes` | Convenience: find by name then read |

### `ZipEntry`

| Field | Type | Description |
|-------|------|-------------|
| `name` | `str` | File name (UTF-8) |
| `size` | `int` | Uncompressed size |
| `compressed_size` | `int` | Compressed size |
| `method` | `int` | 0 = Stored, 8 = DEFLATE |
| `crc32` | `int` | CRC-32 of uncompressed content |
| `is_directory` | `bool` | True if name ends with `/` |
| `local_offset` | `int` | Byte offset of Local Header |

### Convenience functions

| Function | Description |
|----------|-------------|
| `zip_bytes(entries, compress=True) → bytes` | Create archive from list of `(name, data)` |
| `unzip(data) → dict[str, bytes]` | Extract all files; skip directories |
| `crc32(data, initial=0) → int` | CRC-32 (polynomial 0xEDB88320) |
| `dos_datetime(year, month, day, ...) → int` | Encode MS-DOS timestamp |
| `raw_deflate(data) → bytes` | Encode a raw RFC 1951 stream |
| `raw_inflate(data, max_output=...) → bytes` | Strictly decode a raw stream |
| `raw_inflate_counted(data, max_output=...) → RawInflateResult` | Decode and report exact input consumption |
| `RAW_INFLATE_MAX_OUTPUT` | Absolute 256 MiB raw-decode ceiling |
| `RAW_INFLATE_ERROR_CODES` | Ordered tuple of the 14 stable error IDs |
| `RawInflateError` | Typed failure carrying a stable `.code` |
| `RawInflateResult` | Immutable output and `bytes_consumed` result |

## Installation

```bash
pip install coding-adventures-zip
```

## Security Notes

- **Zip slip**: `unzip()` returns a plain dict; no paths are written to disk.
  Any disk-writing wrapper must strip `..` components and absolute prefixes.
- **Decompression bombs**: DEFLATE output is capped at 256 MiB and ZIP readers
  lower the limit to the entry's declared size before decoding.
- **Container cavities**: method 8 reads reject trailing compressed bytes and
  any exact uncompressed-size mismatch before CRC verification.
- **Payload-blind failures**: raw decoder errors never include attacker bytes,
  lengths, offsets, names, or paths.
- **CRC-32 is not cryptographic**: it detects accidental corruption only.
- **Encryption**: entries with the encrypted flag set raise `ValueError`.
