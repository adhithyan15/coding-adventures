# Changelog

## Unreleased

- Added `build-wasm.ps1` so Windows/PowerShell workspaces can build
  `pkg/engram_engine.wasm` with the same output layout as `build-wasm.sh`.

## 0.1.0

- Added the Engram linear-memory WASM ABI over `engram-core-wasm`.
- Added a dependency-free JavaScript loader that installs a Mosaic host adapter
  for generated React and Electron shells.
- Preserved generated app `hostIntent` responses and exposed an optional
  `onHostIntent` callback for browser open/edit, Anki import/export, and note
  workflow actions.
