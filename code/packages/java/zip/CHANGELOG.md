# Changelog — java/zip

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased] — 2026-08-03

### Fixed

- Rescued from an orphaned branch (`worktree-feat+zstd-and-catchups`) that was
  never opened as a PR and had drifted ~3 months behind `main`. Re-verified
  against the current toolchain (Java 21 / Gradle 8.14.4) and current
  `java/lzss` API surface:
  - `java/lzss`'s public API had been renamed since this branch was cut
    (`Lzss` → `LZSS`, and the standalone `LzssToken` type folded into
    `LZSS.Token` with nested `LZSS.Literal` / `LZSS.Match` records). Updated
    the DEFLATE compressor's LZSS call site in `Zip.java` to the current API;
    method signature and arity were unchanged so this was a pure rename.
- Wired up `jacoco` coverage reporting and an 80% line-coverage gate
  (`jacocoTestCoverageVerification`), matching the convention used by other
  Java packages in this repo (e.g. `mini-sqlite`). Measured coverage: 349/420
  lines (~83%). Updated `BUILD` / `BUILD_windows` to run
  `gradle test jacocoTestReport jacocoTestCoverageVerification`.
- Confirmed `layout.buildDirectory = file("gradle-build")` override (avoids
  the case-insensitive-filesystem collision between Gradle's default `build/`
  and the sibling `BUILD` script) and the absence of a pinned
  `java { toolchain { languageVersion } }` (so CI's `actions/setup-java`
  controls the JDK) were both already correct in the ported code.
- All 12 JUnit 5 tests (TC-01 through TC-12) pass unchanged against the
  current toolchain.

### Security

- Fixed a MEDIUM finding from the pre-push security review: `compressed_size`,
  `uncompressed_size`, and `local_offset` fields read from the (attacker-
  controlled) Central Directory were narrowed from the wire's unsigned
  32-bit representation to a Java `int` *before* being range-checked. A
  crafted value of `0xFFFFFFFF` would narrow to `-1`, slip past an int-only
  `> data.length` bounds check (a negative number is never greater than a
  positive length), and surface as an undocumented
  `NegativeArraySizeException` / `ArrayIndexOutOfBoundsException` instead of
  the `IOException` the public API promises — a denial-of-service /
  API-contract bug for callers who (reasonably) only catch `IOException`
  around untrusted ZIP input. Fixed by doing all offset/size bounds
  arithmetic in `long` before narrowing, rejecting out-of-`int`-range values
  explicitly, and adding a `offset < 0` guard to the low-level `readU16`/
  `readU32` helpers. Added `malformedCompressedSizeRejectedCleanly` and
  `malformedLocalOffsetRejectedCleanly` regression tests (14 tests total).
  Not memory-unsafe in either direction — the JVM bounds-checks all array
  accesses — but the fix restores the documented `IOException` contract.

## [0.1.0] — 2026-04-24

### Added

- Initial implementation of the ZIP archive format (CMP09) in Java.
- `Zip.ZipWriter`: sequential in-memory archive builder.
  - `addFile(name, data, compress)` — auto-selects DEFLATE or Stored.
  - `addDirectory(name)` — directory entry with Unix mode 0o040755.
  - `finish()` — emits Local Headers, Central Directory, and EOCD.
- `Zip.ZipReader`: EOCD-first random-access archive reader.
  - `entries()` — parsed entry list (names only, lazy data).
  - `read(name)` — decompress + CRC-32 verify on demand.
- RFC 1951 DEFLATE compressor: single fixed-Huffman block via LZSS.
- RFC 1951 DEFLATE decompressor: stored and fixed-Huffman blocks.
- `Zip.zip(List<ZipEntry>)` and `Zip.unzip(byte[])` convenience API.
- CRC-32 with table-driven reflected polynomial 0xEDB88320.
- UTF-8 filename support (GP flag bit 11).
- Fall-back to Stored when DEFLATE expands the data (incompressible inputs).
- 256 MB decompression bomb guard in the DEFLATE decompressor.
- 12 JUnit 5 tests (TC-01 through TC-12) mirroring the C# reference suite.
- Depends on `com.codingadventures:lzss` via Gradle composite build.
