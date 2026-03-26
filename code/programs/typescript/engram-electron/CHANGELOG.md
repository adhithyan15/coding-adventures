# Changelog — engram-electron

## 0.2.0 — 2026-03-25

Initial release of the standalone Electron wrapper.

### Added

- **`electron/main.ts`** — Electron main process. Dev mode loads Vite dev server
  (`VITE_DEV_SERVER_URL`). Prod mode loads `renderer/index.html` (the engram-app
  Vite build, copied by the BUILD step).

- **`electron-builder.yml`** — packages `renderer/` + `dist-electron/` into
  platform installers: macOS (dmg + zip), Windows (nsis + portable), Linux (AppImage).

- **`BUILD`** — installs all transitive deps, builds engram-app, copies `dist/` to
  `renderer/`, compiles the main process.

### Extracted from `engram-app` 0.1.0

The Electron configuration and `electron/` directory previously lived in
`engram-app`. Splitting them out lets `engram-app` be deployed as a plain web
app (GitHub Pages) while this package handles native desktop distribution.
