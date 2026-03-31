# Browser Extensions — Hello World

## Overview

A browser extension is a small program that runs inside a web browser, adding features or
modifying behavior. Every browser extension is, at its core, just HTML, CSS, and JavaScript
bundled with a manifest file that tells the browser what to load and when.

This spec covers the anatomy of a browser extension, the cross-browser landscape (Chrome,
Firefox, Safari), and the design of a toolkit that lets us write an extension once and
ship it to all three browsers.

## What We're Building

1. **browser-extension-toolkit** — A reusable TypeScript library package with:
   - A cross-browser API shim
   - A manifest transformer (one manifest → per-browser variants)
   - A Vite plugin for multi-browser builds
   - A scaffold generator for creating new extensions

2. **hello-world-extension** — A minimal popup extension that says "Hello World" when
   you click the toolbar icon. Built with the toolkit.

---

## Concepts

### The anatomy of a browser extension

Every extension has these parts:

```
┌─────────────────────────────────────────────────────┐
│                   manifest.json                      │
│  (declares name, version, permissions, entry points) │
└────────┬────────────────┬───────────────┬───────────┘
         │                │               │
    ┌────▼────┐    ┌──────▼──────┐  ┌─────▼──────┐
    │  Popup  │    │  Background │  │  Content   │
    │  (HTML) │    │   Script    │  │  Scripts   │
    └─────────┘    └─────────────┘  └────────────┘
```

**Manifest** — The `manifest.json` file is the extension's contract with the browser.
It declares the extension's name, version, permissions it needs, and which files to load.
Think of it as `package.json` for the browser.

**Popup** — The small window that appears when you click the extension's toolbar icon.
It's a regular HTML page with its own CSS and JavaScript. The popup is created when the
icon is clicked and destroyed when the user clicks away. This means it has no persistent
state — every click starts fresh.

**Background script (Service Worker)** — A long-lived script that runs in the background,
even when the popup is closed. In Manifest V3, this is a Service Worker, which means it
can be suspended by the browser when idle and woken up when needed. Use this for:
- Listening to browser events (tab changes, network requests)
- Maintaining state between popup opens/closes
- Running periodic tasks

**Content scripts** — JavaScript files injected into web pages. They can read and modify
the page's DOM but run in an isolated world (they can't access the page's JavaScript
variables, and the page can't access theirs). Use this for:
- Modifying how web pages look
- Reading page content
- Adding UI elements to existing pages

For our Hello World extension, we only need the **manifest** and a **popup**. The
background service worker will be minimal (just logs on install). No content scripts.

### Manifest V3 format

Manifest V3 (MV3) is the current extension API standard. Chrome requires it; Firefox
supports it alongside MV2. Here's the format:

```json
{
  "manifest_version": 3,
  "name": "My Extension",
  "version": "1.0.0",
  "description": "What this extension does",

  "action": {
    "default_popup": "popup.html",
    "default_icon": {
      "16": "icons/icon-16.png",
      "48": "icons/icon-48.png",
      "128": "icons/icon-128.png"
    }
  },

  "icons": {
    "16": "icons/icon-16.png",
    "48": "icons/icon-48.png",
    "128": "icons/icon-128.png"
  },

  "background": {
    "service_worker": "background.js"
  },

  "permissions": ["storage", "tabs"],

  "content_scripts": [
    {
      "matches": ["https://*.example.com/*"],
      "js": ["content.js"]
    }
  ]
}
```

**Key fields:**

| Field | Purpose |
|-------|---------|
| `manifest_version` | Must be `3` for MV3 |
| `name` / `version` / `description` | Extension metadata |
| `action` | Defines the toolbar icon and popup |
| `icons` | Extension icons at various sizes |
| `background.service_worker` | Path to background script |
| `permissions` | Browser APIs the extension needs access to |
| `content_scripts` | Scripts to inject into web pages |

### Popup lifecycle

The popup's lifecycle is simple but important to understand:

```
User clicks icon
       │
       ▼
Browser creates popup window
       │
       ▼
popup.html is loaded
       │
       ▼
popup.js executes
       │
       ▼
User interacts with popup
       │
       ▼
User clicks outside popup
       │
       ▼
Popup is DESTROYED (DOM gone, JS context gone)
```

Because the popup is destroyed on close, any state you need to persist must be saved
elsewhere — typically via `chrome.storage` or by messaging the background service worker.

### Service Worker lifecycle (background script)

In MV3, the background script is a Service Worker. Unlike MV2's persistent background
pages, Service Workers are **ephemeral**:

