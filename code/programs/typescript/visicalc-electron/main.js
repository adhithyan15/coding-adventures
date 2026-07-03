// main.js — Electron main process for the VisiCalc demo.
//
// Cross-platform desktop wrapper that hosts the *exact same* React
// bundle produced by the sibling `visicalc/` program's Vite build.  No host-code
// duplication: the .mil/.mll/.msl sources compile once, the React
// emitter produces `Grid.tsx` + `FormulaBar.tsx`, Vite bundles them
// into `dist/`, and this main process loads `dist/index.html` into
// a `BrowserWindow`.
//
// Why a separate Electron host at all?  Web vs. Electron differ in
// two ways the user-facing app eventually has to care about:
//
//   1. Native window chrome — Electron gives a real titlebar, OS
//      menus, dock/taskbar icon, file dialogs, etc.  Web doesn't.
//   2. Filesystem access — VisiCalc will eventually open and save
//      .calc / .ods / .csv files.  In a browser those go through
//      File System Access API (Chromium-only); in Electron they go
//      through `dialog.showOpenDialog` + `fs.promises`.  Same
//      product, two host-bindings.
//
// For v0.1.0 the only job is "open a window that loads the React
// bundle and renders the dataset identically to the web build."
// Filesystem wire-up is deferred to v0.2.0.

const { app, BrowserWindow } = require("electron");
const path = require("path");
const url = require("url");

/**
 * Resolve the file:// URL of the React bundle's `dist/index.html`.
 *
 * The Electron host lives at:
 *   code/programs/typescript/visicalc-electron/main.js
 *
 * The compiled React bundle lives in the sibling program:
 *   code/programs/typescript/visicalc/dist/index.html
 *
 * One `..` climbs out of `visicalc-electron/` and we descend into the
 * `visicalc/` sibling's `dist/`.
 */
function distIndexUrl() {
  const distIndex = path.resolve(
    __dirname,
    "..", "visicalc", "dist", "index.html",
  );
  return url.pathToFileURL(distIndex).toString();
}

/**
 * Create the main app window.  Hosts the React bundle without any
 * preload-script bridge (v0.1.0 — purely visual).  Filesystem
 * permissions are deliberately NOT granted so a future hostile
 * formula can't reach into the user's home directory until we
 * surface a real File menu.
 */
function createMainWindow() {
  const win = new BrowserWindow({
    width: 1280,
    height: 800,
    title: "VisiCalc — Mosaic Electron demo",
    backgroundColor: "#1E1E1E",
    webPreferences: {
      // Keep node integration OFF and contextIsolation ON — Electron
      // security best practice.  The React bundle doesn't need to
      // reach Node APIs for v0.1.0.
      nodeIntegration: false,
      contextIsolation: true,
      sandbox: true,
    },
  });

  win.loadURL(distIndexUrl());

  // Defence in depth: explicitly deny any attempt to open a new
  // window or navigate away from the local React bundle.  Without
  // these, a hostile DOM that somehow got into the renderer (e.g.
  // via a future XSS in the React app) could open external URLs
  // or pivot the existing window to phishing pages.  The sandbox
  // already blocks Node access; this closes the navigation door.
  win.webContents.setWindowOpenHandler(() => ({ action: "deny" }));
  win.webContents.on("will-navigate", (event, navigationUrl) => {
    if (navigationUrl !== win.webContents.getURL()) {
      event.preventDefault();
    }
  });

  // Re-open devtools only in development to help debug rendering
  // issues; off by default for end users.
  if (process.env.VISICALC_ELECTRON_DEBUG === "1") {
    win.webContents.openDevTools({ mode: "detach" });
  }

  return win;
}

// Only wire the lifecycle handlers when we're actually running
// under the electron binary.  When `main.js` is `require()`-d from
// plain Node (e.g. the BUILD-time smoke test), `app` is undefined
// because Electron's IPC primitives are only available in the
// electron process — so we skip lifecycle wiring and just expose
// `distIndexUrl` for testing.
if (app && typeof app.whenReady === "function") {
  app.whenReady().then(() => {
    createMainWindow();

    app.on("activate", () => {
      // macOS convention — re-open a window when the dock icon is
      // clicked and no windows are open.
      if (BrowserWindow.getAllWindows().length === 0) {
        createMainWindow();
      }
    });
  });

  app.on("window-all-closed", () => {
    // On non-macOS, quitting the last window quits the app.
    if (process.platform !== "darwin") {
      app.quit();
    }
  });
}

// Export the URL helper so the test in package.json can `require`
// this file and exercise the path-resolution logic without booting
// Electron.
module.exports = { distIndexUrl };
