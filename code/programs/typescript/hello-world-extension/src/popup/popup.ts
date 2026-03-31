/**
 * Popup Script
 * ============
 *
 * This script runs when the user clicks the extension's toolbar icon
 * and the popup HTML is loaded. It has access to:
 *
 * 1. The popup's DOM (the elements in popup.html)
 * 2. The browser extension API (via the cross-browser shim)
 *
 * What this script does
 * ---------------------
 * Detects the extension's runtime information and displays it in the
 * popup. This is a small way to verify that:
 * - The popup loaded correctly
 * - The browser API shim works
 * - The manifest is being read
 *
 * Popup lifecycle reminder
 * ------------------------
 * The popup is DESTROYED when the user clicks outside of it. This
 * script runs from scratch every time. There's no persistent state
 * unless you explicitly save it (e.g., via browser.storage).
 *
 * ```
 * Click icon → popup.html loads → this script runs
 *           → user clicks away → popup destroyed
 *           → click icon again → fresh popup.html → this script runs again
 * ```
 */

import { getBrowserAPI } from "../lib/browser-api";

/**
 * Initialize the popup by populating the browser info element.
 *
 * We export this function so it can be tested independently of the
 * DOMContentLoaded event. The test can call `initPopup()` directly
 * after setting up the DOM.
 */
export function initPopup(): void {
  const info = document.getElementById("browser-info");
  if (!info) return;

  try {
    // Use the cross-browser shim to access the extension API.
    // This works identically on Chrome, Firefox, and Safari.
    const api = getBrowserAPI();
    const manifest = api.runtime.getManifest();

    // Display which extension is running and its version.
    // This confirms the manifest was loaded and the API works.
    info.textContent = `Running ${manifest.name} v${manifest.version}`;
  } catch {
    // If we're running outside an extension context (e.g., opening
    // popup.html directly in a browser tab), the API won't be available.
    // Show a helpful message instead of crashing.
    info.textContent = "Running outside extension context";
  }
}

// Wait for the DOM to be fully loaded before running our code.
// This ensures all HTML elements exist when we try to access them.
document.addEventListener("DOMContentLoaded", initPopup);
