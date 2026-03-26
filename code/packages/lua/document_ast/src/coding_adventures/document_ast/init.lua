-- document_ast — Format-Agnostic Intermediate Representation for Documents
-- =========================================================================
--
-- The Document AST is the "LLVM IR of documents" — a stable, typed tree
-- that every front-end parser produces and every back-end renderer consumes.
-- With a shared IR, N front-ends × M back-ends requires only N + M
-- implementations instead of N × M.
--
--   Markdown ─────────────────────────────────► HTML
--   reStructuredText ─────► Document AST ─────► PDF
--   HTML ──────────────────────────────────────► Plain text
--   DOCX ──────────────────────────────────────► DOCX
--
-- === Design Principles ===
--
--   1. Semantic, not notational — nodes carry meaning, not syntax
--   2. Resolved, not deferred   — all link references resolved before IR
--   3. Format-agnostic          — RawBlockNode/RawInlineNode carry a `format` tag
--   4. Immutable and typed      — tables with a `type` field for discrimination
--   5. Minimal and stable       — only universal document concepts
--
-- === Node Discrimination ===
--
-- Every node is a Lua table with a `type` field. Use a chain of if/elseif
-- comparisons (Lua has no switch statement) to dispatch on node type:
--
--   if node.type == "heading" then
--     -- use node.level, node.children
--   elseif node.type == "text" then
--     -- use node.value
--   end
--
-- === Key design decisions ===
--
-- **No `link_definition`** — links in the IR are always fully resolved.
-- Markdown's `[text][label]` reference syntax is resolved by the front-end;
-- the IR only ever contains `link { destination = "…" }`.
--
-- **`raw_block` / `raw_inline`** instead of `html_block` / `html_inline`.
-- A `format` field (`"html"`, `"latex"`, …) identifies the target back-end.
-- Renderers skip nodes with an unknown `format`.
--
-- Spec: TE00 — Document AST
--
-- @module coding_adventures.document_ast

local M = {}

-- ─── Block Node Constructors ──────────────────────────────────────────────────
--
-- Block nodes form the structural skeleton of a document. They live at the
-- top level of the document and can be nested (e.g. blockquotes, list items).

--- Create the root document node.
--
-- Every IR value is exactly one document node. An empty document has an empty
-- `children` array. The document node is the only node type that cannot appear
-- as a child of another node.
--
--   document
--     ├── heading (level 1)
--     ├── paragraph
--     └── list (ordered, tight)
--           ├── list_item
--           └── list_item
--
-- @param children  table — list of block nodes
-- @return table    { type="document", children={…} }
function M.document(children)
  return { type = "document", children = children or {} }
end

--- Create a heading node.
--
-- Semantically corresponds to `<h1>`–`<h6>` in HTML, `=====` / `-----`
-- underlines in RST, `\section{}` / `\subsection{}` in LaTeX.
-- Levels beyond 6 are clamped to 6.
--
--   heading { level=2, children={ text { value="Hello" } } }
--   → <h2>Hello</h2>
--
-- @param level     number — heading level 1–6
-- @param children  table  — list of inline nodes
-- @return table    { type="heading", level=N, children={…} }
function M.heading(level, children)
  return { type = "heading", level = level, children = children or {} }
end

--- Create a paragraph node.
--
-- A block of prose. Contains one or more inline nodes — text, emphasis,
-- links, and soft breaks between the original source lines.
--
--   paragraph { children={ text{value="Hello "}, emphasis{…} } }
--   → <p>Hello <em>world</em></p>
--
-- @param children  table — list of inline nodes
-- @return table    { type="paragraph", children={…} }
function M.paragraph(children)
  return { type = "paragraph", children = children or {} }
end

--- Create a code block node.
--
-- A block of literal code or pre-formatted text. The `value` is raw —
-- not decoded for HTML entities and not processed for inline markup.
-- The `value` field always ends with `\n`.
--
--   code_block { language="lua", value="local x = 1\n" }
--   → <pre><code class="language-lua">local x = 1\n</code></pre>
--
-- @param language  string|nil — syntax language hint, nil when unknown
-- @param value     string     — raw source code including trailing newline
-- @return table    { type="code_block", language=…, value=… }
function M.code_block(language, value)
  return { type = "code_block", language = language, value = value or "" }
end

--- Create a blockquote node.
--
-- A block of content set apart as a quotation or aside.
-- Can contain any block nodes, including nested blockquotes.
--
--   blockquote { children={ paragraph{…} } }
--   → <blockquote>\n<p>quote</p>\n</blockquote>
--
-- @param children  table — list of block nodes
-- @return table    { type="blockquote", children={…} }
function M.blockquote(children)
  return { type = "blockquote", children = children or {} }
end

