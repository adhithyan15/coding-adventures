# kotlin/zip — CMP09: ZIP Archive Format

Pure Kotlin implementation of the ZIP archive format (PKZIP, 1989), the ninth
entry in the compression series. ZIP is the universal container format underlying
`.jar`, `.docx`, `.apk`, `.wheel`, and countless others.

## How it fits in the stack

```
CMP00 (LZ77,    1977) — Sliding-window backreferences.
CMP01 (LZ78,    1978) — Explicit dictionary (trie).
CMP02 (LZSS,    1982) — LZ77 + flag bits.
CMP03 (LZW,     1984) — LZ78 + pre-initialized alphabet; GIF.
CMP04 (Huffman, 1952) — Entropy coding.
CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib.
CMP09 (ZIP,     1989) — DEFLATE container; universal archive.  ← THIS PACKAGE
```

This package depends on `kotlin/lzss` for LZ77 tokenisation inside DEFLATE.

## Usage

### Create an archive

```kotlin
val writer = ZipWriter()
writer.addFile("hello.txt", "hello, world!".toByteArray())
writer.addFile("data.bin", someBytes, compress = false) // force Stored
writer.addDirectory("subdir/")
val archive: ByteArray = writer.finish()
```

### Read an archive

```kotlin
val reader = ZipReader(archive)
for (entry in reader.entries) {
    println("${entry.name}")
}
val data = reader.read("hello.txt")
```

### One-shot convenience

```kotlin
// Zip a list of ZipEntry objects
val archive = ZipArchive.zip(listOf(
    ZipEntry("a.txt", "hello".toByteArray()),
    ZipEntry("b.txt", "world".toByteArray())
))

// Unzip back to a list
val entries: List<ZipEntry> = ZipArchive.unzip(archive)
```

## Wire format

ZIP stores Local File Headers before each entry's data, then a Central Directory
at the end. The Central Directory is the authoritative index — readers scan to
the End of Central Directory Record (EOCD) first, then seek to the CD.

All integers are **little-endian**.

```
[Local File Header + compressed data] × N
[Central Directory Header] × N
[End of Central Directory Record]
```

## Compression

Each file is independently compressed with RFC 1951 DEFLATE (method 8) or stored
verbatim (method 0). This implementation uses fixed-Huffman blocks (BTYPE=01)
with the LZSS package for LZ77 match finding. DEFLATE is only used when it
actually reduces size; otherwise the entry falls back to Stored automatically.

## Running tests

```bash
cd code/packages/kotlin/zip
gradle test
```
