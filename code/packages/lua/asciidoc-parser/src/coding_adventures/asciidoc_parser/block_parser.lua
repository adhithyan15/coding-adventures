-- AsciiDoc Block Parser
-- =====================
--
-- A line-oriented state machine that converts AsciiDoc source into a list of
-- block-level AST node tables.
--
-- === Overview ===
--
-- The parser processes the input one line at a time.  It maintains a `state`
-- variable that tracks what kind of block is currently being accumulated:
--
--   normal         — top-level; dispatch each new line to the right state
--   paragraph      — accumulating paragraph lines
--   code_block     — inside a `----` delimited fenced code block
--   literal_block  — inside a `....` delimited literal (pre) block
--   passthrough_block — inside a `++++` passthrough block (raw HTML/content)
--   quote_block    — inside a `____` quote/sidebar block
--   unordered_list — accumulating `*` list items
--   ordered_list   — accumulating `.` list items
--
-- === Line dispatch (normal state) ===
--
--   blank line     → stay in normal, flush any pending state
--   `// comment`   → skip (AsciiDoc single-line comment)
--   `[source,lang]`→ record pending_language for the next code block
--   `= text`       → heading level 1
--   `== text`      → heading level 2  (up to `======` for level 6)
--   `'''` (3+)     → thematic_break
--   `----` (4+)    → enter code_block mode
--   `....` (4+)    → enter literal_block mode
--   `++++` (4+)    → enter passthrough_block mode
--   `____` (4+)    → enter quote_block mode
--   `* text`       → start/continue unordered_list
--   `. text`       → start/continue ordered_list
--   other text     → enter paragraph mode
--
-- === Nesting ===
--
-- Quote blocks (`____`) contain paragraphs parsed recursively.
-- List items are single-level; nested bullets (`**`, `..`) are treated as
-- continuation of the current list depth for simplicity.
--
-- @module coding_adventures.asciidoc_parser.block_parser

local inline_parser = require("coding_adventures.asciidoc_parser.inline_parser")

local M = {}

-- ─── Node constructors ────────────────────────────────────────────────────────

local function document_node(children)
  return { type = "document", children = children }
end

local function heading_node(level, children)
  return { type = "heading", level = level, children = children }
end

local function paragraph_node(children)
  return { type = "paragraph", children = children }
end

local function code_block_node(language, value)
  return { type = "code_block", language = language or "", value = value }
end

local function thematic_break_node()
  return { type = "thematic_break" }
end

local function blockquote_node(children)
  return { type = "blockquote", children = children }
end

local function list_node(ordered, items)
  return { type = "list", ordered = ordered, children = items }
end

local function list_item_node(children)
  return { type = "list_item", children = children }
end

local function passthrough_node(value)
  -- A passthrough block is raw content that passes through the renderer
  -- without modification.  It maps to the Document AST "raw_block" type.
  return { type = "raw_block", value = value }
end

-- ─── Helpers ─────────────────────────────────────────────────────────────────

--- Trim leading and trailing whitespace from a string.
local function trim(s)
  return s:match("^%s*(.-)%s*$")
end

--- True if the string is blank (empty or only whitespace).
local function is_blank(s)
  return s:match("^%s*$") ~= nil
end

--- Parse inline content from a text string.
local function inlines(text)
  return inline_parser.parse_inline(text)
end

--- Parse a heading line.  Returns (level, inline_children) or nil.
--
-- AsciiDoc heading syntax: `= Title`, `== Section`, …, `====== Level 6`
-- The number of `=` signs gives the level (1–6).
local function parse_heading_line(line)
  local signs, rest = line:match("^(=+) (.+)$")
  if signs and #signs >= 1 and #signs <= 6 then
    return #signs, inlines(trim(rest))
  end
  return nil
end

