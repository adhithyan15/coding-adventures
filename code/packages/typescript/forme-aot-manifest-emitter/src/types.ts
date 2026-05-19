/**
 * types.ts — WebAppManifest and supporting shapes.
 *
 * Subset of the W3C Web App Manifest spec
 * (https://www.w3.org/TR/appmanifest/) covering the fields
 * needed for FM00 v0 PWA support.  Fields the spec marks as
 * "may be ignored" are omitted; v1 can add them as need
 * materialises.
 *
 * @module types
 */

/**
 * Display modes per spec §6.4.  Allowlist-validated.
 *
 * - `fullscreen` — uses entire display
 * - `standalone` — app-like, hides browser UI
 * - `minimal-ui` — minimal browser controls
 * - `browser` — default tab in a browser
 */
export type DisplayMode = "fullscreen" | "standalone" | "minimal-ui" | "browser";

/**
 * One icon entry per spec §6.5.
 *
 *   - `src` (required) — URL to the icon image (http(s):// or
 *     root-relative).
 *   - `sizes` (optional) — space-separated size pairs (`"48x48
 *     96x96"`) or `"any"`.
 *   - `type` (optional) — MIME type (`"image/png"`).
 *   - `purpose` (optional) — `"any"`, `"maskable"`,
 *     `"monochrome"`, or space-separated combo
 *     (`"any maskable"`).
 */
export interface ManifestIcon {
  readonly src: string;
  readonly sizes?: string;
  readonly type?: string;
  readonly purpose?: string;
}

/**
 * Web app manifest top-level shape.  Every field optional —
 * the spec doesn't strictly require any single field, though
 * `name` and `icons` are the bare minimum for installability
 * in most browsers.
 *
 *   - `name` — full app name (shown in install prompt).
 *   - `short_name` — short app name (shown on home screen).
 *   - `description` — long-form description.
 *   - `lang` — BCP 47 language tag (`"en"`, `"en-US"`).
 *   - `dir` — text direction (`"ltr"`, `"rtl"`, `"auto"`).
 *   - `start_url` — URL launched when app starts (http(s)://
 *     OR root-relative).
 *   - `scope` — URL scope the manifest applies to (http(s)://
 *     OR root-relative).
 *   - `display` — display mode (allowlisted).
 *   - `orientation` — preferred orientation (passthrough; spec
 *     enumerates ~9 values but emitter accepts any string).
 *   - `theme_color` — hex colour (`#rgb`, `#rgba`, `#rrggbb`,
 *     `#rrggbbaa`).
 *   - `background_color` — hex colour.
 *   - `icons` — array of `ManifestIcon`.
 */
export interface WebAppManifest {
  readonly name?: string;
  readonly short_name?: string;
  readonly description?: string;
  readonly lang?: string;
  readonly dir?: "ltr" | "rtl" | "auto" | string;
  readonly start_url?: string;
  readonly scope?: string;
  readonly display?: DisplayMode | string;
  readonly orientation?: string;
  readonly theme_color?: string;
  readonly background_color?: string;
  readonly icons?: readonly ManifestIcon[];
}
