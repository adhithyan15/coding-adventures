# CodingAdventures.Zip.FSharp

Pure F# implementation of the ZIP archive format (CMP09 in the series).

ZIP bundles one or more files into a single `.zip` archive, compressing each
entry independently with DEFLATE (method 8) or storing it verbatim (method 0).
The same container format underlies Java JARs, Office Open XML (`.docx`),
Android APKs, Python wheels, and many other real-world formats.

## How it fits in the stack

```
CMP00 LZ77     →  CMP02 LZSS  →  CMP05 DEFLATE  →  CMP09 ZIP  ← THIS PACKAGE
CMP04 Huffman  ↗
```

This package depends on `CodingAdventures.Lzss.FSharp` for the LZSS
match-finding step inside the DEFLATE compressor.

## Usage

### Write a ZIP archive

```fsharp
open CodingAdventures.Zip.FSharp
open System.Text

// Build incrementally
let writer = ZipWriter()
writer.AddFile("hello.txt", Encoding.UTF8.GetBytes("hello, world!"))
writer.AddDirectory("docs/")
writer.AddFile("docs/readme.txt", Encoding.UTF8.GetBytes("see readme"))
let archive : byte[] = writer.Finish()

// Or one-shot via the convenience module
let entries = [
    { Name = "a.txt"; Data = Encoding.UTF8.GetBytes("file a") }
    { Name = "b.txt"; Data = Encoding.UTF8.GetBytes("file b") }
]
let archive2 = ZipArchive.zip entries
```

### Read a ZIP archive

```fsharp
// List all entries (Data is empty until Read is called)
let reader = ZipReader(archive)
for entry in reader.Entries do
    printfn "%s" entry.Name

// Random-access: read only one file
let bytes = reader.Read("hello.txt")

// Raw RFC 1951 without ZIP/zlib/gzip framing
let raw = RawRfc1951.rawDeflate bytes
let decoded = RawRfc1951.rawInflateCounted raw RawRfc1951.maxOutput

// One-shot extract everything
let all : ZipEntry list = ZipArchive.unzip archive
```

`RawRfc1951` accepts stored, fixed-Huffman, dynamic-Huffman, and multi-block
streams. Its caller-lowerable 256 MiB ceiling is checked before every output
append or copy, counted inflate reports exact compressed bytes consumed, and
`RawInflateError.Code` uses the shared 14-code portable taxonomy. `crc32`
supports incremental checksums. For one-shot extraction with a lower aggregate
budget, use `ZipArchive.unzipWithLimit`.

## Wire format

```
[Local File Header]  30 + name_len + extra_len bytes
[File Data]          compressed_size bytes
...
[Central Directory Header]  46 + name_len bytes  (one per entry)
...
[End of Central Directory]  22 bytes
```

All integers are little-endian. Filenames are UTF-8 (General Purpose Bit 11).

## Compression policy

| Condition | Method |
|---|---|
| Empty file | Stored (0) |
| DEFLATE smaller than original | DEFLATE (8) |
| DEFLATE >= original (random/binary data) | Stored (0) |
| compress=false | Stored (0) |

## Running tests

```bash
cd code/packages/fsharp/zip
mkdir -p .dotnet .artifacts
HOME="$PWD/.dotnet" DOTNET_SKIP_FIRST_TIME_EXPERIENCE=1 DOTNET_CLI_HOME="$PWD/.dotnet" \
  dotnet test tests/CodingAdventures.Zip.Tests/CodingAdventures.Zip.Tests.fsproj \
  --disable-build-servers --artifacts-path .artifacts
```

Or simply run the `BUILD` script from the package root.

## Security notes

- **CRC-32 is not cryptographic** — it detects accidental corruption only.
  For tamper detection, use AES-GCM or a signed manifest.
- **Decompression bomb guard**: output is capped at 256 MB.
- **Encrypted entries** are rejected with a clear `InvalidDataException`.
- **Portable raw RFC 1951 decoding** supports Stored, fixed Huffman, dynamic
  Huffman, and multi-block streams with stable payload-blind failures.
- **Offset/size arithmetic is overflow-checked**: the Central Directory's
  `Compressed_Size`, `Uncompressed_Size`, `Relative_Offset_Of_Local_Header`,
  `CD_Offset` and `CD_Size` fields are attacker-controlled `uint32` values;
  `ZipReader` validates them in `int64` before narrowing to `int` so a
  malformed archive fails closed with `InvalidDataException` instead of
  silently misbehaving.
