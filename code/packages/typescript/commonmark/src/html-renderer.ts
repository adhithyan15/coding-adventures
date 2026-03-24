/**
 * HTML Renderer
 *
 * Converts a CommonMark AST (produced by the parser) into an HTML string.
 *
 * === Design ===
 *
 * The renderer is a simple recursive tree walk. Each node type maps to one
 * or more HTML elements. The mapping is defined by the CommonMark spec HTML
 * rendering rules (§Appendix C).
 *
 * Key decisions:
 *
 *   - HTML characters in text content and attribute values are escaped via
 *     `escapeHtml` (encodes `& < > "`). This prevents XSS.
 *   - HTML block and HTML inline nodes are passed through verbatim — the
 *     spec says raw HTML is not re-encoded.
 *   - Code block content is escaped but not processed for Markdown.
 *   - Tight lists suppress `<p>` tags around list item content.
 *   - Link `href` attributes use the URL as-is (already normalised by parser).
 *   - ATX/setext headings produce `<h1>` through `<h6>`.
 *
 * === Tight vs Loose Lists ===
 *
 * A tight list is one where no list items are separated by blank lines and
 * no item contains a blank line internally. In tight lists, the `<p>` tag
 * around paragraph content is omitted:
 *
 *   Tight:   <li>item text</li>
 *   Loose:   <li><p>item text</p></li>
 *
 * The `tight` flag on ListNode controls this. When rendering list items,
 * we pass the flag down and strip `<p>` wrappers from ParagraphNode children.
 *
 * @module html-renderer
 */

import type {
  DocumentNode, BlockNode, InlineNode,
  HeadingNode, ParagraphNode, CodeBlockNode, BlockquoteNode,
  ListNode, ListItemNode, ThematicBreakNode, HtmlBlockNode,
  LinkDefinitionNode, TextNode, EmphasisNode, StrongNode,
  CodeSpanNode, LinkNode, ImageNode, AutolinkNode, HtmlInlineNode,
  HardBreakNode, SoftBreakNode,
} from "./types.js";
import { escapeHtml } from "./entities.js";
import { normalizeUrl } from "./scanner.js";

// ─── Public Entry Point ────────────────────────────────────────────────────────

/**
 * Render a CommonMark AST to an HTML string.
 *
 * @param document  The root document node from the parser.
 * @returns         An HTML string representing the document.
 *
 * @example
 * ```typescript
 * const { parse } = await import("@coding-adventures/commonmark");
 * const ast = parse("# Hello\n\nWorld\n");
 * const html = toHtml(ast);
 * // html = "<h1>Hello</h1>\n<p>World</p>\n"
 * ```
 */
export function toHtml(document: DocumentNode): string {
  return renderBlocks(document.children, false);
}

// ─── Block Rendering ──────────────────────────────────────────────────────────

/**
 * Render a sequence of block nodes to HTML.
 *
 * @param blocks  The block nodes to render.
 * @param tight   Whether this is inside a tight list (suppresses `<p>` tags).
 */
function renderBlocks(blocks: readonly BlockNode[], tight: boolean): string {
  return blocks.map(b => renderBlock(b, tight)).join("");
}

function renderBlock(block: BlockNode, tight: boolean): string {
  switch (block.type) {
    case "document":
      return renderBlocks(block.children, false);

    case "heading":
      return renderHeading(block);

    case "paragraph":
      return renderParagraph(block, tight);

    case "code_block":
      return renderCodeBlock(block);

    case "blockquote":
      return renderBlockquote(block);

    case "list":
      return renderList(block);

    case "list_item":
      // ListItemNode is rendered by renderList; direct call uses non-tight
      return renderListItem(block, false);

    case "thematic_break":
      return "<hr />\n";

    case "html_block":
      // Raw HTML passes through verbatim, no escaping
      return block.value;

    case "link_definition":
      // Link definitions are not rendered — they are consumed during parsing
      return "";

    default:
      return "";
  }
}

// ─── Block Node Renderers ─────────────────────────────────────────────────────

/**
 * Render an ATX or setext heading.
 *
 * Input:  `# Hello world`  (HeadingNode with level=1)
 * Output: `<h1>Hello world</h1>\n`
 */
function renderHeading(node: HeadingNode): string {
  const inner = renderInlines(node.children);
  return `<h${node.level}>${inner}</h${node.level}>\n`;
}

/**
 * Render a paragraph.
 *
 * In tight list context, the `<p>` wrapper is omitted and only the content
 * is emitted (followed by a newline).
 *
 * Input:  `Hello *world*`
 * Output: `<p>Hello <em>world</em></p>\n`
 */
