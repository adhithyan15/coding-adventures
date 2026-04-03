/**
 * main.ts — Electron main process for the Journal desktop app.
 *
 * === Architecture ===
 *
 * This is a thin wrapper around the journal-app React SPA. The renderer
 * is a standard Vite + React app — it has no Electron dependencies and
 * runs identically in a browser or here.
 *
 * In production, the Vite build output lives in dist/ and the compiled
 * main process lives in dist-electron/. electron-builder bundles both
 * into a platform-specific installer.
 *
 * === Development workflow ===
 *
 * 1. npm run dev             → starts Vite dev server on :5173
 * 2. npm run electron:dev    → compiles main.ts, launches Electron loading :5173
 *
 * === Security model ===
 *
 * nodeIntegration: false
 *   The renderer (React app) cannot access Node.js APIs. A compromised
 *   page cannot read the filesystem or spawn processes.
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
    title: "Journal",
    webPreferences: {
      nodeIntegration: false,
      contextIsolation: true,
    },
  });

  if (process.env.VITE_DEV_SERVER_URL) {
    // Development: load from Vite dev server for hot module replacement.
    win.loadURL(process.env.VITE_DEV_SERVER_URL);
  } else {
    // Production: load the pre-built web app from dist/.
    // __dirname resolves to dist-electron/ at runtime, so ../dist/ is correct.
    win.loadFile(path.join(__dirname, "../dist/index.html"));
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
