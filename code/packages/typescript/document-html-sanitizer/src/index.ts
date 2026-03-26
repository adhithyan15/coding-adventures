/**
 * @coding-adventures/document-html-sanitizer
 *
 * Pattern-based HTML string sanitizer. String in, string out.
 *
 * This package has NO dependency on @coding-adventures/document-ast.
 * It operates on opaque HTML strings and can be used:
 *
 *   - As a safety net after rendering: sanitizeHtml(toHtml(parse(md)), HTML_STRICT)
 *   - On HTML from external sources: sanitizeHtml(cmsApiResponse.body, HTML_STRICT)
 *   - On user-pasted HTML from rich-text editors
 *
 * === Quick Start ===
 *
 * ```typescript
 * import { sanitizeHtml, HTML_STRICT } from "@coding-adventures/document-html-sanitizer";
 *
 * // Strip scripts, event handlers, and dangerous URLs from any HTML string
 * const safe = sanitizeHtml(rawHtml, HTML_STRICT);
 *
 * // Two-stage pipeline (belt and suspenders)
 * const safeHtml = sanitizeHtml(
 *   toHtml(sanitize(parse(userMarkdown), STRICT)),
 *   HTML_STRICT
 * );
 * ```
 *
 * === Design ===
 *
 * Default mode: regex/string operations (portable to all target languages).
 * DOM mode: supply a `domAdapter` in the policy for higher-fidelity parsing.
 *
 * Spec: TE02 — Document Sanitization §Stage 2
 *
 * @module index
 */

// Public API
export { sanitizeHtml } from "./html-sanitizer.js";

// Policy types and presets
export type { HtmlSanitizationPolicy, HtmlSanitizerDomAdapter, DomVisitor } from "./policy.js";
export { HTML_STRICT, HTML_RELAXED, HTML_PASSTHROUGH } from "./policy.js";

// URL utilities (exported for callers who want to reuse the scheme check)
export { stripControlChars, extractScheme, isSchemeAllowed } from "./url-utils.js";
