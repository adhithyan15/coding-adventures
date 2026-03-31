/**
 * Background Service Worker
 * =========================
 *
 * This service worker has two jobs:
 *
 * 1. Open the side panel when the user clicks the extension icon.
 *    Chrome uses the `sidePanel` API, Firefox uses `sidebar_action`.
 *    The service worker abstracts this difference.
 *
 * 2. Log when the extension is installed (for debugging).
 *
 * Side Panel Abstraction
 * ----------------------
 * Chrome (114+):
 *   - `chrome.sidePanel.open({ windowId })` opens the panel
 *   - Manifest declares `"side_panel": { "default_path": "panel.html" }`
 *   - Requires the `sidePanel` permission
 *
 * Firefox:
 *   - `browser.sidebarAction.open()` opens the sidebar
 *   - Manifest declares `"sidebar_action": { "default_panel": "panel.html" }`
 *   - No extra permission needed
 *
 * Both use the same HTML/CSS/JS — only the manifest entry and the
 * open-on-click wiring differ.
 *
 * Why the service worker handles this
 * ------------------------------------
 * Without a `default_popup` in the manifest, clicking the extension icon
 * does nothing by default. We register an `action.onClicked` listener
 * to open the side panel instead. This listener MUST be at the top level
 * (synchronous registration) per the Service Worker spec.
 */

import { getBrowserAPI } from "../lib/browser-api";

// CRITICAL: Event listeners MUST be registered synchronously at the top
// level. See hello-world-extension's service-worker.ts for a detailed
// explanation of why.

try {
  const api = getBrowserAPI();

  // ---------------------------------------------------------------
  // Open side panel when the extension icon is clicked
  // ---------------------------------------------------------------
  // Since there's no default_popup in the manifest, the action.onClicked
  // event fires when the user clicks the toolbar icon. We use it to
  // toggle the side panel open.

  api.action.onClicked.addListener(async (tab: { windowId?: number }) => {
    try {
      if (api.sidePanel?.open) {
        // Chrome's sidePanel API
        await api.sidePanel.open({ windowId: tab.windowId });
      } else if (api.sidebarAction?.open) {
        // Firefox's sidebar_action API
        await api.sidebarAction.open();
      }
    } catch (err) {
      console.error("Failed to open side panel:", err);
    }
  });

  // ---------------------------------------------------------------
  // Log installation for debugging
  // ---------------------------------------------------------------

  api.runtime.onInstalled.addListener((details) => {
    console.log(`Bookmarks extension installed (reason: ${details.reason})`);
  });
} catch {
  // Running outside an extension context (e.g., in tests).
  console.log("Service worker loaded outside extension context");
}
