# @coding-adventures/visicalc-electron

Electron desktop wrapper around the Mosaic VisiCalc React demo.

This program is the **eighth** cross-backend VisiCalc visualisation —
sibling to the web React build, the SwiftUI / Qt / Flutter / Compose
desktops, and the upcoming native iOS / Android targets.  Its job is
narrow: prove that the exact React bundle the web demo produces can
also boot inside a native desktop window via Electron, with no
host-code duplication.

## Why bother?

VisiCalc, as the "first guinea pig" for Mosaic UI's cross-platform
story, has to render and behave the same on:

- **Web** — React, HTML static, WebComponent
- **Native desktop** — SwiftUI on macOS, Qt on Linux, XAML on
  Windows, Compose for Desktop everywhere
- **Native mobile** — SwiftUI on iOS, Compose on Android
- **Through cross-platform UI kits** — Flutter (mobile + desktop +
  web), Electron (desktop)

Electron specifically gives end users:

1. A real titlebar, OS menus, dock/taskbar icon, native window
   controls — things a browser tab can't deliver.
2. Filesystem access via `dialog.showOpenDialog` + `fs.promises`
   (deferred to v0.2.0).

The same `code/programs/mosaic/visicalc/Grid.{mil,desktop.mll,dark.msl}` and
`FormulaBar.{mil,desktop.mll,dark.msl}` sources drive both the web
build and this Electron host.  When the eventual `.touch.mll`
variant lands we'll also see the same React components in a
mobile-shaped layout — same emitter, same dataset, different shell.

## How it works

```
code/programs/typescript/visicalc-electron/main.js
                                            │
                                            ▼  loads
                                       file:///…/
                                       code/programs/typescript/visicalc/dist/index.html
                                            │
                                            ▼  bundles
                                       code/programs/typescript/visicalc/src/
                                       (Vite build of the React app)
                                            │
                                            ▼  generated from
                                       code/programs/mosaic/visicalc/
                                       (the Mosaic IR: .mil/.mll/.msl)
```

`main.js` is intentionally tiny: a single `BrowserWindow` with
sandbox + contextIsolation on, no preload bridge, no IPC — the
React bundle is a pure visual surface for v0.1.0.

## Run it

```bash
# from this directory
npm install
npm start                # builds the React bundle first, then launches Electron
```

The `start` script triggers `npm run build` in `code/programs/typescript/visicalc/`,
which produces `dist/index.html` + bundled assets, then launches
Electron pointing at the dist URL.

For a faster dev loop once the React bundle is already built:

```bash
npm run start:dev        # skips the React rebuild
```

For devtools:

```bash
VISICALC_ELECTRON_DEBUG=1 npm run start:dev
```

## Security posture

- `nodeIntegration: false`
- `contextIsolation: true`
- `sandbox: true`
- No preload script (no Node APIs exposed to the renderer)
- No `webSecurity: false`, no `allowRunningInsecureContent: true`

These are Electron's defence-in-depth defaults.  The React bundle
runs in a sandboxed Chromium renderer that cannot reach `fs`,
`child_process`, or `net` — equivalent in attack surface to a
regular web page hosted at `file://`.

## v0.2.0 — filesystem wire-up

A real VisiCalc has to read and write spreadsheets.  v0.2.0 will
add:

- A preload script exposing a narrow IPC bridge:
  `window.visicalc = { openFile, saveFile }` returning `Promise<string>`.
- A native menu (`Menu.buildFromTemplate(...)`) with File → Open /
  Save / Save As, wired to `dialog.showOpenDialog` + `fs.promises`.
- A `.calc` file format parser shared with the web demo's
  `state.ts`.

That work belongs in a separate PR.

## Status

`v0.1.0` — single `BrowserWindow` loading the React bundle.
Tested locally on macOS arm64 against Electron 33.