--- Create a list node.
--
-- An ordered (numbered) or unordered (bulleted) list.
--
-- **Tight vs loose.** The `tight` flag is a rendering hint from the source.
-- A tight list is written without blank lines between items; a loose list
-- has blank lines. In HTML, tight lists suppress `<p>` wrappers around
-- paragraph content.
--
-- **Ordered list start.** `start` records the opening item number. 1 is the
-- default; 42 means the list begins at forty-two. nil for unordered.
--
--   list { ordered=false, start=nil, tight=true, children={…} }
--   → <ul>\n<li>item1</li>\n<li>item2</li>\n</ul>
--
-- @param ordered   boolean    — true for ordered (numbered), false for unordered
-- @param start     number|nil — opening number for ordered lists; nil for unordered
-- @param tight     boolean    — true if no blank lines between items
-- @param children  table      — list of list_item nodes
-- @return table    { type="list", ordered=…, start=…, tight=…, children={…} }
function M.list(ordered, start, tight, children)
  return { type = "list", ordered = ordered, start = start, tight = tight, children = children or {} }
end

--- Create a list item node.
--
-- One item in a list node. Contains block-level content.
--
-- @param children  table — list of block nodes
-- @return table    { type="list_item", children={…} }
function M.list_item(children)
  return { type = "list_item", children = children or {} }
end

--- Create a task list item node.
--
-- @param checked   boolean — whether the checkbox is checked
-- @param children  table   — list of block nodes
-- @return table    { type="task_item", checked=…, children={…} }
function M.task_item(checked, children)
  return { type = "task_item", checked = checked == true, children = children or {} }
end

--- Create a thematic break node.
--
-- A visual separator between sections. Leaf node — no children.
-- In HTML renders as `<hr />`. In RST `----`. In plain text `---`.
--
-- @return table  { type="thematic_break" }
function M.thematic_break()
  return { type = "thematic_break" }
end

--- Create a raw block node.
--
-- A block of raw content to be passed through verbatim to a specific back-end.
-- The `format` field identifies the target renderer (e.g. `"html"`, `"latex"`).
-- Back-ends that do not recognise `format` MUST skip this node silently.
--
-- This generalises `HtmlBlockNode`: `raw_block { format="html", value=… }` is
-- equivalent to the CommonMark html_block node.
--
--   Back-end contract:
--     format matches output → emit value verbatim (no escaping)
--     format does not match → skip silently
--
--   format     HTML back-end    LaTeX back-end    plain-text
--   ─────────  ─────────────    ──────────────    ──────────
--   "html"     emit             skip              skip
--   "latex"    skip             emit              skip
--
-- @param format  string — target back-end format tag
-- @param value   string — raw content
-- @return table  { type="raw_block", format=…, value=… }
function M.raw_block(format, value)
  return { type = "raw_block", format = format, value = value or "" }
end

--- Create a table node.
function M.table(align, children)
  return { type = "table", align = align or {}, children = children or {} }
end

--- Create a table row node.
function M.table_row(is_header, children)
  return { type = "table_row", is_header = is_header == true, children = children or {} }
end

--- Create a table cell node.
function M.table_cell(children)
  return { type = "table_cell", children = children or {} }
end

-- ─── Inline Node Constructors ─────────────────────────────────────────────────
--
-- Inline nodes live inside block nodes that contain prose content: headings,
-- paragraphs, and list items. They represent formatted text spans, links,
-- images, and structural characters within a paragraph.

--- Create a text node.
--
-- Plain text with no markup. All HTML character references (`&amp;`, `&#65;`,
-- `&#x41;`) are decoded into their Unicode equivalents before being stored.
-- The `value` field contains the final, display-ready Unicode string.
--
-- Adjacent text nodes are automatically merged during inline parsing — a
-- well-formed IR never has two consecutive text node siblings.
--
--   "Hello &amp; world" → text { value="Hello & world" }
--
-- @param value  string — decoded Unicode string, ready for display
-- @return table { type="text", value=… }
function M.text(value)
  return { type = "text", value = value or "" }
end

--- Create an emphasis node.
--
-- Stressed emphasis. In HTML renders as `<em>`. In Markdown, `*text*` or
-- `_text_`. In RST, `:emphasis:`. In DOCX, italic text.
--
--   emphasis { children={ text{value="hello"} } }
--   → <em>hello</em>
--
-- @param children  table — list of inline nodes
-- @return table    { type="emphasis", children={…} }
function M.emphasis(children)
  return { type = "emphasis", children = children or {} }
end

--- Create a strong node.
--
-- Strong importance. In HTML renders as `<strong>`. In Markdown, `**text**`
-- or `__text__`. In RST, `**bold**`. In DOCX, bold text.
--
--   strong { children={ text{value="bold"} } }
--   → <strong>bold</strong>
--
-- @param children  table — list of inline nodes
-- @return table    { type="strong", children={…} }
function M.strong(children)
  return { type = "strong", children = children or {} }
end

--- Create a strikethrough node.
function M.strikethrough(children)
  return { type = "strikethrough", children = children or {} }
end

--- Create a code span node.
--
-- Inline code. The value is raw — not decoded for HTML entities and not
-- processed for Markdown. Leading and trailing spaces are stripped when
-- the content is surrounded by spaces on both sides.
--
--   `` `const x = 1` `` → code_span { value="const x = 1" }
--   → <code>const x = 1</code>
--
-- @param value  string — raw code content, not decoded
-- @return table { type="code_span", value=… }
function M.code_span(value)
  return { type = "code_span", value = value or "" }
