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
| `crc32(_:initial:)` | CRC-32 (polynomial 0xEDB88320). |
| `dosDatetime(year:month:day:hour:minute:second:)` | MS-DOS timestamp. |
| `dosEpoch` | `0x00210000` — 1980-01-01 00:00:00. |
| `ZipError` | Error enum: `.malformed`, `.crcMismatch`, `.notFound`, `.unsupported`. |

## Running tests

```bash
swift test --enable-code-coverage
```
