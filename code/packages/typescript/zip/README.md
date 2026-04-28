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
| `ZipReader` | Parses an in-memory ZIP archive. |
| `ZipReader#entries()` | List all `ZipEntry` metadata objects. |
| `ZipReader#read(entry)` | Decompress and return one entry's bytes. |
| `ZipReader#readByName(name)` | Convenience wrapper for `read`. |
| `zipBytes(entries, compress?)` | One-shot compress. |
| `unzip(data)` | One-shot decompress → `Map<string, Uint8Array>`. |
| `crc32(data, initial?)` | CRC-32 (polynomial 0xEDB88320). |
| `dosDatetime(...)` | Encode MS-DOS timestamp. |
| `DOS_EPOCH` | Constant `0x00210000` — 1980-01-01 00:00:00. |

## Design notes

**Why inline DEFLATE?** The repo's `@coding-adventures/deflate` package uses a custom non-RFC-1951 wire format for educational isolation. ZIP requires raw RFC 1951 DEFLATE with no zlib wrapper, so DEFLATE is reimplemented inline here.

**BigInt accumulator.** JavaScript's bitwise operators are 32-bit. The DEFLATE bit buffer can hold up to ~48 bits, so `BitWriter`/`BitReader` use a `bigint` accumulator to avoid silent truncation.

**Auto-compression.** `addFile` tries DEFLATE and falls back to Stored if the compressed form is not smaller. This matches the PKZIP/Info-ZIP heuristic.

**EOCD-first reading.** `ZipReader` scans from the end of the file for the End of Central Directory record, then navigates to the Central Directory. This matches the ZIP specification and handles comments correctly.

## Running tests

```bash
npm install
npx vitest run --coverage
```
