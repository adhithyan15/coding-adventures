-- AsciiDoc Inline Parser
-- ======================
--
-- Converts a plain-text string (the content of a paragraph, heading, list
-- item, etc.) into a list of inline AST node tables.
--
-- === AsciiDoc inline syntax ===
--
-- AsciiDoc inline markup differs from CommonMark in a few important ways:
--
--   * `*bold*`    → strong  (CommonMark uses `**bold**` for strong)
--   * `_italic_`  → emphasis
--   * `**bold**`  → unconstrained strong (can appear mid-word)
--   * `__italic__`→ unconstrained emphasis
--   * `` `code` ``  → inline code (verbatim, no nested parsing)
--   * `link:url[text]`   → hyperlink
--   * `image:url[alt]`   → inline image
--   * `<<anchor,text>>`  → cross-reference link
--   * `https://...`      → bare URL (auto-linked)
--   * `+` at end of line → hard line break
--
-- === Parsing strategy ===
--
-- We use a simple left-to-right scan (no backtracking). At each position
-- we try the patterns in priority order:
--
--   1. hard break:          `  \n` or `+\n`
--   2. soft break:          `\n`
--   3. inline code:         `` `...` ``  (verbatim — no nested parsing)
--   4. unconstrained bold:  `**...**`    (check before `*`)
--   5. unconstrained italic:`__...__`    (check before `_`)
--   6. constrained bold:    `*...*`
--   7. constrained italic:  `_..._`
--   8. link macro:          `link:url[text]`
--   9. image macro:         `image:url[alt]`
--  10. cross-reference:     `<<anchor,text>>`
--  11. bare https/http URL: `https://...` or `http://...`
--  12. plain text:          everything else
--
-- @module coding_adventures.asciidoc_parser.inline_parser

local M = {}

-- ─── HTML escaping ────────────────────────────────────────────────────────────

-- Escape characters that are significant in HTML.
-- We do this at the node level (value fields) so the renderer can emit
-- node values directly into HTML.
local function escape_html(s)
  s = s:gsub("&", "&amp;")
  s = s:gsub("<", "&lt;")
  s = s:gsub(">", "&gt;")
  s = s:gsub('"', "&quot;")
  return s
end

-- ─── Node constructors ────────────────────────────────────────────────────────

-- These mirror the Document AST schema.  Each node is a plain Lua table.

local function text_node(value)
  return { type = "text", value = value }
end

local function code_span_node(value)
  return { type = "code_span", value = value }
end

local function strong_node(children)
  return { type = "strong", children = children }
end

local function emph_node(children)
  return { type = "emph", children = children }
end

local function link_node(href, children)
  return { type = "link", href = href, children = children }
end

local function image_node(src, alt)
  return { type = "image", src = src, alt = alt }
end

local function hard_break_node()
  return { type = "hard_break" }
end

local function soft_break_node()
  return { type = "soft_break" }
end

-- ─── Recursive inline parser ─────────────────────────────────────────────────

