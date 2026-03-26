-- String Scanner
-- ==============
--
-- A cursor-based scanner over a string. Used by both the block parser
-- (to scan individual lines) and the inline parser (to scan inline
-- content character by character).
--
-- === Design ===
--
-- The scanner maintains a position `pos` into the string (1-based, Lua-style).
-- All read operations advance `pos`. The scanner never backtracks on its own —
-- callers must save and restore `pos` explicitly when lookahead fails.
--
-- This is the same pattern used by hand-rolled recursive descent parsers
-- everywhere: try to match, if it fails, restore the saved position.
--
--   local saved = s.pos
--   if not s:match("```") then
--     s.pos = saved  -- backtrack
--   end
--
-- === Lua vs TypeScript differences ===
--
-- TypeScript uses 0-based string indexing; Lua uses 1-based. All positions
-- in this module are 1-based. The `pos` field points to the next character
-- to be consumed (the character at `source:sub(pos, pos)`).
--
-- When `pos > #source`, the scanner is exhausted (done).
--
-- === Character classification ===
--
-- GFM cares about several Unicode character categories:
--   - ASCII punctuation: !"#$%&'()*+,-./:;<=>?@[\]^_`{|}~
--   - Unicode punctuation (for emphasis rules)
--   - ASCII whitespace: space, tab, CR, LF, FF
--   - Unicode whitespace
--
-- Lua's utf8 library provides utf8.codepoint and utf8.char for Unicode
-- operations. For pattern matching, Lua patterns (%p, %s, etc.) cover
-- ASCII characters; Unicode properties require external libraries.
-- We use a manual code-point table for Unicode punctuation/whitespace.
--
-- @module coding_adventures.commonmark_parser.scanner

local M = {}

-- ─── Scanner Object ───────────────────────────────────────────────────────────
--
-- The Scanner is a Lua table (object) with `source` (the string being scanned)
-- and `pos` (1-based current position). When pos > #source, done is true.

local Scanner = {}
Scanner.__index = Scanner

--- Create a new Scanner.
--
-- @param source  string — the string to scan
-- @param start   number — initial position (1-based, default 1)
-- @return Scanner
function Scanner.new(source, start)
  return setmetatable({
    source = source,
    pos = start or 1,
  }, Scanner)
end

--- True if the scanner has consumed all input.
-- @return boolean
function Scanner:done()
  return self.pos > #self.source
end

