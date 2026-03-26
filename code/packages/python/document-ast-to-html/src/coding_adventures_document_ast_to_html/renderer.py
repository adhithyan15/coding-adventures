"""Document AST → HTML Renderer.

Converts a Document AST (produced by any front-end parser) into an HTML string.
The renderer is a simple recursive tree walk — each node type maps to HTML elements
following the CommonMark spec HTML rendering rules.

=== Security ===

- Text content and attribute values are HTML-escaped via escape_html().
- RawBlockNode and RawInlineNode content is passed through verbatim when
  format == "html" — this is intentional and spec-required.
- Link and image URLs are sanitized to block dangerous schemes:
  javascript:, vbscript:, data:, blob:.
"""

from __future__ import annotations

import re
from dataclasses import dataclass

from coding_adventures_document_ast import (
    AutolinkNode,
    BlockNode,
    BlockquoteNode,
    CodeBlockNode,
    DocumentNode,
    HeadingNode,
    ImageNode,
    InlineNode,
    LinkNode,
    ListItemNode,
    ListNode,
    ParagraphNode,
    RawBlockNode,
    RawInlineNode,
    StrikethroughNode,
    TableAlignment,
    TableCellNode,
    TableNode,
    TableRowNode,
    TaskItemNode,
)

_URL_SAFE = frozenset(
    "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz"
    "0123456789-._~:/?#@!$&'()*+,;=%"
)

_HEX_RE = re.compile(r"[0-9A-Fa-f]{2}")


def escape_html(value: str | None) -> str:
    if value is None:
        return ""
    return (
        value.replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
        .replace('"', "&quot;")
    )


def normalize_url(url: str) -> str:
    result: list[str] = []
    index = 0
    while index < len(url):
        ch = url[index]
        if ch in _URL_SAFE or (ch == "%" and index + 2 < len(url) and _HEX_RE.match(url, index + 1)):
            result.append(ch)
        else:
            result.extend(f"%{byte:02X}" for byte in ch.encode("utf-8"))
        index += 1
    return "".join(result)

# ─── Render Options ───────────────────────────────────────────────────────────


@dataclass
class RenderOptions:
    """Options for to_html().

    sanitize: When True, all RawBlockNode and RawInlineNode nodes are dropped
    from the output (their value is not emitted, regardless of format).

    **You MUST set sanitize=True when rendering untrusted Markdown** (e.g.
    user-supplied content in a web application). Raw HTML passthrough is a
    CommonMark spec requirement and is enabled by default, but it means an
    attacker who can write Markdown can inject arbitrary HTML into the output.

    Default: False (raw HTML passes through verbatim — spec-compliant).
    """

    sanitize: bool = False


# ─── Public Entry Point ────────────────────────────────────────────────────────


def to_html(document: DocumentNode, options: RenderOptions | None = None) -> str:
    """Render a Document AST to an HTML string.

    The input is a DocumentNode as produced by any front-end parser that
    implements the Document AST spec (TE00). The output is a valid HTML fragment.

    ⚠️  **Security notice**: Raw HTML passthrough is enabled by default
    (required for CommonMark spec compliance). If you render **untrusted**
    Markdown (user content, third-party data), pass RenderOptions(sanitize=True)
    to strip all raw HTML from the output. Without this, an attacker who controls
    the Markdown source can inject arbitrary HTML into the rendered page.

    Example:
        # Trusted Markdown (documentation, static content):
        html = to_html(parse("# Hello\\n\\nWorld\\n"))

        # Untrusted Markdown (user-supplied content):
        html = to_html(parse(user_markdown), RenderOptions(sanitize=True))
    """
    if options is None:
        options = RenderOptions()
    return _render_blocks(document["children"], tight=False, options=options)


# ─── Block Rendering ──────────────────────────────────────────────────────────


def _render_blocks(blocks: list[BlockNode], tight: bool, options: RenderOptions) -> str:
    """Render a sequence of block nodes to HTML."""
    return "".join(_render_block(b, tight=tight, options=options) for b in blocks)


def _render_block(block: BlockNode, tight: bool, options: RenderOptions) -> str:
    """Dispatch a block node to its specific renderer."""
    block_type = block["type"]

    if block_type == "document":
        return _render_blocks(block["children"], tight=False, options=options)
    elif block_type == "heading":
        return _render_heading(block, options=options)
    elif block_type == "paragraph":
        return _render_paragraph(block, tight=tight, options=options)
    elif block_type == "code_block":
        return _render_code_block(block)
    elif block_type == "blockquote":
        return _render_blockquote(block, options=options)
    elif block_type == "list":
        return _render_list(block, options=options)
    elif block_type == "list_item":
        # ListItemNode is rendered by render_list; direct call uses non-tight
        return _render_list_item(block, tight=False, options=options)
    elif block_type == "task_item":
        return _render_task_item(block, tight=False, options=options)
    elif block_type == "thematic_break":
        return "<hr />\n"
    elif block_type == "raw_block":
        return _render_raw_block(block, options=options)
    elif block_type == "table":
        return _render_table(block, options=options)
    else:
        return ""