```
Extension installed / Browser started
       │
       ▼
Service Worker starts → runs initialization code
       │
       ▼
Idle for ~30 seconds
       │
       ▼
Browser SUSPENDS the Service Worker (saves memory)
       │
       ▼
Event occurs (alarm, message, web request, etc.)
       │
       ▼
Browser WAKES the Service Worker → event handler runs
       │
       ▼
Idle again → suspended again → ...
```

This means you cannot rely on global variables for persistent state. Use
`chrome.storage` instead. For our Hello World extension, the service worker just
logs a message when the extension is first installed.

---

## Cross-Browser Differences

### The three browsers

| Aspect | Chrome | Firefox | Safari |
|--------|--------|---------|--------|
| API standard | Manifest V3 | Manifest V3 (also V2) | Manifest V3 (WebExtensions) |
| API namespace | `chrome.*` | `browser.*` (promise-based) | `browser.*` |
| Packaging | `.crx` / zip | `.xpi` / zip | Native macOS/iOS app |
| Distribution | Chrome Web Store | addons.mozilla.org | Mac App Store |
| Background | Service Worker only | Service Worker or Event Page | Service Worker |
| Promise support | Yes (modern Chrome) | Native | Yes |

### API namespace differences

Chrome uses `chrome.*`:
```typescript
chrome.storage.local.get("key", (result) => { ... });
// or with promises (modern Chrome):
const result = await chrome.storage.local.get("key");
```

Firefox uses `browser.*` with native promises:
```typescript
const result = await browser.storage.local.get("key");
```

Safari uses `browser.*` like Firefox.

**Our solution:** A thin shim that exposes a unified `browserAPI` object:

```typescript
// src/lib/browser-api.ts
export const browserAPI =
  typeof browser !== "undefined" ? browser : chrome;
```

Extensions import `browserAPI` instead of using `chrome` or `browser` directly.

### Manifest differences

Most of the manifest is identical across browsers. The differences:

| Field | Chrome | Firefox | Safari |
|-------|--------|---------|--------|
| `browser_specific_settings.gecko` | Ignored | Required (extension ID) | Ignored |
| `background.service_worker` | Required | Supported | Supported |
| `background.scripts` | Not supported (MV3) | Supported (MV2 only) | Not supported |

**Our solution:** Write one base manifest. The build step produces per-browser variants:

- **Chrome:** Strip `browser_specific_settings`
- **Firefox:** Keep as-is (Chrome-specific fields are ignored by Firefox)
- **Safari:** Strip `browser_specific_settings`, keep everything else

### Safari's special requirements

Safari Web Extensions must be wrapped in a native macOS/iOS app. Apple provides a
converter tool:

```bash
xcrun safari-web-extension-converter /path/to/extension/dist/
```

This generates an Xcode project that wraps the extension in a native app container.
You then build and run it from Xcode. The extension code itself (HTML/CSS/JS) is
identical — the wrapper is just Apple's distribution requirement.

We don't commit the Xcode project to the repo (it's generated, platform-specific).
The README documents how to run the converter.

---

## Cross-Browser Build Pipeline

### Design

Write once, build for all browsers. The pipeline:

```
  src/ (TypeScript)          manifest.json (base)
       │                           │
       ▼                           ▼
  Vite compiles TS → JS     Manifest transformer
       │                           │
       ▼                           ▼
  ┌────────────────────────────────────┐
  │         Vite plugin orchestrates   │
  └──┬──────────────┬─────────────┬───┘
     │              │             │
     ▼              ▼             ▼
  dist/chrome/   dist/firefox/  dist/safari/
  (no gecko      (full           (no gecko
   settings)      manifest)       settings)
```

### Manifest transformer

The manifest transformer reads the base `manifest.json` and produces browser-specific
variants. It's a pure function:

```typescript
type Browser = "chrome" | "firefox" | "safari";

function transformManifest(
  base: ManifestV3,
  browser: Browser
): ManifestV3;
```

**Chrome transform:**
- Remove `browser_specific_settings` (Chrome warns on unknown keys)

**Firefox transform:**
- Keep everything (Firefox ignores Chrome-specific keys)
- Ensure `browser_specific_settings.gecko.id` is present

**Safari transform:**
- Remove `browser_specific_settings`
- Otherwise identical to Chrome

### Vite plugin

The Vite plugin:
1. Reads the base `manifest.json`
2. Runs the manifest transformer for each target browser
3. Copies the compiled JS, HTML, CSS, and icons into each `dist/<browser>/` directory
4. Writes the transformed manifest into each directory

