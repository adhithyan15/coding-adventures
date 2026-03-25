/**
 * HTML Sanitization Policy — Types and Named Presets
 *
 * The HTML sanitizer operates on raw HTML strings (not Document AST nodes).
 * Its policy controls which elements, attributes, and URL schemes are allowed.
 *
 * Unlike the AST sanitizer, the HTML sanitizer cannot be semantically precise
 * about node types — it only sees tag names and attribute names, not structured
 * document intent. This is why the AST sanitizer is the preferred approach
 * (spec TE02 §Stage 1) and the HTML sanitizer is the safety net (§Stage 2).
 *
 * Design (Decision 5 from spec): no DOM dependency by default. The sanitizer
 * uses regex/string operations for portability across all target languages —
 * Go, Python, Rust, Elixir, and Lua do not have a browser DOM.
 *
 * Spec: TE02 — Document Sanitization §Stage 2
 *
 * @module policy
 */

// ─── DOM Adapter ──────────────────────────────────────────────────────────────
//
// The DOM adapter is an escape hatch for browser environments where a real
// HTML parser is available. When provided, the sanitizer parses the HTML into
// a DOM, walks it with a visitor, and serializes back to a string.
//
// By keeping the DOM adapter optional and interface-based, we avoid a hard
// dependency on any specific DOM implementation. The caller supplies the glue.

/**
 * Visitor callback interface for DOM-mode sanitization.
 *
 * The sanitizer calls these methods for each node in the parsed DOM:
 *
 *   element(): called for each element node. Return a new Map of safe attributes
 *              to keep, or `false` to drop the element entirely.
 *
 *   comment(): called for each comment node. Return the comment text to keep
 *              (possibly rewritten), or `false` to drop the comment.
 */
export interface DomVisitor {
  /**
   * Called for each HTML element during the DOM walk.
   *
   * @param tagName     Lowercase tag name (e.g. "div", "script")
   * @param attributes  All attributes of the element as a Map<name, value>
   * @returns           A new Map of sanitized attributes to use, or false to drop the element
   */
  element(tagName: string, attributes: Map<string, string>): false | Map<string, string>;

  /**
   * Called for each HTML comment during the DOM walk.
   *
   * @param value  The comment text (without <!-- and -->)
   * @returns      The comment text to keep, or false to drop the comment
   */
  comment(value: string): false | string;
}

/**
 * Adapter interface for DOM-capable environments (browser, jsdom, etc.).
 *
 * Implement this interface to use the high-fidelity DOM-mode sanitizer path.
 * The adapter decouples the sanitizer from any specific DOM implementation.
 *
 * @example
 * // Browser implementation
 * const browserAdapter: HtmlSanitizerDomAdapter = {
 *   parse: (html) => {
 *     const parser = new DOMParser();
 *     return parser.parseFromString(html, "text/html");
 *   },
 *   walk: (dom, visitor) => {
 *     // ... walk the DOM tree calling visitor.element() / visitor.comment()
 *   },
 *   serialize: (dom) => {
 *     return (dom as Document).body.innerHTML;
 *   },
 * };
 */
export interface HtmlSanitizerDomAdapter {
  /** Parse an HTML string into a DOM representation. */
  parse(html: string): unknown;
  /** Walk the DOM, calling the visitor for each element and comment. */
  walk(dom: unknown, visitor: DomVisitor): void;
  /** Serialize the (mutated) DOM back to an HTML string. */
  serialize(dom: unknown): string;
}

// ─── HtmlSanitizationPolicy ───────────────────────────────────────────────────

/**
 * Policy that controls what the HTML string sanitizer removes from raw HTML.
 *
 * All fields are optional. Omitting a field uses the HTML_PASSTHROUGH default
 * (no sanitization). Use the named presets HTML_STRICT, HTML_RELAXED, or
 * HTML_PASSTHROUGH as starting points.
 *
 * Note: the `on*` attribute pattern (onclick, onload, etc.) is ALWAYS stripped
 * when any elements are being sanitized. This is not optional — event handler
 * injection is the most common XSS vector in HTML and must always be blocked.
 * The `dropAttributes` list adds to this default, not replaces it.
 */
export interface HtmlSanitizationPolicy {