--- Build list items accumulated as {text_string, text_string, …}.
local function build_list_items(raw_items)
  local items = {}
  for _, text in ipairs(raw_items) do
    items[#items + 1] = list_item_node(inlines(text))
  end
  return items
end

-- ─── Main parse function ──────────────────────────────────────────────────────

--- Parse an AsciiDoc string into a list of block AST nodes.
--
-- This is the core function of the block parser.  It splits the text into
-- lines and drives the state machine described above.
--
-- @param text  string — AsciiDoc source
-- @return table — ordered list of block AST nodes
function M.parse_blocks(text)
  -- Normalize line endings to \n, ensure a trailing newline.
  text = text:gsub("\r\n", "\n"):gsub("\r", "\n")
  if text:sub(-1) ~= "\n" then text = text .. "\n" end

  -- Split into lines (strip the trailing newline from each).
  local lines = {}
  for line in text:gmatch("([^\n]*)\n") do
    lines[#lines + 1] = line
  end

  local blocks           = {}
  local state            = "normal"
  local i                = 1
  local n                = #lines

  -- State variables
  local para_lines       = {}   -- accumulated paragraph lines
  local code_lines       = {}   -- accumulated code/literal block lines
  local code_language    = ""   -- set by [source,lang] attribute
  local pending_language = nil  -- language from the last [source,…] block
  local list_ordered     = false
  local list_raw_items   = {}   -- raw text of list items

  -- Flush helpers: emit a block from accumulated state and reset.

  local function flush_paragraph()
    if #para_lines > 0 then
      local text_content = table.concat(para_lines, "\n")
      blocks[#blocks + 1] = paragraph_node(inlines(text_content))
      para_lines = {}
    end
  end

  local function flush_list()
    if #list_raw_items > 0 then
      local items = build_list_items(list_raw_items)
      blocks[#blocks + 1] = list_node(list_ordered, items)
      list_raw_items = {}
    end
  end

  -- ── State machine ──────────────────────────────────────────────────────────

  while i <= n do
    local line = lines[i]
    i = i + 1

    -- ── code_block state ──────────────────────────────────────────────────
    -- Collect lines verbatim until we see another `----` fence.
    if state == "code_block" then
      if line:match("^%-%-%-%-+%s*$") then
        -- Closing fence: emit the code block.
        local value = table.concat(code_lines, "\n")
        if #code_lines > 0 then value = value .. "\n" end
        blocks[#blocks + 1] = code_block_node(code_language, value)
        code_lines    = {}
        code_language = ""
        state         = "normal"
      else
        code_lines[#code_lines + 1] = line
      end

    -- ── literal_block state ───────────────────────────────────────────────
    -- Same as code_block but uses `....` fence.  Emits a code_block node
    -- without a language annotation (literal/pre content).
    elseif state == "literal_block" then
      if line:match("^%.%.%.%.+%s*$") then
        local value = table.concat(code_lines, "\n")
        if #code_lines > 0 then value = value .. "\n" end
        blocks[#blocks + 1] = code_block_node("", value)
        code_lines = {}
        state      = "normal"
      else
        code_lines[#code_lines + 1] = line
      end

    -- ── passthrough_block state ───────────────────────────────────────────
    -- Content between `++++` fences is raw (not parsed for markup).
    elseif state == "passthrough_block" then
      if line:match("^%+%+%+%++%s*$") then
        local value = table.concat(code_lines, "\n")
        if #code_lines > 0 then value = value .. "\n" end
        blocks[#blocks + 1] = passthrough_node(value)
        code_lines = {}
        state      = "normal"
      else
        code_lines[#code_lines + 1] = line
      end

    -- ── quote_block state ─────────────────────────────────────────────────
    -- Content between `____` fences is parsed recursively as AsciiDoc.
    elseif state == "quote_block" then
      if #line >= 4 and line:match("^_+%s*$") then
        local inner = table.concat(code_lines, "\n")
        local inner_blocks = M.parse_blocks(inner)
        blocks[#blocks + 1] = blockquote_node(inner_blocks)
        code_lines = {}
        state      = "normal"
      else
        code_lines[#code_lines + 1] = line
      end

    -- ── paragraph state ───────────────────────────────────────────────────
    -- Accumulate lines until a blank line or a structural line is reached.
    elseif state == "paragraph" then
      if is_blank(line) then
        flush_paragraph()
        state = "normal"
      elseif line:match("^=+ ") or line:match("^%-%-%-%-+%s*$")
          or line:match("^%.%.%.%.+%s*$") or line:match("^%+%+%+%++%s*$")
          or line:match("^____%s*$")
          or (line:match("^'''[' ]*$") and not line:match("[^' ]"))
          or line:match("^%* ") or line:match("^%. ") then
        -- Structural element interrupts the paragraph.
        flush_paragraph()
        state = "normal"
        i = i - 1  -- re-process this line in normal state
      else
        para_lines[#para_lines + 1] = line
      end

    -- ── unordered_list state ──────────────────────────────────────────────
    elseif state == "unordered_list" then
      if is_blank(line) then
        flush_list()
        state = "normal"
      elseif line:match("^%*+ (.+)$") then
        -- New list item (any depth `*`, `**`, etc.)
        local item_text = line:match("^%*+ (.+)$")
        list_raw_items[#list_raw_items + 1] = item_text
      else
        -- Non-list line interrupts the list.
        flush_list()
        state = "normal"
        i = i - 1
      end

    -- ── ordered_list state ────────────────────────────────────────────────
    elseif state == "ordered_list" then
      if is_blank(line) then
        flush_list()
        state = "normal"
      elseif line:match("^%.+ (.+)$") then
        local item_text = line:match("^%.+ (.+)$")
        list_raw_items[#list_raw_items + 1] = item_text
      else
        flush_list()
        state = "normal"
        i = i - 1
      end

    -- ── normal state ──────────────────────────────────────────────────────
    else  -- state == "normal"

      -- Blank line: nothing to flush from normal state.
      if is_blank(line) then
        -- (intentionally empty)

      -- Comment: single-line AsciiDoc comment `// text`.
      elseif line:match("^//") then
        -- Skip silently.

      -- Attribute block: `[source,lang]` sets the language for the next block.
      -- AsciiDoc attributes are square-bracket annotations before a block.
      elseif line:match("^%[source,(.-)%]%s*$") then
        pending_language = line:match("^%[source,(.-)%]%s*$")

      -- Other attribute blocks (not source) — consume and ignore.
      elseif line:match("^%[.-%]%s*$") then
        -- Ignored attribute block.

      -- Heading: `= H1`, `== H2`, …, `====== H6`
      elseif line:match("^=+ ") then
        local level, children = parse_heading_line(line)
        if level then
          blocks[#blocks + 1] = heading_node(level, children)
        end

      -- Thematic break: three or more single-quote characters `'''`
      -- We check that the line starts with ''' and contains only ' and spaces.
      elseif line:match("^'''[' ]*$") and not line:match("[^' ]") then
        blocks[#blocks + 1] = thematic_break_node()

      -- Code block fence: four or more dashes `----`
      elseif line:match("^%-%-%-%-+%s*$") then
        code_language    = pending_language or ""
        pending_language = nil
        code_lines       = {}
        state            = "code_block"

      -- Literal block fence: four or more dots `....`
      elseif line:match("^%.%.%.%.+%s*$") then
        pending_language = nil
        code_lines       = {}
        state            = "literal_block"

      -- Passthrough block fence: four or more plus signs `++++`
      elseif line:match("^%+%+%+%++%s*$") then
        code_lines       = {}
        state            = "passthrough_block"

      -- Quote block fence: four or more underscores `____`
      elseif line:match("^____%s*$") then
        code_lines       = {}
        state            = "quote_block"

      -- Unordered list item: `* text` or `** text` (any depth)
      elseif line:match("^%*+ (.+)$") then
        list_ordered   = false
        list_raw_items = {}
        local item_text = line:match("^%*+ (.+)$")
        list_raw_items[#list_raw_items + 1] = item_text
        state = "unordered_list"

      -- Ordered list item: `. text` or `.. text` (any depth)
      elseif line:match("^%.+ (.+)$") then
        list_ordered   = true
        list_raw_items = {}
        local item_text = line:match("^%.+ (.+)$")
        list_raw_items[#list_raw_items + 1] = item_text
        state = "ordered_list"

      -- Everything else starts a paragraph.
      else
        pending_language = nil
        para_lines = { line }
        state      = "paragraph"
      end
    end
  end

  -- ── End-of-input: flush any open state ────────────────────────────────────
  if state == "paragraph" then
    flush_paragraph()
  elseif state == "code_block" or state == "literal_block" then
    -- Unclosed code block: emit what we have (tolerant parsing).
    local value = table.concat(code_lines, "\n")
    if #code_lines > 0 then value = value .. "\n" end
    blocks[#blocks + 1] = code_block_node(code_language, value)
  elseif state == "passthrough_block" then
    local value = table.concat(code_lines, "\n")
    if #code_lines > 0 then value = value .. "\n" end
    blocks[#blocks + 1] = passthrough_node(value)
  elseif state == "quote_block" then
    local inner = table.concat(code_lines, "\n")
    local inner_blocks = M.parse_blocks(inner)
    blocks[#blocks + 1] = blockquote_node(inner_blocks)
  elseif state == "unordered_list" or state == "ordered_list" then
    flush_list()
  end

  return blocks
end

return M
