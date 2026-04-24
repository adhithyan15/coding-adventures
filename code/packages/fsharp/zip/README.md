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

// One-shot extract everything
let all : ZipEntry list = ZipArchive.unzip archive
```

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
- **Dynamic Huffman blocks** (BTYPE=10) are not decompressed — only Stored and
  fixed Huffman blocks are supported (matching what this compressor emits).