# ─── Block Node Renderers ─────────────────────────────────────────────────────


def _render_heading(node: HeadingNode, options: RenderOptions) -> str:
    """Render an ATX or setext heading.

    Example:
        HeadingNode { level: 1, children: [TextNode { value: "Hello" }] }
        → <h1>Hello</h1>\\n
    """
    inner = _render_inlines(node["children"], options=options)
    level = node["level"]
    return f"<h{level}>{inner}</h{level}>\n"


def _render_paragraph(node: ParagraphNode, tight: bool, options: RenderOptions) -> str:
    """Render a paragraph.

    In tight list context, the <p> wrapper is omitted and only the inner
    content is emitted (followed by a newline).

    Example:
        ParagraphNode → <p>Hello <em>world</em></p>\\n
        ParagraphNode (tight) → Hello <em>world</em>\\n
    """
    inner = _render_inlines(node["children"], options=options)
    if tight:
        return inner + "\n"
    return f"<p>{inner}</p>\n"


def _render_code_block(node: CodeBlockNode) -> str:
    """Render a fenced or indented code block.

    The content is HTML-escaped but not Markdown-processed.
    If the block has a language (info string), the <code> tag gets a
    class="language-<lang>" attribute per CommonMark convention.

    Example:
        CodeBlockNode { language: "ts", value: "const x = 1;\\n" }
        → <pre><code class="language-ts">const x = 1;\\n</code></pre>\\n
    """
    escaped = escape_html(node["value"])
    lang = node["language"]
    if lang:
        return f'<pre><code class="language-{escape_html(lang)}">{escaped}</code></pre>\n'
    return f"<pre><code>{escaped}</code></pre>\n"


def _render_blockquote(node: BlockquoteNode, options: RenderOptions) -> str:
    """Render a blockquote.

    Example:
        BlockquoteNode → <blockquote>\\n<p>…</p>\\n</blockquote>\\n
    """
    inner = _render_blocks(node["children"], tight=False, options=options)
    return f"<blockquote>\n{inner}</blockquote>\n"


def _render_list(node: ListNode, options: RenderOptions) -> str:
    """Render an ordered or unordered list.

    Ordered lists with a start number other than 1 get a start attribute.
    The tight flag is passed to each list item so <p> tags are omitted.

    The start attribute is only emitted when node.start is a safe integer
    to prevent attribute injection.

    Example:
        ListNode { ordered: False, tight: True }
        → <ul>\\n<li>item1</li>\\n<li>item2</li>\\n</ul>\\n

        ListNode { ordered: True, start: 3, tight: False }
        → <ol start="3">\\n<li><p>item1</p>\\n</li>\\n</ol>\\n
    """
    tag = "ol" if node["ordered"] else "ul"
    start = node.get("start")
    # Guard: only emit start when it is a valid integer and not 1
    start_attr = ""
    if node["ordered"] and start is not None and start != 1 and isinstance(start, int):
        start_attr = f' start="{start}"'
    items = "".join(_render_list_child(item, tight=node["tight"], options=options) for item in node["children"])
    return f"<{tag}{start_attr}>\n{items}</{tag}>\n"


def _render_list_child(node: ListItemNode | TaskItemNode, tight: bool, options: RenderOptions) -> str:
    if node["type"] == "task_item":
        return _render_task_item(node, tight=tight, options=options)
    return _render_list_item(node, tight=tight, options=options)


def _render_list_item(node: ListItemNode, tight: bool, options: RenderOptions) -> str:
    """Render a single list item.

    Tight single-paragraph items: <li>text</li> (no <p> wrapper).
    All other items (multiple blocks, non-paragraph first child):
      <li>\\ncontent\\n</li>.

    An empty item renders as <li></li>.
    """
    if not node["children"]:
        return "<li></li>\n"

    first_child = node["children"][0]

    if tight and first_child["type"] == "paragraph":
        # Tight list: first paragraph is inlined (no <p> wrapper)
        first_content = _render_inlines(first_child["children"], options=options)
        if len(node["children"]) == 1:
            # Only one child — simple tight item
            return f"<li>{first_content}</li>\n"
        # Multiple children: inline the first paragraph, then block-render the rest
        rest = _render_blocks(node["children"][1:], tight=tight, options=options)
        return f"<li>{first_content}\n{rest}</li>\n"

    # Loose or non-paragraph first child: block-level format with newlines.
    inner = _render_blocks(node["children"], tight=tight, options=options)
    last_child = node["children"][-1] if node["children"] else None
    if tight and last_child and last_child["type"] == "paragraph" and inner.endswith("\n"):
        # Strip trailing \n so it is flush with </li>
        return f"<li>\n{inner[:-1]}</li>\n"
    return f"<li>\n{inner}</li>\n"


