# Changelog

## Unreleased

- SEC-010: `decrypt` negated the raw shift. GHC `Int` is fixed-width, so `negate minBound == minBound` and `decrypt text minBound` shifted the wrong way, returning wrong plaintext. `decrypt` now reduces the shift mod 26 before negating. `encrypt` was already safe (`mod` reduces first). Tests: `decrypt` at `minBound` plus round trips for {minBound, minBound+1, -1, 0, 25, 26, maxBound}.
- `BUILD_windows` now runs `cabal test all` instead of skipping with "haskell support
  not enabled in windows CI yet", so this package is built and tested on Windows.
- The Darwin BUILD now runs Cabal with the Command Line Tools developer dir
  (`DEVELOPER_DIR=/Library/Developer/CommandLineTools`), so GHC 9.4.8's CLT SDK
  sysroot gets the matching linker on macOS CI with Command Line Tools 27.

## 0.1.0 - 2026-07-12

- Add case-preserving Caesar encryption, decryption, and ROT13.
- Add brute-force enumeration of all non-trivial shifts.
- Add chi-squared English frequency analysis.
- Add tests for normalization, Unicode pass-through, round trips, attacks,
  and the frequency table.
