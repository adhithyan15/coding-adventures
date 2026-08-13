# @coding-adventures/zip

ZIP archive format (PKZIP 1989) implemented from scratch in TypeScript — **CMP09** in the compression series.

## What it does

Creates and reads `.zip` files that are byte-compatible with standard ZIP tools (macOS Archive Utility, WinZip, Info-ZIP, Python's `zipfile`, etc.). Each file entry is compressed with RFC 1951 DEFLATE (fixed Huffman, method 8) or stored verbatim (method 0) if compression doesn't help.

## Where it fits

```
CMP02 (LZSS,    1982) — LZ77 + flag bits       ← dependency
CMP05 (DEFLATE, 1996) — LZ77 + Huffman         ← inlined here (raw RFC 1951)
CMP09 (ZIP,     1989) — DEFLATE container      ← this package
```

ZIP uses the same DEFLATE algorithm as gzip/PNG/zlib, but without the zlib wrapper and without a shared dictionary across entries.

## Installation

```bash
npm install @coding-adventures/zip
```

## Usage

### Create an archive

```typescript
import { zipBytes, ZipWriter } from "@coding-adventures/zip";

// Convenience: array of [name, data] pairs
const archive = zipBytes([
  ["hello.txt", new TextEncoder().encode("Hello, ZIP!")],
  ["data.bin",  new Uint8Array([1, 2, 3])],
]);

// Full control with ZipWriter
const w = new ZipWriter();
w.addDirectory("docs/");
w.addFile("docs/readme.txt", new TextEncoder().encode("Read me"));
const zip = w.finish();
```

### Read an archive

```typescript
import { ZipReader, unzip } from "@coding-adventures/zip";

// Convenience: decompress everything
const files = unzip(archive);
console.log(new TextDecoder().decode(files.get("hello.txt")!));

// Fine-grained: list entries, read by name
const reader = new ZipReader(archive);
for (const entry of reader.entries()) {
  console.log(entry.name, entry.size, entry.method);
}
const data = reader.readByName("hello.txt");
```

### CRC-32 utility

```typescript
import { crc32 } from "@coding-adventures/zip";
crc32(new TextEncoder().encode("hello world")); // 0x0D4A1185
```

## API

| Symbol | Description |
|--------|-------------|
| `ZipWriter` | Incrementally builds an archive in memory. |
| `ZipWriter#addFile(name, data, compress?)` | Add a file entry. |
| `ZipWriter#addDirectory(name)` | Add a directory entry (name must end with `/`). |
| `ZipWriter#finish()` | Emit the complete archive as `Uint8Array`. |
| `ZipReader` | Parses an in-memory ZIP archive. Takes an optional `{ maxOutput }` ceiling. |
| `ZipReader#entries()` | List all `ZipEntry` metadata objects. |
| `ZipReader#read(entry)` | Decompress and return one entry's bytes. |
| `ZipReader#readByName(name)` | Convenience wrapper for `read`. |
| `zipBytes(entries, compress?)` | One-shot compress. |
| `unzip(data)` | One-shot decompress → `Map<string, Uint8Array>`. |
| `crc32(data, initial?)` | CRC-32 (polynomial 0xEDB88320). |
| `rawDeflate(data)` | Compress to a raw RFC 1951 stream — no ZIP, zlib, or gzip framing. |
| `rawInflate(data, maxOutput?)` | Decompress a raw RFC 1951 stream. Reads all three block types. |
| `rawInflateCounted(data, maxOutput?)` | Decompress and report the exact number of input bytes consumed. |
| `RawInflateError` | Stable payload-blind failure with a portable `code`. |
| `RAW_INFLATE_MAX_OUTPUT` | Hard 256 MiB output ceiling. Callers may only lower it. |
| `dosDatetime(...)` | Encode MS-DOS timestamp. |
| `DOS_EPOCH` | Constant `0x00210000` — 1980-01-01 00:00:00. |

### Raw DEFLATE, on its own

DEFLATE is not a ZIP feature that happens to live here. It is the compressor
inside `zlib`, `gzip`, and PNG's `IDAT` chunk, and those three differ from ZIP
only in what they wrap around it — zlib a two-byte header and a trailing
Adler-32, gzip a ten-byte header and a trailing CRC-32, ZIP nothing at all.

```typescript
import { rawDeflate, rawInflate } from "@coding-adventures/zip";

const raw = rawDeflate(new TextEncoder().encode("hello hello hello"));
rawInflate(raw); // the original bytes
```

A second copy of this in another package would be a second place for a
bit-packing bug to hide, which is why it is exported rather than duplicated.

**`rawInflate` reads bytes you did not write.** Malformed input throws a
`RawInflateError` with a stable, payload-blind `code`; it never returns partial
or wrong output. Output is capped at 256 MiB by default, and you should lower it whenever you know the
answer's size:

```typescript
rawInflate(untrusted, 1 << 20); // refuse anything over 1 MB
```

The limit must be a non-negative safe integer at or below the hard ceiling and
is validated before the output buffer is allocated. `rawInflateCounted` also
reports where BFINAL ended the stream, excluding whole trailing bytes so zlib,
gzip, and PNG wrappers can reject covert cavities.

The cap matters because DEFLATE's expansion ratio reaches **1032:1** — a
two-symbol pair copies up to 258 bytes — so a few hundred kilobytes of hostile
input can demand hundreds of megabytes of output.

`ZipReader.read` uses the entry's declared uncompressed size as a cap too, but
only as the *smaller* of it and the reader's own ceiling. The declared size is
four bytes the archive chose: trusting it alone would swap a fixed limit for an
attacker-chosen one, and the CRC-32 that catches the lie runs only after the
memory is already committed. Lower the reader's ceiling with
`new ZipReader(bytes, { maxOutput: 1 << 20 })`.

## Design notes

**Why inline DEFLATE?** The repo's `@coding-adventures/deflate` package uses a custom non-RFC-1951 wire format for educational isolation. ZIP requires raw RFC 1951 DEFLATE with no zlib wrapper, so DEFLATE is reimplemented inline here.

**Writing and reading are deliberately asymmetric.** The encoder emits one
fixed-Huffman block (BTYPE=01); the decoder reads stored (00), fixed (01) *and*
dynamic (10) blocks. That is not an oversight in either direction. Fixed
Huffman is simple, fast, and produces a perfectly legal archive that every tool
accepts — so writing more is optional. But dynamic Huffman is what zlib and
Info-ZIP emit for anything but the smallest inputs, so *reading* less means
failing on most archives the world actually produces. Reading is exact rather
than broadly permissive: malformed trees and reserved symbols fail closed.

The same asymmetry shows up one level down, in length symbol **285**. RFC 1951
gives length 258 — the longest match DEFLATE can express — two encodings:
symbol 284 with five extra bits, and symbol 285 with none. The encoder keeps
using 284 so its output stays byte-stable; the decoder accepts both, because a
258-byte match is exactly what a long run of one byte produces and most
encoders reach for the cheaper symbol.

The decoder's conformance is checked against **foreign zlib encoders and
decoders** rather than against itself: round-tripping our encoder through our
decoder only proves the two agree with each other. The shared neutral corpus is
also consumed by Python's standard zlib. Its one documented oracle exception is
the RFC 1951 `HDIST + 1 = 32` header: the final two reserved-symbol slots may be
advertised with zero lengths even though a default zlib build rejects that
valid field width. Symbols 30 and 31 still fail if actually decoded.

**BigInt accumulator.** JavaScript's bitwise operators are 32-bit. The DEFLATE bit buffer can hold up to ~48 bits, so `BitWriter`/`BitReader` use a `bigint` accumulator to avoid silent truncation.

**Auto-compression.** `addFile` tries DEFLATE and falls back to Stored if the compressed form is not smaller. This matches the PKZIP/Info-ZIP heuristic.

**EOCD-first reading.** `ZipReader` scans from the end of the file for the End of Central Directory record, then navigates to the Central Directory. This matches the ZIP specification and handles comments correctly.

## Running tests

```bash
npm install
npx vitest run --coverage
python -m pytest ../../../../scripts/tests/test_zip_raw_rfc1951_fixtures.py -q
```