def _render_task_item(node: TaskItemNode, tight: bool, options: RenderOptions) -> str:
    checkbox = '<input type="checkbox" disabled="" checked="" />' if node["checked"] else '<input type="checkbox" disabled="" />'

    if not node["children"]:
        return f"<li>{checkbox}</li>\n"

    first_child = node["children"][0]
    if tight and first_child["type"] == "paragraph":
        first_content = _render_inlines(first_child["children"], options=options)
        content = checkbox if not first_content else f"{checkbox} {first_content}"
        if len(node["children"]) == 1:
            return f"<li>{content}</li>\n"
        rest = _render_blocks(node["children"][1:], tight=tight, options=options)
        return f"<li>{content}\n{rest}</li>\n"

    inner = _render_blocks(node["children"], tight=tight, options=options)
    return f"<li>{checkbox}\n{inner}</li>\n"


def _render_raw_block(node: RawBlockNode, options: RenderOptions) -> str:
    """Render a raw block node.

    If options.sanitize is True, this node is always skipped.
    Otherwise, if format == "html", emit the raw value verbatim (not escaped).
    Skip silently for any other format.

    Example:
        RawBlockNode { format: "html", value: "<div>raw</div>\\n" }
        → <div>raw</div>\\n                (sanitize: False — default)
        → (empty string)                  (sanitize: True)

        RawBlockNode { format: "latex", value: "\\\\textbf{x}\\n" }
        → (empty string)                  (always skipped — wrong format)
    """
    if options.sanitize:
        return ""
    if node["format"] == "html":
        return node["value"]
    return ""


def _render_table(node: TableNode, options: RenderOptions) -> str:
    header = None
    body_rows = []
    for row in node["children"]:
        if row["isHeader"] and header is None:
            header = row
        else:
            body_rows.append(row)

    parts = ["<table>\n"]
    if header is not None:
        parts.append("<thead>\n")
        parts.append(_render_table_row(header, node["align"], options=options))
        parts.append("</thead>\n")
    if body_rows:
        parts.append("<tbody>\n")
        parts.extend(_render_table_row(row, node["align"], options=options) for row in body_rows)
        parts.append("</tbody>\n")
    parts.append("</table>\n")
    return "".join(parts)


def _render_table_row(
    node: TableRowNode,
    alignments: list[TableAlignment],
    options: RenderOptions,
) -> str:
    cells = []
    for index, cell in enumerate(node["children"]):
        align = alignments[index] if index < len(alignments) else None
        cells.append(_render_table_cell(cell, node["isHeader"], align, options=options))
    return "<tr>\n" + "".join(cells) + "</tr>\n"


def _render_table_cell(
    node: TableCellNode,
    header: bool,
    align: TableAlignment,
    options: RenderOptions,
) -> str:
    tag = "th" if header else "td"
    align_attr = "" if align is None else f' align="{align}"'
    return f"<{tag}{align_attr}>{_render_inlines(node['children'], options=options)}</{tag}>\n"


# ─── Inline Rendering ─────────────────────────────────────────────────────────


def _render_inlines(nodes: list[InlineNode], options: RenderOptions) -> str:
    """Render a sequence of inline nodes to HTML."""
    return "".join(_render_inline(n, options=options) for n in nodes)


def _render_inline(node: InlineNode, options: RenderOptions) -> str:
    """Dispatch an inline node to its specific renderer."""
    node_type = node["type"]

    if node_type == "text":
        return escape_html(node["value"])
    elif node_type == "emphasis":
        return f"<em>{_render_inlines(node['children'], options=options)}</em>"
    elif node_type == "strong":
        return f"<strong>{_render_inlines(node['children'], options=options)}</strong>"
    elif node_type == "strikethrough":
        return f"<del>{_render_inlines(node['children'], options=options)}</del>"
    elif node_type == "code_span":
        return f"<code>{escape_html(node['value'])}</code>"
    elif node_type == "link":
        return _render_link(node, options=options)
    elif node_type == "image":
        return _render_image(node)
    elif node_type == "autolink":
        return _render_autolink(node)
    elif node_type == "raw_inline":
        return _render_raw_inline(node, options=options)
    elif node_type == "hard_break":
        return "<br />\n"
    elif node_type == "soft_break":
        # CommonMark spec §6.12: a soft line break renders as a newline,
        # which browsers collapse to a space. We emit "\n" per the spec.
        return "\n"
    else:
        return ""


