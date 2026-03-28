/**
 * Document AST → HTML Renderer
 *
 * Converts a Document AST (produced by any front-end parser) into an HTML
 * string. The renderer is a simple recursive tree walk — each node type maps
 * to HTML elements following the CommonMark spec HTML rendering rules
 * (§Appendix C) for nodes that originate from Markdown, and sensible defaults
 * for nodes from other front-ends.
 *
 * === Node mapping ===
 *
 *   DocumentNode      → rendered children
 *   HeadingNode       → <h1>…</h1> through <h6>…</h6>
 *   ParagraphNode     → <p>…</p>  (omitted in tight list context)
 *   CodeBlockNode     → <pre><code [class="language-X"]>…</code></pre>
 *   BlockquoteNode    → <blockquote>\n…</blockquote>
 *   ListNode          → <ul> or <ol [start="N"]>
 *   ListItemNode      → <li>…</li>
 *   ThematicBreakNode → <hr />
 *   RawBlockNode      → verbatim if format="html", skipped otherwise
 *
 *   TextNode          → HTML-escaped text
 *   EmphasisNode      → <em>…</em>
 *   StrongNode        → <strong>…</strong>
 *   CodeSpanNode      → <code>…</code>
 *   LinkNode          → <a href="…" [title="…"]>…</a>
 *   ImageNode         → <img src="…" alt="…" [title="…"] />
 *   AutolinkNode      → <a href="[mailto:]…">…</a>
 *   RawInlineNode     → verbatim if format="html", skipped otherwise
 *   HardBreakNode     → <br />\n
 *   SoftBreakNode     → \n
 *
 * === Tight vs Loose Lists ===
 *
 * A tight list suppresses `<p>` tags around paragraph content in list items:
 *
 *   Tight:   <li>item text</li>
 *   Loose:   <li><p>item text</p></li>
 *
 * The `tight` flag on `ListNode` controls this.
 *
 * === Security ===
 *
 * - Text content and attribute values are HTML-escaped via `escapeHtml`.
 * - `RawBlockNode` and `RawInlineNode` content is passed through verbatim when
 *   `format === "html"` — this is intentional and spec-required.
 * - Link and image URLs are sanitized to block dangerous schemes:
 *   `javascript:`, `vbscript:`, `data:`, `blob:`.
 *
 * @module html-renderer
 */

import type {
  DocumentNode, BlockNode, InlineNode,
  HeadingNode, ParagraphNode, CodeBlockNode, BlockquoteNode,
  ListNode, ListItemNode, TaskItemNode, RawBlockNode,
  TableNode, TableRowNode, TableCellNode,
  TextNode,
  LinkNode, ImageNode, AutolinkNode, RawInlineNode,
} from "@coding-adventures/document-ast";
import { escapeHtml } from "./entities.js";
import { normalizeUrl } from "./scanner.js";

// ─── Public Entry Point ────────────────────────────────────────────────────────

// ─── Render Options ───────────────────────────────────────────────────────────

/**
 * Options for `toHtml()`.
 */
export interface RenderOptions {
  /**
   * When `true`, all `RawBlockNode` and `RawInlineNode` nodes are **dropped**
   * from the output (their `value` is not emitted, regardless of `format`).
   *
   * **You MUST set `sanitize: true` when rendering untrusted Markdown** (e.g.
   * user-supplied content in a web application). Raw HTML passthrough is a
   * CommonMark spec requirement and is enabled by default, but it means an
   * attacker who can write Markdown can inject arbitrary HTML — including
   * `<script>` tags — into the rendered output.
   *
   * Default: `false` (raw HTML passes through verbatim — spec-compliant).
   *
   * @deprecated Use `@coding-adventures/document-ast-sanitizer` instead.
   *
   * Before: `toHtml(doc, { sanitize: true })`
   * After:  `toHtml(sanitize(doc, STRICT))` where `sanitize` and `STRICT`
   *         are imported from `@coding-adventures/document-ast-sanitizer`.
   *
   * The boolean flag is too coarse — it cannot express policies like
   * "allow HTML raw blocks but not LaTeX", "strip images but keep links",
   * or "clamp headings to level 3". The dedicated sanitizer package supports
   * all of these via a `SanitizationPolicy` object. See spec TE02.
   *
   * This option will be removed in v1.0.0.
   *
   * @example
   * ```typescript
   * // Before (deprecated):
   * const html = toHtml(parse(userInput), { sanitize: true });
   *
   * // After (recommended):
   * import { sanitize, STRICT } from "@coding-adventures/document-ast-sanitizer";
   * const html = toHtml(sanitize(parse(userInput), STRICT));
   * ```
   */
  readonly sanitize?: boolean;
}

