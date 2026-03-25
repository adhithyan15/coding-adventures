/**
 * @coding-adventures/document-ast-to-html
 *
 * Renders a Document AST (from @coding-adventures/document-ast) to an HTML string.
 *
 * This is one back-end in the Document AST ecosystem. It accepts any
 * `DocumentNode` produced by any front-end parser (Markdown, RST, HTML, …)
 * and emits a valid HTML fragment.
 *
 * === Quick Start ===
 *
 * ```typescript
 * import { parse }  from "@coding-adventures/commonmark-parser";
 * import { toHtml } from "@coding-adventures/document-ast-to-html";
 *
 * const html = toHtml(parse("# Hello\n\nWorld\n"));
 * // → "<h1>Hello</h1>\n<p>World</p>\n"
 * ```
 *
 * === Consuming a Document AST directly ===
 *
 * ```typescript
 * import type { DocumentNode } from "@coding-adventures/document-ast";
 * import { toHtml } from "@coding-adventures/document-ast-to-html";
 *
 * // Build a document programmatically
 * const doc: DocumentNode = {
 *   type: "document",
 *   children: [
 *     {
 *       type: "heading",
 *       level: 1,
 *       children: [{ type: "text", value: "Hello" }],
 *     },
 *   ],
 * };
 *
 * toHtml(doc);
 * // → "<h1>Hello</h1>\n"
 * ```
 *
 * @module index
 */

export { toHtml } from "./html-renderer.js";
export type { RenderOptions } from "./html-renderer.js";
