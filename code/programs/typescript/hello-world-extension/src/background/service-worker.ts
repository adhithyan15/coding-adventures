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
 * Manifest V3 requires background scripts to be Service Workers
 * (replacing the persistent "background pages" from Manifest V2).
 *
 * Why this matters
 * ----------------
 * ```
 * Extension installed → Service Worker starts → runs this file
 *                    → idle for ~30s → browser SUSPENDS it
 *                    → event fires → browser WAKES it → handler runs
 *                    → idle again → suspended again → ...
 * ```
 *
 * Because the worker can be suspended at any time:
 * - Don't rely on global variables for state → use browser.storage
 * - Register all event listeners at the TOP LEVEL → not inside callbacks
 * - Don't use setTimeout/setInterval → use browser.alarms
 *
 * What this file does
 * -------------------
 * For this hello-world extension, we just log a message when the
 * extension is first installed. A real extension would register
 * listeners for browser events, set up alarms, etc.
 */

import { getBrowserAPI } from "../lib/browser-api";

// ==========================================================================
// Event Registration
// ==========================================================================
//
// CRITICAL: Event listeners MUST be registered synchronously at the top
// level of the service worker. The Service Worker spec requires this —
// if you register listeners inside an async callback or setTimeout,
// the browser may miss events because the listeners weren't registered
// when the worker started.
//
// This is why we don't do:
//   setTimeout(() => { api.runtime.onInstalled.addListener(...) }, 0);  // BAD
//   fetchConfig().then(() => { api.runtime.onInstalled.addListener(...) }); // BAD
//
// Instead, we register everything at the top level:

try {
  const api = getBrowserAPI();

  /**
   * Called when the extension is first installed, updated, or the browser
   * is updated.
   *
   * The `details.reason` tells us why:
   * - "install"  → user just installed the extension
   * - "update"   → extension was updated to a new version
   * - "browser_update" → the browser itself was updated
   */
  api.runtime.onInstalled.addListener((details) => {
    console.log(`Hello World extension installed (reason: ${details.reason})`);
  });
} catch {
  // Running outside an extension context (e.g., in tests).
  // The service worker can't do anything without the browser API,
  // so we just log and exit gracefully.
  console.log("Service worker loaded outside extension context");
}
