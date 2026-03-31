/**
 * Manifest Transformer
 * ====================
 *
 * The Problem
 * -----------
 * A browser extension's `manifest.json` is mostly the same across Chrome,
 * Firefox, and Safari — but not entirely. Each browser has specific fields
 * it requires or ignores:
 *
 * | Field                            | Chrome   | Firefox  | Safari   |
 * |----------------------------------|----------|----------|----------|
 * | `browser_specific_settings`      | Warns    | Required | Ignored  |
 * | `background.service_worker`      | Required | Supported| Supported|
 * | `action`                         | Yes      | Yes      | Yes      |
 *
 * Chrome actively warns about unknown keys like `browser_specific_settings`,
 * which clutters the console. Firefox needs the `gecko.id` field for
 * extension identification. Safari ignores Firefox-specific fields.
 *
 * The Solution
 * ------------
 * Write ONE manifest with all fields. The transformer produces clean,
 * browser-specific variants:
 *
 * ```
 *   manifest.json (base, includes everything)
 *         │
 *         ├── transformManifest(base, "chrome")  → no gecko settings
 *         ├── transformManifest(base, "firefox") → keeps everything
 *         └── transformManifest(base, "safari")  → no gecko settings
 * ```
 *
 * This is a pure function — no side effects, no file I/O. It takes a
 * manifest object and returns a new one. The build pipeline (Vite plugin)
 * handles reading/writing files.
 */

/**
 * The three browsers we target.
 *
 * "chrome" covers all Chromium-based browsers: Chrome, Edge, Brave, Opera, Arc.
 * "firefox" covers Firefox and Firefox-based browsers.
 * "safari" covers Safari on macOS and iOS.
 */
export type Browser = "chrome" | "firefox" | "safari";

/**
 * A Manifest V3 object.
 *
 * We define only the fields we need to transform. The rest are passed
 * through unchanged via the index signature. This means the transformer
 * works with any valid MV3 manifest, even if it has fields we haven't
 * explicitly typed.
 */
export interface ManifestV3 {
  /** Must be 3 for Manifest V3. */
  manifest_version: 3;

  /** Extension name shown in the toolbar and extension manager. */
  name: string;

  /** Semantic version string (e.g., "1.0.0"). */
  version: string;

  /** Short description shown in the extension store. */
  description?: string;

  /**
   * Toolbar button configuration.
   * `default_popup` is the HTML file shown when the user clicks the icon.
   */
  action?: {
    default_popup?: string;
    default_icon?: Record<string, string>;
    default_title?: string;
  };

  /** Extension icons at various sizes. */
  icons?: Record<string, string>;

  /**
   * Background script configuration.
   * In MV3, this is always a service worker (a JS file, not an HTML page).
   */
  background?: {
    service_worker?: string;
    type?: string;
  };

  /** Permissions the extension requests (e.g., "storage", "tabs"). */
  permissions?: string[];

  /**
   * Browser-specific settings.
   * Firefox requires `gecko.id` for extension identification.
   * Chrome warns about this field. Safari ignores it.
   */
  browser_specific_settings?: {
    gecko?: {
      id?: string;
      strict_min_version?: string;
      strict_max_version?: string;
    };
    [key: string]: unknown;
  };

  /** Content scripts to inject into web pages. */
  content_scripts?: Array<{
    matches: string[];
    js?: string[];
    css?: string[];
    run_at?: string;
  }>;

  /** Allow any other MV3 fields to pass through. */
  [key: string]: unknown;
}

/**
 * Transform a base manifest for a specific browser.
 *
 * This is a pure function — it returns a new object without modifying
 * the input. The transformations are minimal and well-defined:
 *
 * **Chrome:**
 * - Removes `browser_specific_settings` (Chrome warns on unknown keys)
 *
 * **Firefox:**
 * - Keeps everything as-is (Firefox ignores Chrome-specific keys)
 * - Validates that `browser_specific_settings.gecko.id` exists
 *
 * **Safari:**
 * - Removes `browser_specific_settings` (Safari ignores them)
 *
 * @param base - The base manifest containing all browser fields
 * @param browser - Which browser to transform for
 * @returns A new manifest object tailored for the target browser
 *
 * @example
 * ```typescript
 * const base = {
 *   manifest_version: 3,
 *   name: "My Extension",
 *   version: "1.0.0",
 *   browser_specific_settings: {
 *     gecko: { id: "my-ext@example.com" }
 *   }
 * };
 *
 * const chrome = transformManifest(base, "chrome");
 * // chrome.browser_specific_settings → undefined (removed)
 *
 * const firefox = transformManifest(base, "firefox");
 * // firefox.browser_specific_settings.gecko.id → "my-ext@example.com"
 * ```
 */
export function transformManifest(
  base: ManifestV3,
  browser: Browser,
): ManifestV3 {
  // Start with a shallow clone. We don't need a deep clone because we
  // only remove top-level keys or read nested values — we never mutate
  // nested objects.
  const result = { ...base };

  switch (browser) {
    case "chrome":
      // Chrome warns about unknown manifest keys. The
      // `browser_specific_settings` key is Firefox-specific and triggers
      // a warning in Chrome's extension console. Remove it for a clean build.
      delete result.browser_specific_settings;
      // Chrome doesn't support Firefox's sidebar_action — remove it.
      delete result.sidebar_action;
      break;

    case "firefox":
      // Firefox is the most permissive — it ignores keys it doesn't
      // recognize. We keep everything as-is but validate that the
      // required `gecko.id` field exists.
      if (!result.browser_specific_settings?.gecko?.id) {
        throw new Error(
          "Firefox manifest requires browser_specific_settings.gecko.id. " +
            "Add it to your base manifest.json:\n\n" +
            '  "browser_specific_settings": {\n' +
            '    "gecko": {\n' +
            '      "id": "your-extension@your-domain.com"\n' +
            "    }\n" +
            "  }",
        );
      }
      // Firefox doesn't support Chrome's side_panel API — remove it
      // and strip the sidePanel permission.
      delete result.side_panel;
      if (Array.isArray(result.permissions)) {
        result.permissions = result.permissions.filter(
          (p) => p !== "sidePanel",
        );
      }
      break;

    case "safari":
      // Safari ignores `browser_specific_settings` but we remove it for
      // cleanliness. The output is fed into Apple's
      // `safari-web-extension-converter` which only cares about standard
      // MV3 fields.
      delete result.browser_specific_settings;
      // Safari doesn't support either sidebar API natively.
      delete result.sidebar_action;
      delete result.side_panel;
      if (Array.isArray(result.permissions)) {
        result.permissions = result.permissions.filter(
          (p) => p !== "sidePanel",
        );
      }
      break;
  }

  return result;
}
