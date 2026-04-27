# CMP09 — ZIP (PKZIP Archive Format)

## Overview

ZIP is a **lossless archive format** created by Phil Katz and Gary Conway for PKWARE in
1989. It bundles one or more files into a single `.zip` file, optionally compressing each
entry independently using **DEFLATE** (CMP05) or storing it uncompressed. ZIP is the
foundational archive format for Windows, Java JARs, Office Open XML (`.docx`/`.xlsx`),
Open Document Format, Android APKs, Python wheels, and many more.

```
Series:
  CMP00 (LZ77,     1977) — Sliding-window backreferences.
  CMP01 (LZ78,     1978) — Explicit dictionary (trie).
  CMP02 (LZSS,     1982) — LZ77 + flag bits; no wasted literals.
  CMP03 (LZW,      1984) — LZ78 + pre-initialised alphabet; GIF.
  CMP04 (Huffman,  1952) — Entropy coding; prerequisite for DEFLATE.
  CMP05 (DEFLATE,  1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib.
  CMP06 (Brotli,   2013) — DEFLATE successor; HTTP/2 standard.
  CMP07 (ZStd,     2016) — FSE + LZ77; Linux kernel / npm / macOS.
  CMP08 (LZMA,     2001) — Range coding + LZ77; 7-Zip / XZ.
  CMP09 (ZIP,      1989) — DEFLATE container; universal archive.  ← YOU ARE HERE
```

ZIP is used wherever a portable multi-file archive is needed:
- `.zip` files on every OS
- `.jar` / `.war` / `.ear` — Java archives
- `.docx` / `.xlsx` / `.pptx` — Office Open XML (Microsoft Office)
- `.odt` / `.ods` — Open Document Format (LibreOffice)
- `.apk` / `.aab` — Android packages
- `.whl` — Python wheels (pip)
- `.epub` — E-book format
- `.xpi` — Firefox extensions

## Historical Context

Phil Katz reverse-engineered and improved on ARC (1985, System Enhancement Associates),
then co-authored PKZIP 1.0 in 1989. The format became a de-facto standard before any
ISO/IETF process existed. PKWARE publishes the **Application Note** (APPNOTE.TXT) that
serves as the normative specification; the current version (6.3.10) is freely available.

Key milestones:
- 1989: PKZIP 1.0 — Store + Shrink (LZW) + Reduce (LZ77) + Implode methods
- 1993: PKZIP 2.0 — DEFLATE added as method 8 (the dominant method ever since)
- 2001: ZIP64 extensions for files > 4 GB
- 2003: AES-256 encryption (APPNOTE.TXT §7.2)
- 2020: APPNOTE 6.3.9 — Zstandard (method 93) officially registered

This spec covers the core ZIP format: **Stored** (method 0) and **DEFLATE** (method 8),
without encryption, spanning (multi-disk), or ZIP64. This handles the vast majority of
real-world ZIP files and all the derived formats listed above.

## Key Concepts

### Independent Per-Entry Compression

Unlike gzip (which compresses a single file) or tar+gzip (which tarballs first), ZIP
compresses each **entry** independently. This means:
- Any single file can be extracted without decompressing others
- Different entries can use different compression methods
- Random access is possible via the **Central Directory** at the end of the file

### Local File Header + Central Directory

ZIP has a dual-structure design that enables both streaming write and random-access read:

```
┌─────────────────────────────────────────────────────┐
│  [Local File Header + data for entry 1]             │
│  [Local File Header + data for entry 2]             │
│  ...                                                │
│  [Data Descriptor for entry N if streaming]         │
│                                                     │
│  ══════════ Central Directory ══════════            │
│  [Central Directory Header for entry 1]             │
│  [Central Directory Header for entry 2]             │
│  ...                                                │
│  [End of Central Directory Record]                  │
└─────────────────────────────────────────────────────┘
```