--- Number of characters remaining.
-- @return number
function Scanner:remaining()
  return math.max(0, #self.source - self.pos + 1)
end

--- Peek at the character at `pos + offset` without advancing.
-- Returns "" if out of bounds.
-- @param offset  number — byte offset from current pos (default 0)
-- @return string — single character or ""
function Scanner:peek(offset)
  offset = offset or 0
  local idx = self.pos + offset
  if idx < 1 or idx > #self.source then return "" end
  return self.source:sub(idx, idx)
end

--- Peek at `n` characters starting at `pos` without advancing.
-- @param n  number — number of characters
-- @return string
function Scanner:peek_slice(n)
  return self.source:sub(self.pos, self.pos + n - 1)
end

--- Advance `pos` by one and return the consumed character.
-- Returns "" if already done.
-- @return string — the consumed character
function Scanner:advance()
  if self.pos > #self.source then return "" end
  local ch = self.source:sub(self.pos, self.pos)
  self.pos = self.pos + 1
  return ch
end

--- Advance `pos` by `n` characters.
-- @param n  number — number of characters to skip
function Scanner:skip(n)
  self.pos = math.min(self.pos + n, #self.source + 1)
end

--- If the next characters exactly match `str`, advance past them and return true.
-- Otherwise leave `pos` unchanged and return false.
-- @param str  string — the literal string to match
-- @return boolean
function Scanner:match(str)
  local len = #str
  if self.source:sub(self.pos, self.pos + len - 1) == str then
    self.pos = self.pos + len
    return true
  end
  return false
end

--- Consume characters while the predicate returns true.
-- Returns the consumed string.
-- @param pred  function(ch: string) -> boolean
-- @return string — the consumed substring
function Scanner:consume_while(pred)
  local start = self.pos
  while self.pos <= #self.source and pred(self.source:sub(self.pos, self.pos)) do
    self.pos = self.pos + 1
  end
  return self.source:sub(start, self.pos - 1)
end

--- Consume the rest of the line (up to but not including the newline).
-- @return string — content up to (but not including) the next \n
function Scanner:consume_line()
  local start = self.pos
  while self.pos <= #self.source and self.source:sub(self.pos, self.pos) ~= "\n" do
    self.pos = self.pos + 1
  end
  return self.source:sub(start, self.pos - 1)
end

--- Return the rest of the input from current pos without advancing.
-- @return string
function Scanner:rest()
  return self.source:sub(self.pos)
end

--- Return a slice of source from `start` to current pos (exclusive).
-- @param start  number — 1-based start position
-- @return string
function Scanner:slice_from(start)
  return self.source:sub(start, self.pos - 1)
end

--- Skip ASCII spaces and tabs. Returns number of characters skipped.
-- @return number — count of skipped characters
function Scanner:skip_spaces()
  local start = self.pos
  while self.pos <= #self.source do
    local ch = self.source:sub(self.pos, self.pos)
    if ch == " " or ch == "\t" then
      self.pos = self.pos + 1
    else
      break
    end
  end
  return self.pos - start
end

--- Count leading spaces/tabs without advancing. Returns virtual column.
-- Tabs expand to the next 4-space tab stop.
-- @return number — virtual indentation width
function Scanner:count_indent()
  local indent = 0
  local i = self.pos
  while i <= #self.source do
    local ch = self.source:sub(i, i)
    if ch == " " then
      indent = indent + 1
      i = i + 1
    elseif ch == "\t" then
      indent = indent + (4 - (indent % 4))
      i = i + 1
    else
      break
    end
  end
  return indent
end

--- Advance past exactly `n` virtual spaces of indentation (expanding tabs).
-- @param n  number — number of virtual spaces to consume
function Scanner:skip_indent(n)
  local remaining = n
  while remaining > 0 and self.pos <= #self.source do
    local ch = self.source:sub(self.pos, self.pos)
    if ch == " " then
      self.pos = self.pos + 1
      remaining = remaining - 1
    elseif ch == "\t" then
      local tab_width = 4 - ((self.pos - 1) % 4)
      if tab_width <= remaining then
        self.pos = self.pos + 1
        remaining = remaining - tab_width
      else
        break -- partial tab — don't consume
      end
    else
      break
    end
  end
end

-- Export the Scanner constructor
M.Scanner = Scanner

-- ─── Character Classification ─────────────────────────────────────────────────

-- ASCII punctuation characters as defined by GFM.
-- These are exactly: ! " # $ % & ' ( ) * + , - . / : ; < = > ? @ [ \ ] ^ _ ` { | } ~
local ASCII_PUNCT_SET = {}
for _, ch in ipairs({
  "!", '"', "#", "$", "%", "&", "'", "(", ")", "*", "+", ",",
  "-", ".", "/", ":", ";", "<", "=", ">", "?", "@", "[", "\\",
  "]", "^", "_", "`", "{", "|", "}", "~"
}) do
  ASCII_PUNCT_SET[ch] = true
end

--- True if `ch` is an ASCII punctuation character (GFM definition).
-- Used in the emphasis rules to determine flanking delimiter runs.
-- @param ch  string — single character
-- @return boolean
function M.is_ascii_punctuation(ch)
  return ASCII_PUNCT_SET[ch] == true
end

-- Unicode punctuation ranges (non-ASCII).
-- GFM defines Unicode punctuation as: ASCII punctuation OR any
-- character in Unicode categories P* (punctuation) or S* (symbol).
--
-- Since Lua doesn't have built-in Unicode property support, we maintain
-- a table of common Unicode punctuation/symbol ranges and code points.
-- This covers the characters tested by the GFM spec test suite.
--
-- Code point ranges (hex):
--   0x00A1-0x00BF  Latin-1 punctuation & symbols (¡ through ¿)
--   0x2010-0x2027  General punctuation (hyphens, dashes, etc.)
--   0x2030-0x205E  More punctuation
--   0x2060-0x2064  Word joiners
--   0x2066-0x2069  Bidi marks
--   0x20A0-0x20CF  Currency symbols
--   0x2100-0x214F  Letterlike symbols
--   0x2150-0x218F  Number forms
--   0x2190-0x21FF  Arrows
--   0x2200-0x22FF  Mathematical operators
--   0x2300-0x23FF  Miscellaneous technical
--   0x2400-0x24FF  Control pictures
--   0x25A0-0x25FF  Geometric shapes
--   0x2600-0x26FF  Miscellaneous symbols
--   0x2700-0x27BF  Dingbats
--   0x3000-0x303F  CJK Symbols and Punctuation
--   0xFE50-0xFE6F  Small Form Variants
--   0xFF01-0xFF0F, 0xFF1A-0xFF20, 0xFF3B-0xFF40, 0xFF5B-0xFF65  Fullwidth punct
--
-- This is the same approach used by the cmark C reference implementation.
local function is_unicode_punct_codepoint(cp)
  if cp < 0x80 then return false end -- ASCII handled separately
  -- Latin-1 punctuation block
  if cp >= 0x00A1 and cp <= 0x00BF then return true end
  -- ¦ BROKEN BAR, § SECTION SIGN, © COPYRIGHT, « LEFT-POINTING DOUBLE ANGLE QUOTATION
  -- Various currency and symbol characters
  if cp == 0x00A6 or cp == 0x00A7 or cp == 0x00A9 then return true end
  if cp == 0x00AB or cp == 0x00AE or cp == 0x00B0 then return true end
  if cp == 0x00B1 or cp == 0x00BB or cp == 0x00BF then return true end
  -- General punctuation (U+2000-U+206F)
  if cp >= 0x2000 and cp <= 0x206F then return true end
  -- Currency symbols (U+20A0-U+20CF)
  if cp >= 0x20A0 and cp <= 0x20CF then return true end
  -- Letterlike symbols (U+2100-U+214F)
  if cp >= 0x2100 and cp <= 0x214F then return true end
  -- Number forms (U+2150-U+218F)
  if cp >= 0x2150 and cp <= 0x218F then return true end
  -- Arrows and math operators (U+2190-U+22FF)
  if cp >= 0x2190 and cp <= 0x22FF then return true end
  -- Miscellaneous technical (U+2300-U+23FF)
  if cp >= 0x2300 and cp <= 0x23FF then return true end
  -- Control pictures (U+2400-U+24FF)
  if cp >= 0x2400 and cp <= 0x24FF then return true end
  -- Geometric shapes (U+25A0-U+25FF)
  if cp >= 0x25A0 and cp <= 0x25FF then return true end
  -- Miscellaneous symbols and Dingbats
  if cp >= 0x2600 and cp <= 0x27BF then return true end
  -- CJK Symbols and Punctuation (U+3000-U+303F)
  if cp >= 0x3000 and cp <= 0x303F then return true end
  -- Fullwidth punctuation
  if cp >= 0xFE50 and cp <= 0xFE6F then return true end
  if cp >= 0xFF01 and cp <= 0xFF0F then return true end
  if cp >= 0xFF1A and cp <= 0xFF20 then return true end
  if cp >= 0xFF3B and cp <= 0xFF40 then return true end
  if cp >= 0xFF5B and cp <= 0xFF65 then return true end
  return false
end

--- True if `ch` is a Unicode punctuation character for GFM flanking.
--
-- GFM defines this (per the cmark reference implementation) as any
-- ASCII punctuation character OR any character in Unicode categories:
--   Pc, Pd, Pe, Pf, Pi, Po, Ps (punctuation) or Sm, Sc, Sk, So (symbols).
--
-- @param ch  string — single character (may be multi-byte UTF-8)
-- @return boolean
function M.is_unicode_punctuation(ch)
  if ch == "" then return false end
  -- ASCII punctuation is a subset
  if ASCII_PUNCT_SET[ch] then return true end
  -- Decode the first UTF-8 code point
  local ok, cp = pcall(function()
    -- utf8.codepoint is available in Lua 5.3+
    return utf8.codepoint(ch, 1)
  end)
  if not ok or cp == nil then return false end
  return is_unicode_punct_codepoint(cp)
end

--- True if `ch` is ASCII whitespace: space (U+0020), tab (U+0009),
-- newline (U+000A), form feed (U+000C), carriage return (U+000D).
-- @param ch  string — single character
-- @return boolean
function M.is_ascii_whitespace(ch)
  return ch == " " or ch == "\t" or ch == "\n" or ch == "\r" or ch == "\f"
end

-- Unicode whitespace code points beyond ASCII:
--   U+00A0 NO-BREAK SPACE
--   U+1680 OGHAM SPACE MARK
--   U+2000-U+200A various spaces (EN QUAD through HAIR SPACE)
--   U+202F NARROW NO-BREAK SPACE
--   U+205F MEDIUM MATHEMATICAL SPACE
--   U+3000 IDEOGRAPHIC SPACE
local UNICODE_WHITESPACE_EXTRA = {
  [0x00A0] = true,
  [0x1680] = true,
  [0x202F] = true,
  [0x205F] = true,
  [0x3000] = true,
}
for cp = 0x2000, 0x200A do
  UNICODE_WHITESPACE_EXTRA[cp] = true
end

--- True if `ch` is Unicode whitespace (any code point with Unicode
-- property White_Space=yes).
-- @param ch  string — single character (may be multi-byte UTF-8)
-- @return boolean
function M.is_unicode_whitespace(ch)
  if ch == "" then return false end
  -- ASCII whitespace first
  if M.is_ascii_whitespace(ch) then return true end
  -- Check non-ASCII whitespace
  local ok, cp = pcall(function()
    return utf8.codepoint(ch, 1)
  end)
  if not ok or cp == nil then return false end
  return UNICODE_WHITESPACE_EXTRA[cp] == true
end

--- True if `ch` is an ASCII digit (0-9).
-- @param ch  string — single character
-- @return boolean
function M.is_digit(ch)
  return ch >= "0" and ch <= "9"
end

--- Normalize a link label per GFM:
--   - Strip leading and trailing whitespace
--   - Collapse internal whitespace runs to a single space
--   - Fold to lowercase
--
-- Two labels are equivalent if their normalized forms are equal.
--
-- GFM §4.7: normalization uses Unicode case-folding. Lua's
-- string.lower() handles ASCII; for the ß → ss case, we post-process.
--
-- @param label  string — raw link label text
-- @return string — normalized label
-- Unicode-aware lowercase for use in normalize_link_label.
-- Converts uppercase code points to lowercase using Unicode case folding tables.
-- Covers the major scripts: Latin, Latin Extended, Greek, Cyrillic, Armenian.
-- Lua's string.lower() only handles ASCII; we handle the rest via utf8.codepoint.
local function unicode_lower_char(cp)
  -- ASCII uppercase A-Z → a-z
  if cp >= 0x41 and cp <= 0x5A then return cp + 32 end

  -- Latin-1 Supplement uppercase (À–Ö, Ø–Þ) → lowercase (à–ö, ø–þ)
  if cp >= 0xC0 and cp <= 0xD6 then return cp + 32 end
  if cp >= 0xD8 and cp <= 0xDE then return cp + 32 end

  -- ß (U+00DF) stays as ß but folds to "ss" — handled separately below
  -- Ÿ (U+0178) → ÿ (U+00FF)
  if cp == 0x178 then return 0xFF end

  -- Latin Extended-A (U+0100–U+017E): paired uppercase/lowercase
  -- Even code points are uppercase, odd are lowercase for most pairs
  if cp >= 0x100 and cp <= 0x12E then
    if cp % 2 == 0 then return cp + 1 end
  end
  if cp >= 0x130 and cp <= 0x136 then
    if cp % 2 == 0 then return cp + 1 end
  end
  -- U+0130 İ → U+0069 i (Turkish dotted I)
  if cp == 0x130 then return 0x69 end
  if cp >= 0x139 and cp <= 0x148 then
    if cp % 2 == 1 then return cp + 1 end
  end
  if cp >= 0x14A and cp <= 0x177 then
    if cp % 2 == 0 then return cp + 1 end
  end

  -- Latin Extended-B and IPA (U+0180–U+024F): selected mappings
  if cp == 0x181 then return 0x253 end
  if cp == 0x186 then return 0x254 end
  if cp == 0x189 then return 0x256 end
  if cp == 0x18A then return 0x257 end
  if cp == 0x18B then return 0x18C end
  if cp == 0x18E then return 0x1DD end
  if cp == 0x18F then return 0x259 end
  if cp == 0x190 then return 0x25B end
  if cp == 0x193 then return 0x260 end
  if cp == 0x194 then return 0x263 end
  if cp == 0x196 then return 0x269 end
  if cp == 0x197 then return 0x268 end
  if cp == 0x198 then return 0x199 end
  if cp == 0x19C then return 0x26F end
  if cp == 0x19D then return 0x272 end
  if cp == 0x19F then return 0x275 end
  if cp >= 0x1A0 and cp <= 0x1A5 then
    if cp % 2 == 0 then return cp + 1 end
  end
  if cp == 0x1A7 then return 0x1A8 end
  if cp == 0x1A9 then return 0x283 end
  if cp == 0x1AC then return 0x1AD end
  if cp == 0x1AE then return 0x288 end
  if cp >= 0x1AF and cp <= 0x1B0 then
    if cp % 2 == 1 then return cp + 1 end
  end
  if cp == 0x1B1 then return 0x28A end
  if cp == 0x1B2 then return 0x28B end
  if cp == 0x1B3 then return 0x1B4 end
  if cp == 0x1B5 then return 0x1B6 end
  if cp == 0x1B7 then return 0x292 end
  if cp == 0x1B8 then return 0x1B9 end
  if cp == 0x1BC then return 0x1BD end
  if cp == 0x1C4 then return 0x1C6 end
  if cp == 0x1C5 then return 0x1C6 end
  if cp == 0x1C7 then return 0x1C9 end
  if cp == 0x1C8 then return 0x1C9 end
  if cp == 0x1CA then return 0x1CC end
  if cp == 0x1CB then return 0x1CC end
  if cp >= 0x1CD and cp <= 0x1DC then
    if cp % 2 == 1 then return cp + 1 end
  end
  if cp >= 0x1DE and cp <= 0x1EF then
    if cp % 2 == 0 then return cp + 1 end
  end
  if cp == 0x1F1 then return 0x1F3 end
  if cp == 0x1F2 then return 0x1F3 end
  if cp >= 0x1F4 and cp <= 0x1F5 then
    if cp % 2 == 0 then return cp + 1 end
  end
  if cp >= 0x1F8 and cp <= 0x21F then
    if cp % 2 == 0 then return cp + 1 end
  end
  if cp >= 0x222 and cp <= 0x233 then
    if cp % 2 == 0 then return cp + 1 end
  end

  -- Greek uppercase Α–Ω (U+0391–U+03A9) → lowercase α–ω (U+03B1–U+03C9)
  if cp >= 0x391 and cp <= 0x3A9 then return cp + 32 end
  -- Greek extended uppercase (Ϊ, Ϋ, etc.)
  if cp == 0x386 then return 0x3AC end
  if cp == 0x388 then return 0x3AD end
  if cp == 0x389 then return 0x3AE end
  if cp == 0x38A then return 0x3AF end
  if cp == 0x38C then return 0x3CC end
  if cp == 0x38E then return 0x3CD end
  if cp == 0x38F then return 0x3CE end
  if cp == 0x3DA then return 0x3DB end
  if cp == 0x3DC then return 0x3DD end
  if cp == 0x3DE then return 0x3DF end
  if cp == 0x3E0 then return 0x3E1 end
  if cp >= 0x3E2 and cp <= 0x3EF then
    if cp % 2 == 0 then return cp + 1 end
  end

  -- Cyrillic uppercase А–Я (U+0410–U+042F) → lowercase а–я (U+0430–U+044F)
  if cp >= 0x410 and cp <= 0x42F then return cp + 32 end
  -- Cyrillic Ё (U+0401) → ё (U+0451)
  if cp == 0x400 then return 0x450 end
  if cp == 0x401 then return 0x451 end
  if cp >= 0x402 and cp <= 0x40F then return cp + 0x50 end
  if cp >= 0x460 and cp <= 0x481 then
    if cp % 2 == 0 then return cp + 1 end
  end
  if cp >= 0x48A and cp <= 0x4BF then
    if cp % 2 == 0 then return cp + 1 end
  end
  if cp >= 0x4C1 and cp <= 0x4CE then
    if cp % 2 == 1 then return cp + 1 end
  end
  if cp >= 0x4D0 and cp <= 0x52F then
    if cp % 2 == 0 then return cp + 1 end
  end

  -- Armenian (U+0531–U+0556) → lowercase (U+0561–U+0586)
  if cp >= 0x531 and cp <= 0x556 then return cp + 48 end

  return cp
end

--- Apply Unicode case folding to a UTF-8 string.
-- Processes each Unicode code point and converts uppercase to lowercase.
local function unicode_lower(s)
  local result = {}
  local i = 1
  while i <= #s do
    local b = string.byte(s, i)
    local cp, char_len
    if b < 0x80 then
      cp = b; char_len = 1
    elseif b < 0xE0 then
      cp = (b - 0xC0) * 0x40 + (string.byte(s, i+1) - 0x80)
      char_len = 2
    elseif b < 0xF0 then
      cp = (b - 0xE0) * 0x1000 + (string.byte(s, i+1) - 0x80) * 0x40 + (string.byte(s, i+2) - 0x80)
      char_len = 3
    else
      cp = (b - 0xF0) * 0x40000 + (string.byte(s, i+1) - 0x80) * 0x1000 + (string.byte(s, i+2) - 0x80) * 0x40 + (string.byte(s, i+3) - 0x80)
      char_len = 4
    end
    local lower_cp = unicode_lower_char(cp)
    -- Encode lower_cp back to UTF-8
    if lower_cp < 0x80 then
      result[#result+1] = string.char(lower_cp)
    elseif lower_cp < 0x800 then
      result[#result+1] = string.char(0xC0 + math.floor(lower_cp / 0x40), 0x80 + (lower_cp % 0x40))
    elseif lower_cp < 0x10000 then
      result[#result+1] = string.char(0xE0 + math.floor(lower_cp / 0x1000), 0x80 + math.floor((lower_cp % 0x1000) / 0x40), 0x80 + (lower_cp % 0x40))
    else
      result[#result+1] = string.char(0xF0 + math.floor(lower_cp / 0x40000), 0x80 + math.floor((lower_cp % 0x40000) / 0x1000), 0x80 + math.floor((lower_cp % 0x1000) / 0x40), 0x80 + (lower_cp % 0x40))
    end
    i = i + char_len
  end
  return table.concat(result)
end

function M.normalize_link_label(label)
  -- Trim, collapse whitespace, Unicode case fold
  local result = label:match("^%s*(.-)%s*$")  -- trim
  result = result:gsub("%s+", " ")             -- collapse whitespace
  result = unicode_lower(result)               -- Unicode case folding
  -- ß (U+00DF, UTF-8: 0xC3 0x9F) and ẞ (U+1E9E) both Unicode-fold to "ss"
  result = result:gsub("\xC3\x9F", "ss")       -- ß → ss
  result = result:gsub("\xE1\xBA\x9E", "ss")  -- ẞ (U+1E9E, UTF-8: E1 BA 9E) → ss
  return result
end

--- Normalize a URL: percent-encode characters that should not appear
-- unencoded in HTML href/src attributes.
--
-- Characters already percent-encoded (%XX) are left alone.
-- Safe characters (alphanumeric, -._~:/?#@!$&'()*+,;=%) pass through.
-- Everything else is percent-encoded.
--
-- @param url  string — the URL to normalize
-- @return string — normalized URL
function M.normalize_url(url)
  -- Characters that are safe to appear in URLs unencoded:
  -- unreserved: A-Z a-z 0-9 - . _ ~
  -- reserved (kept): : / ? # [ ] @ ! $ & ' ( ) * + , ; =
  -- percent-encoding: %
  -- Also keep already-encoded sequences (%XX)
  return url:gsub("([^%w%-%._~:/?#@!$&'()*+,;=%%])", function(ch)
    -- Percent-encode this character
    return string.format("%%%02X", string.byte(ch))
  end)
end

--- Apply backslash escapes: replace `\X` with `X` only when X is ASCII
-- punctuation. Non-punctuation backslash sequences are left as-is.
--
-- GFM §2.4: "Any ASCII punctuation character may be backslash-escaped.
-- All other characters preceded by a backslash are treated literally."
--
-- Examples:
--   `\*`  → `*`   (punctuation escape)
--   `\\`  → `\`   (backslash is punctuation)
--   `\b`  → `\b`  (not punctuation — kept as-is)
--
-- @param s  string — text potentially containing backslash escapes
-- @return string — text with backslash escapes applied
function M.apply_backslash_escapes(s)
  return (s:gsub("\\(.)", function(ch)
    if M.is_ascii_punctuation(ch) then
      return ch
    else
      return "\\" .. ch
    end
  end))
end

return M