/**
 * Render a Document AST to an HTML string.
 *
 * The input is a `DocumentNode` as produced by any front-end parser that
 * implements the Document AST spec (TE00). The output is a valid HTML fragment.
 *
 * ⚠️  **Security notice**: Raw HTML passthrough is enabled by default
 * (required for CommonMark spec compliance). If you render **untrusted**
 * Markdown (user content, third-party data), pass `{ sanitize: true }` to
 * strip all raw HTML from the output. Without this, an attacker who controls
 * the Markdown source can inject arbitrary HTML into the rendered page.
 *
 * @param document  The root document node.
 * @param options   Render options. Pass `{ sanitize: true }` for untrusted input.
 * @returns         An HTML string representing the document.
 *
 * @example
 * ```typescript
 * // Trusted Markdown (documentation, static content):
 * const html = toHtml(parse("# Hello\n\nWorld\n"));
 *
 * // Untrusted Markdown (user-supplied content):
 * const html = toHtml(parse(userMarkdown), { sanitize: true });
 * ```
 */
export function toHtml(document: DocumentNode, options: RenderOptions = {}): string {
  return renderBlocks(document.children, false, options);
}

// ─── Block Rendering ──────────────────────────────────────────────────────────

/**
 * Render a sequence of block nodes to HTML.
 *
 * @param blocks    The block nodes to render.
 * @param tight     Whether this is inside a tight list (suppresses `<p>` tags).
 * @param options   Render options (passed down the recursion).
 */
function renderBlocks(blocks: readonly BlockNode[], tight: boolean, options: RenderOptions): string {
  return blocks.map(b => renderBlock(b, tight, options)).join("");
}

function renderBlock(block: BlockNode, tight: boolean, options: RenderOptions): string {
  switch (block.type) {
    case "document":
      return renderBlocks(block.children, false, options);

    case "heading":
      return renderHeading(block, options);

    case "paragraph":
      return renderParagraph(block, tight, options);

    case "code_block":
      return renderCodeBlock(block);

    case "blockquote":
      return renderBlockquote(block, options);

    case "list":
      return renderList(block, options);

    case "list_item":
      // ListItemNode is rendered by renderList; direct call uses non-tight
      return renderListItem(block, false, options);

    case "task_item":
      return renderTaskItem(block, false, options);

    case "thematic_break":
      return "<hr />\n";

    case "raw_block":
      return renderRawBlock(block, options);

    case "table":
      return renderTable(block, options);

    case "table_row":
      return renderTableRow(block, options);

    case "table_cell":
      return renderTableCell(block, false, null, options);

    default:
      return "";
  }
}

// ─── Block Node Renderers ─────────────────────────────────────────────────────

/**
 * Render an ATX or setext heading.
 *
 * ```
 * HeadingNode { level: 1, children: [TextNode { value: "Hello" }] }
 * → <h1>Hello</h1>\n
 * ```
 */
function renderHeading(node: HeadingNode, options: RenderOptions): string {
  const inner = renderInlines(node.children, options);
  return `<h${node.level}>${inner}</h${node.level}>\n`;
}

/**
 * Render a paragraph.
 *
 * In tight list context, the `<p>` wrapper is omitted and only the inner
 * content is emitted (followed by a newline).
 *
 * ```
 * ParagraphNode → <p>Hello <em>world</em></p>\n
 * ParagraphNode (tight) → Hello <em>world</em>\n
 * ```
 */
