# Changelog

## Unreleased

- `BUILD_windows` now runs `cabal test` instead of skipping with "haskell support
  not enabled in windows CI yet", so this package is built and tested on Windows.
- `BUILD` selects the Command Line Tools linker on Darwin (`DEVELOPER_DIR=/Library/Developer/CommandLineTools`), scoped to Cabal. GHC 9.4.8 links against the CLT 27 SDK stubs, whose `arm64e.x1` entries the default Xcode linker rejects, so macOS CI failed while building dependencies, before any test ran. Same fix as trig/point2d/bezier2d (lesson: ghc-must-select-the-linker-matching-its-hardcoded-macos-sdk).
- Added graph, node, and edge property bags with synchronized `weight` edge metadata.

## 0.1.0

- Added the first Haskell graph package.
