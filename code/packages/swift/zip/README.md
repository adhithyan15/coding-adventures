# coding_adventures_zip (Swift)

ZIP archive format (PKZIP 1989) implemented from scratch in Swift — **CMP09** in the compression series.

## What it does

Creates and reads `.zip` files byte-compatible with standard ZIP tools (macOS Archive Utility, Info-ZIP, Python's `zipfile`, etc.). Each entry is compressed with RFC 1951 DEFLATE (method 8) or stored verbatim (method 0) if compression doesn't help.

## Where it fits

```
CMP02 (LZSS,    1982) — LZ77 + flag bits       ← dependency
CMP05 (DEFLATE, 1996) — LZ77 + Huffman         ← inlined here (raw RFC 1951)
CMP09 (ZIP,     1989) — DEFLATE container      ← this package
```

## Installation

In `Package.swift`:

```swift
.package(path: "../zip")
```

## Usage

### Create an archive

```swift
// One-shot
let archive = zip([
    ("hello.txt", Array("Hello, ZIP!".utf8)),
    ("data.bin",  [1, 2, 3]),
])

// Fine-grained control
var w = ZipWriter()
w.addDirectory("docs/")
w.addFile("docs/readme.txt", data: Array("Read me".utf8), compress: true)
let bytes = w.finish()
```

### Read an archive

```swift
// One-shot
let files = try unzip(archive)
print(String(bytes: files["hello.txt"]!, encoding: .utf8)!)  // "Hello, ZIP!"

// Fine-grained
let reader = try ZipReader(archive)
for entry in reader.entries() {
    print("\(entry.name)  \(entry.size) bytes")
}
let data = try reader.readByName("hello.txt")
```

### CRC-32

```swift
crc32(Array("hello world".utf8))  // 0x0D4A_1185
```

### Raw RFC 1951

ZIP method 8 carries raw DEFLATE without ZIP, zlib, or gzip framing. The
package exposes that owned codec directly:

```swift
let encoded = rawDeflate(Array("hello hello hello".utf8))
let decoded = try rawInflate(encoded)
let counted = try rawInflateCounted(encoded)
assert(counted.output == decoded)
assert(counted.bytesConsumed == encoded.count)
```

`rawInflate` and `rawInflateCounted` accept a caller-lowerable `maxOutput`
ceiling. The hard ceiling is `rawInflateMaxOutput` (256 MiB). Failures throw
`RawInflateError` with one stable payload-blind code from
`rawInflateErrorCodes`; no partial output is returned.

## API

| Function / Type | Description |
|-----------------|-------------|
| `ZipWriter` | Builds a ZIP archive in memory. |
| `ZipWriter.addFile(_:data:compress:)` | Add a file entry. |
| `ZipWriter.addDirectory(_:)` | Add a directory entry. |
| `ZipWriter.finish()` | Return completed archive as `[UInt8]`. |
| `ZipReader.init(_:)` | Parse a ZIP archive binary. Throws on malformed input. |
| `ZipReader.entries()` | List all `ZipEntry` values. |
| `ZipReader.read(_:)` | Decompress and CRC-validate an entry. |
| `ZipReader.readByName(_:)` | Convenience wrapper. |
| `zip(_:compress:)` | One-shot compress. |
| `unzip(_:)` | One-shot decompress → `[String: [UInt8]]`. |
| `rawDeflate(_:)` | Encode a raw RFC 1951 stream. |
| `rawInflate(_:maxOutput:)` | Decode stored, fixed, dynamic, and multi-block raw streams. |
| `rawInflateCounted(_:maxOutput:)` | Decode and report the exact number of input bytes consumed. |
| `rawInflateMaxOutput` | Default and hard 256 MiB output ceiling. |
| `rawInflateErrorCodes` | Ordered stable raw-inflate error taxonomy. |
| `RawInflateError` | Typed payload-blind raw-inflate failure. |
| `RawInflateResult` | Decoded bytes plus exact consumed-byte count. |
| `crc32(_:initial:)` | CRC-32 (polynomial 0xEDB88320). |
| `dosDatetime(year:month:day:hour:minute:second:)` | MS-DOS timestamp. |
| `dosEpoch` | `0x00210000` — 1980-01-01 00:00:00. |
| `ZipError` | Error enum: `.malformed`, `.crcMismatch`, `.notFound`, `.unsupported`. |

## Security boundary

- DEFLATE output is capped at 256 MiB, and callers may only lower that limit.
- The ZIP reader requires the raw inflater to consume the entire declared
  compressed payload and requires the exact declared uncompressed size before
  checking CRC-32, rejecting suffix cavities and size mismatches.
- Raw-inflate errors contain no input bytes, offsets, lengths, paths, or partial
  output.
- CRC-32 detects accidental corruption; it is not authentication.
- Production is pure in-memory computation. Fixture reads and the independent
  Python/zlib interoperability oracle are test-only.

## Running tests

```bash
swift test --enable-code-coverage
```
