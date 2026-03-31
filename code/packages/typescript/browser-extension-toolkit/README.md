# @coding-adventures/browser-extension-toolkit

A cross-browser toolkit for building browser extensions that ship to Chrome, Firefox, and Safari from a single TypeScript codebase.

## The Problem

Browser extensions are HTML, CSS, and JavaScript — but each browser has different API namespaces (`chrome.*` vs `browser.*`) and manifest requirements. Writing and maintaining browser-specific code is tedious and error-prone.

## The Solution

Write your extension once. This toolkit handles the differences:

| Component | What it does |
|-----------|-------------|
| **Browser API Shim** | Normalizes `chrome.*` / `browser.*` into a single import |
| **Manifest Transformer** | Produces Chrome, Firefox, and Safari manifests from one base |
| **Vite Plugin** | Orchestrates multi-browser builds |
| **Scaffold Generator** | Creates new extension projects with all boilerplate wired up |

## Quick Start

### Create a new extension

```bash
npx @coding-adventures/browser-extension-toolkit scaffold my-extension \
  --description "What my extension does"

cd my-extension
npm install
npm run dev
```

### Use in an existing extension

```typescript
// Use the cross-browser API shim
import { getBrowserAPI } from "@coding-adventures/browser-extension-toolkit";

const api = getBrowserAPI();
const manifest = api.runtime.getManifest();

// Transform manifests for different browsers
import { transformManifest } from "@coding-adventures/browser-extension-toolkit";

const base = JSON.parse(fs.readFileSync("manifest.json", "utf-8"));
const chromeManifest = transformManifest(base, "chrome");
const firefoxManifest = transformManifest(base, "firefox");
```

## How It Fits in the Stack

This is a standalone utility package. It has no dependencies on other packages in the monorepo. Extensions built with this toolkit live in `code/programs/typescript/` and depend on this package via `file:` references.

```
browser-extension-toolkit (this package)
    ↑
    └── hello-world-extension (first extension, uses this toolkit)
    └── future-extension-2 (any future extension)
```
