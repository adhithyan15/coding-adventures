/**
 * Background Service Worker
 * =========================
 *
 * What is a Service Worker?
 * -------------------------
 * A Service Worker is a JavaScript file that runs in the background,
 * separate from any web page. In the context of browser extensions:
 *
 * - It has NO access to the DOM (`document` and `window` are undefined)
 * - It CAN use the browser extension API (tabs, storage, alarms, etc.)
 * - The browser can SUSPEND it when idle and WAKE it for events
 * - Global variables are NOT persistent (they reset on wake)
 *
 * Why this is minimal for v1
 * --------------------------
 * All bookmark CRUD operations happen directly in the popup, which has
 * its own IndexedDB access. No message passing between popup and service
 * worker is needed.
 *
 * Future versions may use the service worker for:
 * - Cloud sync scheduling (Google Drive, OneDrive)
 * - Badge updates (show count of bookmarks for current tab)
 * - Context menu integration (right-click to bookmark)
 */

import { getBrowserAPI } from "../lib/browser-api";

// CRITICAL: Event listeners MUST be registered synchronously at the top
// level. See hello-world-extension's service-worker.ts for a detailed
// explanation of why.

try {
  const api = getBrowserAPI();

  api.runtime.onInstalled.addListener((details) => {
    console.log(`Bookmarks extension installed (reason: ${details.reason})`);
  });
} catch {
  // Running outside an extension context (e.g., in tests).
  console.log("Service worker loaded outside extension context");
}
