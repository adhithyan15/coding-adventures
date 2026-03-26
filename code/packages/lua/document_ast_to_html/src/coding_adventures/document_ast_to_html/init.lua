-- Document AST → HTML Renderer
-- ==============================
--
-- Converts a Document AST (produced by any front-end parser) into an HTML
-- string. The renderer is a simple recursive tree walk — each node type maps
-- to HTML elements following the CommonMark spec HTML rendering rules.
--
-- === Node mapping ===
--
--   document      → rendered children
--   heading       → <h1>…</h1> through <h6>…</h6>
--   paragraph     → <p>…</p>  (omitted in tight list context)
--   code_block    → <pre><code [class="language-X"]>…</code></pre>
--   blockquote    → <blockquote>\n…</blockquote>
--   list          → <ul> or <ol [start="N"]>
--   list_item     → <li>…</li>
--   thematic_break → <hr />
--   raw_block     → verbatim if format="html", skipped otherwise
--
--   text          → HTML-escaped text
--   emphasis      → <em>…</em>
--   strong        → <strong>…</strong>
--   code_span     → <code>…</code>
--   link          → <a href="…" [title="…"]>…</a>
--   image         → <img src="…" alt="…" [title="…"] />
--   autolink      → <a href="[mailto:]…">…</a>
--   raw_inline    → verbatim if format="html", skipped otherwise
--   hard_break    → <br />\n
--   soft_break    → \n
--
-- === Tight vs Loose Lists ===
--
-- A tight list suppresses `<p>` tags around paragraph content in list items:
--
--   Tight:   <li>item text</li>
--   Loose:   <li><p>item text</p></li>
--
-- The `tight` flag on list nodes controls this.
--
-- === Security ===
--
-- - Text content and attribute values are HTML-escaped via escape_html.
-- - raw_block and raw_inline content is passed through verbatim when
--   format === "html" — this is intentional and spec-required.
-- - Link and image URLs are sanitized to block dangerous schemes:
--   javascript:, vbscript:, data:, blob:.
--
-- @module coding_adventures.document_ast_to_html

local M = {}

-- ─── HTML Escaping ────────────────────────────────────────────────────────────

--- Escape HTML special characters for safe output.
-- & → &amp;, < → &lt;, > → &gt;, " → &quot;
local function escape_html(text)
  return text
    :gsub("&", "&amp;")
    :gsub("<", "&lt;")
    :gsub(">", "&gt;")
    :gsub('"', "&quot;")
end

-- ─── URL Sanitization ─────────────────────────────────────────────────────────
--
-- CommonMark spec §C.3 intentionally leaves URL sanitization to the implementor.
-- Without scheme filtering, user-controlled Markdown is vulnerable to XSS via
-- javascript: and data: URIs.
--
-- We use a targeted blocklist of schemes that are execution-capable in browsers:
--   javascript:  — executes JS
--   vbscript:    — executes VBScript (IE legacy)
--   data:        — can embed scripts
--   blob:        — same-origin script execution
--
-- All other schemes (irc:, ftp:, mailto:, custom:, etc.) pass through unchanged.
-- Relative URLs (no scheme) always pass through.

local DANGEROUS_SCHEME_PATTERN = "^[%s%z\000-\031\127-\159]*[Jj][Aa][Vv][Aa][Ss][Cc][Rr][Ii][Pp][Tt]:"
  .. "|^[%s%z\000-\031\127-\159]*[Vv][Bb][Ss][Cc][Rr][Ii][Pp][Tt]:"
  .. "|^[%s%z\000-\031\127-\159]*[Dd][Aa][Tt][Aa]:"
  .. "|^[%s%z\000-\031\127-\159]*[Bb][Ll][Oo][Bb]:"

