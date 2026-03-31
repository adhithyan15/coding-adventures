# Changelog

## 0.1.0 — 2026-03-29

### Added
- `getBrowserAPI()` — Cross-browser API shim that normalizes `chrome.*` / `browser.*` namespaces
- `transformManifest()` — Transforms a base Manifest V3 for Chrome, Firefox, or Safari
- `buildManifests()` — Batch transforms a manifest for multiple target browsers
- `webExtensionPlugin()` — Vite plugin for multi-browser extension builds
- `scaffold()` — Generates a new extension project with all boilerplate (manifest, vite config, popup, service worker, tests, BUILD, README, CHANGELOG)
- `expandTemplate()` — Template engine for `{{variable}}` placeholder replacement
- `generateFiles()` — Generates file list without writing to disk (testable, composable)
- CLI tool (`extension-scaffold`) for creating extensions from the command line
