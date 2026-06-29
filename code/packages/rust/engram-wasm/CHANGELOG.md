# Changelog

## 0.1.0

- Added the Engram linear-memory WASM ABI over `engram-core-wasm`.
- Added a dependency-free JavaScript loader that installs a Mosaic host adapter
  for generated React and Electron shells.
- Preserved generated app `hostIntent` responses and exposed an optional
  `onHostIntent` callback for browser open/edit, Anki import/export, and note
  workflow actions.
