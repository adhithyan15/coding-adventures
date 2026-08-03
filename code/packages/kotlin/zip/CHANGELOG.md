# Changelog — kotlin/zip

## [Unreleased] — 2026-08-03

### Fixed

- Rescued this package from a stale, never-merged branch
  (`worktree-feat+zstd-and-catchups`, last touched 2026-04-24) and brought it
  up to date with current `main`. The branch predated an `lzss` API rename:
  `Lzss` → `LZSS` and the nested `LzssToken.Literal` / `LzssToken.Match` were
  flattened to top-level `Literal` / `Match` implementing a `Token` sealed
  interface. Updated `Zip.kt`'s imports and token-matching `when` branches
  (aliased to `LzssLiteral` / `LzssMatch` to avoid any future naming
  collisions) to compile against the current `kotlin/lzss` package.
- Verified against the current toolchain (Gradle 8.14.4, Kotlin 2.1.20, JDK
  21): `gradle test` passes all 23 tests (TC-01 through TC-12 plus CRC-32 and
  DEFLATE unit tests). Ad hoc JaCoCo measurement shows 367/409 lines covered
  (~89.7%), well above the repo's 80% bar.
- Confirmed `build.gradle.kts` already set `layout.buildDirectory =
  file("gradle-build")` before the `plugins` block, avoiding the known
  `build/` vs `BUILD` case-insensitive filesystem collision.

### Security

- Hardened `ZipReader` against a crafted/malicious Central Directory (which is
  unauthenticated — no signature covers it). Every offset/size field parsed
  from the CD or EOCD (`local_offset`, `compressed_size`, `uncompressed_size`,
  `cd_offset`, `cd_size`) is a logically-unsigned 32-bit wire value that
  `readU32` reinterprets as a signed Kotlin `Int` with no prior validation.
  Without checks, a crafted archive could make these values negative or
  cause 32-bit `Int` arithmetic to overflow past a `data.size` bounds check,
  reaching `ByteArray` indexing/allocation and throwing an uncaught
  `ArrayIndexOutOfBoundsException` / `NegativeArraySizeException` /
  `IllegalArgumentException` instead of the documented `IOException` (JVM
  array bounds checking means this was never memory-unsafe, but it broke the
  API's error-handling contract and is a DoS/availability risk for callers
  that only catch `IOException`, per the KDoc). Fixed by:
  - Rejecting negative `cd_offset`/`cd_size` and computing the CD bounds
    check in `Long` before comparing against `data.size`.
  - Requiring the full 46-byte fixed Central Directory record to be present
    (not just its 4-byte signature) before reading any other field.
  - Rejecting negative `local_offset`/`compressed_size`/`uncompressed_size`
    in `readEntry()`, and validating the Local Header offset and the
    computed data range in `Long` before slicing `data`.
  - Added TC-SEC01..05 regression tests covering each of the above.

## [0.1.0] — 2026-04-24

### Added

- Initial implementation of the ZIP archive format (CMP09) in Kotlin.
- `ZipWriter` — incremental in-memory writer supporting DEFLATE (method 8) and
  Stored (method 0) entries; file and directory entries.
- `ZipReader` — EOCD-first parser with random-access decompression and CRC-32
  verification.
- `ZipArchive` — convenience `zip()` and `unzip()` one-shot functions.
- `ZipEntry` — data class carrying `name` and `data` fields.
- `crc32()` — table-driven CRC-32 (polynomial 0xEDB88320) with incremental support.
- `dosDt()` — MS-DOS packed datetime encoder.
- Internal `deflateCompress()` / `deflateDecompress()` — raw RFC 1951 DEFLATE
  (fixed Huffman BTYPE=01) using the `lzss` package for LZ77 match-finding.
- Full test suite: TC-01 through TC-12 matching the Rust and C# reference
  implementations, plus CRC-32 and DEFLATE unit tests.
- Security limits: 256 MB decompression bomb guard, LEN/NLEN validation,
  CRC-32 mismatch detection.