**Why two headers?**  
When writing a ZIP file sequentially (streaming), the compressor doesn't know the
compressed size until after compression. It writes the Local Header with size=0, then
the data, then a **Data Descriptor** with the actual sizes. The Central Directory
written at the end has the correct sizes, enabling reliable extraction.

When reading, you seek to the End of Central Directory (EOCD) at the end of the file,
read all Central Directory entries, then seek to each Local Header to extract data.

### CRC-32

ZIP uses **CRC-32** (Cyclic Redundancy Check, polynomial 0xEDB88320 reflected) to
detect corruption of the decompressed content. The CRC is computed over the original
(uncompressed) bytes and stored in the headers. Extractors verify it after
decompression.

CRC-32 is **not** a cryptographic hash — it detects accidents, not attacks. For
integrity against malicious modification, use ZIP's AES encryption or a separate
hash (SHA-256 in a manifest, etc.).

### Compression Methods

| Method ID | Name      | Description                             |
|-----------|-----------|-----------------------------------------|
| 0         | Stored    | No compression; raw bytes               |
| 8         | Deflated  | DEFLATE (RFC 1951) — the standard       |
| 12        | Bzip2     | bzip2 — rarely used                     |
| 14        | LZMA      | LZMA — 7-Zip-style; uncommon in .zip    |
| 93        | Zstandard | ZStd — registered 2020; growing usage   |
| 99        | AES       | AES-encrypted payload; method in ExData |

This implementation supports **method 0 (Stored)** and **method 8 (DEFLATE)**.

### File Naming and Encoding

ZIP originally used IBM Code Page 437 (DOS) for file names. APPNOTE.TXT §4.4.4 defines:
- Bit 11 of General_Purpose_Bit_Flag = 0 → name is CP437
- Bit 11 of General_Purpose_Bit_Flag = 1 → name is UTF-8

