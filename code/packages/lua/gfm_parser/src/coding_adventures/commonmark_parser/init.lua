-- GFM Parser
-- =================
--
-- GitHub Flavored Markdown parser.
--
-- Parses Markdown source text into a Document AST — the format-agnostic IR
-- defined in coding_adventures.document_ast. The result is a document node
-- ready for any back-end renderer (HTML, PDF, plain text, ...).
--
-- === Two-Phase Architecture ===
--
-- GFM parsing is inherently two-phase:
--
--   Phase 1 (parse_blocks): Block structure
--     Input text → lines → block tree with raw inline content strings
--
--   Phase 2 (parse_inline): Inline content
--     Each block's raw content → inline nodes (emphasis, links, etc.)
--
-- The phases cannot be merged because block structure determines where
-- inline content lives. A `*` that starts a list item is structural;
-- a `*` inside paragraph text may be emphasis.
--
-- === Quick Start ===
--
--   local cm = require("coding_adventures.commonmark_parser")
--
--   local doc = cm.parse("# Hello\n\nWorld *with* emphasis.\n")
--   doc.type                    -- "document"
--   doc.children[1].type        -- "heading"
--   doc.children[2].type        -- "paragraph"
--
-- === Dependencies ===
--
-- This module is self-contained except for coding_adventures.document_ast
-- (the AST node constructors). The scanner.lua and entities.lua modules
-- are loaded as submodules within the same package.
--
-- @module coding_adventures.commonmark_parser

local ast = require("coding_adventures.document_ast")
local scanner_mod = require("coding_adventures.commonmark_parser.scanner")
local entities_mod = require("coding_adventures.commonmark_parser.entities")

local Scanner = scanner_mod.Scanner
local is_ascii_punctuation = scanner_mod.is_ascii_punctuation
local is_unicode_punctuation = scanner_mod.is_unicode_punctuation
local is_ascii_whitespace = scanner_mod.is_ascii_whitespace
local is_unicode_whitespace = scanner_mod.is_unicode_whitespace
local normalize_link_label = scanner_mod.normalize_link_label
local normalize_url = scanner_mod.normalize_url
local apply_backslash_escapes = scanner_mod.apply_backslash_escapes
local decode_entity = entities_mod.decode_entity
local decode_entities = entities_mod.decode_entities_full
local escape_html = entities_mod.escape_html

local M = {}

-- ─── Utility Functions ────────────────────────────────────────────────────────

--- True if the line is blank (empty or only whitespace).
local function is_blank(line)
  return line:match("^%s*$") ~= nil
end

--- Count leading virtual spaces, expanding tabs to 4-space tab stops.
-- `base_col` is the virtual column of `line[1]` in the original document.
-- Returns the number of virtual indentation spaces (relative to base_col).
local function indent_of(line, base_col)
  base_col = base_col or 0
  local col = base_col
  for i = 1, #line do
    local ch = line:sub(i, i)
    if ch == " " then
      col = col + 1
    elseif ch == "\t" then
      col = col + (4 - (col % 4))
    else
      break
    end
  end
  return col - base_col
end

--- Strip exactly `n` virtual spaces of leading indentation.
-- Returns [stripped_line, next_base_col].
-- Handles partial tabs by prepending leftover virtual spaces.
local function strip_indent(line, n, base_col)
  base_col = base_col or 0
  local remaining = n
  local col = base_col
  local i = 1
  while remaining > 0 and i <= #line do
    local ch = line:sub(i, i)
    if ch == " " then
      i = i + 1
      remaining = remaining - 1
      col = col + 1
    elseif ch == "\t" then
      local w = 4 - (col % 4)
      if w <= remaining then
        i = i + 1
        remaining = remaining - w
        col = col + w
      else
        -- Partial tab: consume the tab but prepend leftover virtual spaces
        local leftover = w - remaining
        return string.rep(" ", leftover) .. line:sub(i + 1), col + remaining
      end
    else
      break
    end
  end
  return line:sub(i), col
end

