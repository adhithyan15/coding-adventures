# Changelog

## Unreleased

- Added APKG byte-buffer exports to the WASM ABI and JS loader. Browser WASM
  builds return an explicit native-host delegation error, while Electron uses a
  native sidecar for real package import/export.
- Added `build-wasm.ps1` so Windows/PowerShell workspaces can build
  `pkg/engram_engine.wasm` with the same output layout as `build-wasm.sh`.
- `installEngramMosaicHost` now dispatches `mosaic-host-ready` after installing
  `window.mosaicHost`, matching the generated React/Electron refresh hook.

## 0.1.0

- Added the Engram linear-memory WASM ABI over `engram-core-wasm`.
- Added a dependency-free JavaScript loader that installs a Mosaic host adapter
  for generated React and Electron shells.
- Preserved generated app `hostIntent` responses and exposed an optional
  `onHostIntent` callback for browser open/edit, Anki import/export, and note
  workflow actions.
