# Bookmarks Extension

A browser extension for saving annotated bookmarks. Click the toolbar icon to open a side panel where you can add notes to the current page and save it. Works in Chrome and Firefox.

## Features

- Side panel UI that persists while you browse
- Save the current tab with a title and note
- Browse, search, edit, and delete saved bookmarks
- Local storage via IndexedDB (no account required)
- Pluggable storage backend for future cloud sync
- Cross-browser sidebar abstraction (Chrome sidePanel + Firefox sidebar_action)

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
4. Click the extension icon — the side panel opens

### Firefox

1. Open `about:debugging#/runtime/this-firefox`
2. Click **Load Temporary Add-on...**
3. Select `dist/manifest.json`
4. The sidebar appears in the browser's sidebar area

### Safari

1. Run `xcrun safari-web-extension-converter dist/`
2. Open the generated Xcode project
3. Build and run (Cmd+R)
4. Enable in Safari Preferences → Extensions

## Architecture

### Storage (Strategy pattern)

```
BookmarkStorage (interface)
├── IndexedDBStorage (default — browser-local)
├── GoogleDriveStorage (future)
└── OneDriveStorage (future)
```

Consumer code calls `createStorage()` which returns the active backend.
Swapping backends is a one-line change in `src/storage/index.ts`.

### Sidebar abstraction

The service worker detects which sidebar API is available and opens the right one:

| Browser | API | Manifest key |
|---------|-----|-------------|
| Chrome | `chrome.sidePanel.open()` | `side_panel` |
| Firefox | `browser.sidebarAction.open()` | `sidebar_action` |

Both use the same panel HTML/CSS/JS — only the manifest entries and open mechanism differ.

## Project Structure

```
src/
├── lib/browser-api.ts           # Cross-browser API shim
├── storage/
│   ├── bookmark-storage.ts      # Interface + types
│   ├── indexeddb-storage.ts     # IndexedDB implementation
│   └── index.ts                 # Factory
├── panel/
│   ├── panel.html               # Side panel UI
│   ├── panel.ts                 # Panel logic
│   └── panel.css                # Styles
└── background/
    └── service-worker.ts        # Opens side panel on icon click
```

## Part of coding-adventures

This extension is built with the [browser-extension-toolkit](../../../packages/typescript/browser-extension-toolkit/) and follows the patterns established by the [hello-world-extension](../hello-world-extension/).