function renderParagraph(node: ParagraphNode, tight: boolean, options: RenderOptions): string {
  const inner = renderInlines(node.children, options);
  if (tight) return inner + "\n";
  return `<p>${inner}</p>\n`;
}

/**
 * Render a fenced or indented code block.
 *
 * The content is HTML-escaped but not Markdown-processed.
 * If the block has a language (info string), the `<code>` tag gets a
 * `class="language-<lang>"` attribute per CommonMark convention.
 *
 * ```
 * CodeBlockNode { language: "ts", value: "const x = 1;\n" }
 * → <pre><code class="language-ts">const x = 1;\n</code></pre>\n
 * ```
 */
function renderCodeBlock(node: CodeBlockNode): string {
  const escaped = escapeHtml(node.value);
  if (node.language) {
    return `<pre><code class="language-${escapeHtml(node.language)}">${escaped}</code></pre>\n`;
  }
  return `<pre><code>${escaped}</code></pre>\n`;
}

/**
 * Render a blockquote.
 *
 * ```
 * BlockquoteNode → <blockquote>\n<p>…</p>\n</blockquote>\n
 * ```
 */
function renderBlockquote(node: BlockquoteNode, options: RenderOptions): string {
  const inner = renderBlocks(node.children, false, options);
  return `<blockquote>\n${inner}</blockquote>\n`;
}

/**
 * Render an ordered or unordered list.
 *
 * Ordered lists with a start number other than 1 get a `start` attribute.
 * The `tight` flag is passed to each list item so `<p>` tags are omitted.
 *
 * The `start` attribute is only emitted when `node.start` is a safe integer
 * to prevent attribute injection from programmatically constructed nodes.
 *
 * ```
 * ListNode { ordered: false, tight: true }
 * → <ul>\n<li>item1</li>\n<li>item2</li>\n</ul>\n
 *
 * ListNode { ordered: true, start: 3, tight: false }
 * → <ol start="3">\n<li><p>item1</p>\n</li>\n</ol>\n
 * ```
 */
function renderList(node: ListNode, options: RenderOptions): string {
  const tag = node.ordered ? "ol" : "ul";
  // Guard: only emit `start` when it is a valid safe integer. Floating-point,
  // NaN, or Infinity values would produce invalid HTML attribute values.
  const startAttr =
    node.ordered && node.start !== null && node.start !== 1 &&
    Number.isSafeInteger(node.start)
      ? ` start="${node.start}"`
      : "";
  const items = node.children
    .map(item => item.type === "task_item"
      ? renderTaskItem(item, node.tight, options)
      : renderListItem(item, node.tight, options))
    .join("");
  return `<${tag}${startAttr}>\n${items}</${tag}>\n`;
}

/**
 * Render a single list item.
 *
 * Tight single-paragraph items: `<li>text</li>` (no `<p>` wrapper).
 * All other items (multiple blocks, non-paragraph first child):
 *   `<li>\ncontent\n</li>`.
 *
 * An empty item renders as `<li></li>`.
 */
function renderListItem(node: ListItemNode, tight: boolean, options: RenderOptions): string {
  if (node.children.length === 0) {
    return `<li></li>\n`;
  }

  if (tight && node.children[0]?.type === "paragraph") {
    // Tight list: first paragraph is inlined (no <p> wrapper)
    const firstPara = node.children[0] as ParagraphNode;
    const firstContent = renderInlines(firstPara.children, options);
    if (node.children.length === 1) {
      // Only one child — simple tight item
      return `<li>${firstContent}</li>\n`;
    }
    // Multiple children: inline the first paragraph, then block-render the rest
    const rest = renderBlocks(node.children.slice(1), tight, options);
    return `<li>${firstContent}\n${rest}</li>\n`;
  }

  // Loose or non-paragraph first child: block-level format with newlines.
  const inner = renderBlocks(node.children, tight, options);
  const lastChild = node.children[node.children.length - 1];
  if (tight && lastChild?.type === "paragraph" && inner.endsWith("\n")) {
    // Strip trailing \n so it is flush with </li>
    return `<li>\n${inner.slice(0, -1)}</li>\n`;
  }
  return `<li>\n${inner}</li>\n`;
}

