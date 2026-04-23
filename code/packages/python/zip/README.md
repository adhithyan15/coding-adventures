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
raw RFC 1951 DEFLATE codec (fixed Huffman, BTYPE=01). The existing `deflate` package uses
a custom wire format and is intentionally not used here.

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

## Installation

```bash
pip install coding-adventures-zip
```

## Security Notes

- **Zip slip**: `unzip()` returns a plain dict; no paths are written to disk.
  Any disk-writing wrapper must strip `..` components and absolute prefixes.
- **Decompression bombs**: output is capped at 256 MB; a `ValueError` is raised if exceeded.
- **CRC-32 is not cryptographic**: it detects accidental corruption only.
- **Encryption**: entries with the encrypted flag set raise `ValueError`.
