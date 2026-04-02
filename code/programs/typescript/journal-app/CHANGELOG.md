# Changelog

All notable changes to the Journal App will be documented in this file.

## [0.1.0] - 2026-04-02

### Added

- Initial release of the Journal app
- Entry CRUD (create, read, update, delete)
- Split-pane markdown editor with live GFM preview (150ms debounce)
- Timeline view with entries grouped by date (reverse chronological)
- Read-only entry view with rendered markdown
- Pluggable storage via `@coding-adventures/storage` interface
- Default IndexedDB backend with MemoryStorage fallback
- Welcome seed entry on first visit demonstrating GFM syntax
- Flux state management via `@coding-adventures/store`
- Fire-and-forget persistence middleware
- Electron desktop app wrapper with security defaults
- Dark theme via `@coding-adventures/ui-components`
- i18n support with English locale
- Hash-based routing (works with file:// and GitHub Pages)
- Unit tests for reducer, persistence middleware, and preview pipeline
- Component tests for Timeline, EntryCard, EntryEditor, and EntryView
- GitHub Actions release workflow (`journal-v*` tags)
- GitHub Actions deploy workflow for GitHub Pages
- electron-builder config for macOS (DMG), Windows (NSIS), Linux (AppImage)
