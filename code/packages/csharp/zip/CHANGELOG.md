# Changelog — CodingAdventures.Zip (CSharp)

## [0.1.0] — 2026-04-24

### Added

- `ZipWriter` — incremental in-memory ZIP archive builder.
  - `AddFile(name, data, compress=true)` — DEFLATE if smaller, Stored otherwise.
  - `AddDirectory(name)` — directory entry (trailing `/`).
  - `Finish()` — writes Central Directory + EOCD, returns complete archive bytes.
- `ZipReader` — EOCD-first random-access ZIP reader.
  - `ZipReader(byte[])` — parses archive, validates EOCD + Central Directory.
  - `Entries` — `IReadOnlyList<ZipEntry>` of all entries.
  - `Read(string name)` / `ReadByName(string name)` — decompress on demand, CRC verify.
- `ZipArchive` — one-shot convenience API (`Zip` / `Unzip`).
- `ZipEntry` record — `(string Name, byte[] Data)`.
- Internal raw RFC 1951 DEFLATE encoder (fixed Huffman, BTYPE=01) backed by LZSS.
- Internal raw RFC 1951 DEFLATE decoder (stored blocks + fixed Huffman blocks).
- CRC-32 (polynomial 0xEDB88320, table-driven).
- 12 xUnit test cases covering stored round-trip, DEFLATE round-trip, multiple files,
  directory entries, CRC mismatch detection, random-access reads, incompressible data,
  empty files, large files, Unicode filenames, nested paths, and empty archives.
