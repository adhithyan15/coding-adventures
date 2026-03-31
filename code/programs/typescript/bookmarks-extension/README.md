# Bookmarks Extension

A browser extension for saving annotated bookmarks. Click the toolbar icon, see the current tab's URL, write a note about it, and save. Works in Chrome, Firefox, and Safari.

## Features

- Save the current tab with a title and note
- Browse, search, edit, and delete saved bookmarks
- Local storage via IndexedDB (no account required)
- Pluggable storage backend for future cloud sync

## Development

```bash
npm install
npm run dev          # Watch mode — rebuilds on changes
npm run build        # One-time build to dist/
npm run build:release  # Build for Chrome, Firefox, and Safari
npm test             # Run tests
npm run test:coverage  # Run tests with coverage
```

## Loading the extension

### Chrome

1. Open `chrome://extensions`
2. Enable **Developer mode**
3. Click **Load unpacked** and select the `dist/` directory

### Firefox

1. Open `about:debugging#/runtime/this-firefox`
2. Click **Load Temporary Add-on...**
3. Select `dist/manifest.json`

### Safari

1. Run `xcrun safari-web-extension-converter dist/`
2. Open the generated Xcode project
3. Build and run (Cmd+R)
4. Enable in Safari Preferences → Extensions

## Architecture

The extension uses a **Strategy pattern** for storage:

```
BookmarkStorage (interface)
├── IndexedDBStorage (default — browser-local)
├── GoogleDriveStorage (future)
└── OneDriveStorage (future)
```

Consumer code calls `createStorage()` which returns the active backend.
Swapping backends is a one-line change in `src/storage/index.ts`.

## Project Structure

```
src/
├── lib/browser-api.ts           # Cross-browser API shim
├── storage/
│   ├── bookmark-storage.ts      # Interface + types
│   ├── indexeddb-storage.ts     # IndexedDB implementation
│   └── index.ts                 # Factory
├── popup/
│   ├── popup.html               # Extension popup UI
│   ├── popup.ts                 # Popup logic
│   └── popup.css                # Styles
└── background/
    └── service-worker.ts        # Background script
```

## Part of coding-adventures

This extension is built with the [browser-extension-toolkit](../../../packages/typescript/browser-extension-toolkit/) and follows the patterns established by the [hello-world-extension](../hello-world-extension/).
