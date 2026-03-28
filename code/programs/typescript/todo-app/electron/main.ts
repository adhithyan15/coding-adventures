/**
 * main.ts — Electron main process for the Todo app.
 *
 * === What is the main process? ===
 *
 * Every Electron app has exactly one main process. It runs in Node.js
 * (not in the browser) and is responsible for:
 *
 *   1. Creating and managing BrowserWindows (each is a Chromium instance)
 *   2. OS integration: native menus, system tray, file dialogs
 *   3. App lifecycle: startup, shutdown, dock icon behavior on macOS
 *
 * The renderer process (our React + Lattice app) loads inside the window
 * and has full IndexedDB access for offline data persistence.
 *
 * === Security model ===
 *
 * nodeIntegration: false — the renderer CANNOT access Node.js APIs
 * contextIsolation: true — renderer runs in a separate JavaScript context
 *
 * These two settings prevent malicious code from accessing the filesystem.
 *
 * === Development vs Production ===
 *
 * Development: Loads from Vite dev server (hot module replacement).
 *   Set VITE_DEV_SERVER_URL env var when running `electron:dev`.
 *
 * Production: Loads the built dist/index.html from disk (fully offline).
 */

import { app, BrowserWindow } from "electron";
import path from "path";

/**
 * createWindow — opens the app's main window.
 */
function createWindow(): void {
  const win = new BrowserWindow({
    width: 1024,
    height: 768,
    title: "Todo — Offline Task Manager",
    webPreferences: {
      nodeIntegration: false,
      contextIsolation: true,
    },
  });

  // In development, load from Vite dev server for hot reload
  if (process.env.VITE_DEV_SERVER_URL) {
    win.loadURL(process.env.VITE_DEV_SERVER_URL);
  } else {
    // In production, load the Vite-built index.html from disk
    win.loadFile(path.join(__dirname, "../dist/index.html"));
  }
}

// App ready — create the main window
app.whenReady().then(createWindow);

// macOS: re-create window when dock icon is clicked
app.on("activate", () => {
  if (BrowserWindow.getAllWindows().length === 0) {
    createWindow();
  }
});

// Windows/Linux: quit when all windows are closed
app.on("window-all-closed", () => {
  if (process.platform !== "darwin") {
    app.quit();
  }
});