function renderParagraph(node: ParagraphNode, tight: boolean): string {
  const inner = renderInlines(node.children);
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
 * Input:  ```typescript\nconst x = 1;\n```
 * Output: `<pre><code class="language-typescript">const x = 1;\n</code></pre>\n`
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
 * Input:  `> quoted text`
 * Output: `<blockquote>\n<p>quoted text</p>\n</blockquote>\n`
 */
function renderBlockquote(node: BlockquoteNode): string {
  const inner = renderBlocks(node.children, false);
  return `<blockquote>\n${inner}</blockquote>\n`;
}

/**
 * Render an ordered or unordered list.
 *
 * Ordered lists with a start number other than 1 get a `start` attribute.
 * The `tight` flag is passed to each list item so that paragraph `<p>` tags
 * are omitted in tight lists.
 *
 * Input (tight):   `- item1\n- item2`
 * Output:          `<ul>\n<li>item1</li>\n<li>item2</li>\n</ul>\n`
 *
 * Input (loose):   `- item1\n\n- item2`
 * Output:          `<ul>\n<li><p>item1</p>\n</li>\n<li><p>item2</p>\n</li>\n</ul>\n`
 */
function renderList(node: ListNode): string {
  const tag = node.ordered ? "ol" : "ul";
  const startAttr =
    node.ordered && node.start !== null && node.start !== 1
      ? ` start="${node.start}"`
      : "";
  const items = node.children
    .map(item => renderListItem(item, node.tight))
    .join("");
  return `<${tag}${startAttr}>\n${items}</${tag}>\n`;
}

/**
 * Render a single list item.
 *
 * Tight single-paragraph items: `<li>text</li>` (no `<p>` wrapper, no newlines).
 * All other items (multiple blocks, non-paragraph blocks): `<li>\ncontent\n</li>`.
 *
 * An empty item renders as `<li></li>`.
 */
function renderListItem(node: ListItemNode, tight: boolean): string {
  if (node.children.length === 0) {
    return `<li></li>\n`;
  }

  if (tight && node.children[0]?.type === "paragraph") {
    // Tight list: first paragraph is inlined (no <p> wrapper)
    const firstPara = node.children[0] as ParagraphNode;
    const firstContent = renderInlines(firstPara.children);
    if (node.children.length === 1) {
      // Only one child — simple tight item
      return `<li>${firstContent}</li>\n`;
    }
    // Multiple children: inline the first paragraph, then block-render the rest
    const rest = renderBlocks(node.children.slice(1), tight);
    return `<li>${firstContent}\n${rest}</li>\n`;
  }

  // Loose or non-paragraph first child: block-level format with newlines.
  // In tight mode, the last child (if a tight paragraph) renders without <p>
  // tags and without a trailing newline — it should be flush with </li>.
  const inner = renderBlocks(node.children, tight);
  const lastChild = node.children[node.children.length - 1];
  if (tight && lastChild?.type === "paragraph" && inner.endsWith("\n")) {
    // Strip the trailing \n from the tight paragraph so it is flush with </li>
    return `<li>\n${inner.slice(0, -1)}</li>\n`;
  }
  return `<li>\n${inner}</li>\n`;
}

// ─── Inline Rendering ─────────────────────────────────────────────────────────

function renderInlines(nodes: readonly InlineNode[]): string {
  return nodes.map(renderInline).join("");
}

function renderInline(node: InlineNode): string {
  switch (node.type) {
    case "text":
      // Escape HTML special chars in text content
      return escapeHtml(node.value);

    case "emphasis":
      return `<em>${renderInlines(node.children)}</em>`;

    case "strong":
      return `<strong>${renderInlines(node.children)}</strong>`;

    case "code_span":
      // Code span content is escaped but not Markdown-processed
      return `<code>${escapeHtml(node.value)}</code>`;

    case "link":
      return renderLink(node);

    case "image":
      return renderImage(node);

    case "autolink":
      return renderAutolink(node);

    case "html_inline":
      // Raw HTML inline — pass through verbatim
      return node.value;

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

/**
 * Render an inline link `[text](url "title")` or `[text][ref]`.
 *
 * The URL is HTML-escaped in the `href` attribute. The title (if present)
 * goes in a `title` attribute.
 */
function renderLink(node: LinkNode): string {
  const href = escapeHtml(node.destination);
  const titleAttr = node.title !== null
    ? ` title="${escapeHtml(node.title)}"`
    : "";
  const inner = renderInlines(node.children);
  return `<a href="${href}"${titleAttr}>${inner}</a>`;
}

/**
 * Render an inline image `![alt](url "title")`.
 *
 * The `alt` attribute uses the pre-computed plain-text alt value. All
 * markup has already been stripped by the parser. We just HTML-escape it.
 */
function renderImage(node: ImageNode): string {
  const src   = escapeHtml(node.destination);
  const alt   = escapeHtml(node.alt);
  const titleAttr = node.title !== null
    ? ` title="${escapeHtml(node.title)}"`
    : "";
  return `<img src="${src}" alt="${alt}"${titleAttr} />`;
}

/**
 * Render an autolink `<url>` or `<email>`.
 *
 * For email autolinks, the destination gets a `mailto:` prefix.
 * The link text is the raw address (HTML-escaped).
 */
function renderAutolink(node: AutolinkNode): string {
  const href = node.isEmail
    ? `mailto:${escapeHtml(node.destination)}`
    : escapeHtml(normalizeUrl(node.destination));
  const text = escapeHtml(node.destination);
  return `<a href="${href}">${text}</a>`;
}
