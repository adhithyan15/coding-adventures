/**
 * @coding-adventures/document-ast-sanitizer
 *
 * Policy-driven AST sanitizer for the Document AST (TE00).
 *
 * This package slots between the parser and the renderer:
 *
 *   parse(markdown)          ← TE01 — CommonMark Parser
 *         ↓
 *   sanitize(doc, policy)    ← THIS PACKAGE (TE02)
 *         ↓
 *   toHtml(doc)              ← TE00 — document-ast-to-html
 *
 * === Quick Start ===
 *
 * ```typescript
 * import { sanitize, STRICT } from "@coding-adventures/document-ast-sanitizer";
 *
 * // User-generated content — use STRICT
 * const safe = sanitize(parse(userMarkdown), STRICT);
 * const html = toHtml(safe);
 *
 * // Custom policy — allow HTML blocks but restrict headings
 * const html = toHtml(sanitize(parse(editorMarkdown), {
 *   ...RELAXED,
 *   minHeadingLevel: 2,
 *   allowedUrlSchemes: ["http", "https"],
 * }));
 * ```
 *
 * === Design ===
 *
 * The sanitizer is **pure and immutable** — it never mutates the input document.
 * Policies are plain data objects (composable via spread, JSON-serializable).
 * Every node type in the Document AST is handled explicitly — unknown node
 * types are never silently passed through.
 *
 * Spec: TE02 — Document Sanitization
 *
 * @module index
 */

// Re-export the public API

export { sanitize } from "./sanitizer.js";
export type { SanitizationPolicy } from "./policy.js";
export { STRICT, RELAXED, PASSTHROUGH } from "./policy.js";

// URL utilities are also exported for callers who want to reuse the scheme
// extraction logic in their own pipeline stages.
export { stripControlChars, extractScheme, isSchemeAllowed } from "./url-utils.js";