end

--- Create a link node.
--
-- A hyperlink with resolved destination. The `destination` is always a fully
-- resolved URL — all reference indirections have been resolved by the
-- front-end. The IR never contains unresolved reference links.
--
-- Links cannot be nested — a link node cannot contain another link node.
--
--   link { destination="https://example.com", title="Example",
--          children={ text{value="click here"} } }
--   → <a href="https://example.com" title="Example">click here</a>
--
-- @param destination  string — fully resolved URL
-- @param title        string|nil — optional tooltip/hover text
-- @param children     table — list of inline nodes
-- @return table       { type="link", destination=…, title=…, children={…} }
function M.link(destination, title, children)
  return { type = "link", destination = destination or "", title = title, children = children or {} }
end

--- Create an image node.
--
-- An embedded image. Like link, `destination` is always the fully resolved URL.
-- The `alt` field is the plain-text fallback description (all inline markup stripped).
--
-- Back-ends that cannot embed images (plain text, plain-text email) should
-- render the `alt` text instead.
--
--   image { destination="cat.png", alt="a cat", title=nil }
--   → <img src="cat.png" alt="a cat" />
--
-- @param destination  string — fully resolved image URL
-- @param title        string|nil — optional tooltip/hover text
-- @param alt          string — plain-text alt description, markup stripped
-- @return table       { type="image", destination=…, title=…, alt=… }
function M.image(destination, title, alt)
  return { type = "image", destination = destination or "", title = title, alt = alt or "" }
end

--- Create an autolink node.
--
-- A URL or email address presented as a direct link, without custom link text.
-- The link text in all back-ends is the raw address itself.
--
-- Why preserve `is_email`? Two reasons:
--   1. HTML back-ends need to prepend `mailto:` for email autolinks.
--   2. Other back-ends may format email addresses differently from URLs.
--
--   autolink { destination="user@example.com", is_email=true }
--   → <a href="mailto:user@example.com">user@example.com</a>
--
-- @param destination  string  — the URL or email address (without < >)
-- @param is_email     boolean — true for email autolinks
-- @return table       { type="autolink", destination=…, is_email=… }
function M.autolink(destination, is_email)
  return { type = "autolink", destination = destination or "", is_email = is_email or false }
end

--- Create a raw inline node.
--
-- An inline span of raw content to be passed through verbatim to a specific
-- back-end. The same back-end contract applies as for raw_block.
--
-- Generalises HtmlInlineNode: `raw_inline { format="html", value=… }` is
-- equivalent to the CommonMark html_inline node.
--
--   raw_inline { format="html", value="<em>raw</em>" }
--   → (HTML back-end) <em>raw</em>
--   → (LaTeX back-end) (nothing)
--
-- @param format  string — target back-end format tag
-- @param value   string — raw content
-- @return table  { type="raw_inline", format=…, value=… }
function M.raw_inline(format, value)
  return { type = "raw_inline", format = format, value = value or "" }
end

--- Create a hard break node.
--
-- A forced line break within a paragraph.
-- Forces `<br />` in HTML, `\newline` in LaTeX, a literal `\n` in plain-text.
-- In Markdown, produced by two or more trailing spaces before a newline, or
-- a backslash `\` immediately before a newline.
--
-- @return table  { type="hard_break" }
function M.hard_break()
  return { type = "hard_break" }
end

--- Create a soft break node.
--
-- A soft line break — a newline within a paragraph that is not a hard break.
-- In HTML, soft breaks render as `\n` (browsers collapse to a single space).
-- The IR preserves soft breaks so back-ends controlling line-wrapping can
-- make the right choice.
--
-- @return table  { type="soft_break" }
function M.soft_break()
  return { type = "soft_break" }
end

-- ─── Node Type Predicates ─────────────────────────────────────────────────────
--
-- Helper functions to test whether a node is a block or inline node type.
-- These mirror the TypeScript union types `BlockNode` and `InlineNode`.

--- The set of all block node type strings.
M.BLOCK_TYPES = {
  document = true,
  heading = true,
  paragraph = true,
  code_block = true,
  blockquote = true,
  list = true,
  list_item = true,
  task_item = true,
  thematic_break = true,
  raw_block = true,
  table = true,
  table_row = true,
  table_cell = true,
}

--- The set of all inline node type strings.
M.INLINE_TYPES = {
  text = true,
  emphasis = true,
  strong = true,
  strikethrough = true,
  code_span = true,
  link = true,
  image = true,
  autolink = true,
  raw_inline = true,
  hard_break = true,
  soft_break = true,
}

--- True if `node` is a block node.
-- @param node  table — any AST node
-- @return boolean
function M.is_block(node)
  return node ~= nil and M.BLOCK_TYPES[node.type] == true
end

--- True if `node` is an inline node.
-- @param node  table — any AST node
-- @return boolean
function M.is_inline(node)
  return node ~= nil and M.INLINE_TYPES[node.type] == true
end

return M
