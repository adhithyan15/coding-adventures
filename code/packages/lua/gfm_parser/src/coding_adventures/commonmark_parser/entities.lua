-- HTML Entity Decoder / Encoder
-- ==============================
--
-- GFM requires decoding three forms of HTML character references
-- within text content and link destinations:
--
--   Named:   &amp;  → &    &lt; → <    &gt; → >    &quot; → "
--   Decimal: &#65;  → A    &#169; → ©
--   Hex:     &#x41; → A    &#xA9; → ©
--
-- This module implements decoding for all three forms. Named entities use
-- a lookup table covering all HTML5 named character references.
--
-- === Why entity decoding matters ===
--
-- GFM says: "An entity reference consists of & + any of the valid
-- HTML5 named entities + ;". The decoded character should appear in the
-- output, not the raw reference. So `&copy;` in source text should render
-- as `©` in HTML (and `&copy;` in HTML source).
--
-- There are ~2125 HTML5 named entities. We include all of them here for
-- full GFM compliance.
--
-- @module coding_adventures.commonmark_parser.entities

-- Load the ~2125-entry named entity table from entity_table.lua
local NAMED_ENTITIES = require("coding_adventures.commonmark_parser.entity_table")

local M = {}

--- Decode a single HTML character reference.
--
-- Handles three forms:
--   Named:    &amp;    → "&"
--   Decimal:  &#65;    → "A"
--   Hex:      &#x41;   → "A"
--
-- For numeric references, code point 0 and values above U+10FFFF
-- are replaced with the Unicode replacement character U+FFFD.
--
-- Unrecognised named references are returned as-is (not decoded).
--
-- @param ref  string — the full entity reference including & and ;
-- @return string — decoded character or original ref if unrecognised
function M.decode_entity(ref)
  if not ref:match("^&") or not ref:match(";$") then
    return ref
  end

  -- Strip the & and ;
  local inner = ref:sub(2, -2)

  -- Numeric reference: &#NNN; or &#xHHH;
  if inner:sub(1, 1) == "#" then
    local rest = inner:sub(2)
    local code_point

    if rest:sub(1, 1) == "x" or rest:sub(1, 1) == "X" then
      -- Hex reference: &#xHHH;
      code_point = tonumber(rest:sub(2), 16)
    else
      -- Decimal reference: &#NNN;
      code_point = tonumber(rest, 10)
    end

    -- Invalid or out-of-range code point → replacement character
    if code_point == nil or code_point == 0 or code_point > 0x10FFFF then
      return "\xEF\xBF\xBD" -- U+FFFD REPLACEMENT CHARACTER in UTF-8
    end

    -- Encode the code point as UTF-8
    -- Lua 5.4+ has utf8.char, but we also support Lua 5.3 for wider compat.
    -- We implement UTF-8 encoding manually for portability.
    if code_point < 0x80 then
      return string.char(code_point)
    elseif code_point < 0x800 then
      return string.char(
        0xC0 + math.floor(code_point / 0x40),
        0x80 + (code_point % 0x40)
      )
    elseif code_point < 0x10000 then
      return string.char(
        0xE0 + math.floor(code_point / 0x1000),
        0x80 + math.floor((code_point % 0x1000) / 0x40),
        0x80 + (code_point % 0x40)
      )
    else
      -- 4-byte UTF-8 (supplementary plane characters)
      return string.char(
        0xF0 + math.floor(code_point / 0x40000),
        0x80 + math.floor((code_point % 0x40000) / 0x1000),
        0x80 + math.floor((code_point % 0x1000) / 0x40),
        0x80 + (code_point % 0x40)
      )
    end
  end

  -- Named reference: &name;
  -- Look up in the HTML5 named entity table.
  local decoded = NAMED_ENTITIES[inner]
  if decoded ~= nil then
    return decoded
  end

  -- Unknown reference — return as-is (per GFM spec)
  return ref
end

--- Decode all HTML character references in a string.
--
-- Scans for `&...;` patterns and replaces each recognised reference
-- with its decoded character. Unrecognised references are left as-is.
--
-- @param text  string — input text possibly containing entity references
-- @return string — text with all recognised entities decoded
--
-- @example
--   decode_entities("Tom &amp; Jerry")          -- "Tom & Jerry"
--   decode_entities("&#x1F600; smile")          -- "😀 smile"
--   decode_entities("&lt;p&gt;hello&lt;/p&gt;") -- "<p>hello</p>"
function M.decode_entities(text)
  -- Fast path: no & means no entities
  if not text:find("&", 1, true) then
    return text
  end

  -- Replace all &...; patterns. We use a Lua pattern that matches the three forms:
  --   &name;    — named entity (1-32 alphanumeric chars)
  --   &#DDD;    — decimal numeric entity
  --   &#xHHH;   — hex numeric entity
  -- Note: Lua patterns don't support alternation (|), so we use a more general
  -- pattern and let decode_entity filter out invalid ones.
  return (text:gsub("&[#a-zA-Z][a-zA-Z0-9]*;", M.decode_entity)
             :gsub("&#[xX][0-9a-fA-F]+;", M.decode_entity)
             :gsub("&#[0-9]+;", M.decode_entity))
end

--- Decode all HTML character references in a string (single-pass version).
--
-- This is the primary function called by the parser. It uses a combined
-- pattern approach to handle all three entity forms in one pass.
--
-- @param text  string — input text
-- @return string — decoded text
function M.decode_entities_full(text)
  if not text:find("&", 1, true) then return text end
  -- We need to match any of:
  --   &[a-zA-Z][a-zA-Z0-9]{0,31};   named
  --   &#[0-9]{1,7};                  decimal
  --   &#[xX][0-9a-fA-F]{1,6};       hex
  -- Lua patterns can't do alternation directly, but we can use a greedy
  -- pattern and validate inside the replacement function.
  -- Per GFM spec §2.5: decimal refs must have 1-7 digits, hex refs 1-6 digits.
  -- Sequences outside these limits are not entity references and pass through as text.
  return (text:gsub("&[^;%s]+;", function(match)
    -- Validate the match is one of the three forms with length constraints
    if match:match("^&[a-zA-Z][a-zA-Z0-9]*;$") then
      return M.decode_entity(match)
    elseif match:match("^&#([0-9]+);$") then
      local digits = match:match("^&#([0-9]+);$")
      if #digits >= 1 and #digits <= 7 then
        return M.decode_entity(match)
      end
    elseif match:match("^&#[xX]([0-9a-fA-F]+);$") then
      local hexdigits = match:match("^&#[xX]([0-9a-fA-F]+);$")
      if #hexdigits >= 1 and #hexdigits <= 6 then
        return M.decode_entity(match)
      end
    end
    return match -- not a valid entity reference
  end))
end

--- Escape HTML special characters for safe output.
--
-- Encodes the four characters with HTML significance:
--   &  → &amp;
--   <  → &lt;
--   >  → &gt;
--   "  → &quot;
--
-- Note: we do NOT escape apostrophes because GFM's reference
-- implementation uses double-quoted attributes.
--
-- @param text  string — raw text to escape
-- @return string — HTML-safe text
--
-- @example
--   escape_html("Hello & <World>")  -- "Hello &amp; &lt;World&gt;"
--   escape_html('"quoted"')         -- "&quot;quoted&quot;"
function M.escape_html(text)
  -- Process in order: & first (must be first to avoid double-escaping),
  -- then < > "
  return text
    :gsub("&", "&amp;")
    :gsub("<", "&lt;")
    :gsub(">", "&gt;")
    :gsub('"', "&quot;")
end

return M
