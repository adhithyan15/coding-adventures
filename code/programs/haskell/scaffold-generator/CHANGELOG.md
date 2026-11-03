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
- Generated packages' `BUILD` now selects the Command Line Tools linker on
  macOS: the two-line explanation, then
  `if [ "$(uname -s)" = Darwin ]; then export DEVELOPER_DIR=/Library/Developer/CommandLineTools; fi; `
  in front of the unchanged `command -v cabal` guard. GHC 9.4.8's macOS bindist
  hardcodes a CLT SDK sysroot that an older default Xcode linker cannot read
  (`arm64e.x1`), and `build-tool --validate-build-files` rejects a Haskell
  BUILD without the selection. The generated `BUILD_windows` keeps its skip
  record and never carries the POSIX prefix. Both are exported
  (`buildFileContents`, `buildWindowsFileContents`) and pinned by tests
  (spec section 5.8).

## 0.1.0

- Added the first Haskell scaffold-generator implementation for Haskell packages and programs.
