/**
 * types.ts — public signatures for the HTML document emitter.
 *
 * The emitter is the final assembly stage in the FM00 v0 head /
 * body pipeline.  It takes pre-built `head` and `body` chunks
 * (already produced by the sibling emitters — meta-link-tags,
 * style-tag, script-tag, etc.) and wraps them in the canonical
 * `<!doctype html>...</html>` shell with optional `lang`, `dir`,
 * and extra `<html>` / `<body>` attribute maps.
 *
 * `head` and `body` are passthrough strings — we don't escape
 * them because they're already trusted output from upstream
 * FM00 emitters that did their own validation + escaping.  The
 * attribute maps DO get full validation: keys must be lowercase
 * ASCII identifier characters (plus dashes) so attacker keys
 * like `onload`, `__proto__`, or `style; alert(1)` can't sneak
 * into the tag, and values pass through `escapeHtmlAttr`.
 *
 * @module types
 */

/**
 * `dir` attribute allowlist.  Per HTML Living Standard §3.2.6.4.
 */
export type DocDirection = "ltr" | "rtl" | "auto";

/**
 * Top-level config consumed by `generateHtmlDocument`.
 *
 *   - `head`      — required.  HTML string for the `<head>`
 *                   interior (e.g. concatenation of
 *                   `generateStyleTags(...)` +
 *                   `generateMetaLinkTags(...)` + ...).
 *                   Passthrough; NOT escaped.
 *   - `body`      — required.  HTML string for the `<body>`
 *                   interior.  Passthrough; NOT escaped.
 *   - `lang`      — optional BCP-47 language tag, e.g. `"en"`,
 *                   `"en-US"`, `"zh-Hant-HK"`.  Conservative
 *                   regex (subset of BCP-47): one ASCII alpha
 *                   primary subtag, optional dash-separated
 *                   alphanumeric subsequent subtags.  Empty
 *                   string → throw.
 *   - `dir`       — optional direction allowlist.
 *   - `htmlAttrs` — optional extra `<html>` attributes.  Map of
 *                   attribute name → value.  Names: lowercase
 *                   ASCII letters/digits/dashes/colons starting
 *                   with a letter, length ≤ 64.  Reserved names
 *                   (`lang`, `dir`, `xmlns`) AND any `on*` event
 *                   handler are rejected.
 *   - `bodyAttrs` — optional extra `<body>` attributes; same
 *                   validation rules.
 */
export interface HtmlDocConfig {
  readonly head: string;
  readonly body: string;
  readonly lang?: string;
  readonly dir?: DocDirection;
  readonly htmlAttrs?: Readonly<Record<string, string>>;
  readonly bodyAttrs?: Readonly<Record<string, string>>;
}