function renderTaskItem(node: TaskItemNode, tight: boolean, options: RenderOptions): string {
  const checkbox = node.checked
    ? '<input type="checkbox" disabled="" checked="" /> '
    : '<input type="checkbox" disabled="" /> ';

  if (node.children.length === 0) {
    return `<li>${checkbox}</li>\n`;
  }

  if (tight && node.children[0]?.type === "paragraph") {
    const firstPara = node.children[0] as ParagraphNode;
    const firstContent = renderInlines(firstPara.children, options);
    if (node.children.length === 1) {
      return `<li>${checkbox}${firstContent}</li>\n`;
    }
    const rest = renderBlocks(node.children.slice(1), tight, options);
    return `<li>${checkbox}${firstContent}\n${rest}</li>\n`;
  }

  const inner = renderBlocks(node.children, tight, options);
  return `<li>${checkbox}\n${inner}</li>\n`;
}

/**
 * Render a raw block node.
 *
 * If `options.sanitize` is `true`, this node is **always skipped** —
 * raw HTML must not appear in sanitized output.
 *
 * Otherwise, if `format === "html"`, emit the raw value verbatim (not escaped).
 * Skip silently for any other format.
 *
 * ```
 * RawBlockNode { format: "html", value: "<div>raw</div>\n" }
 * → <div>raw</div>\n                (sanitize: false — default)
 * → (empty string)                  (sanitize: true)
 *
 * RawBlockNode { format: "latex", value: "\\textbf{x}\n" }
 * → (empty string)                  (always skipped — wrong format)
 * ```
 */
function renderRawBlock(node: RawBlockNode, options: RenderOptions): string {
  if (options.sanitize) return "";
  if (node.format === "html") return node.value;
  return "";
}

function renderTable(node: TableNode, options: RenderOptions): string {
  const headerRows = node.children.filter((row) => row.isHeader);
  const bodyRows = node.children.filter((row) => !row.isHeader);
  const thead = headerRows.length > 0
    ? `<thead>\n${headerRows.map((row) => renderTableRow(row, options, node.align)).join("")}</thead>\n`
    : "";
  const tbody = bodyRows.length > 0
    ? `<tbody>\n${bodyRows.map((row) => renderTableRow(row, options, node.align)).join("")}</tbody>\n`
    : "";
  return `<table>\n${thead}${tbody}</table>\n`;
}

function renderTableRow(
  node: TableRowNode,
  options: RenderOptions,
  alignments: readonly ("left" | "right" | "center" | null)[] = [],
): string {
  const cells = node.children
    .map((cell, index) => renderTableCell(cell, node.isHeader, alignments[index] ?? null, options))
    .join("");
  return `<tr>\n${cells}</tr>\n`;
}

function renderTableCell(
  node: TableCellNode,
  isHeader: boolean,
  alignment: "left" | "right" | "center" | null,
  options: RenderOptions,
): string {
  const tag = isHeader ? "th" : "td";
  const alignAttr = alignment === null ? "" : ` align="${alignment}"`;
  return `<${tag}${alignAttr}>${renderInlines(node.children, options)}</${tag}>\n`;
}

// ─── Inline Rendering ─────────────────────────────────────────────────────────

function renderInlines(nodes: readonly InlineNode[], options: RenderOptions): string {
  return nodes.map(n => renderInline(n, options)).join("");
}

function renderInline(node: InlineNode, options: RenderOptions): string {
  switch (node.type) {
    case "text":
      // Route through renderText for a single canonical escaping path
      return renderText(node);

    case "emphasis":
      return `<em>${renderInlines(node.children, options)}</em>`;

    case "strong":
      return `<strong>${renderInlines(node.children, options)}</strong>`;

    case "strikethrough":
      return `<del>${renderInlines(node.children, options)}</del>`;

    case "code_span":
      // Code span content is escaped but not Markdown-processed
      return `<code>${escapeHtml(node.value)}</code>`;

    case "link":
      return renderLink(node, options);

    case "image":
      return renderImage(node);

    case "autolink":
      return renderAutolink(node);

    case "raw_inline":
      return renderRawInline(node, options);

    case "hard_break":
      return "<br />\n";

    case "soft_break":
      // CommonMark spec §6.12: a soft line break renders as a newline,
      // which browsers collapse to a space. We emit "\n" per the spec.
      return "\n";

    default:
      return "";
  }
}

