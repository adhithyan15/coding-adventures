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
| `Unzip(byte[] data, long maxTotalBytes = 512 MiB)` | Extract all entries from an archive. Throws `InvalidDataException` if the combined decompressed size of all entries exceeds `maxTotalBytes` (decompression-bomb guard). |

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

## Security hardening

This package parses and produces archives that may cross a trust boundary
(untrusted uploads, third-party `.zip` files), so it defends against a few
adversarial-input classes beyond simple format conformance:

- **Zip-slip / path traversal — write side**: `ZipWriter.AddFile`/`AddDirectory`
  normalize every entry name before writing it — backslashes become forward
  slashes, a leading Windows drive letter (`C:`) is dropped, and empty, `.`,
  and `..` segments are dropped — the same `normalize_part_name` pattern used
  by `rust/opc-writer`. An archive *produced by this package* can therefore
  never contain a `..`-shaped, absolute, or drive-rooted entry name.
  **This does not cover the read side.** Per `code/specs/CMP09-zip.md`'s
  Security Considerations, `ZipReader`/`ZipArchive.Unzip` return entry names
  from a third-party archive verbatim and unsanitized — the in-memory API
  itself performs no filesystem I/O so it is "not directly vulnerable," but
  any caller that writes extracted entries to disk (e.g.
  `File.WriteAllBytes(Path.Combine(outDir, entry.Name), entry.Data)`) is
  responsible for sanitizing `entry.Name` first, exactly as it would be with
  any other ZIP reader.
- **Integer-overflow-safe bounds checking**: every offset/size field read
  from an untrusted archive (`cd_offset`, `cd_size`, `local_offset`,
  `compressed_size`, `uncompressed_size`) is attacker-controlled and can be
  as large as `0xFFFFFFFF`. All arithmetic that combines such a field with a
  position is done in `long` and range-checked *before* narrowing to `int`,
  so a value ≥ `0x80000000` can't wrap negative and slip past a bounds check.
- **ZIP32 field-width limits enforced on write**: more than 65535 entries, or
  a single entry name encoding to more than 65535 UTF-8 bytes, is rejected
  outright rather than silently truncated (which would desynchronize a
  declared length from the bytes actually written and corrupt the archive).
- **Decompression-bomb guards**: `DeflateDecompressor` caps any single
  entry's decompressed output at 256 MiB, and `ZipArchive.Unzip` separately
  caps the *aggregate* decompressed size across all entries in one archive
  (default 512 MiB, overridable via `maxTotalBytes`) — several
  moderately-sized entries can each stay under the per-entry cap while still
  exhausting memory in total.

## Dependencies

- `CodingAdventures.Lzss` — LZ77/LZSS match-finder used inside the DEFLATE encoder.
