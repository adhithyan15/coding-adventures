/**
 * main.ts — Electron main process for the Engram app.
 *
 * === What is the main process? ===
 *
 * Every Electron app has exactly one main process. It runs in Node.js
 * (not in the browser) and is responsible for:
 *
 *   1. Creating and managing BrowserWindows (each is a Chromium instance)
 *   2. OS integration: native menus, system tray, file dialogs
 *   3. App lifecycle: startup, shutdown, dock icon behavior on macOS
 *   4. IPC: receiving messages from renderer processes and responding
 *
 * The main process is the "backend" of your desktop app, except it runs
 * on the user's machine, not a remote server.
 *
 * === Security model ===
 *
 * Two critical settings:
 *
 *   nodeIntegration: false
 *     The renderer process (your React app) CANNOT access Node.js APIs
 *     like `fs`, `child_process`, or `require`. This prevents a malicious
 *     web page from reading your filesystem.
 *
 *   contextIsolation: true
 *     The renderer runs in a separate JavaScript context from Electron's
 *     internal code. Even if someone injects code into the page, they
 *     cannot access Electron APIs.
 *
 * If we need OS access in the renderer later (e.g., file open dialog),
 * we'll add a preload.ts script that uses contextBridge to expose
 * specific, safe APIs — never the full Node.js surface.
 *
 * === Development vs Production ===
 *
 * In development: the renderer loads from the Vite dev server URL
 * (hot module replacement, instant reload on code changes).
 *
 * In production: the renderer loads the built dist/index.html file
 * from disk (no server needed, fully offline).
 */

import { app, BrowserWindow } from "electron";
import path from "path";

/**
 * createWindow — opens the app's main window.
 *
 * BrowserWindow is Electron's wrapper around a Chromium rendering engine.
 * Each window is an independent process (Chromium's multi-process architecture).
 * Our app uses a single window, but Electron supports multiple.
 */
function createWindow(): void {
  const win = new BrowserWindow({
    width: 1024,
    height: 768,
    title: "Engram",
    webPreferences: {
      // Security: renderer cannot access Node.js
      nodeIntegration: false,
      // Security: separate JS context for Electron internals
      contextIsolation: true,
    },
  });

  // In development, load from Vite dev server for hot reload.
  // Set VITE_DEV_SERVER_URL env var when running `electron:dev`.
  if (process.env.VITE_DEV_SERVER_URL) {
    win.loadURL(process.env.VITE_DEV_SERVER_URL);
  } else {
    // In production, load the Vite-built index.html from disk.
    // __dirname is dist-electron/ (where this file is compiled to),
    // so ../dist/index.html points to the Vite output.
    win.loadFile(path.join(__dirname, "../dist/index.html"));
  }
}

// app.whenReady() resolves when Electron has finished initializing.
// This is the earliest point where you can create windows.
app.whenReady().then(createWindow);

// === macOS dock behavior ===
//
// On macOS, apps typically stay running even when all windows are closed
// (the dock icon persists). Clicking the dock icon should re-open the window.
// On Windows and Linux, closing all windows quits the app.

app.on("activate", () => {
  if (BrowserWindow.getAllWindows().length === 0) {
    createWindow();
  }
});

app.on("window-all-closed", () => {
  if (process.platform !== "darwin") {
    app.quit();
  }
});
