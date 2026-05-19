/**
 * types.ts — public signatures for the style tag emitter.
 *
 * Two complementary tag types:
 *
 *   - `<link rel="stylesheet" href="...">` — external stylesheet.
 *     Supports SRI integrity + crossorigin + media query + the
 *     `disabled` attribute (which lets JS swap stylesheets
 *     in/out without re-downloading).
 *   - `<style>...css...</style>` — inline CSS.  Supports a
 *     `media` query.  Inline CSS rejects any literal `</style>`
 *     sequence in its body (case-insensitive) because that
 *     would close the style block early and let arbitrary HTML
 *     follow — the canonical XSS vector for inline-style sinks.
 *
 * @module types
 */

/**
 * `crossorigin` attribute value.  Same allowlist as the sibling
 * `forme-aot-script-tag-emitter` / `forme-aot-meta-link-tags`.
 */
export type CrossOrigin = "anonymous" | "use-credentials";

/**
 * External stylesheet `<link>` descriptor.
 *
 *   - `href`         — required.  http(s):// or root-relative.
 *   - `media`        — optional CSS media query
 *                      (e.g. `"screen"`, `"print"`,
 *                      `"(max-width: 600px)"`).  Passes through
 *                      HTML-attribute-escaped; CSS validation is
 *                      explicitly out of scope (no usable
 *                      pure-JS media-query parser; we'd lock
 *                      callers out of valid queries by
 *                      gatekeeping).
 *   - `integrity`    — optional SRI string.  Same format as
 *                      `forme-aot-script-tag-emitter`:
 *                      `<algo>-<base64>` with
 *                      `algo ∈ {sha256, sha384, sha512}` and
 *                      the per-algo base64 length + padding
 *                      enforced.
 *   - `crossorigin`  — optional `anonymous | use-credentials`.
 *   - `disabled`     — optional boolean.  When `true`, emits the
 *                      `disabled` attribute.  Browsers download
 *                      the stylesheet but don't apply it until
 *                      JS toggles the property.
 */
export interface StylesheetLink {
  readonly href: string;
  readonly media?: string;
  readonly integrity?: string;
  readonly crossorigin?: CrossOrigin;
  readonly disabled?: boolean;
}

/**
 * Inline `<style>` block descriptor.
 *
 *   - `css`   — required CSS source text.  Emitted between
 *               `<style>` and `</style>` verbatim EXCEPT that
 *               any literal `</style>` sequence (case-insensitive)
 *               in the body is rejected at validation time —
 *               that would close the style block early and let
 *               arbitrary HTML follow.
 *   - `media` — optional CSS media query (same passthrough as
 *               above).
 */
export interface InlineStyle {
  readonly css: string;
  readonly media?: string;
}

/**
 * Top-level config consumed by `generateStyleTags`.
 *
 * Output order is fixed: every `stylesheets[]` entry first (in
 * caller's array order), then every `inline[]` entry (in
 * caller's order).  External stylesheets first is the
 * conventional order — they start loading earlier and the
 * cascade resolves predictably.
 */
export interface StyleConfig {
  readonly stylesheets?: readonly StylesheetLink[];
  readonly inline?: readonly InlineStyle[];
}