Modern ZIP writers (7-Zip, Python's zipfile, Info-ZIP) set bit 11 and write UTF-8. This
implementation always writes UTF-8 and sets bit 11.

Directory entries are identified by a trailing `/` in the file name.

## Wire Format

All multi-byte integers are **little-endian** unless otherwise stated.

### Local File Header

```
Offset  Size  Value
──────  ────  ──────────────────────────────────────────────
0       4     Signature: 0x04034B50  ("PK\x03\x04")
4       2     Version_Needed: 20 (DEFLATE) or 10 (Stored)
6       2     General_Purpose_Bit_Flag
              bit  0: encrypted (0 in this impl)
              bit  3: data descriptor follows (set for streaming writes)
              bit 11: UTF-8 filename (always set)
8       2     Compression_Method: 0 or 8
10      4     Last_Mod_File_Time (MS-DOS format)
14      4     Last_Mod_File_Date (MS-DOS format)
18      4     CRC-32 of uncompressed data (0 if bit 3 set)
22      4     Compressed_Size   (0 if bit 3 set)
26      4     Uncompressed_Size (0 if bit 3 set)
30      2     File_Name_Length  (n)
32      2     Extra_Field_Length (e)
34      n     File_Name (UTF-8)
34+n    e     Extra_Field (variable; see §Extra Fields)
34+n+e  …     File_Data (compressed or raw bytes)
```

### Data Descriptor (optional, follows File_Data when bit 3 set)

```
Offset  Size  Value
──────  ────  ──────────────────────────────────────────────
0       4     Signature: 0x08074B50  ("PK\x07\x08") — optional but recommended
4       4     CRC-32
8       4     Compressed_Size
12      4     Uncompressed_Size
```

Present when General_Purpose_Bit_Flag bit 3 = 1 (streaming write). Readers must handle
archives both with and without the optional signature.

### Central Directory Header

One per entry; all written after the last Local File Header block.

```
Offset  Size  Value
──────  ────  ──────────────────────────────────────────────
0       4     Signature: 0x02014B50  ("PK\x01\x02")
4       2     Version_Made_By: 0x031E (Unix, v30 = supports 4.5 spec)
6       2     Version_Needed: 20 or 10
8       2     General_Purpose_Bit_Flag (same as Local Header)
10      2     Compression_Method
12      4     Last_Mod_File_Time
16      4     Last_Mod_File_Date
20      4     CRC-32
24      4     Compressed_Size
28      4     Uncompressed_Size
32      2     File_Name_Length (n)
34      2     Extra_Field_Length (e)
36      2     File_Comment_Length (c) — 0 in this impl
38      2     Disk_Number_Start: 0
40      2     Internal_File_Attributes: 0
42      4     External_File_Attributes
              Unix: (mode << 16), e.g., 0o100644 << 16 for regular file
              Directory: 0o040755 << 16
46      4     Relative_Offset_Of_Local_Header  (byte offset from start of ZIP)
50      n     File_Name (UTF-8)
50+n    e     Extra_Field
50+n+e  c     File_Comment (empty)
```

### End of Central Directory Record (EOCD)

```
Offset  Size  Value
──────  ────  ──────────────────────────────────────────────
0       4     Signature: 0x06054B50  ("PK\x05\x06")
4       2     Disk_Number: 0
6       2     Disk_With_CD_Start: 0
8       2     Num_Entries_On_Disk
10      2     Num_Entries_Total
12      4     CD_Size (bytes)
16      4     CD_Offset (byte offset from start of ZIP)
20      2     Comment_Length: 0
```

### MS-DOS Date/Time Encoding

```
DOS Time (16-bit):
  bits 15–11: hours   (0–23)
  bits 10–5:  minutes (0–59)
  bits  4–0:  seconds / 2 (0–29, representing 0–58s in 2s increments)

DOS Date (16-bit):
  bits 15–9: year - 1980  (0–127, representing 1980–2107)
  bits  8–5: month        (1–12)
  bits  4–0: day          (1–31)

Combine into a 32-bit value: (date << 16) | time
```

When the source file's mtime is unavailable, use a fixed timestamp:
1980-01-01 00:00:00 → `(0 << 16) | 0 = 0x00210000`.

### Extra Fields

Extra fields are `(tag: uint16, size: uint16, data: bytes)` TLV records. This impl
writes no extra fields by default (Extra_Field_Length = 0). Readers must skip unknown
extra fields without error.

Common tags (for reference):
```
0x0001 — ZIP64 extended information
0x5455 — Unix extended timestamp (UTC mtime)
0x7875 — Unix UID/GID
```

### DEFLATE Raw vs Wrapped

DEFLATE inside ZIP uses **raw DEFLATE** (RFC 1951) — **no zlib wrapper** (no 2-byte
`CMF/FLG` header, no Adler-32 checksum). The raw DEFLATE bit-stream starts at
`File_Data` immediately.

This is the same raw DEFLATE stream that CMP05 produces and consumes.

### CRC-32 Algorithm

Polynomial: 0xEDB88320 (reflected form of 0x04C11DB7)

```
Precompute crc_table[256]:
  for n in 0..255:
    c = n
    for _ in 0..7:
      if c & 1: c = 0xEDB88320 ^ (c >> 1)
      else:     c = c >> 1
    crc_table[n] = c

crc32(data, initial=0xFFFFFFFF):
  crc = initial
  for byte in data:
    crc = crc_table[(crc ^ byte) & 0xFF] ^ (crc >> 8)
  return crc ^ 0xFFFFFFFF
```

Update-able: `crc = crc32(chunk, initial=~previous_crc)` for streaming.

## Write API

The write API produces a complete `.zip` file in memory:

```
ZipWriter:
  new() → ZipWriter
  add_file(name: str, data: bytes, compress: bool = true) → void
  add_directory(name: str) → void
  finish() → bytes

# Convenience
zip_bytes(entries: [(name, data)]) → bytes
```

Implementation strategy:
1. For each entry: compress if `compress=true` and compressed < original, else store.
2. Accumulate Local Headers + data into a buffer; record each entry's offset.
3. Append Central Directory headers.
4. Append EOCD record.
5. Return the assembled buffer.

## Read API

The read API extracts files from a `.zip` archive:

```
ZipEntry:
  name: str
  size: int                  # uncompressed size
  compressed_size: int
  method: int                # 0 = stored, 8 = deflate
  crc32: int
  is_directory: bool

ZipReader:
  new(data: bytes) → ZipReader
  entries() → [ZipEntry]
  read(entry: ZipEntry) → bytes     # decompress + verify CRC
  read_by_name(name: str) → bytes   # convenience

# Convenience
unzip(data: bytes) → {name: bytes}  # returns all entries as a dict/map
```

Implementation strategy:
1. Find EOCD by scanning backwards from the end for signature `0x06054B50`.
2. Read all Central Directory headers to build the entry list.
3. For `read(entry)`: seek to `entry.local_offset`, skip the Local Header (re-read
   File_Name_Length + Extra_Field_Length to find data start), read `compressed_size`
   bytes, decompress, verify CRC-32.

## Public API Summary

```
# Write
zip_bytes(entries: [(name: str, data: bytes)]) → bytes
# or class-based ZipWriter for incremental builds

# Read
unzip(data: bytes) → {name: str → data: bytes}
# or class-based ZipReader for large archives
```

## Package Naming

| Language   | Package name                 | Module / namespace             |
|------------|------------------------------|--------------------------------|
| Python     | `coding-adventures-zip`      | `coding_adventures_zip`        |
| Go         | module `…/go/zip`            | package `zip`                  |
| Ruby       | `coding_adventures_zip`      | `CodingAdventures::Zip`        |
| TypeScript | `@coding-adventures/zip`     | `CodingAdventures.Zip`         |
| Rust       | `coding-adventures-zip`      | `coding_adventures_zip`        |
| Elixir     | `:coding_adventures_zip`     | `CodingAdventures.Zip`         |
| Lua        | `coding-adventures-zip`      | `coding_adventures.zip`        |
| Perl       | `CodingAdventures::Zip`      | `CodingAdventures::Zip`        |
| Swift      | `CodingAdventuresZip`        | `CodingAdventures.Zip`         |

**Dependencies:** each ZIP package depends on the corresponding language's `deflate`
(CMP05) package for the DEFLATE codec. The CRC-32 implementation is inlined (no separate
package — it's a trivial table-driven function).

## Test Cases

### TC-1: Round-trip single file (Stored)
```
data = b"hello, world"
archive = zip_bytes([("hello.txt", data)], compress=False)
result  = unzip(archive)
assert result["hello.txt"] == data
```

### TC-2: Round-trip single file (DEFLATE)
```
text    = ("the quick brown fox jumps over the lazy dog " * 10).encode()
archive = zip_bytes([("text.txt", text)])
result  = unzip(archive)
assert result["text.txt"] == text
```

### TC-3: Multiple files in one archive
```
files = [
    ("a.txt", b"file A content"),
    ("b.txt", b"file B content"),
    ("c.bin", bytes(range(256))),
]
archive = zip_bytes(files)
result  = unzip(archive)
for name, data in files:
    assert result[name] == data
```

### TC-4: Directory entry
```
w = ZipWriter()
w.add_directory("mydir/")
w.add_file("mydir/file.txt", b"contents")
archive = w.finish()
entries = ZipReader(archive).entries()
names   = {e.name for e in entries}
assert "mydir/" in names
assert "mydir/file.txt" in names
```

### TC-5: CRC-32 verification
```
# Corrupt the decompressed content's CRC in the header; reader must raise an error
archive = zip_bytes([("f.txt", b"test")])
# Flip a byte in the CRC-32 field (bytes 18–21 of the Local Header)
corrupted = bytearray(archive)
corrupted[18] ^= 0xFF   # corrupt CRC
try:
    unzip(bytes(corrupted))
    assert False, "should have raised"
except Exception:
    pass   # CRC mismatch detected
```

### TC-6: EOCD detection and multi-file random access
```
# 10 files; verify that reading file 5 doesn't require reading files 1–4
files   = [(f"f{i}.txt", f"content {i}".encode()) for i in range(10)]
archive = zip_bytes(files)
reader  = ZipReader(archive)
entry5  = next(e for e in reader.entries() if e.name == "f5.txt")
assert reader.read(entry5) == b"content 5"
```

### TC-7: Incompressible data stored without compression
```
# Random-ish data should be stored (method=0) when DEFLATE would make it larger
import os
data    = os.urandom(1024) if hasattr(os, 'urandom') else bytes(range(256)) * 4
archive = zip_bytes([("random.bin", data)])   # impl must choose Store when deflate > original
result  = unzip(archive)
assert result["random.bin"] == data
# Check method: reader should report method=0 for this entry
reader = ZipReader(archive)
entry  = reader.entries()[0]
assert entry.method == 0   # stored
```

### TC-8: Empty file
```
archive = zip_bytes([("empty.txt", b"")])
result  = unzip(archive)
assert result["empty.txt"] == b""
```

### TC-9: Large file (multi-block DEFLATE)
```
data    = b"abcdefghij" * 10000   # 100 KB
archive = zip_bytes([("big.bin", data)])
result  = unzip(archive)
assert result["big.bin"] == data
assert len(archive) < len(data)   # DEFLATE must compress repetitive data
```

### TC-10: Cross-compatibility with system ZIP tools
```
# Write a ZIP with our library; unzip with the system `unzip` CLI
# Write a ZIP with the system `zip` CLI; read with our library
# Both directions must round-trip all files exactly
```
*Manual or subprocess-based. The goal is format interoperability with Info-ZIP/7-Zip.*

### TC-11: Unicode filename
```
archive = zip_bytes([("日本語/résumé.txt", b"content")])
result  = unzip(archive)
assert "日本語/résumé.txt" in result
assert result["日本語/résumé.txt"] == b"content"
```

### TC-12: Nested paths
```
files = [
    ("root.txt",           b"root"),
    ("dir/file.txt",       b"nested"),
    ("dir/sub/deep.txt",   b"deep"),
]
archive = zip_bytes(files)
result  = unzip(archive)
for name, data in files:
    assert result[name] == data
```

## Security Considerations

- **Zip slip (path traversal)**: file names containing `../` or absolute paths (e.g.,
  `/etc/passwd`) can escape the target directory on extraction. Implementations must
  strip or reject names containing `..` components or beginning with `/` before writing
  to disk. The in-memory `unzip()` API returns a plain dict and is not directly
  vulnerable, but any disk-writing wrapper must sanitise paths.
- **Zip bomb / decompression bomb**: a small ZIP can expand to gigabytes. Impose a
  configurable `max_uncompressed_bytes` limit (default: 256 MB); return an error if
  exceeded. Similarly, cap `Num_Entries_Total` (e.g., 65535) before pre-allocating entry
  arrays.
- **CRC-32 is not a security check**: CRC detects accidental corruption only. Do not use
  it to verify that files have not been tampered with.
- **EOCD search**: scan for the EOCD signature from the end of the file, limited to the
  last 65535 + 22 bytes (EOCD comment can be up to 65535 bytes). Reject files where no
  valid EOCD is found rather than searching unboundedly.
- **Local Header vs Central Directory**: the authoritative source for sizes is the
  **Central Directory**, not the Local Header. Use Central Directory offsets for seeking;
  re-read Local Header only to get File_Name_Length + Extra_Field_Length for the skip
  calculation.
- **Method validation**: reject entries with unsupported methods (anything other than 0
  and 8 in this implementation) rather than silently producing garbage output.
- **Encryption**: entries with bit 0 of General_Purpose_Bit_Flag set are encrypted.
  Return a clear error rather than attempting to decompress encrypted data.
