# coding-adventures/go/zip

Go implementation of the **ZIP archive format** (CMP09, PKZIP 1989) — part of the
[coding-adventures](https://github.com/adhithyan15/coding-adventures) compression series.

## What Is ZIP?

ZIP is a lossless archive format that bundles one or more files into a single `.zip` file,
optionally compressing each entry independently using **DEFLATE** (method 8) or storing it
verbatim (method 0).

## How It Fits the Stack

```
CMP02 (LZSS,    1982) — LZ77 + flag bits.  ← dependency
CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib.
CMP09 (ZIP,     1989) — DEFLATE container; universal archive.  ← YOU ARE HERE
```

This package uses `coding-adventures/go/lzss` for LZ77 tokenization and owns the
raw RFC 1951 codec required by ZIP. The encoder emits stored or fixed-Huffman
blocks. The decoder accepts stored, fixed, and dynamic-Huffman blocks, including
multi-block streams, overlapping copies, and the full 32 KiB distance window.

## Usage

### Writing

```go
import zip "github.com/adhithyan15/coding-adventures/code/packages/go/zip"

// Convenience
archive := zip.Zip([]struct{ Name string; Data []byte }{
    {"hello.txt", []byte("Hello, world!")},
    {"data.bin",  bytes.Repeat([]byte{0xAB}, 1024)},
})

// Incremental
zw := zip.NewZipWriter()
zw.AddDirectory("docs/")
zw.AddFile("docs/readme.txt", []byte("See README."), true)
archive := zw.Finish()
```

### Reading

```go
// Convenience — extract everything
files, err := zip.Unzip(archive)
fmt.Println(string(files["hello.txt"]))

// Random access
zr, err := zip.NewZipReader(archive)
for _, e := range zr.Entries() {
    fmt.Printf("%s: %d bytes (method %d)\n", e.Name, e.Size, e.Method)
}
data, err := zr.ReadByName("hello.txt")
```

## API Reference

| Type / Function | Description |
|-----------------|-------------|
| `NewZipWriter() *ZipWriter` | Create empty writer |
| `(*ZipWriter).AddFile(name, data, compress)` | Add file; DEFLATE if smaller |
| `(*ZipWriter).AddDirectory(name)` | Add directory entry |
| `(*ZipWriter).Finish() []byte` | Write CD + EOCD; return archive |
| `NewZipReader(data) (*ZipReader, error)` | Parse archive |
| `(*ZipReader).Entries() []ZipEntry` | All entries |
| `(*ZipReader).Read(entry) ([]byte, error)` | Decompress + verify CRC |
| `(*ZipReader).ReadByName(name) ([]byte, error)` | Find by name then read |
| `Zip(entries) []byte` | Create archive from slice |
| `Unzip(data) (map[string][]byte, error)` | Extract all files |
| `RawDeflate(data) []byte` | Encode raw RFC 1951 bytes without ZIP/zlib/gzip framing |
| `RawInflate(data, maxOutput...) ([]byte, error)` | Strictly decode raw RFC 1951 with an optional lower output cap |
| `RawInflateCounted(data, maxOutput...) (RawInflateResult, error)` | Decode and report exact compressed bytes consumed |
| `RawInflateMaxOutput` | Hard/default 256 MiB output ceiling |
| `RawInflateError` | Typed failure carrying one stable payload-blind error code |
| `CRC32(data, initial) uint32` | CRC-32 (polynomial 0xEDB88320) |
| `DOSDatetime(y,m,d,h,min,s) uint32` | MS-DOS timestamp encoder |
| `DOSEpoch` | Constant: 1980-01-01 00:00:00 |

`ZipReader.Read` uses counted decoding and rejects a declared compressed payload
that contains suffix bytes. It also requires the decoded size to match the
Central Directory exactly; excess output is never silently trimmed.

CRC-32 detects accidental corruption. It is not authentication and must not be
used as a cryptographic integrity check.
