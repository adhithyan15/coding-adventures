/**
 * Cross-Browser API Shim (Local Re-export)
 * =========================================
 *
 * This module re-exports the browser API shim from the toolkit.
 * Every file in this extension imports from here rather than
 * directly from the toolkit package.
 *
 * Why a local re-export?
 * ----------------------
 * 1. Single import path for the entire extension codebase
 * 2. Easy to swap the shim if we ever need custom behavior
 * 3. Import paths stay short: `../lib/browser-api` vs
 *    `@coding-adventures/browser-extension-toolkit`
 */

export { getBrowserAPI } from "@coding-adventures/browser-extension-toolkit";
export type { BrowserAPI } from "@coding-adventures/browser-extension-toolkit";
