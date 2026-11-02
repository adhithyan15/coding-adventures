# Changelog

## Unreleased

- `BUILD` runs Cabal with `DEVELOPER_DIR=/Library/Developer/CommandLineTools` on Darwin so GHC 9.4.8 links with the Command Line Tools linker that accepts the CLT SDK's `arm64e.x1` stubs; other platforms are unchanged.
- Added schema-v1 capability manifests for generated Haskell libraries and
  programs, including the program template's standard-output declaration.
- Tightened package names to the repository's ASCII kebab-case contract and
  rejected control characters and Unicode line separators before descriptions
  reach Cabal metadata.
- Added golden tests for both capability profiles.

## 0.1.0

- Added the first Haskell scaffold-generator implementation for Haskell packages and programs.
