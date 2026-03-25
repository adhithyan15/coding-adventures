// index.d.ts -- TypeScript type definitions for @coding-adventures/commonmark-native
// ==================================================================================
//
// These type definitions describe the native functions exposed by the Rust addon.
// The actual implementation is in src/lib.rs; these types exist so TypeScript
// consumers get full IntelliSense and type checking.

/**
 * Convert CommonMark Markdown to HTML.
 *
 * Raw HTML blocks in the Markdown are passed through unchanged — this is
 * required for full CommonMark 0.31.2 spec compliance.
 *
 * **Security warning**: Raw HTML passthrough means that if you pass untrusted
 * user-supplied Markdown, attackers can inject arbitrary HTML (including
 * `<script>` tags). Use {@link markdownToHtmlSafe} for user content.
 *
 * @param markdown - A string of CommonMark Markdown.
 * @returns The rendered HTML string.
 *
 * @example
 * ```typescript
 * import { markdownToHtml } from "@coding-adventures/commonmark-native";
 *
 * markdownToHtml("# Hello\n\nWorld\n");
 * // → "<h1>Hello</h1>\n<p>World</p>\n"
 *
 * markdownToHtml("Hello **world** and *em*\n");
 * // → "<p>Hello <strong>world</strong> and <em>em</em></p>\n"
 *
 * markdownToHtml("<div>raw</div>\n\nparagraph\n");
 * // → "<div>raw</div>\n<p>paragraph</p>\n"
 * ```
 */
export function markdownToHtml(markdown: string): string;

/**
 * Convert CommonMark Markdown to HTML, stripping all raw HTML.
 *
 * This is the safe variant for **untrusted user-supplied Markdown** (comments,
 * forum posts, chat messages, wiki edits). All `RawBlockNode` and
 * `RawInlineNode` values are dropped before rendering — attackers cannot
 * inject `<script>` tags or other HTML through Markdown.
 *
 * All standard Markdown syntax (headings, emphasis, links, code blocks, lists,
 * blockquotes, etc.) is still rendered correctly.
 *
 * @param markdown - A string of CommonMark Markdown (possibly untrusted).
 * @returns The rendered HTML string with all raw HTML stripped.
 *
 * @example
 * ```typescript
 * import { markdownToHtmlSafe } from "@coding-adventures/commonmark-native";
 *
 * // Script tag is stripped — only the bold text remains
 * markdownToHtmlSafe("<script>alert(1)</script>\n\n**bold**\n");
 * // → "<p><strong>bold</strong></p>\n"
 *
 * // Regular Markdown works normally
 * markdownToHtmlSafe("# Hello\n\nWorld\n");
 * // → "<h1>Hello</h1>\n<p>World</p>\n"
 * ```
 */
export function markdownToHtmlSafe(markdown: string): string;