// ─── Inline Node Renderers ────────────────────────────────────────────────────

/**
 * Render plain text — HTML-escape special characters to prevent XSS.
 *
 * The four characters with HTML significance in text content are encoded:
 *   &  → &amp;
 *   <  → &lt;
 *   >  → &gt;
 *   "  → &quot;
 */
function renderText(node: TextNode): string {
  return escapeHtml(node.value);
}

/**
 * Render a raw inline node.
 *
 * If `options.sanitize` is `true`, this node is always skipped.
 * Otherwise, if `format === "html"`, emit the raw value verbatim (not escaped).
 * Skip silently for any other format.
 *
 * ```
 * RawInlineNode { format: "html", value: "<em>hi</em>" }
 * → <em>hi</em>            (sanitize: false — default)
 * → (empty string)         (sanitize: true)
 *
 * RawInlineNode { format: "latex", value: "\\emph{x}" }
 * → (empty string)         (always skipped — wrong format)
 * ```
 */
function renderRawInline(node: RawInlineNode, options: RenderOptions): string {
  if (options.sanitize) return "";
  if (node.format === "html") return node.value;
  return "";
}

// ─── URL Sanitization ─────────────────────────────────────────────────────────
//
// CommonMark spec §C.3 intentionally leaves URL sanitization to the implementor.
// Without scheme filtering, user-controlled Markdown is vulnerable to XSS via
// `javascript:` and `data:` URIs — both are valid URL characters that
// HTML-escaping does not neutralize.
//
// The spec explicitly allows arbitrary URL schemes in autolinks (spec examples
// 596, 598, 599, 601 use irc://, made-up://, localhost:5001/, etc.), so we
// cannot use a safe-scheme allowlist. Instead we use a targeted blocklist of
// the schemes that are execution-capable in browsers:
//
//   javascript:  — executes JS in the browser's origin (universal risk)
//   vbscript:    — executes VBScript (IE legacy, still blocked by practice)
//   data:        — can embed scripts as data:text/html or data:text/javascript
//
// All other schemes (irc:, ftp:, mailto:, made-up:, etc.) pass through unchanged.
// Relative URLs (no scheme) always pass through unchanged.

// Block schemes that can execute code in the browser. `blob:` is included
// because a same-origin blob URL (`blob:https://origin/uuid`) constructed via
// `URL.createObjectURL(new Blob(["<script>…"], {type:"text/html"}))` can
// execute scripts when followed.
const DANGEROUS_SCHEME = /^(?:javascript|vbscript|data|blob):/i;

// Characters stripped before scheme detection. These are code points that
// WHATWG URL parsers and some browsers silently ignore when parsing a URL
// scheme, which can allow bypasses like "java\rscript:" == "javascript:".
//
// Stripped:
//   U+0000–U+001F   C0 controls (includes TAB \t, LF \n, CR \r, etc.)
//   U+007F–U+009F   DEL + C1 controls (historically stripped by some parsers)
//   U+200B          ZERO WIDTH SPACE
//   U+200C          ZERO WIDTH NON-JOINER
//   U+200D          ZERO WIDTH JOINER
//   U+2060          WORD JOINER
//   U+FEFF          BOM / ZERO WIDTH NO-BREAK SPACE
//
// NOT stripped: U+0020 SPACE. Literal spaces are invalid in link destinations
// per CommonMark (the parser never produces them), but hand-built DocumentNode
// values from other front-ends may have percent-unencoded spaces in relative
// URLs. Stripping them would silently corrupt those destinations. The caller
// (normalizeUrl) percent-encodes spaces when necessary.
const URL_CONTROL_CHARS = /[\u0000-\u001F\u007F-\u009F\u200B-\u200D\u2060\uFEFF]/gu;

