# Changelog

## [Unreleased]

### Fixed

- Haskell scaffolds now emit a `BUILD` that selects the Command Line Tools
  linker on macOS: the two-line explanation, then
  `if [ "$(uname -s)" = Darwin ]; then export DEVELOPER_DIR=/Library/Developer/CommandLineTools; fi; cabal test all`.
  GHC 9.4.8's macOS bindist hardcodes a CLT SDK sysroot that an older default
  Xcode linker cannot read (`arm64e.x1`), and `build-tool --validate-build-files`
  rejects a Haskell BUILD without the selection.
- Haskell scaffolds now also emit a `BUILD_windows` holding the plain
  `cabal test all`, because `cmd /C` cannot parse the POSIX prefix. Windows
  previously ran the plain `BUILD`, so its behaviour is unchanged. Tests pin
  both files exactly (spec section 5.8).

## [1.0.1] - 2026-04-18

### Fixed

- recognize git worktrees where `.git` is a file instead of a directory

## [1.0.0] - 2026-03-21

### Added

- Initial Python implementation of scaffold-generator
- Full parity with Go implementation