### Browser API shim

A minimal module that normalizes the browser extension API:

```typescript
// Detect which global is available
const api = typeof browser !== "undefined"
  ? browser      // Firefox, Safari
  : chrome;      // Chrome, Edge, Opera

export const browserAPI = api;
```

Extensions import from this module:
```typescript
import { browserAPI } from "../lib/browser-api";
browserAPI.storage.local.get("key");
```

This keeps extension code browser-agnostic.

---

## Scaffold Generator

### Purpose

Creating a new extension involves many files with specific conventions:
manifest.json, package.json, vite.config.ts, tsconfig.json, vitest.config.ts,
BUILD, README.md, CHANGELOG.md, popup template, service worker template.

The scaffold generator automates this. Run one command, get a fully wired extension
project ready to develop.

### Usage

```bash
npx @coding-adventures/browser-extension-toolkit scaffold my-extension \
  --description "What my extension does"
```

This creates:
```
code/programs/typescript/my-extension/
├── manifest.json
├── package.json
├── tsconfig.json
├── vite.config.ts
├── vitest.config.ts
├── BUILD
├── README.md
├── CHANGELOG.md
├── src/
│   ├── lib/
│   │   └── browser-api.ts
│   ├── popup/
│   │   ├── popup.html
│   │   ├── popup.ts
│   │   └── popup.css
│   └── background/
│       └── service-worker.ts
└── tests/
    └── popup.test.ts
```

### Template system

Templates use `{{variable}}` placeholders:

```
// package.json.template
{
  "name": "{{name}}",
  "version": "0.1.0",
  "description": "{{description}}",
  ...
}
```

The scaffold function replaces placeholders with user-provided values.

---

## Hello World Extension — Detailed Design

### Popup UI

A simple page that displays "Hello World" with minimal styling:

```html
<!DOCTYPE html>
<html>
<head>
  <link rel="stylesheet" href="popup.css">
</head>
<body>
  <div class="container">
    <h1>Hello World!</h1>
    <p>This is my first browser extension.</p>
    <p id="browser-info"></p>
  </div>
  <script src="popup.js"></script>
</body>
</html>
```

The popup script detects which browser it's running in and displays that info —
a small way to verify the cross-browser shim works:

```typescript
import { browserAPI } from "../lib/browser-api";

document.addEventListener("DOMContentLoaded", () => {
  const info = document.getElementById("browser-info");
  if (info) {
    const runtime = browserAPI.runtime.getManifest();
    info.textContent = `Running ${runtime.name} v${runtime.version}`;
  }
});
```

### Background service worker

Minimal — just logs on install:

```typescript
import { browserAPI } from "../lib/browser-api";

browserAPI.runtime.onInstalled.addListener((details) => {
  console.log(`Hello World extension installed (reason: ${details.reason})`);
});
```

### Icons

Simple placeholder icons — a colored square with "HW" text. We'll generate these
as inline SVGs and include pre-rendered PNGs at 16×16, 48×48, and 128×128.

---

## Package Structure

### browser-extension-toolkit (library)

**Location:** `code/packages/typescript/browser-extension-toolkit/`

**Dependencies:** None (standalone library)

**Exports:**
- `browserAPI` — cross-browser API shim
- `transformManifest()` — manifest transformer
- `scaffold()` — scaffold generator function
- `webExtensionPlugin()` — Vite plugin

### hello-world-extension (program)

**Location:** `code/programs/typescript/hello-world-extension/`

**Dependencies:**
- `@coding-adventures/browser-extension-toolkit` (via `file:`)

---

## Testing Strategy

### browser-extension-toolkit tests

- **manifest-transformer.test.ts** — Verify each browser transform:
  - Chrome: `browser_specific_settings` is removed
  - Firefox: `browser_specific_settings.gecko.id` is preserved
  - Safari: `browser_specific_settings` is removed
  - All: core fields (name, version, action, icons) are preserved

- **browser-api.test.ts** — Verify shim selection:
  - When `browser` global exists → returns `browser`
  - When only `chrome` global exists → returns `chrome`

- **scaffold.test.ts** — Verify scaffold output:
  - Correct files are generated
  - Template variables are replaced
  - Generated manifest is valid

### hello-world-extension tests

- **popup.test.ts** — Verify popup logic:
  - Browser info is displayed after DOMContentLoaded
  - Graceful behavior when elements are missing