  /**
   * HTML element names (lowercase) to remove entirely, including all their
   * content. This is a destructive drop — unlike the AST sanitizer's link
   * promotion, the element AND its children are removed.
   *
   * Risk catalog (spec §Dangerous Elements Removed by Default):
   *
   *   script   — direct JavaScript execution (highest risk)
   *   style    — CSS expression() attacks, data exfiltration
   *   iframe   — framing attacks, clickjacking
   *   object   — plugin execution (Flash, Java)
   *   embed    — same as object
   *   applet   — Java applet execution (legacy)
   *   form     — CSRF, credential phishing
   *   input    — data capture, autofill attacks
   *   meta     — redirect via http-equiv="refresh"
   *   base     — base URL hijacking (breaks all relative links)
   *   link     — CSS import, DNS prefetch exfiltration
   *   noscript — parser-context abuse in some browsers
   *
   * Default: all of the above (see HTML_STRICT preset)
   */
  readonly dropElements?: readonly string[];

  /**
   * Attribute names (lowercase) to strip from every element.
   *
   * The `on*` pattern (onclick, onmouseover, onload, etc.) is always stripped
   * regardless of this list. `srcdoc` and `formaction` are also stripped by
   * default when dropElements includes iframe/form.
   *
   * Add names here to strip additional attributes (e.g. "data-evil").
   *
   * Default: [] (only the always-stripped set applies)
   */
  readonly dropAttributes?: readonly string[];

  /**
   * Allowlist of URL schemes for href and src attributes.
   *
   * When a URL in href or src has a scheme not in this list, the attribute
   * value is replaced with "" (making the link/image inert).
   *
   * Default: ["http", "https", "mailto", "ftp"]
   * Set to null to allow any scheme (not recommended for untrusted content).
   */
  readonly allowedUrlSchemes?: readonly string[] | null;

  /**
   * If true, HTML comments (<!-- … -->) are stripped from the output.
   *
   * Comments can be used for:
   *   - IE conditional comments: <!--[if IE]><script>…</script><![endif]-->
   *   - Content injection that survives comment stripping in some parsers
   *
   * Default: true (strip comments for maximum safety)
   */
  readonly dropComments?: boolean;

  /**
   * If true, style attributes containing CSS expression() or url() with
   * non-http/https URLs are stripped entirely.
   *
   * CSS expression(alert(1)) is IE's CSS execution mechanism. url() can be
   * used to exfiltrate data by loading an attacker-controlled resource.
   *
   * The full style attribute is removed rather than attempting to parse CSS —
   * CSS parsing is complex and error-prone, making partial fixes risky.
   *
   * Default: true
   */
  readonly sanitizeStyleAttributes?: boolean;

  /**
   * Optional DOM adapter for environments with a real HTML parser.
   *
   * When provided, the sanitizer uses the DOM path (parse → walk → serialize)
   * for higher-fidelity sanitization. When absent, regex/string operations
   * are used for portability.
   *
   * The DOM path is more accurate because it handles:
   *   - Malformed HTML that the regex approach might miss
   *   - Nested elements
   *   - Attribute injection via character encoding tricks
   */
  readonly domAdapter?: HtmlSanitizerDomAdapter;
}

// ─── Named Presets ─────────────────────────────────────────────────────────────

/**
 * HTML_STRICT — untrusted HTML from external sources.
 *
 * Removes all executable elements. Strips event handlers. Sanitizes URLs.
 * Drops comments. Strips dangerous CSS. Use for HTML from third-party
 * systems, user-submitted content, or any source you do not control.
 */
export const HTML_STRICT: HtmlSanitizationPolicy = {
  dropElements: [
    "script", "style", "iframe", "object", "embed", "applet",
    "form", "input", "button", "select", "textarea",
    "noscript", "meta", "link", "base",
  ],
  dropAttributes: [], // on* attributes stripped by default in sanitizer logic
  allowedUrlSchemes: ["http", "https", "mailto"],
  dropComments: true,
  sanitizeStyleAttributes: true,
};

/**
 * HTML_RELAXED — authenticated users / internal tools.
 *
 * Removes high-risk executable elements (script, iframe, object, embed, applet)
 * but allows form elements and style elements. Comments are preserved.
 * URL sanitization still applies. Use for authenticated internal tools where
 * some HTML richness is desired.
 */
export const HTML_RELAXED: HtmlSanitizationPolicy = {
  dropElements: ["script", "iframe", "object", "embed", "applet"],
  dropAttributes: [],
  allowedUrlSchemes: ["http", "https", "mailto", "ftp"],
  dropComments: false,
  sanitizeStyleAttributes: true,
};

/**
 * HTML_PASSTHROUGH — no sanitization.
 *
 * Everything passes through unchanged. Use ONLY for fully trusted HTML
 * from developers / static content that has never been touched by users.
 */
export const HTML_PASSTHROUGH: HtmlSanitizationPolicy = {
  dropElements: [],
  dropAttributes: [],
  allowedUrlSchemes: null,
  dropComments: false,
  sanitizeStyleAttributes: false,
};
