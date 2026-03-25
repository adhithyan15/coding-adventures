/**
 * Sanitization Policy — Types and Named Presets
 *
 * A SanitizationPolicy is a plain data object that tells the sanitizer what
 * to keep, transform, or drop from a Document AST. Policies are composable
 * via object spread:
 *
 *   { ...STRICT, maxHeadingLevel: 3 }
 *
 * Because policies are plain data (no methods, no closures), they are:
 *   - JSON-serializable
 *   - Easy to port to other languages (Go, Rust, Python, etc.)
 *   - Safely comparable by value
 *
 * Spec: TE02 — Document Sanitization
 *
 * @module policy
 */

// ─── SanitizationPolicy ────────────────────────────────────────────────────────

/**
 * Policy that controls what the AST sanitizer keeps, transforms, or drops.
 *
 * All fields are optional. Omitting a field uses the PASSTHROUGH default
 * (keep everything). Use the named presets STRICT, RELAXED, or PASSTHROUGH
 * as starting points and spread-override specific fields.
 *
 * Design principle (Decision 2 from spec): policies are plain data objects.
 * This makes them composable, JSON-serializable, and trivially portable to
 * every language in the multi-language suite.
 */
export interface SanitizationPolicy {

  // ─── Raw node handling ──────────────────────────────────────────────────

  /**
   * Controls which RawBlockNode formats are allowed through.
   *
   * The three modes form a deliberate tradeoff:
   *
   *   "drop-all"    — safest; every raw block is discarded regardless of
   *                   format. Use this for user-generated content where raw
   *                   HTML passthrough would allow XSS.
   *
   *   "passthrough" — no filtering; every raw block passes unchanged.
   *                   Use for fully trusted content (documentation).
   *
   *   string[]      — allowlist of format tags to keep. All others are
   *                   dropped. "Allow HTML but not LaTeX" is a real policy.
   *
   * Default (when omitted): "passthrough"
   *
   * @example
   * allowRawBlockFormats: ["html"]      // keep HTML raw blocks, drop LaTeX
   * allowRawBlockFormats: "drop-all"    // drop everything (user content)
   */
  readonly allowRawBlockFormats?: "drop-all" | "passthrough" | readonly string[];

  /**
   * Controls which RawInlineNode formats are allowed through.
   * Same semantics as allowRawBlockFormats but for inline raw spans.
   *
   * Default (when omitted): "passthrough"
   */
  readonly allowRawInlineFormats?: "drop-all" | "passthrough" | readonly string[];

  // ─── URL scheme policy ──────────────────────────────────────────────────

  /**
   * Allowlist of URL schemes permitted in LinkNode.destination,
   * ImageNode.destination, and AutolinkNode.destination.
   *
   * URL scheme detection:
   *   1. Strip C0 controls + zero-width chars (bypass vectors like java\x00script:)
   *   2. Extract everything before the first ":"
   *   3. Lowercase and check against this list
   *   4. Relative URLs (no ":" before "/" or "?") always pass through
   *
   * When a scheme is disallowed:
   *   - LinkNode / ImageNode: destination is set to "" (link/image rendered inert)
   *   - AutolinkNode: node is dropped entirely (no text to promote)
   *
   * Default (when omitted): ["http", "https", "mailto", "ftp"]
   * Set to `null` to allow any scheme (trusted content only).
   */
  readonly allowedUrlSchemes?: readonly string[] | null;

  // ─── Node type policy ───────────────────────────────────────────────────

  /**
   * If true, all LinkNode instances are dropped.
   *
   * Unlike "drop everything", this is a *promotion*: the link's text children
   * are lifted into the parent container as plain inline nodes. This preserves
   * the visible text ("click here") while removing the hyperlink capability.
   *
   * Example:
   *   LinkNode { destination: "…", children: [TextNode("click here")] }
   *   → TextNode("click here")   (parent now contains the text directly)
   *
   * Default: false (links kept)
   */
  readonly dropLinks?: boolean;

  /**
   * If true, all ImageNode instances are dropped entirely.
   *
   * Note: dropImages takes precedence over transformImageToText. When both
   * are true, the image is dropped (not converted to text).
   *
   * Default: false (images kept)
   */
  readonly dropImages?: boolean;

  /**
   * If true, ImageNode instances are replaced by a TextNode containing their
   * alt text. This gives a plain-text fallback without silencing image refs.
   *
   * Example:
   *   ImageNode { alt: "a cat", destination: "cat.png" }
   *   → TextNode { value: "a cat" }
   *
   * Default: false
   */
  readonly transformImageToText?: boolean;

