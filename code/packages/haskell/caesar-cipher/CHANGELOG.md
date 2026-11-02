# Changelog

## Unreleased

- `BUILD` runs Cabal with `DEVELOPER_DIR=/Library/Developer/CommandLineTools` on Darwin so GHC 9.4.8 links with the Command Line Tools linker that accepts the CLT SDK's `arm64e.x1` stubs; other platforms are unchanged.

## 0.1.0 - 2026-07-12

- Add case-preserving Caesar encryption, decryption, and ROT13.
- Add brute-force enumeration of all non-trivial shifts.
- Add chi-squared English frequency analysis.
- Add tests for normalization, Unicode pass-through, round trips, attacks,
  and the frequency table.
