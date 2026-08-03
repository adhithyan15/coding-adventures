# Changelog — zip (Haskell, CMP09)

## [Unreleased] — 2026-08-03

### Fixed

- Rescued this package from a stale, never-PR'd branch
  (`worktree-feat+zstd-and-catchups`) and verified it against current `main`
  and toolchain (GHC 9.14.1 / cabal-install 3.16.1.0 via `mise`).
- Fixed a build break against the sibling `lzss` package's actual API: the
  module is `LZSS` (not `Lzss`), and it exposes `encodeWith :: Int -> Int ->
  Int -> ByteString -> Either String [Token]` rather than an arity-4
  `encode` that returns `[Token]` directly. `deflateCompress` now calls
  `encodeWith` and unwraps the `Right` case (the fixed window/match
  parameters always satisfy `LZSS`'s own bounds, so `Left` is unreachable).
- Fixed a type mismatch in `encodeToken`: `LZSS.Token`'s `Match` constructor
  carries `matchOffset :: Word16` / `matchLength :: Word8`, but the
  length/distance table lookups are `Int`-indexed. Added explicit
  `fromIntegral` conversions.
- Tightened `zip.cabal` dependency version bounds (`base`, `bytestring`,
  `array`, `lzss`) to match the upper-bound convention used by sibling
  Haskell packages (e.g. `lzss`, `deflate`).
- Confirmed all 12 Hspec test cases genuinely execute and pass (not silently
  skipped) — `ZipEntry(..)` is already exported with its constructor in the
  module's export list, avoiding the Windows-CI silent-skip pitfall.

### Security

- Fixed two HIGH-severity algorithmic-complexity DoS findings from security
  review in `deflateDecompress`: the output accumulator was a plain
  `[Word8]`, so the `maxOut` (256 MB) decompression-bomb guard's `length acc`
  check was O(n) per byte (O(n²) overall), and `copyBackRef`'s list index
  (`!!`) made each back-reference byte O(distance) (up to O(32768) at the
  RFC 1951 maximum). Together, a small crafted archive with long-distance
  back-references could pin a CPU core indefinitely, well before the byte
  cap would ever trigger — the cap existed but the code that was supposed to
  enforce it was itself the bottleneck. Switched the accumulator to
  `Data.Sequence.Seq` (O(1) length, O(log n) indexed access), the same
  technique the sibling `lzss` package already uses in `LZSS.decodeToken`
  for its own overlap-safe decode. Added `containers` to `zip.cabal`'s
  dependencies for `Data.Sequence`.
- Documented the zip-slip / path-traversal hazard in `README.md`: `entryName`
  is returned exactly as read from the archive with no validation. This
  module never writes to disk itself, so there's no traversal bug in this
  code today, but any caller that later writes `entryName` to a filesystem
  path must sanitise it first (reject `..` segments and absolute paths).
- Fixed a MEDIUM/HIGH aggregate decompression-bomb finding from a second
  security-review round: the per-entry 256 MB `deflateDecompress` cap was
  the only limit — a Central Directory listing many entries (optionally all
  pointing at the same or overlapping Local File Header data) could force
  `readZip` to decompress `256 MB × (entry count)` before returning, and
  `readEntry` wasn't actually random-access (it called `readZip` and
  filtered afterward, so looking up one entry paid the cost of decompressing
  every entry). Refactored the reader into a metadata-only Central Directory
  scan (`locateEntries`/`CdEntryMeta`/`parseCentralDirectoryMeta`, no
  decompression) shared by both `readZip` and `readEntry`: `readZip` now
  threads a running total through every entry it materialises and errors
  out once the aggregate decompressed size across the call would exceed
  256 MB, and `readEntry` decompresses only the matching entry. Also capped
  Central Directory entry count at 65535 (its wire field's own maximum),
  per the spec's `Num_Entries_Total` guidance. Added a regression test
  (`readEntry ignores corruption in a different entry`) that proves
  `readEntry` no longer touches unrelated entries.
- Fixed a HIGH-severity finding from a third security-review round: the
  aggregate budget above counted the *post-truncation* `entryData` size
  (`BS.take uncompSize d`), not the actual decode work performed. Because
  `deflateDecompress` always ran to its own flat 256 MB ceiling regardless
  of what the caller expected, an attacker could declare a tiny
  `uncompSize` (even `0`, which also skipped CRC verification under the
  previous check) on a Central Directory entry whose compressed data
  actually decoded to something large — paying up to 256 MB of real decode
  work per entry while contributing 0 counted bytes to `readZip`'s
  aggregate total, silently reopening the "many entries, each an
  expensive bomb" attack from the previous fix. Split `deflateDecompress`
  into `deflateDecompressCapped :: Int -> ByteString -> Either String
  ByteString` (stops decoding the instant output would exceed the given
  cap, still clamped to the absolute 256 MB ceiling) and
  `deflateDecompress = deflateDecompressCapped (256 * 1024 * 1024)`.
  `readLocalData` now calls `deflateDecompressCapped uncompSize`, so decode
  work is bounded by the entry's own declared size — the cost actually
  paid now matches the cost `readZip`'s aggregate budget counts. Also
  removed the `uncompSize > 0` guard on the CRC check: `crc32` of an empty
  `ByteString` is always exactly `0` and `writeZip` always writes `0` for
  empty data, so the guard was already redundant for well-formed archives
  and only made the exploit easier to construct (no need to forge a
  matching CRC). Exported `deflateDecompressCapped` (matching the existing
  "low-level, exported for tests" convention) and added a direct
  regression test proving a declared cap of `0` fails fast on a real
  200 KB payload instead of fully decoding it first.

## [0.1.0] — 2026-04-24

### Added

- `Zip` module with full ZIP archive read/write support (CMP09).
- `writeZip` — build an archive from `(name, data, compress)` triples.
- `readZip` — parse all entries using the EOCD-first strategy.
- `readEntry` — random-access read of a single file by name.
- `zip'` / `unzip'` — convenience wrappers (primed to avoid Prelude clash).
- RFC 1951 DEFLATE (fixed Huffman, BTYPE=01) compressor and decompressor,
  using `lzss` for LZ77 match-finding (window=32768, max=255, min=3).
- Table-driven CRC-32 (polynomial 0xEDB88320) with incremental update support.
- Auto-fallback: DEFLATE is used only when it strictly reduces size; otherwise
  method=0 (Stored) is chosen — handles already-compressed data transparently.
- Directory entry support (names ending with `/`).
- UTF-8 filename support (GP flag bit 11).
- MS-DOS epoch timestamp (1980-01-01 00:00:00) used for all entries.
- 256 MB decompression cap to guard against decompression-bomb attacks.
- 12-case Hspec test suite covering all major behaviours.
