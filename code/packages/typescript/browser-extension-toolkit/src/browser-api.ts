/**
 * Cross-Browser API Shim
 * ======================
 *
 * Why this exists
 * ---------------
 * Chrome and Firefox/Safari expose the same extension APIs under different
 * global names:
 *
 * | Browser        | Global    | Style            |
 * |---------------|-----------|------------------|
 * | Chrome        | `chrome`  | Callbacks + Promises |
 * | Firefox       | `browser` | Promises (native)    |
 * | Safari        | `browser` | Promises (native)    |
 * | Edge, Brave   | `chrome`  | Same as Chrome       |
 *
 * If you write `chrome.storage.local.get(...)` in your extension, it won't
 * work in Firefox. If you write `browser.storage.local.get(...)`, it won't
 * work in Chrome. This shim solves that.
 *
 * How it works
 * ------------
 * We detect which global is available at runtime and export it as
 * `browserAPI`. Your extension code imports from this module instead of
 * referencing globals directly:
 *
 * ```typescript
 * // Instead of:
 * chrome.storage.local.get("key");   // Chrome only!
 * browser.storage.local.get("key");  // Firefox only!
 *
 * // Do this:
 * import { getBrowserAPI } from "@coding-adventures/browser-extension-toolkit";
 * const api = getBrowserAPI();
 * api.storage.local.get("key");      // Works everywhere!
 * ```
 *
 * Why a function instead of a constant?
 * ------------------------------------
 * We use `getBrowserAPI()` instead of a top-level `const browserAPI = ...`
 * because the global (`chrome` or `browser`) might not be available when
 * the module is first imported — for example, in test environments or
 * during server-side rendering. The function defers the lookup to call time.
 *
 * Type safety
 * -----------
 * The `BrowserAPI` type is intentionally broad (`Record<string, unknown>`)
 * because we don't want to pull in Chrome's or Firefox's full type
 * definitions as dependencies. In practice, extension code that uses
 * specific APIs (like `chrome.storage`) should install `@types/chrome`
 * as a dev dependency and cast as needed.
 */

/**
 * A minimal type representing the browser extension API object.
 *
 * We keep this intentionally loose — the actual shape depends on which
 * browser and which permissions the extension has. Extensions that need
 * type-safe access to specific APIs should install `@types/chrome` or
 * `@anthropic-ai/web-ext-types` as dev dependencies.
 */
export interface BrowserAPI {
  runtime: {
    getManifest: () => Record<string, unknown>;
    onInstalled: {
      addListener: (
        callback: (details: { reason: string }) => void,
      ) => void;
    };
    id?: string;
    getURL?: (path: string) => string;
    [key: string]: unknown;
  };
  storage?: {
    local: {
      get: (
        keys: string | string[],
      ) => Promise<Record<string, unknown>>;
      set: (items: Record<string, unknown>) => Promise<void>;
      [key: string]: unknown;
    };
    [key: string]: unknown;
  };
  [key: string]: unknown;
}

/**
 * Returns the browser extension API object for the current environment.
 *
 * Detection order:
 * 1. `browser` global (Firefox, Safari)
 * 2. `chrome` global (Chrome, Edge, Brave, Opera)
 * 3. Throws an error if neither is available
 *
 * The error case happens when this code runs outside a browser extension
 * context — e.g., in Node.js tests without mocking. Tests should mock
 * the global before calling this function.
 *
 * @returns The browser extension API object
 * @throws {Error} If no browser extension API is available
 */
export function getBrowserAPI(): BrowserAPI {
  // Why check `browser` first?
  // Firefox and Safari define both `browser` AND `chrome` (for compatibility),
  // but `browser` is the canonical, promise-based version. Chrome only defines
  // `chrome`. So checking `browser` first gives us the best API on each platform.
  if (typeof globalThis !== "undefined") {
    const g = globalThis as Record<string, unknown>;

    if (g.browser && typeof g.browser === "object") {
      return g.browser as unknown as BrowserAPI;
    }

    if (g.chrome && typeof g.chrome === "object") {
      return g.chrome as unknown as BrowserAPI;
    }
  }

  throw new Error(
    "No browser extension API found. " +
      "This code must run inside a browser extension context, " +
      "or the `browser` / `chrome` global must be mocked for testing.",
  );
}
