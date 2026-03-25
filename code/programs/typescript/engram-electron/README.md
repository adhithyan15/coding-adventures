# engram-electron

Thin Electron wrapper for the [engram-app](../engram-app) web app.

This package contains only the Electron main process. All application logic,
UI components, and business logic live in `engram-app`. At build time the
compiled `engram-app/dist/` is copied into `renderer/` and bundled alongside
the main process by electron-builder.

## Architecture

```
engram-app/          ← React + Vite SPA (runs in browser OR Electron renderer)
engram-electron/     ← this package
  electron/
    main.ts          loads renderer/index.html (prod) or :5173 (dev)
    tsconfig.json
  electron-builder.yml
  package.json       electron + electron-builder only; no React/Vite deps
  BUILD              builds engram-app first, then compiles main process
  renderer/          ← git-ignored; populated by BUILD step
  dist-electron/     ← git-ignored; compiled main.js
  release/           ← git-ignored; electron-builder output
```

## Development workflow

```bash
# Terminal 1 — start the Vite dev server in engram-app
cd code/programs/typescript/engram-app
npm run dev

# Terminal 2 — compile main.ts, then launch Electron against the dev server
cd code/programs/typescript/engram-electron
npm install
npm run build
npm run dev
```

## Production build

```bash
# 1. Build the web app (produces engram-app/dist/)
cd code/programs/typescript/engram-app
npm run build

# 2. Copy dist into renderer/ (electron-builder needs it here)
cp -r dist ../engram-electron/renderer

# 3. Compile main process and package
cd ../engram-electron
npm install
npm run build
npx electron-builder
# Installers written to release/
```

## Releasing

Tag and push — the `release-engram.yml` workflow does the rest:

```bash
git tag engram-v0.2.0
git push origin engram-v0.2.0
```

Builds macOS (.dmg + .zip), Windows (.exe + portable), and Linux (.AppImage)
in parallel and attaches all installers to a GitHub Release.