--- Parse a string of inline AsciiDoc content.
--
-- Returns a list (Lua array) of inline AST node tables.
--
-- @param text  string — raw inline content (may contain newlines from
--              multi-line paragraphs)
-- @return table — ordered list of inline nodes
function M.parse_inline(text)
  local nodes = {}
  local pos   = 1
  local len   = #text

  while pos <= len do
    local rest = text:sub(pos)

    -- 1. Hard break: trailing `  ` (two+ spaces) before newline,
    --    or AsciiDoc `+` at end of line before newline.
    --    In AsciiDoc `+` at end of line = explicit line continuation break.
    local hb_spaces = rest:match("^  +\n")
    local hb_plus   = rest:match("^%+\n")
    if hb_spaces then
      nodes[#nodes + 1] = hard_break_node()
      pos = pos + #hb_spaces
      goto continue
    end
    if hb_plus then
      nodes[#nodes + 1] = hard_break_node()
      pos = pos + #hb_plus
      goto continue
    end

    -- 2. Soft break: single newline (joins wrapped lines with a space in HTML)
    if rest:sub(1, 1) == "\n" then
      nodes[#nodes + 1] = soft_break_node()
      pos = pos + 1
      goto continue
    end

    -- 3. Inline code: `code`
    --    Verbatim — content is NOT parsed for inline markup.
    --    AsciiDoc also allows multiple backticks (``code``) but we handle
    --    the common single-backtick form.
    do
      local code_val = rest:match("^`([^`]*)`")
      if code_val then
        nodes[#nodes + 1] = code_span_node(escape_html(code_val))
        pos = pos + 2 + #code_val  -- 2 for the two backticks
        goto continue
      end
    end

    -- 4. Unconstrained bold: **...**
    --    Must be checked BEFORE constrained `*...*`.
    --    Unconstrained means it can appear mid-word: foo**bar**baz.
    do
      local bold_val = rest:match("^%*%*(.-)%*%*")
      if bold_val then
        nodes[#nodes + 1] = strong_node(M.parse_inline(bold_val))
        pos = pos + 4 + #bold_val  -- 2 for ** + content + 2 for **
        goto continue
      end
    end

    -- 5. Unconstrained italic: __...__
    --    Must be checked BEFORE constrained `_..._`.
    do
      local em_val = rest:match("^__(.-)__")
      if em_val then
        nodes[#nodes + 1] = emph_node(M.parse_inline(em_val))
        pos = pos + 4 + #em_val  -- 2 for __ + content + 2 for __
        goto continue
      end
    end

    -- 6. Constrained bold: *...*
    --    In AsciiDoc, single `*` delimiters produce strong (bold), unlike
    --    CommonMark where single `*` produces emphasis.  This is one of the
    --    most important semantic differences to remember.
    do
      local bold_val = rest:match("^%*([^%*]+)%*")
      if bold_val then
        nodes[#nodes + 1] = strong_node(M.parse_inline(bold_val))
        pos = pos + 2 + #bold_val
        goto continue
      end
    end

    -- 7. Constrained italic: _..._
    do
      local em_val = rest:match("^_([^_]+)_")
      if em_val then
        nodes[#nodes + 1] = emph_node(M.parse_inline(em_val))
        pos = pos + 2 + #em_val
        goto continue
      end
    end

    -- 8. Link macro: link:url[text]
    --    AsciiDoc links use a macro syntax rather than Markdown's [text](url).
    do
      local url, label = rest:match("^link:([^%[]+)%[([^%]]*)%]")
      if url then
        local children = (label ~= "") and M.parse_inline(label)
                         or { text_node(escape_html(url)) }
        nodes[#nodes + 1] = link_node(url, children)
        pos = pos + 5 + #url + 1 + #label + 1  -- "link:" + url + "[" + label + "]"
        goto continue
      end
    end

    -- 9. Image macro: image:url[alt]
    do
      local url, alt = rest:match("^image:([^%[]+)%[([^%]]*)%]")
      if url then
        nodes[#nodes + 1] = image_node(url, escape_html(alt))
        pos = pos + 6 + #url + 1 + #alt + 1  -- "image:" + url + "[" + alt + "]"
        goto continue
      end
    end

    -- 10. Cross-reference: <<anchor,text>> or <<anchor>>
    --     Renders as a hyperlink to #anchor within the same page.
    do
      local anchor, label = rest:match("^<<([^,>]+),([^>]*)>>")
      if anchor then
        local children = (label ~= "") and M.parse_inline(label)
                         or { text_node(escape_html(anchor)) }
        nodes[#nodes + 1] = link_node("#" .. anchor, children)
        pos = pos + 2 + #anchor + 1 + #label + 2  -- "<<" + anchor + "," + label + ">>"
        goto continue
      end
      -- Without label: <<anchor>>
      local anchor_only = rest:match("^<<([^>]+)>>")
      if anchor_only then
        nodes[#nodes + 1] = link_node("#" .. anchor_only,
                                      { text_node(escape_html(anchor_only)) })
        pos = pos + 2 + #anchor_only + 2
        goto continue
      end
    end

    -- 11. Bare https:// or http:// URL
    --     Auto-link everything from the scheme up to the first whitespace or
    --     common terminating punctuation character.
    do
      local url = rest:match("^https?://[^%s<>\"'%]%)]+")
      if url then
        nodes[#nodes + 1] = link_node(url, { text_node(escape_html(url)) })
        pos = pos + #url
        goto continue
      end
    end

    -- 12. Plain text: consume up to the next special character.
    --     Special characters that start inline markup: ` * _ l i h < \n
    --     We grab everything that is definitely safe plain text.
    do
      -- This pattern matches one or more characters that cannot start any
      -- of the inline patterns above.  We must be conservative: the moment
      -- we see a backtick, asterisk, underscore, `l` (link:), `i` (image:),
      -- `h` (https://http://), `<`, or `\n`, we stop the plain-text run.
      local plain = rest:match("^([^`%*_lihH<\n%+]+)")
      if plain then
        nodes[#nodes + 1] = text_node(escape_html(plain))
        pos = pos + #plain
        goto continue
      end

      -- Fallback: consume exactly one character as plain text.
      -- This handles lone `*`, `_`, `l`, `i`, `h`, `<`, etc. that did not
      -- match any structured pattern above.
      local ch = text:sub(pos, pos)
      nodes[#nodes + 1] = text_node(escape_html(ch))
      pos = pos + 1
    end

    ::continue::
  end

  -- Merge consecutive text nodes.
  --
  -- The plain-text scanner must stop at characters like `l`, `i`, `h`, `H`
  -- because they might start inline macros (`link:`, `image:`, `https://`).
  -- This causes words such as "hello" to be split into many single-character
  -- text nodes.  Merging consecutive text nodes restores the full word into a
  -- single node, which is both more correct and easier for callers to test.
  local merged = {}
  for _, node in ipairs(nodes) do
    if node.type == "text" and #merged > 0 and merged[#merged].type == "text" then
      merged[#merged].value = merged[#merged].value .. node.value
    else
      merged[#merged + 1] = node
    end
  end

  return merged
end

return M
