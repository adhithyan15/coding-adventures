# Changelog

## Unreleased

- The Darwin BUILD now runs Cabal with the Command Line Tools developer dir
  (`DEVELOPER_DIR=/Library/Developer/CommandLineTools`), so GHC 9.4.8's CLT SDK
  sysroot gets the matching linker on macOS CI with Command Line Tools 27.
- Added schema-v1 capability manifests for generated Haskell libraries and
  programs, including the program template's standard-output declaration.
- Tightened package names to the repository's ASCII kebab-case contract and
  rejected control characters and Unicode line separators before descriptions
  reach Cabal metadata.
- Added golden tests for both capability profiles.

## 0.1.0

- Added the first Haskell scaffold-generator implementation for Haskell packages and programs.
