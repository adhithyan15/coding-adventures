# Changelog

## Unreleased

- `BUILD` runs Cabal with `DEVELOPER_DIR=/Library/Developer/CommandLineTools` on Darwin so GHC 9.4.8 links with the Command Line Tools linker that accepts the CLT SDK's `arm64e.x1` stubs; other platforms are unchanged.

## 0.1.0

- Added the initial note-to-frequency package.
- Added note parsing for natural notes plus single sharps and flats.
- Added equal-tempered frequency mapping anchored at `A4 = 440 Hz`.
- Added tests that lock down the shared parity vectors across language ports.