/**
 * Sanitize a URL by stripping control characters and blocking dangerous schemes.
 *
 * Returns the sanitized URL, or `""` if the URL uses an execution-capable
 * scheme (`javascript:`, `vbscript:`, `data:`, `blob:`).
 *
 * **What is stripped:**
 *   - C0 controls (U+0000–U+001F): browsers silently remove these before scheme
 *     detection, so `"java\rscript:"` == `"javascript:"` in WHATWG parsers.
 *   - C1 controls + DEL (U+007F–U+009F): historically stripped by some parsers.
 *   - Zero-width / invisible characters (U+200B, U+200C, U+200D, U+2060, U+FEFF).
 *
 * **NOT stripped:** U+0020 SPACE. Literal spaces in relative URLs from
 * hand-built AST nodes are preserved; `normalizeUrl` percent-encodes them
 * when needed.
 *
 * **Returns the stripped string** (not the original), so scrubbed characters
 * cannot appear in `href` or `src` attributes.
 *
 * **Relative URLs** (no scheme) are always returned after control-char stripping.
 */
function sanitizeUrl(url: string): string {
  const stripped = url.replace(URL_CONTROL_CHARS, "");
  if (DANGEROUS_SCHEME.test(stripped)) {
    return "";
  }
  return stripped;
}

/**
 * Render an inline link `[text](url "title")` or resolved reference link.
 *
 * The URL is sanitized (blocks dangerous schemes) and HTML-escaped in the
 * `href` attribute. The title (if present) goes in a `title` attribute.
 *
 * ```
 * LinkNode { destination: "https://x.com", title: "X", children: […] }
 * → <a href="https://x.com" title="X">…</a>
 * ```
 */
function renderLink(node: LinkNode, options: RenderOptions): string {
  const href = escapeHtml(sanitizeUrl(node.destination));
  const titleAttr = node.title !== null
    ? ` title="${escapeHtml(node.title)}"`
    : "";
  const inner = renderInlines(node.children, options);
  return `<a href="${href}"${titleAttr}>${inner}</a>`;
}

/**
 * Render an inline image `![alt](url "title")`.
 *
 * The `alt` attribute uses the pre-computed plain-text value (markup already
 * stripped by the parser).
 *
 * ```
 * ImageNode { destination: "cat.png", alt: "a cat", title: null }
 * → <img src="cat.png" alt="a cat" />
 * ```
 */
function renderImage(node: ImageNode): string {
  const src = escapeHtml(sanitizeUrl(node.destination));
  const alt = escapeHtml(node.alt);
  const titleAttr = node.title !== null
    ? ` title="${escapeHtml(node.title)}"`
    : "";
  return `<img src="${src}" alt="${alt}"${titleAttr} />`;
}

/**
 * Render an autolink `<url>` or `<email>`.
 *
 * For email autolinks, the `href` gets a `mailto:` prefix.
 * The link text is the raw address (HTML-escaped).
 *
 * `sanitizeUrl` is applied to **both** URL and email destinations — email
 * autolinks are not exempt from URL sanitization. In particular, a crafted
 * email destination could embed control characters or a javascript: scheme
 * before the `@`-sign.
 *
 * ```
 * AutolinkNode { destination: "user@example.com", isEmail: true }
 * → <a href="mailto:user@example.com">user@example.com</a>
 *
 * AutolinkNode { destination: "https://example.com", isEmail: false }
 * → <a href="https://example.com">https://example.com</a>
 * ```
 */
function renderAutolink(node: AutolinkNode): string {
  // Sanitize both URL and email destinations. Email autolinks are not exempt:
  // control characters and dangerous schemes must be blocked regardless of
  // the isEmail flag.
  const dest = sanitizeUrl(node.destination);
  const href = node.isEmail
    ? `mailto:${escapeHtml(dest)}`
    : escapeHtml(sanitizeUrl(normalizeUrl(dest)));
  const text = escapeHtml(node.destination);
  return `<a href="${href}">${text}</a>`;
}
