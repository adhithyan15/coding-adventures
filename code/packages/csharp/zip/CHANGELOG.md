# Changelog — CodingAdventures.Zip (CSharp)

## [Unreleased] — 2026-08-03

### Fixed

- Rescued this package from a stale, never-PR'd branch (`worktree-feat+zstd-and-catchups`,
  ~3+ months old) and verified/repaired it against current `main` and the current .NET
  toolchain (.NET 9.0.313):
  - `tests/CodingAdventures.Zip.Tests/CodingAdventures.Zip.Tests.csproj` was missing the
    `coverlet.collector` / `coverlet.msbuild` package references that the rest of the
    `csharp/*` packages now require for `/p:CollectCoverage=true` to actually produce a
    coverage report — without them `dotnet test` silently reported no coverage data
    instead of gating on the threshold. Added both at `6.0.2` to match sibling packages.
  - `BUILD` / `BUILD_windows` used a now-nonstandard `/p:Exclude=[CodingAdventures.Lzss]*`
    coverage filter left over from before the repo settled on the `/p:Include=[<own
    assembly>]*` convention used by every other `csharp/*` package with a project
    dependency (e.g. `chacha20-poly1305`, `hash-map`). Switched to
    `/p:Include=[CodingAdventures.Zip]*`.
  - No source changes were needed in `Zip.cs` — the LZSS API it calls
    (`CodingAdventures.Lzss.Lzss.Encode(data, windowSize:, maxMatch:, minMatch:)` and the
    `LzssLiteral`/`LzssMatch` token records) is unchanged on current `main`.
  - Re-ran all 12 xUnit test cases: all pass. Line coverage 88.18% / method coverage
    95.12%, both above the 80% gate.
  - Added a `csharp` row to `code/specs/CMP09-zip.md`'s Package Naming table (previously
    missing).

### Security

A dedicated security review of the diff (zip-slip/path-traversal, integer overflow on
untrusted size/offset fields, unchecked indexing, decompression bombs) found four real
gaps, three of which mirror bug classes this repo already hardened against in sibling
packages (`ruby/zip`'s parser, `rust/opc-writer`'s part-name normalization). All four are
fixed here, with adversarial regression tests added for each:

- **Zip-slip (write-side defense-in-depth)**: `ZipWriter.AddFile`/`AddDirectory` now
  normalize entry names before writing them — backslashes become forward slashes, a
  leading Windows drive letter (`C:`) is dropped, and empty, `.`, `..` segments are
  dropped — mirroring `rust/opc-writer`'s `normalize_part_name`. (The initial version of
  this fix missed the drive-letter case — caught in a second review round: `"C:/evil.dll"`
  is still `Path.IsPathRooted`-true on Windows even without a leading slash, so
  `Path.Combine(outDir, ...)` would have silently discarded `outDir`.) This package
  performs no filesystem I/O itself, so this isn't directly exploitable by the package's
  own code, but it prevents a `..`-shaped, absolute, or drive-rooted name from ever
  reaching a downstream extractor that naively does
  `File.WriteAllBytes(Path.Combine(outDir, entry.Name), ...)`. A name that normalizes away
  to nothing (e.g. pure `..` segments) is rejected outright. Per
  `code/specs/CMP09-zip.md`'s Security Considerations, this is intentionally write-side
  only — `ZipReader`/`ZipArchive.Unzip` return third-party entry names verbatim, and
  callers writing extracted entries to disk remain responsible for sanitizing them, same
  as with any other ZIP reader.
- **Integer overflow on untrusted offset/size fields bypassing bounds checks**: `cd_offset`,
  `cd_size`, `local_offset`, `compressed_size`, and `uncompressed_size` are attacker-
  controlled `uint` values that were narrowed to `int` before their governing bounds check,
  so a raw value ≥ `0x80000000` could go negative and slip past `offset + n > data.Length`-
  style guards, reaching an unchecked negative-offset span access and throwing
  `ArgumentOutOfRangeException` instead of the documented `InvalidDataException`. Fixed by
  doing the governing arithmetic in `long` and range-checking before narrowing; `ReadU16`/
  `ReadU32` also now explicitly reject a negative offset.
- **ZIP32 field-width limits not enforced on write**: more than 65535 entries silently
  wrapped the EOCD's 16-bit `entries_total` field (the exact bug already fixed in
  `ruby/zip`'s parser, `raise if entries.length > 65535`, applied here symmetrically on the
  write side), and an entry name encoding to more than 65535 UTF-8 bytes silently truncated
  the 16-bit `name_len` field while still writing the full name bytes — desynchronizing the
  declared length from the archive layout. Both now throw instead of corrupting the archive.
- **Decompression bomb via many moderate-sized entries**: `DeflateDecompressor` already
  capped a single entry's decompressed output at 256 MiB, but `ZipArchive.Unzip` had no
  budget across an entire archive, so several entries each individually under that cap
  could still force gigabytes of aggregate allocation in one call. Added an aggregate
  budget (`maxTotalBytes`, default 512 MiB) enforced across all entries in one `Unzip()`
  call.

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
