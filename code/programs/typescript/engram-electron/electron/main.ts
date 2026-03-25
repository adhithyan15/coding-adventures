/**
 * main.ts — Electron main process for the Engram desktop app.
 *
 * === Architecture ===
 *
 * This package is a thin wrapper around the engram-app web app.
 * The renderer is a plain React + Vite SPA — it has no Electron
 * dependencies and runs identically in a browser or here.
 *
 * At build time, the BUILD script copies engram-app's compiled output
 * into renderer/ inside this package. electron-builder then bundles
 * renderer/ alongside the compiled main process.
 *
 * === Development workflow ===
 *
 * 1. In engram-app/:  npm run dev      → starts Vite dev server on :5173
 * 2. In this dir:     npm run build    → compiles main.ts → dist-electron/
 * 3. In this dir:     npm run dev      → launches Electron, loads :5173
 *
 * === Security model ===
 *
 * nodeIntegration: false
 *   The renderer (React app) cannot access Node.js APIs. A malicious page
 *   cannot read the filesystem or spawn processes.
 *
 * contextIsolation: true
 *   Electron's internal APIs are isolated from the renderer's JS context.
 *   Any future IPC bridge must go through a preload script + contextBridge.
 */

import { app, BrowserWindow } from "electron";
import path from "path";

function createWindow(): void {
  const win = new BrowserWindow({
    width: 1024,
    height: 768,
    title: "Engram",
    webPreferences: {
      nodeIntegration: false,
      contextIsolation: true,
    },
  });

  if (process.env.VITE_DEV_SERVER_URL) {
    // Development: load from Vite dev server for hot module replacement.
    win.loadURL(process.env.VITE_DEV_SERVER_URL);
  } else {
    // Production: load the pre-built web app from renderer/.
    // renderer/ is populated by the BUILD step before electron-builder runs.
    // __dirname resolves to dist-electron/ at runtime, so ../renderer/ is correct.
    win.loadFile(path.join(__dirname, "../renderer/index.html"));
  }
}

app.whenReady().then(createWindow);

// macOS: re-open window when clicking the dock icon with no open windows.
app.on("activate", () => {
  if (BrowserWindow.getAllWindows().length === 0) {
    createWindow();
  }
});

// Windows / Linux: quit when the last window closes.
app.on("window-all-closed", () => {
  if (process.platform !== "darwin") {
    app.quit();
  }
});