--- Compute the virtual column reached after consuming `char_count` characters.
local function virtual_col_after(line, char_count, start_col)
  start_col = start_col or 0
  local col = start_col
  for i = 1, math.min(char_count, #line) do
    if line:sub(i, i) == "\t" then
      col = col + (4 - (col % 4))
    else
      col = col + 1
    end
  end
  return col
end

--- Extract info string from a fenced code block opening line.
local function extract_info_string(line)
  local m = line:match("^[`~]+%s*(.*)")
  if not m then return "" end
  -- Only the first word is the language
  local raw = m:match("^%S+") or ""
  return decode_entities(apply_backslash_escapes(raw))
end

-- ─── ATX Heading Detection ────────────────────────────────────────────────────

--- Parse an ATX heading from a line.
-- Returns {level, content} or nil.
-- Lua patterns don't support {n,m} quantifiers, so we use explicit repetition.
local function parse_atx_heading(line)
  -- Strip up to 3 leading spaces
  local trimmed = line:match("^[ ]?[ ]?[ ]?(.*)")
  if not trimmed then return nil end

  -- Must start with # characters
  local hashes = trimmed:match("^(#+)")
  if not hashes then return nil end
  if #hashes > 6 then return nil end

  local after_hashes = trimmed:sub(#hashes + 1)

  -- Must be followed by space, tab, or end of string
  if after_hashes ~= "" and after_hashes:sub(1,1) ~= " " and after_hashes:sub(1,1) ~= "\t" then
    return nil
  end

  -- Extract content (strip the leading space/tab separator)
  local content = ""
  if #after_hashes > 0 then
    content = after_hashes:sub(2)  -- skip the single space/tab separator
    -- Trim trailing whitespace
    content = content:gsub("%s+$", "")
    -- Remove closing hash sequence: space/tab + one or more hashes + optional spaces
    content = content:gsub("[ \t]+#+[ \t]*$", "")
    -- If content is now purely hashes (e.g. `### ###`), it was the closing sequence
    if content:match("^#+[ \t]*$") then content = "" end
    -- Trim leading/trailing whitespace from content
    content = content:match("^%s*(.-)%s*$") or content
  end

  return { level = #hashes, content = content }
end

-- ─── Thematic Break Detection ─────────────────────────────────────────────────

--- True if the line is a thematic break.
-- 0-3 spaces, then 3+ of *, -, or _ optionally separated by spaces/tabs.
local function is_thematic_break(line)
  -- Strip up to 3 leading spaces
  local s = line:match("^[ ]?[ ]?[ ]?(.*)")
  if not s then return false end
  s = s:gsub("%s+$", "")  -- trim trailing whitespace
  if #s == 0 then return false end

  -- Try each character
  for _, ch in ipairs({"*", "-", "_"}) do
    -- All non-whitespace characters must be `ch`
    local stripped = s:gsub("[ \t]", "")
    if #stripped >= 3 then
      local all_same = true
      for i = 1, #stripped do
        if stripped:sub(i, i) ~= ch then all_same = false; break end
      end
      if all_same then
        -- Verify original only has `ch` and spaces/tabs
        local only_ch_and_space = s:match("^[%" .. (ch == "*" and "%*" or ch == "-" and "%-" or "_") .. " \t]+$")
        if only_ch_and_space then return true end
      end
    end
  end
  return false
end

-- ─── Setext Heading Underline Detection ──────────────────────────────────────

--- Returns 1 for `===` underline, 2 for `---` underline, nil otherwise.
local function is_setext_underline(line)
  -- Up to 3 spaces + one or more = or - + optional whitespace
  local s = line:match("^[ ]?[ ]?[ ]?(.*)")
  if not s then return nil end
  s = s:gsub("%s+$", "")
  if s:match("^=+$") then return 1 end
  if s:match("^%-+$") then return 2 end
  return nil
end

-- ─── List Marker Detection ────────────────────────────────────────────────────

--- Parse a list marker from a line.
-- Returns a table with: ordered, start, marker, marker_len, space_after, indent
-- or nil if no marker found.
local function parse_list_marker(line)
  -- Strip up to 3 leading spaces
  local sp_match = line:match("^([ ]?[ ]?[ ]?)")
  local spaces = sp_match or ""
  local rest = line:sub(#spaces + 1)

  -- Unordered: (- * +) + (one or more spaces OR single tab, or end-of-line)
  -- Note: per GFM spec, the space separator is either `+` spaces or
  -- exactly one tab character (not multiple tabs), matching the regex ( +|\t|$).
  local marker = rest:sub(1, 1)
  if marker == "-" or marker == "*" or marker == "+" then
    local after_marker = rest:sub(2)
    if after_marker == "" then
      -- blank-start item
      return {
        ordered = false, start = 1, marker = marker,
        marker_len = #spaces + 1, space_after = 0, indent = #spaces,
      }
    end
    local space_ch = after_marker:sub(1, 1)
    if space_ch == " " then
      -- One or more literal spaces
      local sp_after = after_marker:match("^( +)")
      local space_after_len = sp_after and #sp_after or 0
      return {
        ordered = false, start = 1, marker = marker,
        marker_len = #spaces + 1 + space_after_len,
        space_after = space_after_len, indent = #spaces,
      }
    elseif space_ch == "\t" then
      -- Exactly one tab character (not more)
      return {
        ordered = false, start = 1, marker = marker,
        marker_len = #spaces + 1 + 1,
        space_after = 1, indent = #spaces,
      }
    end
  end

  -- Ordered: 1-9 digits + (. or )) + (one or more spaces OR single tab, or end-of-line)
  local digits = rest:match("^(%d+)")
  if digits and #digits >= 1 and #digits <= 9 then
    local after_digits = rest:sub(#digits + 1)
    local delim = after_digits:sub(1, 1)
    if delim == "." or delim == ")" then
      local after_delim = after_digits:sub(2)
      if after_delim == "" then
        return {
          ordered = true, start = tonumber(digits), marker = delim,
          marker_len = #spaces + #digits + 1, space_after = 0, indent = #spaces,
        }
      end
      local space_ch = after_delim:sub(1, 1)
      if space_ch == " " then
        local sp_after = after_delim:match("^( +)")
        local space_after_len = sp_after and #sp_after or 0
        return {
          ordered = true, start = tonumber(digits), marker = delim,
          marker_len = #spaces + #digits + 1 + space_after_len,
          space_after = space_after_len, indent = #spaces,
        }
      elseif space_ch == "\t" then
        return {
          ordered = true, start = tonumber(digits), marker = delim,
          marker_len = #spaces + #digits + 1 + 1,
          space_after = 1, indent = #spaces,
        }
      end
    end
  end

  return nil
end

-- ─── HTML Block Detection ─────────────────────────────────────────────────────

-- GFM §4.6 defines 7 types of HTML blocks.
-- Each has different opening and closing conditions.

local HTML_BLOCK_6_TAGS = {
  address=true, article=true, aside=true, base=true, basefont=true,
  blockquote=true, body=true, caption=true, center=true, col=true,
  colgroup=true, dd=true, details=true, dialog=true, dir=true,
  div=true, dl=true, dt=true, fieldset=true, figcaption=true,
  figure=true, footer=true, form=true, frame=true, frameset=true,
  h1=true, h2=true, h3=true, h4=true, h5=true, h6=true,
  head=true, header=true, hr=true, html=true, iframe=true,
  legend=true, li=true, link=true, main=true, menu=true,
  menuitem=true, meta=true, nav=true, noframes=true, ol=true,
  optgroup=true, option=true, p=true, param=true, search=true,
  section=true, summary=true, table=true, tbody=true, td=true,
  tfoot=true, th=true, thead=true, title=true, tr=true,
  track=true, ul=true,
}

--- Detect the HTML block type for a line (1–7) or nil if not an HTML block.
local function detect_html_block_type(line)
  local stripped = line:match("^%s*(.-)%s*$")
  stripped = line:gsub("^%s+", "") -- trimStart

  -- Type 1: <script, <pre, <textarea, <style
  if stripped:match("^<[Ss][Cc][Rr][Ii][Pp][Tt][ \t>]") or
     stripped:match("^<[Ss][Cc][Rr][Ii][Pp][Tt]$") or
     stripped:match("^<[Pp][Rr][Ee][ \t>]") or
     stripped:match("^<[Pp][Rr][Ee]$") or
     stripped:match("^<[Tt][Ee][Xx][Tt][Aa][Rr][Ee][Aa][ \t>]") or
     stripped:match("^<[Tt][Ee][Xx][Tt][Aa][Rr][Ee][Aa]$") or
     stripped:match("^<[Ss][Tt][Yy][Ll][Ee][ \t>]") or
     stripped:match("^<[Ss][Tt][Yy][Ll][Ee]$") then
    return 1
  end

  -- Type 2: <!-- comment
  if stripped:match("^<!%-%-") then return 2 end
  -- Type 3: <? processing instruction
  if stripped:match("^<%?") then return 3 end
  -- Type 4: <!DECLARATION
  if stripped:match("^<![A-Z]") then return 4 end
  -- Type 5: <![CDATA[
  if stripped:match("^<!%[CDATA%[") then return 5 end

  -- Type 6: block-level tag
  local tag6 = stripped:match("^</?([a-zA-Z][a-zA-Z0-9]*)[ \t>/]") or
               stripped:match("^</?([a-zA-Z][a-zA-Z0-9]*)$")
  if tag6 and HTML_BLOCK_6_TAGS[tag6:lower()] then return 6 end

  -- Type 7: complete open or close tag (not in type 6 list), ends on blank line.
  -- The open tag must match: <tagname (whitespace+attr)* whitespace? /? >
  -- Attributes must begin with whitespace — this rules out <https://...> which
  -- has `:` immediately after the tag name.
  --
  -- We use a manual character-by-character scan to validate attribute syntax
  -- since Lua patterns don't support the complex attribute regex from the spec.
  do
    -- Close tag: </tagname (whitespace)* >   (no attributes allowed)
    local tag7_close = stripped:match("^</([a-zA-Z][a-zA-Z0-9%-]*)%s*> *$")
    if tag7_close and not HTML_BLOCK_6_TAGS[tag7_close:lower()] then
      return 7
    end
    -- Open tag: scan character by character to validate attribute syntax
    local tag7_name = stripped:match("^<([a-zA-Z][a-zA-Z0-9%-]*)")
    if tag7_name and not HTML_BLOCK_6_TAGS[tag7_name:lower()] then
      local pos = 1 + #tag7_name + 1  -- after '<' + tag_name
      -- After tag name we may have zero or more attributes, optional whitespace, /?, >
      -- Each attribute must be preceded by one or more spaces/tabs.
      local valid = true
      while pos <= #stripped do
        local ch = stripped:sub(pos, pos)
        if ch == ">" then
          -- Check nothing after `>`  (only trailing spaces allowed)
          local rest_after = stripped:sub(pos + 1)
          if rest_after:match("^%s*$") then
            return 7
          end
          valid = false
          break
        elseif ch == "/" then
          if stripped:sub(pos + 1, pos + 1) == ">" then
            local rest_after = stripped:sub(pos + 2)
            if rest_after:match("^%s*$") then
              return 7
            end
          end
          valid = false
          break
        elseif ch == " " or ch == "\t" then
          -- Skip whitespace
          while pos <= #stripped and (stripped:sub(pos,pos) == " " or stripped:sub(pos,pos) == "\t") do
            pos = pos + 1
          end
          if pos > #stripped then valid = false; break end
          local nc = stripped:sub(pos, pos)
          if nc == ">" then
            local rest_after = stripped:sub(pos + 1)
            if rest_after:match("^%s*$") then return 7 end
            valid = false; break
          elseif nc == "/" then
            if stripped:sub(pos+1,pos+1) == ">" then
              local rest_after = stripped:sub(pos + 2)
              if rest_after:match("^%s*$") then return 7 end
            end
            valid = false; break
          elseif nc:match("[a-zA-Z_:]") then
            -- Attribute name
            while pos <= #stripped and stripped:sub(pos,pos):match("[a-zA-Z0-9_:%-.%-]") do
              pos = pos + 1
            end
            -- Optional = value
            local p2 = pos
            while p2 <= #stripped and (stripped:sub(p2,p2) == " " or stripped:sub(p2,p2) == "\t") do
              p2 = p2 + 1
            end
            if p2 <= #stripped and stripped:sub(p2,p2) == "=" then
              p2 = p2 + 1
              while p2 <= #stripped and (stripped:sub(p2,p2) == " " or stripped:sub(p2,p2) == "\t") do
                p2 = p2 + 1
              end
              if p2 > #stripped then valid = false; break end
              local q = stripped:sub(p2, p2)
              if q == '"' or q == "'" then
                p2 = p2 + 1
                while p2 <= #stripped and stripped:sub(p2,p2) ~= q and stripped:sub(p2,p2) ~= "\n" do
                  p2 = p2 + 1
                end
                if p2 > #stripped or stripped:sub(p2,p2) ~= q then valid = false; break end
                p2 = p2 + 1
              else
                -- Unquoted value: no whitespace, ", ', =, <, >, backtick
                if q:match('[%s"\'=<>`]') then valid = false; break end
                while p2 <= #stripped and not stripped:sub(p2,p2):match('[%s"\'=<>`]') do
                  p2 = p2 + 1
                end
              end
              pos = p2
            else
              pos = p2
            end
          else
            valid = false; break
          end
        else
          valid = false; break
        end
      end
      if not valid then
        -- fall through
      end
    end
  end

  return nil
end

--- True if the HTML block with the given type ends on this line.
local function html_block_ends(line, html_type)
  if html_type == 1 then
    return line:lower():match("</script>") ~= nil or
           line:lower():match("</pre>") ~= nil or
           line:lower():match("</textarea>") ~= nil or
           line:lower():match("</style>") ~= nil
  elseif html_type == 2 then
    return line:match("%-%-!?>") ~= nil
  elseif html_type == 3 then
    return line:match("%?>") ~= nil
  elseif html_type == 4 then
    return line:match(">") ~= nil
  elseif html_type == 5 then
    return line:match("%]%]>") ~= nil
  elseif html_type == 6 or html_type == 7 then
    return is_blank(line)
  end
  return false
end

-- ─── Link Reference Definition Parsing ───────────────────────────────────────

--- Parse a link reference definition from text.
-- Returns {label, destination, title, chars_consumed} or nil.
local function parse_link_definition(text)
  -- Link label: up to 3 leading spaces + [...]:
  -- Manual scan (Lua patterns don't support {n,m} quantifiers)
  local i = 1
  -- Skip up to 3 leading spaces
  local space_count = 0
  while space_count < 3 and i <= #text and text:sub(i,i) == " " do
    space_count = space_count + 1
    i = i + 1
  end
  if i > #text or text:sub(i,i) ~= "[" then return nil end
  i = i + 1 -- skip [

  -- Scan the label
  local label_raw = ""
  local match_end
  while i <= #text do
    local ch = text:sub(i, i)
    if ch == "]" then
      i = i + 1
      break
    elseif ch == "[" then
      return nil -- unescaped [ in label is not allowed
    elseif ch == "\\" then
      i = i + 1
      if i <= #text then
        label_raw = label_raw .. "\\" .. text:sub(i, i)
        i = i + 1
      end
    else
      label_raw = label_raw .. ch
      i = i + 1
    end
  end
  if i > #text + 1 then return nil end  -- didn't find ]
  if text:sub(i-1, i-1) ~= "]" then return nil end
  if text:sub(i, i) ~= ":" then return nil end
  i = i + 1  -- skip :
  match_end = i - 1

  if label_raw:match("^%s*$") then return nil end -- empty label not allowed
  local label = normalize_link_label(label_raw)
  local pos = match_end + 1  -- 1-based position after the colon (match_end = pos of :)

  -- Skip whitespace (including one newline)
  local ws = text:sub(pos):match("^[ \t]*\n?[ \t]*")
  if ws then pos = pos + #ws end

  -- Destination
  local destination = ""
  local dest_char = text:sub(pos, pos)

  if dest_char == "<" then
    -- Angle-bracket destination: <url>
    local inner = text:sub(pos + 1):match("^([^<>\n\\]*(?:\\.[^<>\n\\]*)*)>")
    if not inner then
      -- Try a simpler pattern
      local ang_end = text:find(">", pos + 1, true)
      if not ang_end then return nil end
      local ang_content = text:sub(pos + 1, ang_end - 1)
      if ang_content:find("\n") then return nil end
      if ang_content:find("<") then return nil end
      destination = normalize_url(decode_entities(apply_backslash_escapes(ang_content)))
      pos = ang_end + 1
    else
      destination = normalize_url(decode_entities(apply_backslash_escapes(inner)))
      pos = pos + 1 + #inner + 1
    end
  else
    -- Bare destination: no spaces, no control chars, balanced parens
    local depth = 0
    local start = pos
    while pos <= #text do
      local ch = text:sub(pos, pos)
      if ch == "(" then
        depth = depth + 1
        pos = pos + 1
      elseif ch == ")" then
        if depth == 0 then break end
        depth = depth - 1
        pos = pos + 1
      elseif ch:match("[\000-\031%s]") then
        break
      elseif ch == "\\" then
        pos = pos + 2 -- skip \X pair
      else
        pos = pos + 1
      end
    end
    if pos == start then return nil end -- empty destination
    local raw_dest = text:sub(start, pos - 1)
    destination = normalize_url(decode_entities(apply_backslash_escapes(raw_dest)))
  end

  -- Optional title
  local title = nil
  local before_title_pos = pos
  local sp2 = text:sub(pos):match("^[ \t]*\n?[ \t]*")
  if sp2 and #sp2 > 0 then
    pos = pos + #sp2
    local title_char = text:sub(pos, pos)
    local close_char = ""
    if title_char == '"' then close_char = '"'
    elseif title_char == "'" then close_char = "'"
    elseif title_char == "(" then close_char = ")"
    end

    if close_char ~= "" then
      pos = pos + 1 -- skip open char
      local title_start = pos
      local escaped = false
      local found_close = false
      local prev_was_newline = false
      while pos <= #text do
        local ch = text:sub(pos, pos)
        if escaped then
          escaped = false
          prev_was_newline = false
          pos = pos + 1
        elseif ch == "\\" then
          escaped = true
          prev_was_newline = false
          pos = pos + 1
        elseif ch == close_char then
          title = decode_entities(apply_backslash_escapes(text:sub(title_start, pos - 1)))
          pos = pos + 1
          found_close = true
          break
        elseif ch == "\n" then
          if prev_was_newline or close_char == ")" then
            -- Blank line (two newlines in a row) or paren title → fail
            break
          end
          -- Check if next line is blank (starts with \n)
          if pos + 1 <= #text and text:sub(pos + 1, pos + 1) == "\n" then
            break
          end
          prev_was_newline = true
          pos = pos + 1
        else
          prev_was_newline = false
          pos = pos + 1
        end
      end
      if not found_close then
        -- Failed to parse title
        pos = before_title_pos
        title = nil
      end
    else
      pos = before_title_pos
    end
  end

  -- Must be followed by only whitespace until end of line (no other text).
  -- Returns (ok, consumed_chars) where consumed_chars includes the newline.
  local function check_eol(from_pos)
    local rest = text:sub(from_pos)
    -- Skip spaces/tabs
    local sp = rest:match("^([ \t]*)")
    local after_sp = rest:sub(#sp + 1)
    -- Next must be newline or end of string
    if after_sp == "" or after_sp:sub(1, 1) == "\n" then
      local nl_len = after_sp:sub(1, 1) == "\n" and 1 or 0
      return true, #sp + nl_len
    end
    return false, 0
  end

  local ok, eol_len = check_eol(pos)
  if not ok then
    if title ~= nil then
      -- Retry without the title
      pos = before_title_pos
      title = nil
      ok, eol_len = check_eol(pos)
      if not ok then return nil end
      pos = pos + eol_len
    else
      return nil
    end
  else
    pos = pos + eol_len
  end

  return { label = label, destination = destination, title = title, chars_consumed = pos - 1 }
end

-- ─── Phase 1: Block Parser ────────────────────────────────────────────────────
--
-- Container blocks (document, blockquote, list items) form a stack.
-- When a new line arrives, we walk down the stack checking continuations,
-- then add the line's content to the appropriate block.

--- Parse blocks from the normalized input.
-- Returns {document=mutable_doc, link_refs=map}.
local function parse_blocks(input)
  -- Normalize line endings to LF
  local normalized = input:gsub("\r\n", "\n"):gsub("\r", "\n")
  -- Split into lines (without trailing newline)
  local raw_lines = {}
  for line in (normalized .. "\n"):gmatch("([^\n]*)\n") do
    raw_lines[#raw_lines + 1] = line
  end
  -- Remove spurious trailing empty line from the split
  if #raw_lines > 0 and raw_lines[#raw_lines] == "" then
    raw_lines[#raw_lines] = nil
  end

  local link_refs = {} -- map: normalized_label -> {destination, title}
  local root = { kind = "document", children = {} }

  -- Container block stack. Innermost open container is at the end.
  local open_containers = { root }

  -- Track the current open leaf block (paragraph, code block, etc.)
  local current_leaf = nil

  -- Track blank lines for list tightness
  local last_line_was_blank = false
  local last_blank_inner_container = root

  -- HTML block state
  local html_block_mode = false    -- are we inside an HTML block?
  local html_block_type = nil
  -- Fenced code state
  local fenced_mode = false        -- are we inside a fenced code block?

  -- ─── Container helpers ───────────────────────────────────────────────────

  local function last_child(container)
    if container.kind == "document" or container.kind == "blockquote" or container.kind == "list_item" then
      local ch = container.children
      return ch[#ch]
    end
    return nil
  end

  local function add_child(container, block)
    if container.kind == "document" or container.kind == "blockquote" or container.kind == "list_item" then
      container.children[#container.children + 1] = block
    end
  end

  local function remove_last_child(container)
    if container.kind == "document" or container.kind == "blockquote" or container.kind == "list_item" then
      container.children[#container.children] = nil
    end
  end

  -- Forward declaration for finalize_block
  local finalize_block

  local function close_paragraph(leaf, container)
    if leaf and leaf.kind == "paragraph" then
      finalize_block(leaf, container)
    elseif leaf and leaf.kind == "indented_code" then
      -- Trim trailing blank lines
      while #leaf.lines > 0 and leaf.lines[#leaf.lines]:match("^%s*$") do
        leaf.lines[#leaf.lines] = nil
      end
    end
  end

  finalize_block = function(block, container)
    if block.kind == "paragraph" then
      -- Extract link reference definitions from the paragraph
      local text = table.concat(block.lines, "\n")
      while true do
        local def = parse_link_definition(text)
        if not def then break end
        if not link_refs[def.label] then
          link_refs[def.label] = { destination = def.destination, title = def.title }
        end
        text = text:sub(def.chars_consumed + 1)
      end
      -- Update paragraph lines with remaining text
      if text:match("^%s*$") then
        block.lines = {}
      else
        local lines = {}
        for line in (text .. "\n"):gmatch("([^\n]*)\n") do
          lines[#lines + 1] = line
        end
        -- Remove trailing empty line from split
        if #lines > 0 and lines[#lines] == "" then
          lines[#lines] = nil
        end
        if #lines > 0 then
          lines[#lines] = lines[#lines]:gsub("%s+$", "")
        end
        block.lines = lines
      end
    elseif block.kind == "indented_code" then
      -- Trim trailing blank lines
      while #block.lines > 0 and block.lines[#block.lines] == "" do
        block.lines[#block.lines] = nil
      end
    end
  end

  -- ─── Line processing ─────────────────────────────────────────────────────

  for line_idx = 1, #raw_lines do
    local raw_line = raw_lines[line_idx]
    local orig_blank = is_blank(raw_line)

    -- ── Container continuation ─────────────────────────────────────────
    local line_content = raw_line
    local line_base_col = 0
    local new_containers = { root }
    local lazy_para_continuation = false

    local container_idx = 2
    while container_idx <= #open_containers do
      local container = open_containers[container_idx]

      if container.kind == "blockquote" then
        -- Strip blockquote marker `> ` (up to 3 leading spaces, `>`, then optional space)
        local bq_i = 1
        local bq_col = line_base_col
        -- Strip 0-3 leading spaces
        while bq_i <= 3 and bq_i <= #line_content and line_content:sub(bq_i, bq_i) == " " do
          bq_i = bq_i + 1
          bq_col = bq_col + 1
        end
        if bq_i <= #line_content and line_content:sub(bq_i, bq_i) == ">" then
          bq_i = bq_i + 1
          bq_col = bq_col + 1
          if bq_i <= #line_content then
            if line_content:sub(bq_i, bq_i) == " " then
              bq_i = bq_i + 1
              bq_col = bq_col + 1
            elseif line_content:sub(bq_i, bq_i) == "\t" then
              local w = 4 - (bq_col % 4)
              bq_i = bq_i + 1
              if w > 1 then
                line_content = string.rep(" ", w - 1) .. line_content:sub(bq_i)
                line_base_col = bq_col + 1
                new_containers[#new_containers + 1] = container
                container_idx = container_idx + 1
                goto continue_container
              end
              bq_col = bq_col + w
            end
          end
          line_content = line_content:sub(bq_i)
          line_base_col = bq_col
          new_containers[#new_containers + 1] = container
          container_idx = container_idx + 1
        elseif current_leaf and current_leaf.kind == "paragraph" and not orig_blank
            and not is_thematic_break(line_content)
            and not (indent_of(line_content, line_base_col) < 4 and line_content:gsub("^%s+", ""):match("^[`~][`~][`~]"))
            and not parse_atx_heading(line_content) then
          -- Lazy paragraph continuation in blockquote
          local lm = parse_list_marker(line_content)
          local lm_blank = lm and is_blank(line_content:sub(lm.marker_len + 1))
          if not lm or lm_blank then
            new_containers[#new_containers + 1] = container
            container_idx = container_idx + 1
            lazy_para_continuation = true
            break
          end
          break
        else
          break
        end
      elseif container.kind == "list" then
        -- Lists pass through; tightness determined by list item continuation
        new_containers[#new_containers + 1] = container
        container_idx = container_idx + 1
      elseif container.kind == "list_item" then
        local item = container
        local effective_blank = orig_blank or is_blank(line_content)
        local ind = indent_of(line_content, line_base_col)
        if not effective_blank and ind >= item.content_indent then
          line_content, line_base_col = strip_indent(line_content, item.content_indent, line_base_col)
          new_containers[#new_containers + 1] = container
          container_idx = container_idx + 1
        elseif effective_blank then
          if #item.children > 0 or (current_leaf ~= nil and item == open_containers[container_idx]) then
            new_containers[#new_containers + 1] = container
            container_idx = container_idx + 1
          else
            break -- blank-start item: blank line closes it
          end
        elseif current_leaf and current_leaf.kind == "paragraph" and not orig_blank
            and not is_thematic_break(line_content)
            and not parse_list_marker(line_content)
            and not (indent_of(line_content, line_base_col) < 4 and line_content:gsub("^%s+", ""):match("^[`~][`~][`~]"))
            and not parse_atx_heading(line_content) then
          -- Lazy paragraph continuation for list items
          new_containers[#new_containers + 1] = container
          container_idx = container_idx + 1
          lazy_para_continuation = true
          break
        else
          break
        end
      else
        break
      end

      ::continue_container::
    end

    local prev_inner_container = open_containers[#open_containers]
    open_containers = new_containers

    -- Re-check blank status after stripping container markers
    local blank = orig_blank
    if not blank and is_blank(line_content) then
      blank = true
    end

    local current_inner = open_containers[#open_containers]

    -- ── Multi-line block continuation ──────────────────────────────────
    if fenced_mode and current_leaf and current_leaf.kind == "fenced_code" then
      local fence = current_leaf
      if current_inner ~= prev_inner_container then
        -- Container was dropped — force-close the fence
        fence.closed = true
        fenced_mode = false
        current_leaf = nil
        -- Fall through to normal processing
      else
        local stripped_content = line_content:gsub("^%s+", "")
        -- Does this line close the fence?
        local fence_char = fence.fence:sub(1, 1)
        local fence_len = fence.fence_len
        local closing_pattern = "^" .. (fence_char == "`" and "`" or "~") .. string.rep(fence_char == "`" and "`" or "~", fence_len - 1)
        local ind_lc = indent_of(line_content, line_base_col)
        if ind_lc < 4 and stripped_content:match(closing_pattern) then
          -- Verify the closing fence has the right character and only whitespace after
          local fence_match = stripped_content:match("^([`~]+)%s*$")
          if fence_match and #fence_match >= fence_len and fence_match:sub(1,1) == fence_char then
            fence.closed = true
            fenced_mode = false
            current_leaf = nil
            last_line_was_blank = orig_blank
            goto next_line
          end
        end
        -- Strip base indentation from content line
        local fence_line, _ = strip_indent(line_content, fence.base_indent, line_base_col)
        fence.lines[#fence.lines + 1] = fence_line
        last_line_was_blank = orig_blank
        goto next_line
      end
    end

    if html_block_mode and current_leaf and current_leaf.kind == "html_block" then
      local html_block = current_leaf
      if current_inner ~= prev_inner_container then
        html_block.closed = true
        html_block_mode = false
        current_leaf = nil
        -- Fall through
      else
        html_block.lines[#html_block.lines + 1] = line_content
        if html_block_ends(line_content, html_block.html_type) then
          html_block.closed = true
          html_block_mode = false
          current_leaf = nil
        end
        last_line_was_blank = orig_blank
        goto next_line
      end
    end

    -- Finalize current leaf if we left its container
    if current_inner ~= prev_inner_container and current_leaf ~= nil and not lazy_para_continuation then
      finalize_block(current_leaf, prev_inner_container)
      current_leaf = nil
    end

    -- ── Lazy paragraph continuation ──────────────────────────────────
    if lazy_para_continuation and current_leaf and current_leaf.kind == "paragraph" then
      current_leaf.lines[#current_leaf.lines + 1] = line_content
      last_line_was_blank = false
      goto next_line
    end

    -- Close list that can't continue
    while not blank and #open_containers > 1 and open_containers[#open_containers].kind == "list" do
      local list = open_containers[#open_containers]
      local marker = parse_list_marker(line_content)
      if marker and list.ordered == marker.ordered and list.marker == marker.marker
          and not is_thematic_break(line_content) then
        break -- will add a new item
      end
      open_containers[#open_containers] = nil
    end

    local inner_container = open_containers[#open_containers]

    -- ── Blank line handling ──────────────────────────────────────────
    if blank then
      if current_leaf and current_leaf.kind == "paragraph" then
        finalize_block(current_leaf, inner_container)
        current_leaf = nil
      elseif current_leaf and current_leaf.kind == "indented_code" then
        -- Blank lines inside indented code are preserved
        local blank_code_line, _ = strip_indent(raw_line, 4)
        current_leaf.lines[#current_leaf.lines + 1] = blank_code_line
      end

      if inner_container.kind == "list_item" then
        inner_container.had_blank_line = true
      end
      if inner_container.kind == "list" then
        inner_container.had_blank_line = true
      end

      last_line_was_blank = true
      last_blank_inner_container = inner_container
      goto next_line
    end

    -- ── New block detection ──────────────────────────────────────────
    do
      -- Use a repeat/until false with break to simulate TypeScript's labeled while loop
      local restart = true
      while restart do
        restart = false

        -- After a blank line in a list, make the list loose
        if last_line_was_blank and inner_container.kind == "list"
            and (last_blank_inner_container.kind == "list"
                or last_blank_inner_container.kind == "list_item") then
          inner_container.tight = false
        end

        if last_line_was_blank and inner_container.kind == "list_item" then
          inner_container.had_blank_line = true
        end

        local indent = indent_of(line_content, line_base_col)

        -- 1. Fenced code block opener
        local stripped_lc = line_content:gsub("^%s+", "")
        local fence_match = stripped_lc:match("^([`~][`~][`~]+)")
        if fence_match and indent < 4 then
          local fence_char = fence_match:sub(1, 1)
          local fence_len = #fence_match
          local info_line = stripped_lc:sub(fence_len + 1)
          local info_string = extract_info_string(line_content)

          -- Backtick fences cannot have backticks in info string
          if fence_char == "`" and info_line:find("`", 1, true) then
            -- fall through to paragraph
          else
            close_paragraph(current_leaf, inner_container)
            current_leaf = nil

            local fenced_block = {
              kind = "fenced_code",
              fence = fence_char:rep(fence_len),
              fence_len = fence_len,
              base_indent = indent,
              info_string = info_string,
              lines = {},
              closed = false,
            }
            add_child(inner_container, fenced_block)
            current_leaf = fenced_block
            fenced_mode = true
            last_line_was_blank = false
            break
          end
        end

        -- 2. ATX heading
        if indent < 4 then
          local heading = parse_atx_heading(line_content)
          if heading then
            close_paragraph(current_leaf, inner_container)
            current_leaf = nil
            local heading_block = { kind = "heading", level = heading.level, content = heading.content }
            add_child(inner_container, heading_block)
            current_leaf = nil
            last_line_was_blank = false
            break
          end
        end

        -- 3. Thematic break (check before list marker)
        if indent < 4 and is_thematic_break(line_content) then
          if current_leaf and current_leaf.kind == "paragraph" then
            local level = is_setext_underline(line_content)
            if level ~= nil then
              local para = current_leaf
              finalize_block(para, inner_container)
              if #para.lines > 0 then
                local heading_block = {
                  kind = "heading",
                  level = level,
                  content = table.concat(para.lines, "\n"):match("^%s*(.-)%s*$"),
                }
                remove_last_child(inner_container)
                add_child(inner_container, heading_block)
                current_leaf = nil
                last_line_was_blank = false
                break
              end
              remove_last_child(inner_container)
              current_leaf = nil
            end
          end

          close_paragraph(current_leaf, inner_container)
          current_leaf = nil
          add_child(inner_container, { kind = "thematic_break" })
          last_line_was_blank = false
          break
        end

        -- 4. Setext heading underline (when no thematic break matched)
        if indent < 4 and current_leaf and current_leaf.kind == "paragraph" then
          local level = is_setext_underline(line_content)
          if level ~= nil then
            local para = current_leaf
            finalize_block(para, inner_container)
            if #para.lines > 0 then
              local heading_block = {
                kind = "heading",
                level = level,
                content = table.concat(para.lines, "\n"):match("^%s*(.-)%s*$"),
              }
              remove_last_child(inner_container)
              add_child(inner_container, heading_block)
              current_leaf = nil
              last_line_was_blank = false
              break
            end
            remove_last_child(inner_container)
            current_leaf = nil
          end
        end

        -- 5. HTML block
        if indent < 4 then
          local html_type = detect_html_block_type(line_content)
          if html_type ~= nil then
            -- Type 7 cannot interrupt a paragraph
            if html_type ~= 7 or not (current_leaf and current_leaf.kind == "paragraph") then
              close_paragraph(current_leaf, inner_container)
              current_leaf = nil

              local html_block = {
                kind = "html_block",
                html_type = html_type,
                lines = { line_content },
                closed = html_block_ends(line_content, html_type),
              }
              add_child(inner_container, html_block)

              if not html_block.closed then
                current_leaf = html_block
                html_block_mode = true
              end
              last_line_was_blank = false
              break
            end
          end
        end

        -- 6. Blockquote
        if indent < 4 and line_content:gsub("^%s+", ""):sub(1,1) == ">" then
          close_paragraph(current_leaf, inner_container)
          current_leaf = nil

          local bq
          local bq_last = last_child(inner_container)
          if bq_last and bq_last.kind == "blockquote" and not last_line_was_blank then
            bq = bq_last
          else
            bq = { kind = "blockquote", children = {} }
            add_child(inner_container, bq)
          end

          open_containers[#open_containers + 1] = bq

          -- Strip the > marker
          local bq_i = 1
          local bq_col = line_base_col
          while bq_i <= 3 and bq_i <= #line_content and line_content:sub(bq_i, bq_i) == " " do
            bq_i = bq_i + 1
            bq_col = bq_col + 1
          end
          if bq_i <= #line_content and line_content:sub(bq_i, bq_i) == ">" then
            bq_i = bq_i + 1
            bq_col = bq_col + 1
            if bq_i <= #line_content then
              if line_content:sub(bq_i, bq_i) == " " then
                bq_i = bq_i + 1
                bq_col = bq_col + 1
              elseif line_content:sub(bq_i, bq_i) == "\t" then
                local w = 4 - (bq_col % 4)
                bq_i = bq_i + 1
                if w > 1 then
                  line_content = string.rep(" ", w - 1) .. line_content:sub(bq_i)
                  line_base_col = bq_col + 1
                  inner_container = bq
                  if is_blank(line_content) then break end
                  restart = true
                  goto continue_block_detect
                end
                bq_col = bq_col + w
              end
            end
          end
          line_content = line_content:sub(bq_i)
          line_base_col = bq_col
          inner_container = bq

          if is_blank(line_content) then
            last_line_was_blank = false
            break
          end

          restart = true
          goto continue_block_detect
        end

        -- 7. List item
        if indent < 4 then
          local marker = parse_list_marker(line_content)
          if marker ~= nil then
            local list = nil

            if inner_container.kind == "list" then
              local existing = inner_container
              if existing.ordered == marker.ordered and existing.marker == marker.marker then
                list = existing
              end
            end
            if list == nil then
              local list_last = last_child(inner_container)
              if list_last and list_last.kind == "list" then
                local existing = list_last
                if existing.ordered == marker.ordered and existing.marker == marker.marker then
                  list = existing
                end
              end
            end

            local new_line_base_col = virtual_col_after(line_content, marker.marker_len, line_base_col)
            local item_content = line_content:sub(marker.marker_len + 1)

            -- Handle tab separator
            if marker.space_after == 1 then
              local sep_char = line_content:sub(marker.marker_len, marker.marker_len)
              if sep_char == "\t" then
                local sep_col = virtual_col_after(line_content, marker.marker_len - 1, line_base_col)
                local w = 4 - (sep_col % 4)
                if w > 1 then
                  item_content = string.rep(" ", w - 1) .. item_content
                  new_line_base_col = sep_col + 1
                end
              end
            end

            local blank_start = is_blank(item_content)

            -- Empty list items cannot interrupt a paragraph to start a NEW list
            local para_in_current = current_leaf and current_leaf.kind == "paragraph"
                                    and last_child(inner_container) == current_leaf
            local can_interrupt_para = (not marker.ordered or marker.start == 1 or list ~= nil)
              and (not blank_start or not para_in_current)

            if not (current_leaf and current_leaf.kind == "paragraph") or can_interrupt_para then
              if list == nil then
                close_paragraph(current_leaf, inner_container)
                current_leaf = nil
                list = {
                  kind = "list",
                  ordered = marker.ordered,
                  marker = marker.marker,
                  start = marker.start,
                  tight = true,
                  items = {},
                  had_blank_line = false,
                }
                add_child(inner_container, list)
              else
                close_paragraph(current_leaf, inner_container)
                current_leaf = nil
                if list.had_blank_line
                    or (last_line_was_blank
                        and (last_blank_inner_container.kind == "list"
                            or last_blank_inner_container.kind == "list_item")) then
                  list.tight = false
                end
                list.had_blank_line = false
              end

              -- Compute content indent (W+1 rule)
              local normal_indent = marker.marker_len
              local reduced_indent = marker.marker_len - marker.space_after + 1
              local content_indent = (blank_start or marker.space_after >= 5) and reduced_indent or normal_indent

              local item = {
                kind = "list_item",
                marker = marker.marker,
                marker_indent = marker.indent,
                content_indent = content_indent,
                children = {},
                had_blank_line = false,
              }
              list.items[#list.items + 1] = item

              if inner_container ~= list then
                open_containers[#open_containers + 1] = list
              end
              open_containers[#open_containers + 1] = item

              if not blank_start then
                inner_container = item
                if marker.space_after >= 5 then
                  line_base_col = virtual_col_after(line_content, marker.marker_len - marker.space_after + 1, line_base_col)
                  line_content = string.rep(" ", marker.space_after - 1) .. item_content
                else
                  line_base_col = new_line_base_col
                  line_content = item_content
                end
                restart = true
                goto continue_block_detect
              end
              current_leaf = nil
              last_line_was_blank = false
              break
            end
          end
        end

        -- 8. Indented code block (4+ spaces, but NOT inside a paragraph)
        if indent >= 4 and not (current_leaf and current_leaf.kind == "paragraph") then
          local stripped_for_code, _ = strip_indent(line_content, 4, line_base_col)
          if current_leaf and current_leaf.kind == "indented_code" then
            current_leaf.lines[#current_leaf.lines + 1] = stripped_for_code
          else
            close_paragraph(current_leaf, inner_container)
            local icb = { kind = "indented_code", lines = { stripped_for_code } }
            add_child(inner_container, icb)
            current_leaf = icb
          end
          last_line_was_blank = false
          break
        end

        -- 9. Paragraph continuation or new paragraph
        if current_leaf and current_leaf.kind == "paragraph" then
          current_leaf.lines[#current_leaf.lines + 1] = line_content
        else
          close_paragraph(current_leaf, inner_container)
          local para = { kind = "paragraph", lines = { line_content } }
          add_child(inner_container, para)
          current_leaf = para
        end

        last_line_was_blank = false
        break

        ::continue_block_detect::
      end
    end

    ::next_line::
  end

  -- Finalize any remaining open leaf block
  if current_leaf ~= nil then
    local inner_container = open_containers[#open_containers]
    finalize_block(current_leaf, inner_container)
  end

  return { document = root, link_refs = link_refs }
end

-- ─── Phase 2: Inline Parser ──────────────────────────────────────────────────
--
-- Scan raw inline content strings and emit inline AST nodes.
-- Implements the GFM delimiter stack algorithm for emphasis.

--- Try to parse a code span starting at the scanner's current position.
-- Returns a code_span node or nil.
local function try_code_span(sc)
  local saved_pos = sc.pos

  local open_ticks = sc:consume_while(function(c) return c == "`" end)
  local tick_len = #open_ticks

  local content = ""
  while not sc:done() do
    if sc:peek() == "`" then
      local close_pos = sc.pos
      local close_ticks = sc:consume_while(function(c) return c == "`" end)
      if #close_ticks == tick_len then
        -- Matching close found
        -- Normalize line endings → spaces
        content = content:gsub("\r\n", " "):gsub("\r", " "):gsub("\n", " ")
        -- Strip one leading+trailing space if content is not all-space
        if #content >= 2 and content:sub(1,1) == " " and content:sub(-1,-1) == " "
            and content:match("[^ ]") then
          content = content:sub(2, -2)
        end
        return ast.code_span(content)
      end
      -- Wrong number of backticks — treat as content
      content = content .. close_ticks
    else
      content = content .. sc:advance()
    end
  end

  -- No matching close found
  sc.pos = saved_pos
  return nil
end

--- Try to parse an HTML inline construct starting at `<`.
local function try_html_inline(sc)
  if sc:peek() ~= "<" then return nil end
  local saved_pos = sc.pos
  sc:skip(1) -- consume `<`

  local ch = sc:peek()

  -- HTML comment: <!-- ... -->
  if sc:match("!--") then
    local content_start = sc.pos
    if sc:peek() == ">" or sc:peek_slice(2) == "->" then
      -- Invalid comment starter — emit as raw HTML
      local invalid = sc:peek() == ">" and ">" or "->"
      sc:skip(#invalid)
      return ast.raw_inline("html", sc.source:sub(saved_pos, sc.pos - 1))
    end
    while not sc:done() do
      if sc:match("-->") then
        local content = sc.source:sub(content_start, sc.pos - 4)
        if content:sub(-1) == "-" then sc.pos = saved_pos; return nil end
        return ast.raw_inline("html", sc.source:sub(saved_pos, sc.pos - 1))
      end
      sc:skip(1)
    end
    sc.pos = saved_pos
    return nil
  end

  -- Processing instruction: <? ... ?>
  if sc:match("?") then
    while not sc:done() do
      if sc:match("?>") then
        return ast.raw_inline("html", sc.source:sub(saved_pos, sc.pos - 1))
      end
      sc:skip(1)
    end
    sc.pos = saved_pos
    return nil
  end

  -- CDATA section: <![CDATA[ ... ]]>
  if sc:match("![CDATA[") then
    while not sc:done() do
      if sc:match("]]>") then
        return ast.raw_inline("html", sc.source:sub(saved_pos, sc.pos - 1))
      end
      sc:skip(1)
    end
    sc.pos = saved_pos
    return nil
  end

  -- Declaration: <!UPPER...>
  if sc:match("!") then
    if sc:peek():match("[A-Z]") then
      sc:consume_while(function(c) return c ~= ">" end)
      if sc:match(">") then
        return ast.raw_inline("html", sc.source:sub(saved_pos, sc.pos - 1))
      end
    end
    sc.pos = saved_pos
    return nil
  end

  -- Closing tag: </tagname>
  if ch == "/" then
    sc:skip(1)
    local tag = sc:consume_while(function(c) return c:match("[a-zA-Z0-9%-]") ~= nil end)
    if #tag == 0 then sc.pos = saved_pos; return nil end
    sc:skip_spaces()
    if not sc:match(">") then sc.pos = saved_pos; return nil end
    return ast.raw_inline("html", sc.source:sub(saved_pos, sc.pos - 1))
  end

  -- Open tag: <tagname attr...> or <tagname attr.../>
  if ch:match("[a-zA-Z]") then
    local tag_name = sc:consume_while(function(c) return c:match("[a-zA-Z0-9%-]") ~= nil end)
    if #tag_name == 0 then sc.pos = saved_pos; return nil end

    local newlines_in_tag = 0

    while true do
      local space_len = sc:skip_spaces()
      -- Allow at most one newline anywhere in the attribute area
      if newlines_in_tag == 0 and sc:peek() == "\n" then
        newlines_in_tag = newlines_in_tag + 1
        sc:skip(1)
        space_len = space_len + 1 + sc:skip_spaces()
      end
      local next = sc:peek()
      if next == ">" or next == "/" or next == "" then break end
      if next == "\n" then sc.pos = saved_pos; return nil end
      if space_len == 0 then sc.pos = saved_pos; return nil end

      -- Attribute name: must start with ASCII alpha, `_`, or `:`
      if not next:match("[a-zA-Z_:]") then sc.pos = saved_pos; return nil end
      sc:consume_while(function(c) return c:match("[a-zA-Z0-9_:%.%-]") ~= nil end)

      -- Optional `= value`
      local pos_before_eq = sc.pos
      sc:skip_spaces()
      if sc:peek() == "=" then
        sc:skip(1) -- consume `=`
        sc:skip_spaces()
        local q = sc:peek()
        if q == '"' or q == "'" then
          sc:skip(1)
          local closed = false
          while not sc:done() do
            local vc = sc.source:sub(sc.pos, sc.pos)
            if vc == q then sc:skip(1); closed = true; break end
            if vc == "\n" then
              if newlines_in_tag >= 1 then sc.pos = saved_pos; return nil end
              newlines_in_tag = newlines_in_tag + 1
            end
            sc:skip(1)
          end
          if not closed then sc.pos = saved_pos; return nil end
        else
          -- Unquoted value
          local unquoted = sc:consume_while(function(c) return not c:match('[%s"\'=<>`]') end)
          if #unquoted == 0 then sc.pos = saved_pos; return nil end
        end
      else
        sc.pos = pos_before_eq
      end
    end

    local self_close = sc:match("/>")
    if not self_close and not sc:match(">") then sc.pos = saved_pos; return nil end
    return ast.raw_inline("html", sc.source:sub(saved_pos, sc.pos - 1))
  end

  sc.pos = saved_pos
  return nil
end

--- Try to parse an autolink starting at `<`.
local function try_autolink(sc)
  if sc:peek() ~= "<" then return nil end
  local saved_pos = sc.pos
  sc:skip(1)

  local start = sc.pos

  -- Try email autolink: local@domain
  local local_part = sc:consume_while(function(c) return not c:match("[%s<>@]") end)
  if #local_part > 0 and sc:peek() == "@" then
    sc:skip(1)
    local domain_part = sc:consume_while(function(c) return not c:match("[%s<>]") end)
    if #domain_part > 0 and sc:match(">") then
      -- Validate
      if local_part:match("^[a-zA-Z0-9%.!#$%%&'*+/=?^_`{|}~%-]+$") and
         domain_part:match("^[a-zA-Z0-9][a-zA-Z0-9%-]*[a-zA-Z0-9]?%.[a-zA-Z0-9]") then
        return ast.autolink(local_part .. "@" .. domain_part, true)
      end
    end
  end

  -- Retry as URL autolink
  sc.pos = start
  local scheme = sc:consume_while(function(c) return c:match("[a-zA-Z0-9+%-%.]") ~= nil end)
  if #scheme >= 2 and #scheme <= 32 and sc:match(":") then
    local path = sc:consume_while(function(c) return c ~= " " and c ~= "<" and c ~= ">" and c ~= "\n" end)
    if sc:match(">") then
      return ast.autolink(scheme .. ":" .. path, false)
    end
  end

  sc.pos = saved_pos
  return nil
end

--- Skip ASCII spaces/tabs and at most one line ending.
local function skip_optional_spaces_and_newline(sc)
  sc:skip_spaces()
  if sc:peek() == "\n" then
    sc:skip(1)
    sc:skip_spaces()
  elseif sc:peek() == "\r" and sc:peek(1) == "\n" then
    sc:skip(2)
    sc:skip_spaces()
  end
end

--- Try to parse a link/image destination and title after `]`.
-- Returns {destination, title} or nil.
local function try_link_after_close(sc, link_refs, inner_text)
  local saved_pos = sc.pos

  -- ── Inline link: ( destination "title" ) ─────────────────────────────────
  if sc:peek() == "(" then
    local function try_inline_link()
      sc:skip(1) -- consume `(`
      skip_optional_spaces_and_newline(sc)

      local destination = ""

      if sc:peek() == "<" then
        -- Angle-bracket destination
        sc:skip(1)
        local dest_buf = ""
        while not sc:done() do
          local c = sc:peek()
          if c == "\n" or c == "\r" then return nil end
          if c == "\\" then
            sc:skip(1)
            local next = sc:advance()
            dest_buf = dest_buf .. (is_ascii_punctuation(next) and next or "\\" .. next)
          elseif c == ">" then
            sc:skip(1)
            break
          elseif c == "<" then
            return nil
          else
            dest_buf = dest_buf .. sc:advance()
          end
        end
        destination = normalize_url(decode_entities(dest_buf))
      else
        -- Bare destination
        local depth = 0
        local dest_start = sc.pos
        while not sc:done() do
          local c = sc:peek()
          if c == "(" then depth = depth + 1; sc:skip(1)
          elseif c == ")" then
            if depth == 0 then break end
            depth = depth - 1; sc:skip(1)
          elseif c == "\\" then sc:skip(2)
          elseif is_ascii_whitespace(c) then break
          else sc:skip(1)
          end
        end
        local dest_raw = sc.source:sub(dest_start, sc.pos - 1)
        destination = normalize_url(decode_entities(apply_backslash_escapes(dest_raw)))
      end

      skip_optional_spaces_and_newline(sc)

      -- Optional title
      local title = nil
      local q = sc:peek()
      if q == '"' or q == "'" or q == "(" then
        local close_q = q == "(" and ")" or q
        sc:skip(1)
        local title_buf = ""
        while not sc:done() do
          local c = sc:peek()
          if c == "\\" then
            sc:skip(1)
            local next = sc:advance()
            title_buf = title_buf .. (is_ascii_punctuation(next) and next or "\\" .. next)
          elseif c == close_q then
            sc:skip(1)
            title = decode_entities(title_buf)
            break
          elseif c == "\n" and q == "(" then
            break
          else
            title_buf = title_buf .. sc:advance()
          end
        end
      end

      sc:skip_spaces()
      if not sc:match(")") then return nil end
      return { destination = destination, title = title }
    end

    local result = try_inline_link()
    if result ~= nil then return result end
    sc.pos = saved_pos
  end

  -- ── Full reference: [label] or Collapsed reference: [] ───────────────────
  if sc:peek() == "[" then
    sc:skip(1)
    local label_buf = ""
    local valid_label = true
    while not sc:done() do
      local c = sc:peek()
      if c == "]" then sc:skip(1); break end
      if c == "\n" or c == "[" then valid_label = false; break end
      if c == "\\" then
        sc:skip(1)
        if not sc:done() then
          label_buf = label_buf .. "\\" .. sc:advance()
        end
      else
        label_buf = label_buf .. sc:advance()
      end
    end
    if valid_label then
      if label_buf:match("[^ ]") then
        -- Full reference
        local label = normalize_link_label(label_buf)
        local ref = link_refs[label]
        if ref then return { destination = ref.destination, title = ref.title } end
      else
        -- Collapsed reference: [] — use inner text as label
        local label = normalize_link_label(inner_text)
        local ref = link_refs[label]
        if ref then return { destination = ref.destination, title = ref.title } end
      end
    end
    sc.pos = saved_pos
    return nil
  end

  -- ── Shortcut reference: use inner text as label ────────────────────────────
  local label = normalize_link_label(inner_text)
  local ref = link_refs[label]
  if ref then return { destination = ref.destination, title = ref.title } end

  return nil
end

--- Find the most recent active bracket opener index in the bracket stack.
-- Returns -1 if none found.
local function find_active_bracket_opener(bracket_stack, tokens)
  for i = #bracket_stack, 1, -1 do
    local idx = bracket_stack[i]
    local t = tokens[idx]
    if t and t.kind == "bracket" and t.active then
      return i
    end
  end
  return -1
end

--- Extract plain text from a list of inline nodes (for image alt and link labels).
local function extract_plain_text(nodes)
  local result = ""
  for _, node in ipairs(nodes) do
    if node.type == "text" then
      result = result .. node.value
    elseif node.type == "code_span" then
      result = result .. node.value
    elseif node.type == "hard_break" then
      result = result .. "\n"
    elseif node.type == "soft_break" then
      result = result .. " "
    elseif node.type == "emphasis" or node.type == "strong" or node.type == "strikethrough" or node.type == "link" then
      result = result .. extract_plain_text(node.children)
    elseif node.type == "image" then
      result = result .. node.alt
    elseif node.type == "autolink" then
      result = result .. node.destination
    end
  end
  return result
end

--- Resolve the emphasis/strong delimiter stack algorithm.
-- Implements GFM Appendix A.
local function resolve_emphasis(tokens)
  local i = 1
  while i <= #tokens do
    local token = tokens[i]
    if token.kind ~= "delimiter" or not token.can_close or not token.active then
      i = i + 1
      goto continue_resolve
    end

    local closer = token

    -- Search backwards for an opener
    local opener_idx = -1
    for j = i - 1, 1, -1 do
      local t = tokens[j]
      if t.kind ~= "delimiter" or not t.can_open or not t.active or t.char ~= closer.char then
        goto continue_search
      end
      -- Mod-3 rule
      if (t.can_open and t.can_close) or (closer.can_open and closer.can_close) then
        if (t.count + closer.count) % 3 == 0 and t.count % 3 ~= 0 then
          goto continue_search
        end
      end
      opener_idx = j
      break
      ::continue_search::
    end

    if opener_idx == -1 then
      i = i + 1
      goto continue_resolve
    end

    local opener = tokens[opener_idx]

    -- How many delimiter characters to consume?
    local use_len = (closer.char == "~") and 2 or ((opener.count >= 2 and closer.count >= 2) and 2 or 1)
    local is_strong = closer.char ~= "~" and use_len == 2

    -- Collect inner tokens (between opener and closer), recursively resolve them
    local inner_slice = {}
    for k = opener_idx + 1, i - 1 do
      inner_slice[#inner_slice + 1] = tokens[k]
    end
    local inner_nodes = resolve_emphasis(inner_slice)

    local emph_node
    if closer.char == "~" then
      emph_node = ast.strikethrough(inner_nodes)
    elseif is_strong then
      emph_node = ast.strong(inner_nodes)
    else
      emph_node = ast.emphasis(inner_nodes)
    end

    -- Replace inner tokens with the emphasis node
    local node_token = { kind = "node", node = emph_node }
    local inner_count = i - opener_idx - 1
    -- splice: replace tokens[opener_idx+1 .. i-1] with node_token
    for k = opener_idx + 1, opener_idx + inner_count do
      tokens[k] = nil
    end
    -- Pack remaining tokens
    local new_tokens = {}
    for k = 1, #tokens do
      if tokens[k] ~= nil then
        new_tokens[#new_tokens + 1] = tokens[k]
      end
    end
    tokens = new_tokens

    -- Insert node_token at opener_idx + 1
    table.insert(tokens, opener_idx + 1, node_token)
    -- Now opener is at opener_idx, node_token at opener_idx+1, closer shifted left

    -- Reduce delimiter counts
    opener.count = opener.count - use_len
    -- Find new position of closer (it shifted due to inner replacement)
    -- opener_idx + 2 is the new closer position (opener + emphNode + closer)
    local closer_new_idx = opener_idx + 2

    if opener.count == 0 then
      table.remove(tokens, opener_idx)
      -- After removing opener, node_token is at opener_idx, closer at opener_idx+1
      closer_new_idx = opener_idx + 1
      i = opener_idx + 1
    else
      i = opener_idx + 2
    end

    -- Find closer in the tokens array
    local actual_closer = tokens[closer_new_idx]
    if actual_closer and actual_closer.kind == "delimiter" then
      actual_closer.count = actual_closer.count - use_len
      if actual_closer.count == 0 then
        table.remove(tokens, closer_new_idx)
      end
    end

    ::continue_resolve::
  end

  -- Convert remaining tokens to inline nodes
  local result = {}
  for _, tok in ipairs(tokens) do
    if tok.kind == "node" then
      result[#result + 1] = tok.node
    elseif tok.kind == "bracket" then
      result[#result + 1] = ast.text(tok.is_image and "![" or "[")
    elseif tok.kind == "delimiter" then
      -- Unused delimiter run — literal text
      result[#result + 1] = ast.text(tok.char:rep(tok.count))
    end
  end
  return result
end

--- Get the full UTF-8 character ending at byte position `end_pos`.
-- Walks back from end_pos to find the start of the multi-byte character.
local function utf8_char_ending_at(source, end_pos)
  if end_pos < 1 then return "" end
  -- Walk back to find the start of the UTF-8 sequence
  local start = end_pos
  while start > 1 do
    local b = string.byte(source, start)
    if b >= 0x80 and b < 0xC0 then
      -- Continuation byte, keep going back
      start = start - 1
    else
      break
    end
  end
  return source:sub(start, end_pos)
end

--- Get the full UTF-8 character starting at byte position `start_pos`.
local function utf8_char_at(source, start_pos)
  if start_pos > #source then return "" end
  local b = string.byte(source, start_pos)
  if b == nil then return "" end
  local char_len
  if b < 0x80 then char_len = 1
  elseif b < 0xE0 then char_len = 2
  elseif b < 0xF0 then char_len = 3
  else char_len = 4
  end
  return source:sub(start_pos, start_pos + char_len - 1)
end

--- Scan a delimiter run of `*`, `_`, or `~` and compute flanking properties.
-- Uses full UTF-8 characters for pre/post character classification
-- to correctly handle Unicode whitespace and punctuation (e.g. U+00A0).
local function scan_delimiter_run(sc, char)
  local source = sc.source
  local run_start = sc.pos
  -- Get the full UTF-8 character before the run
  local pre_char = run_start > 1 and utf8_char_ending_at(source, run_start - 1) or ""

  local run = sc:consume_while(function(c) return c == char end)
  local count = #run
  -- Get the full UTF-8 character after the run
  local post_char = sc.pos <= #source and utf8_char_at(source, sc.pos) or ""

  local after_whitespace  = post_char == "" or is_unicode_whitespace(post_char)
  local after_punctuation = post_char ~= "" and is_unicode_punctuation(post_char)
  local before_whitespace = pre_char  == "" or is_unicode_whitespace(pre_char)
  local before_punctuation = pre_char ~= "" and is_unicode_punctuation(pre_char)

  -- Left-flanking: not followed by whitespace AND
  --   (not followed by punctuation OR preceded by whitespace/punctuation)
  local left_flanking =
    not after_whitespace and
    (not after_punctuation or before_whitespace or before_punctuation)

  -- Right-flanking: not preceded by whitespace AND
  --   (not preceded by punctuation OR followed by whitespace/punctuation)
  local right_flanking =
    not before_whitespace and
    (not before_punctuation or after_whitespace or after_punctuation)

  local can_open, can_close
  if char == "*" then
    can_open  = left_flanking
    can_close = right_flanking
  elseif char == "~" then
    can_open  = count >= 2
    can_close = count >= 2
  else
    can_open  = left_flanking  and (not right_flanking or before_punctuation)
    can_close = right_flanking and (not left_flanking  or after_punctuation)
  end

  return { kind = "delimiter", char = char, count = count, can_open = can_open, can_close = can_close, active = true }
end

--- Parse raw inline content string into a list of inline nodes.
-- @param raw       string — the raw inline string from the block parser
-- @param link_refs table  — link reference map from phase 1
-- @return table           — list of inline nodes
local function parse_inline(raw, link_refs)
  local sc = Scanner.new(raw)
  local tokens = {}

  local bracket_stack = {} -- stack of token indices for open brackets
  local text_buf = ""

  local function flush_text()
    if #text_buf > 0 then
      tokens[#tokens + 1] = { kind = "node", node = ast.text(text_buf) }
      text_buf = ""
    end
  end

  -- ── Scan Phase ────────────────────────────────────────────────────────────

  while not sc:done() do
    local ch = sc:peek()

    -- 1. Backslash escape
    if ch == "\\" then
      local next = sc:peek(1)
      if next ~= "" and is_ascii_punctuation(next) then
        sc:skip(2)
        text_buf = text_buf .. next
        goto scan_continue
      end
      if next == "\n" then
        sc:skip(2)
        flush_text()
        tokens[#tokens + 1] = { kind = "node", node = ast.hard_break() }
        goto scan_continue
      end
      sc:skip(1)
      text_buf = text_buf .. "\\"
      goto scan_continue
    end

    -- 2. HTML character reference
    if ch == "&" then
      -- Try to match a full entity reference
      -- Per GFM spec, numeric references are only valid with
      -- 1-7 decimal digits or 1-6 hex digits. Longer sequences pass through as text.
      local entity_match
      local rest = sc.source:sub(sc.pos)
      entity_match = rest:match("^(&[a-zA-Z][a-zA-Z0-9]*;)")
        or rest:match("^(&#[0-9][0-9]?[0-9]?[0-9]?[0-9]?[0-9]?[0-9]?;)")
        or rest:match("^(&#[xX][0-9a-fA-F][0-9a-fA-F]?[0-9a-fA-F]?[0-9a-fA-F]?[0-9a-fA-F]?[0-9a-fA-F]?;)")
      if entity_match then
        local decoded = decode_entity(entity_match)
        text_buf = text_buf .. decoded
        sc:skip(#entity_match)
        goto scan_continue
      end
      sc:skip(1)
      text_buf = text_buf .. "&"
      goto scan_continue
    end

    -- 3. Code span
    if ch == "`" then
      local span = try_code_span(sc)
      if span ~= nil then
        flush_text()
        tokens[#tokens + 1] = { kind = "node", node = span }
        goto scan_continue
      end
      -- Not a valid code span — literal backtick run
      local ticks = sc:consume_while(function(c) return c == "`" end)
      text_buf = text_buf .. ticks
      goto scan_continue
    end

    -- 4 & 5. HTML inline and autolinks (both start with `<`)
    if ch == "<" then
      local autolink = try_autolink(sc)
      if autolink ~= nil then
        flush_text()
        tokens[#tokens + 1] = { kind = "node", node = autolink }
        goto scan_continue
      end
      local html = try_html_inline(sc)
      if html ~= nil then
        flush_text()
        tokens[#tokens + 1] = { kind = "node", node = html }
        goto scan_continue
      end
      sc:skip(1)
      text_buf = text_buf .. "<"
      goto scan_continue
    end

    -- Image opener `![`
    if ch == "!" and sc:peek(1) == "[" then
      flush_text()
      bracket_stack[#bracket_stack + 1] = #tokens + 1
      sc:skip(2)
      tokens[#tokens + 1] = { kind = "bracket", is_image = true, active = true, source_pos = sc.pos }
      goto scan_continue
    end

    -- Link opener `[`
    if ch == "[" then
      flush_text()
      bracket_stack[#bracket_stack + 1] = #tokens + 1
      sc:skip(1)
      tokens[#tokens + 1] = { kind = "bracket", is_image = false, active = true, source_pos = sc.pos }
      goto scan_continue
    end

    -- Link/image closer `]`
    if ch == "]" then
      sc:skip(1)

      -- Check for deactivated non-image opener at top of stack
      if #bracket_stack > 0 then
        local top_idx = bracket_stack[#bracket_stack]
        local top_tok = tokens[top_idx]
        if top_tok and top_tok.kind == "bracket" and not top_tok.active and not top_tok.is_image then
          bracket_stack[#bracket_stack] = nil
          text_buf = text_buf .. "]"
          goto scan_continue
        end
      end

      local opener_stack_idx = find_active_bracket_opener(bracket_stack, tokens)

      if opener_stack_idx == -1 then
        text_buf = text_buf .. "]"
        goto scan_continue
      end

      local opener_token_idx = bracket_stack[opener_stack_idx]
      local opener = tokens[opener_token_idx]

      flush_text()

      -- Collect inner text
      local inner_tokens_before = {}
      for k = opener_token_idx + 1, #tokens do
        inner_tokens_before[#inner_tokens_before + 1] = tokens[k]
      end

      local closer_pos = sc.pos - 1
      local inner_text_for_label = sc.source:sub(opener.source_pos, closer_pos - 1)

      local link_result = try_link_after_close(sc, link_refs, inner_text_for_label)

      if link_result == nil then
        opener.active = false
        table.remove(bracket_stack, opener_stack_idx)
        text_buf = text_buf .. "]"
        goto scan_continue
      end

      flush_text()

      -- Extract all tokens after the opener
      local inner_tokens = {}
      for k = opener_token_idx + 1, #tokens do
        inner_tokens[#inner_tokens + 1] = tokens[k]
      end
      -- Remove inner tokens and opener from tokens
      while #tokens > opener_token_idx do
        tokens[#tokens] = nil
      end
      tokens[opener_token_idx] = nil
      -- Compact tokens
      local compact = {}
      for k = 1, opener_token_idx - 1 do
        if tokens[k] then compact[#compact + 1] = tokens[k] end
      end
      tokens = compact
      -- Remove opener from bracket stack
      table.remove(bracket_stack, opener_stack_idx)

      local inner_nodes = resolve_emphasis(inner_tokens)

      if opener.is_image then
        local alt_text = extract_plain_text(inner_nodes)
        tokens[#tokens + 1] = { kind = "node", node = ast.image(link_result.destination, link_result.title, alt_text) }
      else
        tokens[#tokens + 1] = { kind = "node", node = ast.link(link_result.destination, link_result.title, inner_nodes) }
        -- Deactivate all preceding non-image link openers
        for k = 1, #bracket_stack do
          local idx = bracket_stack[k]
          local t = tokens[idx]
          if t and t.kind == "bracket" and not t.is_image then
            t.active = false
          end
        end
      end

      goto scan_continue
    end

    -- 8. Emphasis / strong / strikethrough delimiter run
    if ch == "*" or ch == "_" or (ch == "~" and sc:peek(1) == "~") then
      flush_text()
      local delim = scan_delimiter_run(sc, ch)
      tokens[#tokens + 1] = delim
      goto scan_continue
    end

    -- 6 & 7. Hard break or soft break
    if ch == "\n" then
      sc:skip(1)
      if text_buf:match("[ \t][ \t]+$") then
        text_buf = text_buf:gsub("[ \t]+$", "")
        flush_text()
        tokens[#tokens + 1] = { kind = "node", node = ast.hard_break() }
      else
        text_buf = text_buf:gsub("%s+$", "")
        flush_text()
        tokens[#tokens + 1] = { kind = "node", node = ast.soft_break() }
      end
      goto scan_continue
    end

    -- Regular character
    text_buf = text_buf .. sc:advance()

    ::scan_continue::
  end

  flush_text()

  -- ── Resolve Phase ─────────────────────────────────────────────────────────
  return resolve_emphasis(tokens)
end

-- ─── AST Conversion ───────────────────────────────────────────────────────────
--
-- Convert mutable intermediate blocks into the final AST.
-- Inline content raw strings are stored in a map keyed by unique IDs (numbers)
-- for Phase 2 (inline-parser) to process.

local function convert_to_ast(mutable_doc, link_refs)
  local raw_inline_content = {} -- id -> raw string
  local next_id = 1

  local function next_raw_id(value)
    local id = next_id
    next_id = next_id + 1
    raw_inline_content[id] = value
    return id
  end

  local function split_table_row(line)
    if not line:find("|", 1, true) then return nil end

    local stripped = line:match("^%s*(.-)%s*$") or line
    local had_outer_pipe = stripped:sub(1, 1) == "|" or stripped:sub(-1) == "|"

    local start_idx = 1
    local end_idx = #line
    while start_idx <= end_idx and is_ascii_whitespace(line:sub(start_idx, start_idx)) do
      start_idx = start_idx + 1
    end
    while end_idx >= start_idx and is_ascii_whitespace(line:sub(end_idx, end_idx)) do
      end_idx = end_idx - 1
    end
    if start_idx <= end_idx and line:sub(start_idx, start_idx) == "|" then
      start_idx = start_idx + 1
    end
    if end_idx >= start_idx and line:sub(end_idx, end_idx) == "|" then
      end_idx = end_idx - 1
    end

    local cells, current = {}, {}
    local escaped = false
    local pipe_count = 0
    for i = start_idx, end_idx do
      local ch = line:sub(i, i)
      if escaped then
        current[#current + 1] = ch
        escaped = false
      elseif ch == "\\" then
        current[#current + 1] = ch
        escaped = true
      elseif ch == "|" then
        pipe_count = pipe_count + 1
        cells[#cells + 1] = table.concat(current):match("^%s*(.-)%s*$") or ""
        current = {}
      else
        current[#current + 1] = ch
      end
    end
    cells[#cells + 1] = table.concat(current):match("^%s*(.-)%s*$") or ""

    if pipe_count == 0 and not had_outer_pipe then
      return nil
    end
    return cells
  end

  local function normalize_table_row(cells, width)
    local normalized = {}
    for i = 1, math.min(#cells, width) do
      normalized[#normalized + 1] = cells[i]
    end
    while #normalized < width do
      normalized[#normalized + 1] = ""
    end
    return normalized
  end

  local function try_parse_table(content)
    local lines = {}
    for line in (content .. "\n"):gmatch("(.-)\n") do
      lines[#lines + 1] = line
    end
    if #lines < 2 then return nil end

    local header_cells = split_table_row(lines[1])
    local delimiter_cells = split_table_row(lines[2])
    if header_cells == nil or delimiter_cells == nil then return nil end
    if #header_cells == 0 or #header_cells ~= #delimiter_cells then return nil end

    local align = {}
    for _, cell in ipairs(delimiter_cells) do
      local trimmed = cell:match("^%s*(.-)%s*$") or cell
      if not trimmed:match("^:?[%-][%-][%-]+:?$") then
        return nil
      end
      local left = trimmed:sub(1, 1) == ":"
      local right = trimmed:sub(-1) == ":"
      if left and right then
        align[#align + 1] = "center"
      elseif left then
        align[#align + 1] = "left"
      elseif right then
        align[#align + 1] = "right"
      else
        align[#align + 1] = nil
      end
    end

    local rows = {}
    for i = 3, #lines do
      if lines[i]:match("^%s*$") then return nil end
      local cells = split_table_row(lines[i])
      if cells == nil then return nil end
      rows[#rows + 1] = normalize_table_row(cells, #header_cells)
    end

    return {
      align = align,
      header = normalize_table_row(header_cells, #delimiter_cells),
      rows = rows,
    }
  end

  local function make_table_cell(content)
    local id = next_raw_id(content)
    local node = ast.table_cell({})
    node._raw_id = id
    return node
  end

  local function maybe_convert_task_item(item, item_children)
    if #item.children == 0 or item.children[1].kind ~= "paragraph" or #item_children == 0 then
      return nil
    end

    local lines = {}
    for _, l in ipairs(item.children[1].lines) do
      lines[#lines + 1] = l:gsub("^[ \t]+", "")
    end
    local content = table.concat(lines, "\n")
    local marker, rest = content:match("^%[(.)%](.*)$")
    if marker ~= " " and marker ~= "x" and marker ~= "X" then
      return nil
    end
    if #rest > 0 and not rest:match("^[ \t]") then
      return nil
    end

    local first_child = item_children[1]
    if first_child and first_child.type == "paragraph" and first_child._raw_id ~= nil then
      raw_inline_content[first_child._raw_id] = (rest:gsub("^[ \t]+", ""))
    end

    return ast.task_item(marker == "x" or marker == "X", item_children)
  end

  local function convert_block(block)
    if block.kind == "document" then
      local children = {}
      for _, ch in ipairs(block.children) do
        local converted = convert_block(ch)
        if converted ~= nil then
          children[#children + 1] = converted
        end
      end
      return ast.document(children)

    elseif block.kind == "heading" then
      local id = next_raw_id(block.content)
      local node = ast.heading(block.level, {})
      node._raw_id = id
      return node

    elseif block.kind == "paragraph" then
      if #block.lines == 0 then return nil end
      -- Strip leading whitespace from each line (per GFM)
      local lines = {}
      for _, l in ipairs(block.lines) do
        lines[#lines + 1] = l:gsub("^[ \t]+", "")
      end
      local content = table.concat(lines, "\n")
      local parsed_table = try_parse_table(content)
      if parsed_table ~= nil then
        local rows = {}
        local header_cells = {}
        for _, cell in ipairs(parsed_table.header) do
          header_cells[#header_cells + 1] = make_table_cell(cell)
        end
        rows[#rows + 1] = ast.table_row(true, header_cells)
        for _, row in ipairs(parsed_table.rows) do
          local row_cells = {}
          for _, cell in ipairs(row) do
            row_cells[#row_cells + 1] = make_table_cell(cell)
          end
          rows[#rows + 1] = ast.table_row(false, row_cells)
        end
        return ast.table(parsed_table.align, rows)
      end
      local id = next_raw_id(content)
      local node = ast.paragraph({})
      node._raw_id = id
      return node

    elseif block.kind == "fenced_code" then
      local value = table.concat(block.lines, "\n")
      if #value > 0 then value = value .. "\n" end
      local lang = block.info_string ~= "" and block.info_string or nil
      return ast.code_block(lang, value)

    elseif block.kind == "indented_code" then
      local value = table.concat(block.lines, "\n")
      if #value > 0 then value = value .. "\n" end
      return ast.code_block(nil, value)

    elseif block.kind == "html_block" then
      -- For type 6/7 blocks, a blank line terminates the block and gets pushed
      -- into lines before the mode switches. Trim trailing blank lines.
      local lines = {}
      for _, l in ipairs(block.lines) do lines[#lines + 1] = l end
      while #lines > 0 and lines[#lines]:match("^%s*$") do
        lines[#lines] = nil
      end
      local value = table.concat(lines, "\n")
      if #value > 0 then value = value .. "\n" end
      return ast.raw_block("html", value)

    elseif block.kind == "blockquote" then
      local children = {}
      for _, ch in ipairs(block.children) do
        local converted = convert_block(ch)
        if converted ~= nil then children[#children + 1] = converted end
      end
      return ast.blockquote(children)

    elseif block.kind == "list" then
      local children = {}
      -- A list is loose if:
      --   1. blank lines appeared between items (block.tight was set false), OR
      --   2. blank lines appeared between blocks WITHIN an item that has > 1 block.
      --
      -- An item with had_blank_line=true but only ONE block child is still tight —
      -- the blank line was after the item (between items), not between its own blocks.
      local tight = block.tight
      if tight and not block.had_blank_line then
        for _, item in ipairs(block.items) do
          if item.had_blank_line and #item.children > 1 then
            tight = false
            break
          end
        end
      elseif block.had_blank_line then
        tight = false
      end
      for _, item in ipairs(block.items) do
        local item_children = {}
        for _, ch in ipairs(item.children) do
          local converted = convert_block(ch)
          if converted ~= nil then item_children[#item_children + 1] = converted end
        end
        children[#children + 1] = maybe_convert_task_item(item, item_children) or ast.list_item(item_children)
      end
      return ast.list(block.ordered, block.ordered and block.start or nil, tight, children)

    elseif block.kind == "thematic_break" then
      return ast.thematic_break()
    end

    return nil
  end

  local doc = convert_block(mutable_doc)
  return { document = doc, raw_inline_content = raw_inline_content }
end

--- Walk the block AST and fill in inline content by parsing raw strings.
local function resolve_inline_content(document, raw_inline_content, link_refs)
  local function walk(block)
    if (block.type == "heading" or block.type == "paragraph" or block.type == "table_cell") and block._raw_id ~= nil then
      local raw = raw_inline_content[block._raw_id]
      if raw ~= nil then
        block.children = parse_inline(raw, link_refs)
      end
      block._raw_id = nil
    end

    -- Recurse into container blocks
    if block.children then
      for _, child in ipairs(block.children) do
        walk(child)
      end
    end
  end

  walk(document)
end

-- ─── Public API ───────────────────────────────────────────────────────────────

--- Parse a GitHub Flavored Markdown string into a DocumentNode AST.
--
-- The result conforms to the Document AST spec (TE00) — a format-agnostic IR
-- with all link references resolved and all inline markup parsed.
--
-- @param markdown  string  — the Markdown source string
-- @param _options  table   — optional parse options (reserved; currently unused)
-- @return table            — the root document node
--
-- @example
--   local cm = require("coding_adventures.commonmark_parser")
--   local doc = cm.parse("## Heading\n\n- item 1\n- item 2\n")
--   doc.children[1].type   -- "heading"
--   doc.children[2].type   -- "list"
function M.parse(markdown, _options)
  -- Phase 1: Block parsing
  local result = parse_blocks(markdown)
  local mutable_doc = result.document
  local link_refs = result.link_refs

  local convert_result = convert_to_ast(mutable_doc, link_refs)
  local document = convert_result.document
  local raw_inline_content = convert_result.raw_inline_content

  -- Phase 2: Inline parsing
  resolve_inline_content(document, raw_inline_content, link_refs)

  return document
end

M.VERSION = "0.1.0"

return M