# ─── Inline Node Renderers ────────────────────────────────────────────────────


def _render_raw_inline(node: RawInlineNode, options: RenderOptions) -> str:
    """Render a raw inline node.

    If options.sanitize is True, this node is always skipped.
    Otherwise, if format == "html", emit the raw value verbatim.
    Skip silently for any other format.
    """
    if options.sanitize:
        return ""
    if node["format"] == "html":
        return node["value"]
    return ""


# ─── URL Sanitization ─────────────────────────────────────────────────────────
#
# CommonMark spec §C.3 intentionally leaves URL sanitization to the implementor.
# Without scheme filtering, user-controlled Markdown is vulnerable to XSS via
# javascript: and data: URIs — both are valid URL characters that
# HTML-escaping does not neutralize.
#
# The spec explicitly allows arbitrary URL schemes in autolinks (spec examples
# use irc://, made-up://, localhost:5001/, etc.), so we cannot use a
# safe-scheme allowlist. Instead we use a targeted blocklist of schemes that
# are execution-capable in browsers:
#
#   javascript:  — executes JS in the browser's origin (universal risk)
#   vbscript:    — executes VBScript (IE legacy, still blocked by practice)
#   data:        — can embed scripts as data:text/html or data:text/javascript
#   blob:        — same-origin blob URLs can execute scripts

_DANGEROUS_SCHEME = re.compile(r"^(?:javascript|vbscript|data|blob):", re.IGNORECASE)

# Characters stripped before scheme detection. These are code points that
# WHATWG URL parsers and some browsers silently ignore when parsing a URL
# scheme, which can allow bypasses like "java\rscript:" == "javascript:".
_URL_CONTROL_CHARS = re.compile(
    r"[\u0000-\u001F\u007F-\u009F\u200B-\u200D\u2060\uFEFF]",
)


def _sanitize_url(url: str) -> str:
    """Sanitize a URL by stripping control characters and blocking dangerous schemes.

    Returns the sanitized URL, or "" if the URL uses an execution-capable
    scheme (javascript:, vbscript:, data:, blob:).

    Control characters are stripped because browsers silently remove them
    before scheme detection, allowing "java\\rscript:" == "javascript:".
    """
    stripped = _URL_CONTROL_CHARS.sub("", url)
    if _DANGEROUS_SCHEME.match(stripped):
        return ""
    return stripped


def _render_link(node: LinkNode, options: RenderOptions) -> str:
    """Render an inline link [text](url "title") or resolved reference link.

    The URL is sanitized (blocks dangerous schemes) and HTML-escaped in the
    href attribute. The title (if present) goes in a title attribute.

    Example:
        LinkNode { destination: "https://x.com", title: "X", children: […] }
        → <a href="https://x.com" title="X">…</a>
    """
    href = escape_html(_sanitize_url(node["destination"]))
    title = node.get("title")
    title_attr = f' title="{escape_html(title)}"' if title is not None else ""
    inner = _render_inlines(node["children"], options=options)
    return f'<a href="{href}"{title_attr}>{inner}</a>'


def _render_image(node: ImageNode) -> str:
    """Render an inline image ![alt](url "title").

    The alt attribute uses the pre-computed plain-text value (markup already
    stripped by the parser).

    Example:
        ImageNode { destination: "cat.png", alt: "a cat", title: None }
        → <img src="cat.png" alt="a cat" />
    """
    src = escape_html(_sanitize_url(node["destination"]))
    alt = escape_html(node["alt"])
    title = node.get("title")
    title_attr = f' title="{escape_html(title)}"' if title is not None else ""
    return f'<img src="{src}" alt="{alt}"{title_attr} />'


def _render_autolink(node: AutolinkNode) -> str:
    """Render an autolink <url> or <email>.

    For email autolinks, the href gets a mailto: prefix.
    The link text is the raw address (HTML-escaped).

    sanitize_url is applied to both URL and email destinations — email
    autolinks are not exempt from URL sanitization.

    Example:
        AutolinkNode { destination: "user@example.com", is_email: True }
        → <a href="mailto:user@example.com">user@example.com</a>

        AutolinkNode { destination: "https://example.com", is_email: False }
        → <a href="https://example.com">https://example.com</a>
    """
    dest = _sanitize_url(node["destination"])
    if node["is_email"]:
        href = f"mailto:{escape_html(dest)}"
    else:
        href = escape_html(_sanitize_url(normalize_url(dest)))
    text = escape_html(node["destination"])
    return f'<a href="{href}">{text}</a>'
