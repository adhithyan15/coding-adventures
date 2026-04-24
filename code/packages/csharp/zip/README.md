# CodingAdventures.Zip (CSharp) — CMP09

Pure C# implementation of the ZIP archive format (PKZIP, 1989).

## What it does

Reads and writes `.zip` files — the same format used by Java JARs, Office Open XML
(`.docx`/`.xlsx`), Android APKs, Python wheels, and countless other tools.

Each file is compressed independently using **raw RFC 1951 DEFLATE** (method 8) or
stored verbatim (method 0). CRC-32 integrity checks are applied on extraction.

## Where it fits

```
CMP00 (LZ77,    1977) — Sliding-window backreferences
CMP02 (LZSS,    1982) — LZ77 + flag bits  ← dependency
CMP05 (DEFLATE, 1996) — LZ77 + Huffman   ← algorithm used inside ZIP
CMP09 (ZIP,     1989) — DEFLATE container ← THIS PACKAGE
```

## Usage

```csharp
using CodingAdventures.Zip;
using System.Text;

// ── Write ────────────────────────────────────────────────────────────────────

var writer = new ZipWriter();
writer.AddFile("hello.txt", Encoding.UTF8.GetBytes("hello, world!"));
writer.AddDirectory("docs/");
writer.AddFile("docs/readme.txt", Encoding.UTF8.GetBytes("see readme"), compress: false);
byte[] archive = writer.Finish();

// ── Read ─────────────────────────────────────────────────────────────────────

var reader = new ZipReader(archive);
foreach (var entry in reader.Entries)
    Console.WriteLine($"{entry.Name}");

byte[] data = reader.Read("hello.txt");

// ── Convenience API ──────────────────────────────────────────────────────────

byte[] zipped = ZipArchive.Zip([
    new ZipEntry("a.txt", Encoding.UTF8.GetBytes("file A")),
    new ZipEntry("b.txt", Encoding.UTF8.GetBytes("file B")),
]);

IReadOnlyList<ZipEntry> extracted = ZipArchive.Unzip(zipped);
```

## API

### `ZipWriter`

| Method | Description |
|--------|-------------|
| `AddFile(name, data, compress=true)` | Add a file entry. DEFLATE is used when it reduces size. |
| `AddDirectory(name)` | Add a directory entry (name must end with `/`). |
| `Finish()` | Return the complete archive as `byte[]`. |

### `ZipReader`

| Member | Description |
|--------|-------------|
| `ZipReader(byte[] data)` | Parse an in-memory archive. |
| `Entries` | `IReadOnlyList<ZipEntry>` of all entries (names only; data on demand). |
| `Read(string name)` | Decompress and return the named entry's bytes. Verifies CRC-32. |
| `ReadByName(string name)` | Alias for `Read`. |

### `ZipArchive` (static convenience)

| Method | Description |
|--------|-------------|
| `Zip(IEnumerable<ZipEntry>)` | Compress a list of entries into a ZIP archive. |
| `Unzip(byte[])` | Extract all entries from an archive. |

### `ZipEntry`

```csharp
public record ZipEntry(string Name, byte[] Data);
```

## Format details

- All integers are little-endian.
- Filenames are UTF-8 (GP flag bit 11 = 1).
- Timestamps use the fixed DOS epoch 1980-01-01 00:00:00.
- Compression: DEFLATE (method 8) if it saves space; Stored (method 0) otherwise.
- No encryption, no ZIP64, no multi-disk archives.

## Dependencies

- `CodingAdventures.Lzss` — LZ77/LZSS match-finder used inside the DEFLATE encoder.
