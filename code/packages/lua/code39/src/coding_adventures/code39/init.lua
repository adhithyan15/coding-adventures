-- code39 — Code 39 Barcode Encoder
-- ===================================
--
-- This package is part of the coding-adventures monorepo.
-- It implements the Code 39 barcode standard — a 1D symbology that can encode
-- uppercase letters, digits, and a handful of special characters.
--
-- # Why Code 39 First?
-- =====================
--
-- Code 39 is the perfect teaching barcode because:
--
--   1. It is easy to explain — each character is independent.
--   2. It supports A-Z + 0-9 + symbols, so we can encode real strings.
--   3. The structure is visible in the width patterns — no complex math.
--   4. Start/stop markers (*) make boundaries obvious.
--   5. Checksum is optional, keeping the base version simple.
--
-- # How Code 39 Works
-- ====================
--
-- Every character is encoded as exactly 9 elements: 5 bars + 4 spaces,
-- alternating bar-space-bar-space-...-bar.  Of those 9 elements, exactly
-- 3 are WIDE and 6 are NARROW.
--
-- Example — the character "A":
--
--   A → pattern "BwbwbWbwB"
--   where uppercase = WIDE, lowercase = NARROW, b/B = bar, w/W = space
--
--   Position:  1   2   3   4   5   6   7   8   9
--   Element:   bar sp  bar sp  bar sp  bar sp  bar
--   Width:      W   n   n   n   n   W   n   n   W
--
-- Visual:
--
--   ██ █ █  ██  █ ██
--    (wide) (narrow)(wide)(narrow)(narrow)(wide)
--
-- Characters are separated by a NARROW inter-character gap (a white space).
-- Every barcode starts and ends with "*" (the start/stop character).
--
--   Input: "AB"
--   Encoded: *AB*
--   Printed: [quiet][*][gap][A][gap][B][gap][*][quiet]
--
-- # Pattern Key
-- ==============
--
-- The @patterns table maps each character to a 9-character lowercase string:
--
--   b = narrow bar    B = wide bar
--   w = narrow space  W = wide space
--
-- To get the N/W pattern: uppercase = Wide, lowercase = Narrow.
--
-- # Dependencies
-- ===============
--
-- None — this package is self-contained.  The draw_instructions package is
-- an OPTIONAL dependency for rendering.  If not available, draw_code39()
-- returns an SVG string directly.

local M = {}

-- ============================================================================
-- Complete Code 39 character encoding table
-- ============================================================================
--
-- 44 supported characters:
--   0-9   (digits)
--   A-Z   (uppercase letters)
--   - . SPACE $ / + %   (special characters)
--   *     (start/stop — not valid in user input)
--
-- Each pattern is 9 characters long, using:
--   b/B = narrow/wide BAR
--   w/W = narrow/wide SPACE
--
-- The pattern always starts with a bar and alternates bar-space-bar-...
--
-- Verification: each pattern has exactly 3 uppercase (wide) chars and
-- 6 lowercase (narrow) chars.

M.PATTERNS = {
  ["0"] = "bwbWBwBwb", ["1"] = "BwbWbwbwB", ["2"] = "bwBWbwbwB", ["3"] = "BwBWbwbwb",
  ["4"] = "bwbWBwbwB", ["5"] = "BwbWBwbwb", ["6"] = "bwBWBwbwb", ["7"] = "bwbWbwBwB",
  ["8"] = "BwbWbwBwb", ["9"] = "bwBWbwBwb", ["A"] = "BwbwbWbwB", ["B"] = "bwBwbWbwB",
  ["C"] = "BwBwbWbwb", ["D"] = "bwbwBWbwB", ["E"] = "BwbwBWbwb", ["F"] = "bwBwBWbwb",
  ["G"] = "bwbwbWBwB", ["H"] = "BwbwbWBwb", ["I"] = "bwBwbWBwb", ["J"] = "bwbwBWBwb",
  ["K"] = "BwbwbwbWB", ["L"] = "bwBwbwbWB", ["M"] = "BwBwbwbWb", ["N"] = "bwbwBwbWB",
  ["O"] = "BwbwBwbWb", ["P"] = "bwBwBwbWb", ["Q"] = "bwbwbwBWB", ["R"] = "BwbwbwBWb",
  ["S"] = "bwBwbwBWb", ["T"] = "bwbwBwBWb", ["U"] = "BWbwbwbwB", ["V"] = "bWBwbwbwB",
  ["W"] = "BWBwbwbwb", ["X"] = "bWbwBwbwB", ["Y"] = "BWbwBwbwb", ["Z"] = "bWBwBwbwb",
  ["-"] = "bWbwbwBwB", ["."] = "BWbwbwBwb", [" "] = "bWBwbwBwb", ["$"] = "bWbWbWbwb",
  ["/"] = "bWbWbwbWb", ["+"] = "bWbwbWbWb", ["%"] = "bwbWbWbWb", ["*"] = "bWbwBwBwb",
}

