# Changelog

## 0.1.1 - 2026-11-10

### Security

- SEC-003: `decompress` now rejects a declared original length larger than
  8 x payload bytes before decoding, so a hostile header can no longer ask for
  more symbols than the payload could possibly hold. The existing validation
  (at most 256 symbols, no duplicate symbols, code lengths 1..16, sorted table,
  over-subscription check in unbounded `Integer`, fail-fast on an unmatched
  prefix) is unchanged and now covered by explicit hostile-input tests. Valid
  encoder output still round-trips.

## Unreleased

- `BUILD_windows` now runs `cabal test all` instead of skipping with "haskell support
  not enabled in windows CI yet", so this package is built and tested on Windows.
- The Darwin BUILD now runs Cabal with the Command Line Tools developer dir
  (`DEVELOPER_DIR=/Library/Developer/CommandLineTools`), so GHC 9.4.8's CLT SDK
  sysroot gets the matching linker on macOS CI with Command Line Tools 27.

## 0.1.0 - 2026-07-12

- Add CMP04 compression and decompression for strict `ByteString` values.
- Reuse the deterministic Haskell `huffman-tree` implementation.
- Add canonical code reconstruction and LSB-first bit packing.
- Validate malformed headers, length tables, prefixes, and truncated streams.
- Add round-trip, wire-format, effectiveness, determinism, and error tests.
