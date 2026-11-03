## [Unreleased]

### Fixed

- Haskell scaffolds now emit a `BUILD` that selects the Command Line Tools
  linker on macOS: the two-line explanation, then
  `if [ "$(uname -s)" = Darwin ]; then export DEVELOPER_DIR=/Library/Developer/CommandLineTools; fi; cabal test`.
  GHC 9.4.8's macOS bindist hardcodes a CLT SDK sysroot that an older default
  Xcode linker cannot read (`arm64e.x1`), and `build-tool --validate-build-files`
  rejects a Haskell BUILD without the selection. `BUILD_windows` keeps its
  existing skip record and never carries the POSIX prefix; the test now pins
  both files exactly (spec section 5.8).
- The changelog is now sharded under `CHANGELOG.d/` (one file per entry), so
  concurrent branches no longer collide on a shared `CHANGELOG.md`.