-- ============================================================================
-- Default render configuration
-- ============================================================================

M.DEFAULT_CONFIG = {
  narrow_unit              = 4,
  wide_unit                = 12,
  bar_height               = 120,
  quiet_zone_units         = 10,
  include_human_readable_text = true,
}

-- ============================================================================
-- normalize_code39(data) → normalized_string
-- ============================================================================
--
-- Validation rules:
--   - Convert lowercase to uppercase
--   - Reject '*' (reserved for start/stop)
--   - Reject any character not in the Code 39 alphabet

function M.normalize_code39(data)
  local normalized = string.upper(data)
  for i = 1, #normalized do
    local ch = normalized:sub(i, i)
    if ch == "*" then
      error(string.format(
        'input must not contain "*" because it is reserved for start/stop'))
    end
    if not M.PATTERNS[ch] then
      error(string.format(
        'invalid character: "%s" is not supported by Code 39', ch))
    end
  end
  return normalized
end

-- ============================================================================
-- encode_code39_char(char) → EncodedCharacter
-- ============================================================================
--
-- Returns a table:
--   {
--     char         = "A",
--     is_start_stop = false,
--     pattern      = "NNNNNWNNW",   -- N=narrow, W=wide
--   }
--
-- The pattern uses N/W notation for readability in tests and visualizers.
-- The raw b/w/B/W encoding is preserved in the patterns table for internal use.

function M.encode_code39_char(char)
  local raw = M.PATTERNS[char]
  if not raw then
    error(string.format('unknown Code 39 character: "%s"', char))
  end

  -- Convert b/B/w/W to N/W:  uppercase = Wide, lowercase = Narrow
  local pattern = raw:gsub(".", function(c)
    if c == c:upper() then return "W" else return "N" end
  end)

  return {
    char          = char,
    is_start_stop = (char == "*"),
    pattern       = pattern,
  }
end

-- ============================================================================
-- encode_code39(data) → list of EncodedCharacter
-- ============================================================================
--
-- Wraps the normalized input with start/stop markers and encodes each char.

function M.encode_code39(data)
  local normalized = M.normalize_code39(data)
  local with_markers = "*" .. normalized .. "*"
  local result = {}
  for i = 1, #with_markers do
    result[#result + 1] = M.encode_code39_char(with_markers:sub(i, i))
  end
  return result
end

-- ============================================================================
-- expand_code39_runs(data) → list of BarcodeRun
-- ============================================================================
--
-- Expands encoded characters into alternating bar/space runs, including
-- the inter-character narrow gap between characters (except after the last).
--
-- Each BarcodeRun is a table:
--   {
--     color                 = "bar" | "space",
--     width                 = "narrow" | "wide",
--     source_char           = "A",
--     source_index          = 0,
--     is_inter_character_gap = false | true,
--   }
--
-- Element ordering: bar-space-bar-space-bar-space-bar-space-bar (9 total)

local COLORS = {"bar","space","bar","space","bar","space","bar","space","bar"}

function M.expand_code39_runs(data)
  local encoded = M.encode_code39(data)
  local runs    = {}

  for source_index, enc_char in ipairs(encoded) do
    -- Emit 9 elements for this character
    for elem_idx = 1, 9 do
      local element = enc_char.pattern:sub(elem_idx, elem_idx)
      runs[#runs + 1] = {
        color                 = COLORS[elem_idx],
        width                 = (element == "W") and "wide" or "narrow",
        source_char           = enc_char.char,
        source_index          = source_index - 1,  -- 0-based index
        is_inter_character_gap = false,
      }
    end

    -- Emit inter-character gap (narrow space) after every character except the last
    if source_index < #encoded then
      runs[#runs + 1] = {
        color                 = "space",
        width                 = "narrow",
        source_char           = enc_char.char,
        source_index          = source_index - 1,
        is_inter_character_gap = true,
      }
    end
  end

  return runs
end

-- ============================================================================
-- draw_code39(data, config) → DrawScene or SVG string
-- ============================================================================
--
-- Produces either:
--   a) A draw_instructions DrawScene (if draw_instructions is available)
--   b) A plain SVG string (as a fallback)
--
-- The config table follows DEFAULT_CONFIG structure.

