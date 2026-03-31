# Hello World Extension

A minimal browser extension that shows a "Hello World" popup when you click its toolbar icon. Ships to Chrome, Firefox, and Safari from a single TypeScript codebase.

This is the first extension in the repo, built with the `@coding-adventures/browser-extension-toolkit`.

## What It Does

Click the extension icon in your browser's toolbar. A small popup appears saying "Hello World!" with information about the running extension.

## Loading the Extension

### Chrome

1. Run `npm run build` to produce the `dist/` directory
2. Open `chrome://extensions`
3. Enable "Developer mode" (top right toggle)
4. Click "Load unpacked"
5. Select the `dist/` directory

### Firefox

1. Run `npm run build`
2. Open `about:debugging#/runtime/this-firefox`
3. Click "Load Temporary Add-on..."
4. Select `dist/manifest.json`

### Safari

1. Run `npm run build`
2. Run the Apple converter:
   ```bash
   xcrun safari-web-extension-converter dist/
   ```
3. Open the generated Xcode project
4. Build and run (Cmd+R)

## Development

```bash
npm install
npm run dev     # Watch mode — rebuilds on file changes
npm run build   # One-time production build
npm test        # Run unit tests
```

## How It Fits in the Stack

This extension depends on `@coding-adventures/browser-extension-toolkit` for cross-browser compatibility. The toolkit provides the browser API shim and manifest transformer.

```
browser-extension-toolkit (library)
    ↑
hello-world-extension (this program)
```
