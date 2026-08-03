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
  - Ran a dedicated security review (zip-slip/path-traversal, integer overflow on
    untrusted size/offset fields, unchecked indexing) — see review notes below.
  - Added a `csharp` row to `code/specs/CMP09-zip.md`'s Package Naming table (previously
    missing).

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
