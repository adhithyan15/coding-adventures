/**
 * escape.ts — HTML escaping + URL-scheme allowlist.
 *
 * =============================================================================
 * WHY THIS FILE EXISTS
 * =============================================================================
 *
 * The page-shell renderer concatenates HTML strings.  Every piece
 * of user-supplied text that ends up inside that HTML must be
 * escaped, or the user is one `<script>alert(1)</script>` away
 * from arbitrary JS execution in every reader's browser.
 *
 * This module is the SINGLE PLACE where escaping happens.  The
 * renderer modules call `escapeHtml` / `escapeAttr` / `safeHref`
 * for every user-supplied value; if you grep the source for
 * `${`-templates and don't find a corresponding `escape*` call,
 * that's a bug.
 *
 * =============================================================================
 * THREAT MODEL
 * =============================================================================
 *
 * The renderer treats these as UNTRUSTED:
 *   - `page.title`, `page.description`
 *   - `site.title`, `site.copyright`, `site.version`
 *   - every `sidebar` entry's `label` and `path`
 *   - every `breadcrumb`'s `label` and `href`
 *   - every `toc` entry's `text` and `id`
 *   - `options.bodyClass`, `options.lang`
 *
 * The renderer treats these as TRUSTED (passed through verbatim):
 *   - `page.body` — pre-rendered HTML from upstream markdown
 *     renderer.  Documented as the only exception.
 *   - `options.headExtra` — raw HTML the caller explicitly injects
 *     into `<head>`.  Documented as caller's responsibility.
 *
 * =============================================================================
 * URL SAFETY
 * =============================================================================
 *
 * HTML escaping alone isn't enough for `href` attributes.  Even
 * with proper escaping, `<a href="javascript:alert(1)">` still
 * fires on click because the JS scheme is interpreted by the
 * browser.  `safeHref` runs an allowlist of schemes:
 *
 *   ALLOWED:    relative URLs (no `<scheme>:` prefix),
 *               http://, https://, mailto:, #anchor
 *   REJECTED:   javascript:, data:, vbscript:, file:, anything else
 *
 * Rejected URLs become `"#"` — visually broken (no navigation)
 * but inert.
 *
 * @module escape
 */

/**
 * Escape a string for safe interpolation into HTML element text
 * content or attribute values.  Idempotent (re-running on
 * already-escaped output produces the same result given the
 * escaping is exhaustive over `&<>"'`).
 *
 * Replaces five characters:
 *
 *   &  →  &amp;     (must be FIRST — other replacements emit &)
 *   <  →  &lt;
 *   >  →  &gt;
 *   "  →  &quot;
 *   '  →  &#39;     (numeric reference — &apos; is HTML5-only,
 *                    breaks in some legacy XHTML parsers)
 *
 * Note `&amp;` must come first; otherwise `&` produced by later
 * replacements (e.g. `&lt;`) would get re-escaped to `&amp;lt;`.
 *
 * @param s - The raw string.  Non-string inputs are coerced via
 *            `String(...)`.
 * @returns The HTML-safe equivalent.
 */