--- Strip control characters from a URL string.
-- Removes C0 controls, DEL, C1 controls, and zero-width invisible characters
-- that browsers silently ignore during scheme detection (allowing bypasses).
local function strip_url_controls(url)
  -- Remove C0 controls (U+0000-U+001F) and DEL (U+007F)
  -- Also remove zero-width chars: U+200B, U+200C, U+200D, U+2060, U+FEFF
  -- In Lua we handle the ASCII control chars; Unicode ones are multi-byte
  return url
    :gsub("[\000-\031\127]", "")         -- ASCII controls
    :gsub("\226\128\139", "")            -- U+200B ZERO WIDTH SPACE (UTF-8: E2 80 8B)
    :gsub("\226\128\140", "")            -- U+200C ZERO WIDTH NON-JOINER (E2 80 8C)
    :gsub("\226\128\141", "")            -- U+200D ZERO WIDTH JOINER (E2 80 8D)
    :gsub("\226\129\160", "")            -- U+2060 WORD JOINER (E2 81 A0)
    :gsub("\239\187\191", "")            -- U+FEFF BOM (EF BB BF)
end

--- Sanitize a URL by stripping control characters and blocking dangerous schemes.
-- Returns "" if the URL uses an execution-capable scheme.
local function sanitize_url(url)
  local stripped = strip_url_controls(url)
  -- Check for dangerous scheme (case-insensitive, ignoring control chars)
  local lower = stripped:lower()
  if lower:match("^javascript:") or lower:match("^vbscript:") or
     lower:match("^data:") or lower:match("^blob:") then
    return ""
  end
  return stripped
end

