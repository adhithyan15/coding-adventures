/**
 * preview.ts — GFM markdown rendering pipeline.
 *
 * Wraps the three-step pipeline from the @coding-adventures packages:
 *
 *   1. parse()     — GFM markdown string → DocumentNode AST
 *   2. sanitize()  — AST → sanitized AST (RELAXED policy)
 *   3. toHtml()    — sanitized AST → HTML string
 *
 * === Why RELAXED sanitization? ===
 *
 * Journal entries are the user's own content — not untrusted user-generated
 * input from the internet. RELAXED allows HTML pass-through (for embedding
 * raw HTML snippets), permits common URL schemes (http, https, mailto, ftp),
 * and leaves images and headings unrestricted.
 *
 * If this app ever accepts content from other users (shared journals,
 * imported entries from untrusted sources), switch to STRICT.
 */

import { parse, toHtml } from "@coding-adventures/gfm";
import { sanitize, RELAXED } from "@coding-adventures/document-ast-sanitizer";

/**
 * renderPreview — convert a GFM markdown string to an HTML string.
 *
 * Returns an empty string for empty input (avoids unnecessary AST work).
 */
export function renderPreview(markdown: string): string {
  if (!markdown) return "";
  const ast = parse(markdown);
  const sanitized = sanitize(ast, RELAXED);
  return toHtml(sanitized);
}