export function escapeHtml(s: string): string {
  return String(s)
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

/**
 * Alias for `escapeHtml` — useful for readers grepping for
 * attribute-context escaping.  Same five-char escape set
 * suffices: both quote types are handled, and `<`/`>` prevent
 * an attacker from breaking out of the attribute into a new
 * element even on malformed parsers.
 */
export function escapeAttr(s: string): string {
  return escapeHtml(s);
}

// ─────────────────────────────────────────────────────────────────────
// URL-scheme allowlist
// ─────────────────────────────────────────────────────────────────────

/**
 * The schemes we consider safe to put in an `href` attribute.
 * Anything else (notably `javascript:`, `data:`, `vbscript:`) is
 * rejected and replaced with `"#"`.  Comparison is case-insensitive
 * because browsers don't care.
 */
const SAFE_SCHEMES: ReadonlySet<string> = new Set(["http", "https", "mailto"]);

/**
 * Strip leading and trailing C0 controls (U+0000..U+001F) plus
 * SPACE (U+0020) using explicit index walks.  Linear-time and
 * obviously so — CodeQL's `js/polynomial-redos` query flags any
 * `+` quantifier on user-controllable input regardless of
 * whether the underlying regex is actually polynomial, and
 * explicit loops sidestep the false-positive.
 *
 * @internal
 */
function stripC0Edges(s: string): string {
  let start = 0;
  while (start < s.length && s.charCodeAt(start) <= 0x20) start++;
  let end = s.length;
  while (end > start && s.charCodeAt(end - 1) <= 0x20) end--;
  if (start === 0 && end === s.length) return s;
  return s.slice(start, end);
}

/**
 * Sanitise a URL for use in an `href` attribute.
 *
 * =============================================================================
 * THE WHATWG URL "STRIPPED" CHARACTER PROBLEM
 * =============================================================================
 *
 * Per WHATWG URL Standard §3.2 ("URL parsing"), browsers strip
 * the following characters BEFORE parsing the scheme:
 *
 *   - Leading / trailing C0 controls (U+0000..U+001F) + U+0020 (SPACE)
 *   - ALL occurrences of TAB (U+0009), LF (U+000A), CR (U+000D) —
 *     anywhere in the URL, not just leading/trailing
 *
 * That means a string like `"java\tscript:alert(1)"` (literal tab
 * between "java" and "script") gets parsed by the browser as
 * `"javascript:alert(1)"` — the tab is stripped during URL
 * normalisation.  Similarly `"javascript:alert(1)"` loses
 * its leading C0 control and becomes a JS URL.
 *
 * A naive scheme regex like `/^([A-Za-z][A-Za-z0-9+.\-]*):/` does
 * NOT match these strings (tab isn't a valid scheme character),
 * so the function would misclassify them as "relative URL" and
 * pass them through — letting the browser do the dangerous
 * normalisation step.
 *
 * We pre-emptively normalise the input to match browser behaviour
 * BEFORE classification.  The scheme regex then sees what the
 * browser will see, and the JS-scheme check fires correctly.
 *
 * =============================================================================
 * THE FULL SANITISATION PIPELINE
 * =============================================================================
 *
 *   1. Strip ALL TAB / LF / CR (anywhere in the URL — match WHATWG).
 *   2. Strip leading/trailing C0 controls + SPACE (match WHATWG).
 *   3. Empty → "#" (no navigation).
 *   4. Anchor (#…) → escapeAttr; safe.
 *   5. Try scheme extraction.  No scheme → relative URL → escapeAttr.
 *   6. Scheme in {http, https, mailto} (case-insensitive) →
 *      escapeAttr; safe.
 *   7. Any other scheme → "#" (rejected, inert).
 *
 * @param raw - The candidate URL string.
 * @returns Either the cleaned URL (HTML-escaped) if it's a
 *          relative reference, anchor, or safe-scheme absolute
 *          URL — OR `"#"` if it's a rejected scheme.
 */
export function safeHref(raw: string): string {
  // Step 1: strip TAB/LF/CR from anywhere — these are silently
  // removed by the WHATWG URL parser during normalisation, so we
  // do it first to make sure our scheme regex sees what the
  // browser will see.
  const stripped = String(raw).replace(/[\t\n\r]/g, "");
  // Step 2: strip leading/trailing C0 controls (U+0000..U+001F)
  // plus SPACE (U+0020).  `.trim()` only strips Unicode
  // whitespace — it leaves U+0000..U+0008 / U+000B / U+000C /
  // U+000E..U+001F alone, but the WHATWG parser strips them.
  //
  // We do this with explicit index walks rather than
  // `.replace(/^[\x00-\x20]+/, "")` / `.replace(/[\x00-\x20]+$/, "")`
  // because CodeQL's `js/polynomial-redos` query flags ANY `+`
  // quantifier on a user-controllable anchored regex — even a
  // simple single character class with no nested quantifiers.
  // The regex IS linear in V8, but the static analyser is
  // conservative.  Explicit O(N) loops avoid the warning AND
  // make the bound obvious to readers.
  const cleaned = stripC0Edges(stripped);
  // Step 3: empty → "#".  Better than emitting an empty href
  // which browsers treat as "reload current page".
  if (cleaned === "") return "#";
  // Step 4: pure anchor — always safe.
  if (cleaned.startsWith("#")) return escapeAttr(cleaned);
  // Step 5+6: try to extract a scheme.  RFC 3986: scheme =
  // ALPHA *( ALPHA / DIGIT / "+" / "-" / "." ) followed by ":".
  // The regex is bounded — single non-greedy alternation on a
  // small character class anchored at `^`.  Not super-linear.
  const schemeMatch = /^([A-Za-z][A-Za-z0-9+.\-]*):/.exec(cleaned);
  if (schemeMatch === null) {
    // No scheme → relative URL.  Safe.  (Path-style URLs like
    // `/guide/setup` and `./other.html` end up here too.)
    return escapeAttr(cleaned);
  }
  const scheme = schemeMatch[1]!.toLowerCase();
  if (SAFE_SCHEMES.has(scheme)) {
    return escapeAttr(cleaned);
  }
  // Step 7: rejected scheme.  Emit `"#"` — visually broken (no
  // navigation) but inert.  A noisy alternative would be throwing,
  // but that would let one rogue link kill the whole page render.
  return "#";
}
