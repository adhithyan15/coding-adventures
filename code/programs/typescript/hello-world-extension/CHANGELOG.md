# Changelog

## 0.1.1 — 2026-03-29

### Added
- Multi-browser build script (`npm run build:release`) producing `dist/chrome/`, `dist/firefox/`, `dist/safari/`
- GitHub Actions release workflow — push a tag `hello-world-extension-v*` to get downloadable zip files for each browser

## 0.1.0 — 2026-03-29

### Added
- Popup UI with "Hello World" greeting and extension runtime info
- Background service worker that logs on extension install
- Cross-browser support via browser-extension-toolkit shim
- Dark mode support (follows system preference)
- Vite build configuration for extension bundling
- Unit tests for popup initialization logic
