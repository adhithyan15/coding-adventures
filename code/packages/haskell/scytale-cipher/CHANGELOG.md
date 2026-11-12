# Changelog

## Unreleased - 2026-08-26

- `BUILD_windows` now runs `cabal test all` instead of skipping with "haskell support
  not enabled in windows CI yet", so this package is built and tested on Windows.
- The Darwin BUILD now runs Cabal with the Command Line Tools developer dir
  (`DEVELOPER_DIR=/Library/Developer/CommandLineTools`), so GHC 9.4.8's CLT SDK
  sysroot gets the matching linker on macOS CI with Command Line Tools 27.
- Add a generated native consumer for all 18 language-neutral Scytale fixture cases.
- Pin CR02 Unicode-scalar, combining-sequence, ragged-grid, and literal-padding behavior.
- Return checked brute-force results and reject inputs above 4096 scalars before allocation.
- Replace repeated list indexing with linear grid transposition and add a pure capability profile.

## 0.1.0 - 2026-07-18

- Add padded Scytale encryption and decryption with explicit key validation.
- Add brute-force candidate generation across the shared CR02 key range.
- Add tests for reference vectors, uneven candidates, round trips, character
  preservation, validation, and brute force.
