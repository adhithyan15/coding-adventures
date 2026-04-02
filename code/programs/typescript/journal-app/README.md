# Journal App

A local-first markdown journal with live preview and pluggable storage backends. Write entries in GitHub Flavored Markdown, see a rendered preview in real time, and store everything locally via IndexedDB.

## Features

- **Markdown editing** with split-pane live preview
- **GFM rendering** via `@coding-adventures/gfm` pipeline
- **Date-organized timeline** with entries grouped by creation date
- **Pluggable storage** via the `Storage` interface — swap backends without changing app code
- **Dual platform** — runs as a web app and an Electron desktop app
- **Dark theme** using shared `@coding-adventures/ui-components`

## Architecture

```
┌────────────────┐    ┌─────────────────────┐    ┌──────────────────┐
│  React UI      │───▶│  Flux Store         │───▶│  Storage         │
│  (components)  │    │  (reducer + actions) │    │  (interface)     │
└────────────────┘    └─────────────────────┘    └────────┬─────────┘
                                                          │
                                              ┌───────────┼───────────┐
                                              ▼           ▼           ▼
                                         IndexedDB   MemoryStorage  Future:
                                         (default)   (tests)        GDrive, SQLite
```

## Quick Start

```bash
# Install dependencies
npm install

# Start dev server
npm run dev

# Run tests
npm test

# Build for production
npm run build
```

## Electron

```bash
# Development (start Vite dev server first)
npm run dev
npm run electron:dev

# Package for current platform
npm run electron:build
```

## Dependencies

This app uses several packages built from scratch in this repo:

- `@coding-adventures/gfm` — GFM markdown parser and HTML renderer
- `@coding-adventures/document-ast-sanitizer` — AST-level content sanitization
- `@coding-adventures/storage` — Unified storage interface
- `@coding-adventures/indexeddb` — IndexedDB backend for the storage interface
- `@coding-adventures/store` — Flux-like state management with React hook
- `@coding-adventures/ui-components` — Shared theme, i18n, and UI primitives