  /**
   * Maximum heading level allowed. Headings deeper than this are clamped down.
   *
   * Clamping semantics:
   *   - HeadingNode { level: 5 } with maxHeadingLevel: 3 → level clamped to 3
   *   - "drop" removes all HeadingNode instances entirely
   *
   * Use case: blog comment systems that disallow h1 and h2, or embedding
   * user content in a section that already owns h1–h3.
   *
   * Default (when omitted): 6 (no clamping)
   */
  readonly maxHeadingLevel?: 1 | 2 | 3 | 4 | 5 | 6 | "drop";

  /**
   * Minimum heading level allowed. Headings shallower than this are clamped up.
   *
   * Example: minHeadingLevel: 2 prevents user content from emitting an h1,
   * which is typically reserved for the page title in most page templates.
   *
   *   HeadingNode { level: 1 } → level clamped to 2
   *
   * Default (when omitted): 1 (no clamping)
   */
  readonly minHeadingLevel?: 1 | 2 | 3 | 4 | 5 | 6;

  /**
   * If true, BlockquoteNode instances are dropped (children NOT promoted).
   * The entire blockquote and its content disappear from the output.
   *
   * Default: false
   */
  readonly dropBlockquotes?: boolean;

  /**
   * If true, CodeBlockNode instances are dropped.
   * Default: false
   */
  readonly dropCodeBlocks?: boolean;

  /**
   * If true, CodeSpanNode instances are converted to plain TextNode instances
   * containing the same value. The code formatting is lost but the text is kept.
   *
   * Use case: plain-text renderers or contexts where code formatting is
   * inappropriate (SMS, email subject lines).
   *
   * Default: false
   */
  readonly transformCodeSpanToText?: boolean;
}

// ─── Named Presets ─────────────────────────────────────────────────────────────
//
// Three canonical policies cover the vast majority of real-world use cases.
// All are exported as `const` (not `as const`) to allow spread-override:
//
//   const myPolicy = { ...STRICT, maxHeadingLevel: 3 };
//
// The presets differ along a single axis: trust level.
//
//   STRICT      → unknown authors, public comments, user-generated content
//   RELAXED     → authenticated users, internal wikis, CMS editors
//   PASSTHROUGH → fully trusted authors, documentation, static sites

/**
 * STRICT — for user-generated content (comments, forum posts, chat messages).
 *
 * Drops all raw HTML/format passthrough. Allows only http, https, mailto URLs.
 * Images are converted to alt text (not dropped) so image references in user
 * content degrade gracefully to text descriptions rather than disappearing.
 * Links are kept but URL-sanitized. Headings are clamped to h2–h6 (h1 is
 * reserved for the page title in most page templates).
 *
 * Attack vectors blocked:
 *   - <script> injection via RawBlockNode/RawInlineNode
 *   - javascript: / data: / vbscript: URL injection in links and images
 *   - Page structure manipulation via h1 headings
 */
export const STRICT: SanitizationPolicy = {
  allowRawBlockFormats: "drop-all",
  allowRawInlineFormats: "drop-all",
  allowedUrlSchemes: ["http", "https", "mailto"],
  dropImages: false,
  transformImageToText: true,
  minHeadingLevel: 2,
  maxHeadingLevel: 6,
  dropLinks: false,
  dropBlockquotes: false,
  dropCodeBlocks: false,
  transformCodeSpanToText: false,
};

/**
 * RELAXED — for semi-trusted content (authenticated users, internal wikis).
 *
 * Allows HTML raw blocks (for rich embedded content) but not other formats.
 * Allows http, https, mailto, ftp URLs. Images pass through unchanged.
 * Headings unrestricted. Links unrestricted beyond URL scheme check.
 *
 * This is appropriate when:
 *   - Users are authenticated and accountable
 *   - The content is not shown to untrusted third parties
 *   - You want to allow embedded HTML widgets or diagrams
 */
export const RELAXED: SanitizationPolicy = {
  allowRawBlockFormats: ["html"],
  allowRawInlineFormats: ["html"],
  allowedUrlSchemes: ["http", "https", "mailto", "ftp"],
  dropImages: false,
  transformImageToText: false,
  minHeadingLevel: 1,
  maxHeadingLevel: 6,
  dropLinks: false,
  dropBlockquotes: false,
  dropCodeBlocks: false,
  transformCodeSpanToText: false,
};

/**
 * PASSTHROUGH — for fully trusted content (documentation, static sites).
 *
 * No sanitization. Everything passes through unchanged.
 * Equivalent to not calling sanitize() at all.
 *
 * Use ONLY when the Markdown source is authored by trusted developers,
 * checked into version control, and not editable by end users.
 */
export const PASSTHROUGH: SanitizationPolicy = {
  allowRawBlockFormats: "passthrough",
  allowRawInlineFormats: "passthrough",
  allowedUrlSchemes: null,
  dropImages: false,
  transformImageToText: false,
  minHeadingLevel: 1,
  maxHeadingLevel: 6,
  dropLinks: false,
  dropBlockquotes: false,
  dropCodeBlocks: false,
  transformCodeSpanToText: false,
};