function M.draw_code39(data, config)
  config = config or M.DEFAULT_CONFIG

  local normalized      = M.normalize_code39(data)
  local quiet_px        = config.quiet_zone_units * config.narrow_unit
  local runs            = M.expand_code39_runs(normalized)
  local text_margin     = 8
  local text_font_size  = 16

  -- Compute total width and collect bar rectangles
  local cursor_x = quiet_px
  local rects    = {}

  for _, run in ipairs(runs) do
    local w = (run.width == "wide") and config.wide_unit or config.narrow_unit
    if run.color == "bar" then
      rects[#rects + 1] = {
        x      = cursor_x,
        y      = 0,
        width  = w,
        height = config.bar_height,
        fill   = "#000000",
        meta   = { char = run.source_char, index = run.source_index },
      }
    end
    cursor_x = cursor_x + w
  end

  local total_width = cursor_x + quiet_px
  local text_block  = config.include_human_readable_text
                      and (text_margin + text_font_size + 4)
                      or 0
  local total_height = config.bar_height + text_block

  -- Try to use draw_instructions if available
  local ok, di = pcall(require, "coding_adventures.draw_instructions")
  if ok then
    local instructions = {}
    for _, r in ipairs(rects) do
      instructions[#instructions + 1] = di.draw_rect(
        r.x, r.y, r.width, r.height, r.fill, r.meta)
    end
    if config.include_human_readable_text then
      instructions[#instructions + 1] = di.draw_text(
        math.floor((cursor_x + quiet_px) / 2),
        config.bar_height + text_margin + text_font_size - 2,
        normalized,
        { role = "label" }
      )
    end
    return di.create_scene(
      total_width,
      total_height,
      instructions,
      "#ffffff",
      { label = "Code 39 barcode for " .. normalized, symbology = "code39" }
    )
  end

  -- Fallback: emit plain SVG string
  local svg_parts = {
    string.format('<svg xmlns="http://www.w3.org/2000/svg" width="%d" height="%d" viewBox="0 0 %d %d">',
      total_width, total_height, total_width, total_height),
    string.format('<rect x="0" y="0" width="%d" height="%d" fill="#ffffff"/>',
      total_width, total_height),
  }
  for _, r in ipairs(rects) do
    svg_parts[#svg_parts + 1] = string.format(
      '<rect x="%d" y="%d" width="%d" height="%d" fill="%s"/>',
      r.x, r.y, r.width, r.height, r.fill)
  end
  if config.include_human_readable_text then
    svg_parts[#svg_parts + 1] = string.format(
      '<text x="%d" y="%d" text-anchor="middle" font-size="%d">%s</text>',
      math.floor(total_width / 2),
      config.bar_height + text_margin + text_font_size - 2,
      text_font_size,
      normalized)
  end
  svg_parts[#svg_parts + 1] = '</svg>'

  return {
    svg        = table.concat(svg_parts, "\n"),
    width      = total_width,
    height     = total_height,
    symbology  = "code39",
    data       = normalized,
  }
end

-- ============================================================================
-- Checksum (optional — mod 43)
-- ============================================================================
--
-- Code 39 has an optional modulo-43 checksum.  The check character is
-- appended before the stop marker.
--
-- Character values (0–42):
--   0-9 → 0-9
--   A-Z → 10-35
--   - . SPACE $ / + % → 36-42

local CHECKSUM_VALUES = {}
do
  local chars = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ-. $/+%"
  for i = 1, #chars do
    CHECKSUM_VALUES[chars:sub(i,i)] = i - 1
  end
end

--- Compute the optional mod-43 check character for a Code 39 payload.
-- @param data  Normalized Code 39 string (without start/stop)
-- @return      Single check character string
function M.compute_checksum(data)
  local total = 0
  for i = 1, #data do
    local ch  = data:sub(i, i)
    local val = CHECKSUM_VALUES[ch]
    if not val then
      error(string.format('cannot compute checksum for invalid character "%s"', ch))
    end
    total = total + val
  end
  local idx = (total % 43) + 1
  local chars = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ-. $/+%"
  return chars:sub(idx, idx)
end

return M
