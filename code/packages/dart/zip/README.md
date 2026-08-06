# coding_adventures_zip

ZIP archive format (PKZIP, 1989) for Dart. This package implements the
CMP09 entry in the coding-adventures compression series — a complete ZIP
reader and writer written from scratch, including its own RFC 1951 DEFLATE
codec, with no native dependencies.

## What It Provides

- `zipBytes(entries) → Uint8List` — compress `(name, data)` pairs into a
  valid `.zip` archive
- `unzip(data) → Map<String, Uint8List>` — decompress every file entry in a
  `.zip` archive into a name → bytes map
- `ZipWriter` — incremental archive builder (`addFile`, `addDirectory`, `finish`)
- `ZipReader` — random-access archive reader (`entries`, `read`, `readByName`)
- `crc32`, `deflateCompress`, `inflate`, `dosDatetime` — the building blocks,
  exported for direct use

### Algorithm Highlights

ZIP is a **container** format, not a compression algorithm in its own
right: it bundles one or more files, each compressed independently, behind
two parallel header structures.

1. **Local File Header + File Data**, once per entry, written as the
   archive is built.
2. **Central Directory**, one header per entry, written after all entries —
   this is the *authoritative* record of every entry's size, method, and
   location, letting a reader jump straight to any single file's data
   without scanning the whole archive.
3. **End Of Central Directory (EOCD)** — a fixed-size trailer a reader finds
   by scanning backward from EOF, which points at the Central Directory.

Each entry is compressed with **DEFLATE** (method 8, RFC 1951) or stored
verbatim (method 0) — whichever is smaller. DEFLATE itself combines LZ77
back-references (found via the sibling `coding_adventures_lzss` package)
with Huffman entropy coding.

### Why this package doesn't depend on `coding_adventures_deflate`

The CMP09 spec calls for ZIP to depend on the language's `deflate` (CMP05)
package. This package does not, because `coding_adventures_deflate`'s
`compress`/`decompress` pair uses a private, self-designed wire format for
internal round-tripping — not the standard RFC 1951 bit-stream a real ZIP
entry must carry. Depending on it would produce archives no real `unzip`
could open, and would be unable to read archives from Python, Java,
Microsoft Office, or the `zip`(1) command.

Every other language's `zip` package in this repository (Python, Go, Rust,
Ruby, TypeScript, Elixir, Lua, Swift, Perl) follows the same shape this
package does: depend on the sibling `lzss` package for LZ77 match-finding
only, and implement RFC 1951 framing directly. See `lessons.md` for the
full investigation.

### Archive Layout

```
┌─────────────────────────────────────────────────────┐
│  [Local File Header + File Data]  ← entry 1          │
│  [Local File Header + File Data]  ← entry 2          │
│  ...                                                 │
│  ══════════ Central Directory ══════════             │
│  [Central Dir Header]  ← entry 1 (has local offset)  │
│  [Central Dir Header]  ← entry 2                     │
│  [End of Central Directory Record]                   │
└─────────────────────────────────────────────────────┘
```

### DEFLATE Inside ZIP

ZIP method 8 stores **raw RFC 1951 DEFLATE** — no zlib wrapper. This
package's writer emits fixed-Huffman blocks (BTYPE=01) only, keeping the
encoder simple. Its reader (`inflate`) decodes **all three** RFC 1951
block types — stored, fixed Huffman, and dynamic Huffman — because
real-world producers (`zip`(1), Python's `zipfile`, Java's `jar`, Microsoft
Office) overwhelmingly emit dynamic-Huffman blocks.

### Compression Series

```
CMP00 (LZ77)     — Sliding-window back-references
CMP01 (LZ78)     — Explicit dictionary (trie)
CMP02 (LZSS)     — LZ77 + flag bits
CMP03 (LZW)      — LZ78 + pre-initialised alphabet; GIF
CMP04 (Huffman)  — Entropy coding
CMP05 (DEFLATE)  — LZ77 + Huffman; ZIP/gzip/PNG/zlib
CMP09 (ZIP)      — DEFLATE container; universal archive  ← this package
```

## Usage

```dart
import 'dart:convert';
import 'dart:typed_data';
import 'package:coding_adventures_zip/coding_adventures_zip.dart';

void main() {
  // Convenience functions.
  final archive = zipBytes([
    ('hello.txt', utf8.encode('hello, world!')),
    ('nested/dir/file.txt', utf8.encode('nested content')),
  ]);
  final files = unzip(archive);
  print(utf8.decode(files['hello.txt']!));

  // Incremental writer / random-access reader.
  final w = ZipWriter()
    ..addDirectory('mydir/')
    ..addFile('mydir/file.txt', Uint8List.fromList(utf8.encode('contents')));
  final bytes = w.finish();

  final reader = ZipReader(bytes);
  for (final entry in reader.entries()) {
    print('${entry.name}: ${entry.size} bytes, method ${entry.method}');
  }
  print(utf8.decode(reader.readByName('mydir/file.txt')));
}
```

## Building and Testing

```bash
dart pub get
dart test
```

`test/zip_test.dart` includes a real CLI-interoperability test (TC-10) that
shells out to the system `zip`/`unzip` binaries in both directions. It
skips gracefully (does not fail) when Info-ZIP isn't on `PATH`.

## Limitations

- No ZIP64 (archives/entries > 4 GB, or more than 65535 entries).
- No encryption support — encrypted entries (GP flag bit 0) are rejected
  with a clear error rather than silently producing garbage.
- No multi-disk archives.
- The writer only emits fixed-Huffman DEFLATE blocks; it never chooses
  dynamic Huffman even when it would compress better (the reader supports
  both, so this only affects the ratio of archives *this* package writes).
- Decompression is capped at 256 MB by default (`defaultMaxOutputBytes`) as
  a decompression-bomb guard; pass a different limit to `read`/`unzip`/
  `inflate` if you need more.