--- Normalize a URL: percent-encode characters that should not appear
-- unencoded in HTML href/src attributes.
local function normalize_url(url)
  return url:gsub("([^%w%-%._~:/?#@!$&'()*+,;=%%])", function(ch)
    -- Handle multi-byte UTF-8 by encoding each byte
    local bytes = {}
    for i = 1, #ch do
      bytes[#bytes + 1] = string.format("%%%02X", string.byte(ch, i))
    end
    return table.concat(bytes)
  end)
end

-- ─── Block Rendering ──────────────────────────────────────────────────────────

local render_block     -- forward declaration
local render_blocks    -- forward declaration
local render_inline    -- forward declaration
local render_inlines   -- forward declaration

render_blocks = function(blocks, tight, options)
  local parts = {}
  for _, b in ipairs(blocks) do
    parts[#parts + 1] = render_block(b, tight, options)
  end
  return table.concat(parts)
end

--- Render a heading node.
-- HeadingNode { level=1, children=[text{value="Hello"}] }
-- → <h1>Hello</h1>\n
local function render_heading(node, options)
  local inner = render_inlines(node.children, options)
  return string.format("<h%d>%s</h%d>\n", node.level, inner, node.level)
end

--- Render a paragraph node.
-- In tight list context, the <p> wrapper is omitted and only the inner
-- content is emitted (followed by a newline).
local function render_paragraph(node, tight, options)
  local inner = render_inlines(node.children, options)
  if tight then return inner .. "\n" end
  return "<p>" .. inner .. "</p>\n"
end

--- Render a code block node.
-- Content is HTML-escaped but not Markdown-processed.
-- If the block has a language, the <code> tag gets a class="language-X" attribute.
local function render_code_block(node)
  local escaped = escape_html(node.value)
  if node.language then
    return string.format('<pre><code class="language-%s">%s</code></pre>\n',
      escape_html(node.language), escaped)
  end
  return "<pre><code>" .. escaped .. "</code></pre>\n"
end

--- Render a blockquote node.
local function render_blockquote(node, options)
  local inner = render_blocks(node.children, false, options)
  return "<blockquote>\n" .. inner .. "</blockquote>\n"
end

--- Render an ordered or unordered list.
local function render_list_item(node, tight, options)
  if #node.children == 0 then
    return "<li></li>\n"
  end

  if tight and node.children[1].type == "paragraph" then
    local first_para = node.children[1]
    local first_content = render_inlines(first_para.children, options)
    if #node.children == 1 then
      return "<li>" .. first_content .. "</li>\n"
    end
    -- Multiple children: inline the first paragraph, then block-render the rest
    local rest_children = {}
    for i = 2, #node.children do
      rest_children[#rest_children + 1] = node.children[i]
    end
    local rest = render_blocks(rest_children, tight, options)
    return "<li>" .. first_content .. "\n" .. rest .. "</li>\n"
  end

  -- Loose or non-paragraph first child: block-level format with newlines
  local inner = render_blocks(node.children, tight, options)
  local last_child = node.children[#node.children]
  if tight and last_child and last_child.type == "paragraph" and inner:sub(-1) == "\n" then
    return "<li>\n" .. inner:sub(1, -2) .. "</li>\n"
  end
  return "<li>\n" .. inner .. "</li>\n"
end

local function render_task_item(node, tight, options)
  local checkbox = node.checked
      and '<input type="checkbox" disabled="" checked="" />'
      or '<input type="checkbox" disabled="" />'

  if #node.children == 0 then
    return "<li>" .. checkbox .. "</li>\n"
  end

  if tight and node.children[1].type == "paragraph" then
    local first_para = node.children[1]
    local first_content = render_inlines(first_para.children, options)
    local separator = first_content ~= "" and " " or ""
    if #node.children == 1 then
      return "<li>" .. checkbox .. separator .. first_content .. "</li>\n"
    end
    local rest_children = {}
    for i = 2, #node.children do
      rest_children[#rest_children + 1] = node.children[i]
    end
    local rest = render_blocks(rest_children, tight, options)
    return "<li>" .. checkbox .. separator .. first_content .. "\n" .. rest .. "</li>\n"
  end

  local inner = render_blocks(node.children, tight, options)
  return "<li>" .. checkbox .. "\n" .. inner .. "</li>\n"
end

local function render_list(node, options)
  local tag = node.ordered and "ol" or "ul"
  -- Only emit `start` when it's a valid integer and not 1
  local start_attr = ""
  if node.ordered and node.start ~= nil and node.start ~= 1 then
    -- Guard: only safe integers
    if node.start == math.floor(node.start) then
      start_attr = string.format(' start="%d"', node.start)
    end
  end
  local items = {}
  for _, item in ipairs(node.children) do
    if item.type == "task_item" then
      items[#items + 1] = render_task_item(item, node.tight, options)
    else
      items[#items + 1] = render_list_item(item, node.tight, options)
    end
  end
  return string.format("<%s%s>\n%s</%s>\n", tag, start_attr, table.concat(items), tag)
end

local function render_table_cell(node, header, options)
  local tag = header and "th" or "td"
  local inner = render_inlines(node.children, options)
  return string.format("<%s>%s</%s>\n", tag, inner, tag)
end

local function render_table_row(node, options)
  local cells = {}
  for _, cell in ipairs(node.children) do
    cells[#cells + 1] = render_table_cell(cell, node.is_header, options)
  end
  return "<tr>\n" .. table.concat(cells) .. "</tr>\n"
end

local function render_table(node, options)
  local header_rows, body_rows = {}, {}
  for _, row in ipairs(node.children) do
    if row.is_header then
      header_rows[#header_rows + 1] = row
    else
      body_rows[#body_rows + 1] = row
    end
  end

  local parts = { "<table>\n" }
  if #header_rows > 0 then
    parts[#parts + 1] = "<thead>\n"
    for _, row in ipairs(header_rows) do
      parts[#parts + 1] = render_table_row(row, options)
    end
    parts[#parts + 1] = "</thead>\n"
  end
  if #body_rows > 0 then
    parts[#parts + 1] = "<tbody>\n"
    for _, row in ipairs(body_rows) do
      parts[#parts + 1] = render_table_row(row, options)
    end
    parts[#parts + 1] = "</tbody>\n"
  end
  parts[#parts + 1] = "</table>\n"
  return table.concat(parts)
end

--- Render a raw block node.
-- If sanitize is true, this node is always skipped.
-- Otherwise, if format=="html", emit the raw value verbatim.
local function render_raw_block(node, options)
  if options.sanitize then return "" end
  if node.format == "html" then return node.value end
  return ""
end

render_block = function(block, tight, options)
  local t = block.type
  if t == "document" then
    return render_blocks(block.children, false, options)
  elseif t == "heading" then
    return render_heading(block, options)
  elseif t == "paragraph" then
    return render_paragraph(block, tight, options)
  elseif t == "code_block" then
    return render_code_block(block)
  elseif t == "blockquote" then
    return render_blockquote(block, options)
  elseif t == "list" then
    return render_list(block, options)
  elseif t == "list_item" then
    return render_list_item(block, false, options)
  elseif t == "task_item" then
    return render_task_item(block, false, options)
  elseif t == "thematic_break" then
    return "<hr />\n"
  elseif t == "raw_block" then
    return render_raw_block(block, options)
  elseif t == "table" then
    return render_table(block, options)
  elseif t == "table_row" then
    return render_table_row(block, options)
  elseif t == "table_cell" then
    return render_table_cell(block, false, options)
  else
    return ""
  end
end

-- ─── Inline Rendering ─────────────────────────────────────────────────────────

render_inlines = function(nodes, options)
  local parts = {}
  for _, n in ipairs(nodes) do
    parts[#parts + 1] = render_inline(n, options)
  end
  return table.concat(parts)
end

--- Render a raw inline node.
-- If sanitize is true, always skipped.
-- If format=="html", emit verbatim.
local function render_raw_inline(node, options)
  if options.sanitize then return "" end
  if node.format == "html" then return node.value end
  return ""
end

--- Render an inline link.
local function render_link(node, options)
  local href = escape_html(sanitize_url(node.destination))
  local title_attr = ""
  if node.title ~= nil then
    title_attr = string.format(' title="%s"', escape_html(node.title))
  end
  local inner = render_inlines(node.children, options)
  return string.format('<a href="%s"%s>%s</a>', href, title_attr, inner)
end

--- Render an image.
local function render_image(node)
  local src = escape_html(sanitize_url(node.destination))
  local alt = escape_html(node.alt)
  local title_attr = ""
  if node.title ~= nil then
    title_attr = string.format(' title="%s"', escape_html(node.title))
  end
  return string.format('<img src="%s" alt="%s"%s />', src, alt, title_attr)
end

--- Render an autolink.
local function render_autolink(node)
  local dest = sanitize_url(node.destination)
  local href
  if node.is_email then
    href = "mailto:" .. escape_html(dest)
  else
    href = escape_html(sanitize_url(normalize_url(dest)))
  end
  local text = escape_html(node.destination)
  return string.format('<a href="%s">%s</a>', href, text)
end

render_inline = function(node, options)
  local t = node.type
  if t == "text" then
    return escape_html(node.value)
  elseif t == "emphasis" then
    return "<em>" .. render_inlines(node.children, options) .. "</em>"
  elseif t == "strong" then
    return "<strong>" .. render_inlines(node.children, options) .. "</strong>"
  elseif t == "strikethrough" then
    return "<del>" .. render_inlines(node.children, options) .. "</del>"
  elseif t == "code_span" then
    return "<code>" .. escape_html(node.value) .. "</code>"
  elseif t == "link" then
    return render_link(node, options)
  elseif t == "image" then
    return render_image(node)
  elseif t == "autolink" then
    return render_autolink(node)
  elseif t == "raw_inline" then
    return render_raw_inline(node, options)
  elseif t == "hard_break" then
    return "<br />\n"
  elseif t == "soft_break" then
    -- CommonMark spec §6.12: soft line break renders as a newline
    return "\n"
  else
    return ""
  end
end

-- ─── Public Entry Point ────────────────────────────────────────────────────────

--- Render a Document AST to an HTML string.
--
-- The input is a document node as produced by any front-end parser that
-- implements the Document AST spec (TE00). The output is a valid HTML fragment.
--
-- ⚠️  Security notice: Raw HTML passthrough is enabled by default
-- (required for CommonMark spec compliance). If you render untrusted
-- Markdown (user content, third-party data), pass `{ sanitize=true }` to
-- strip all raw HTML from the output. Without this, an attacker who controls
-- the Markdown source can inject arbitrary HTML into the rendered page.
--
-- @param document  table — the root document node
-- @param options   table — render options (optional)
--   options.sanitize  boolean — when true, drop all raw_block and raw_inline nodes
-- @return string — HTML string representing the document
--
-- @example
--   local html = require("coding_adventures.document_ast_to_html")
--   local cm = require("coding_adventures.commonmark_parser")
--
--   -- Trusted Markdown (documentation, static content):
--   local output = html.to_html(cm.parse("# Hello\n\nWorld\n"))
--
--   -- Untrusted Markdown (user-supplied content):
--   local output = html.to_html(cm.parse(user_input), { sanitize=true })
function M.to_html(document, options)
  options = options or {}
  return render_blocks(document.children, false, options)
end

M.VERSION = "0.1.0"

return M
