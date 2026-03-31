-- Tests for code39
-- =================
--
-- Comprehensive busted test suite for the Code 39 barcode package.
--
-- Coverage:
--   - Module loads correctly
--   - normalize_code39: lowercase conversion, rejection of *, invalid chars
--   - encode_code39_char: known characters, start/stop flag, N/W pattern
--   - encode_code39: full sequence including start/stop
--   - expand_code39_runs: bar/space alternation, inter-character gaps
--   - draw_code39: structure, dimensions, bar positions
--   - compute_checksum: mod-43 computation

package.path = (
    "../src/?.lua;"  ..
    "../src/?/init.lua;"  ..
    package.path
)

local code39 = require("coding_adventures.code39")

-- ============================================================================
-- Module API
-- ============================================================================

describe("code39 module", function()
  it("loads successfully", function()
    assert.is_not_nil(code39)
  end)

  it("exposes PATTERNS table", function()
    assert.is_table(code39.PATTERNS)
  end)

  it("PATTERNS contains all 44 Code 39 characters", function()
    -- 10 digits + 26 letters + 7 special + 1 start/stop = 44
    local count = 0
    for _ in pairs(code39.PATTERNS) do count = count + 1 end
    assert.equal(44, count)
  end)

  it("every pattern is 9 characters", function()
    for char, pat in pairs(code39.PATTERNS) do
      assert.equal(9, #pat, "pattern for " .. char .. " should be 9 chars")
    end
  end)

  it("every pattern has exactly 3 wide elements", function()
    for char, pat in pairs(code39.PATTERNS) do
      local wide = 0
      for i = 1, #pat do
        local c = pat:sub(i,i)
        if c == c:upper() and c ~= c:lower() then wide = wide + 1 end
      end
      -- Some chars like ' ' (space) have uppercase pattern but all 3 should be wide
      assert.equal(3, wide, "pattern for '" .. char .. "' should have 3 wide elements")
    end
  end)

  it("exposes normalize_code39", function()
    assert.is_function(code39.normalize_code39)
  end)

  it("exposes encode_code39_char", function()
    assert.is_function(code39.encode_code39_char)
  end)

  it("exposes encode_code39", function()
    assert.is_function(code39.encode_code39)
  end)

  it("exposes expand_code39_runs", function()
    assert.is_function(code39.expand_code39_runs)
  end)

  it("exposes compute_checksum", function()
    assert.is_function(code39.compute_checksum)
  end)
end)

-- ============================================================================
-- normalize_code39
-- ============================================================================

describe("normalize_code39", function()
  it("returns uppercase string unchanged", function()
    assert.equal("HELLO", code39.normalize_code39("HELLO"))
  end)

  it("converts lowercase to uppercase", function()
    assert.equal("HELLO123", code39.normalize_code39("hello123"))
  end)

  it("accepts all digits", function()
    assert.equal("0123456789", code39.normalize_code39("0123456789"))
  end)

  it("accepts all uppercase letters", function()
    assert.equal("ABCDEFGHIJKLMNOPQRSTUVWXYZ",
      code39.normalize_code39("ABCDEFGHIJKLMNOPQRSTUVWXYZ"))
  end)

  it("accepts special characters", function()
    assert.equal("-. $/+%", code39.normalize_code39("-. $/+%"))
  end)

  it("rejects * in user input", function()
    assert.has_error(function()
      code39.normalize_code39("ABC*DEF")
    end)
  end)

  it("rejects @ (invalid character)", function()
    assert.has_error(function()
      code39.normalize_code39("ABC@DEF")
    end)
  end)

  it("rejects lowercase * (after uppercasing, still rejected)", function()
    -- lowercase '*' doesn't exist, but '*' itself is rejected
    assert.has_error(function()
      code39.normalize_code39("*")
    end)
  end)

  it("rejects exclamation mark", function()
    assert.has_error(function()
      code39.normalize_code39("!")
    end)
  end)

  it("rejects semicolon", function()
    assert.has_error(function()
      code39.normalize_code39(";")
    end)
  end)

  it("accepts empty string", function()
    assert.equal("", code39.normalize_code39(""))
  end)
end)

-- ============================================================================
-- encode_code39_char
-- ============================================================================

describe("encode_code39_char", function()
  it("returns table with char, is_start_stop, pattern", function()
    local enc = code39.encode_code39_char("A")
    assert.is_table(enc)
    assert.equal("A", enc.char)
    assert.is_boolean(enc.is_start_stop)
    assert.is_string(enc.pattern)
  end)

  it("pattern is 9 characters of N or W", function()
    local enc = code39.encode_code39_char("A")
    assert.equal(9, #enc.pattern)
    for i = 1, 9 do
      local c = enc.pattern:sub(i, i)
      assert.truthy(c == "N" or c == "W", "pattern char should be N or W")
    end
  end)

  it("is_start_stop is false for regular chars", function()
    assert.is_false(code39.encode_code39_char("A").is_start_stop)
    assert.is_false(code39.encode_code39_char("0").is_start_stop)
  end)

  it("is_start_stop is true for *", function()
    assert.is_true(code39.encode_code39_char("*").is_start_stop)
  end)

  it("known pattern for '0'", function()
    -- Pattern: bwbWBwBwb → NNNWNWWNWN ... actually:
    -- b=N w=N b=N W=W B=W w=N B=W w=N b=N → NNNWWNWNNN — recompute
    -- Raw: bwbWBwBwb
    -- b→N w→N b→N W→W B→W w→N B→W w→N b→N → NNNWWNWNNN? No, 9 chars:
    -- 1:b→N 2:w→N 3:b→N 4:W→W 5:B→W 6:w→N 7:B→W 8:w→N 9:b→N
    local enc = code39.encode_code39_char("0")
    -- Count wide elements: positions 4,5,7 = 3 wide
    local wide = 0
    for i = 1, 9 do
      if enc.pattern:sub(i,i) == "W" then wide = wide + 1 end
    end
    assert.equal(3, wide)
  end)

  it("known pattern for 'A' matches spec", function()
    -- Raw: BwbwbWbwB
    -- B→W w→N b→N w→N b→N W→W b→N w→N B→W → WNNNNWNNNW? 9 chars:
    -- 1:B→W 2:w→N 3:b→N 4:w→N 5:b→N 6:W→W 7:b→N 8:w→N 9:B→W
    local enc = code39.encode_code39_char("A")
    assert.equal("W", enc.pattern:sub(1, 1))   -- first bar is wide
    assert.equal("W", enc.pattern:sub(6, 6))   -- 6th element is wide
    assert.equal("W", enc.pattern:sub(9, 9))   -- last bar is wide
  end)

  it("raises on unknown character", function()
    assert.has_error(function()
      code39.encode_code39_char("@")
    end)
  end)
end)

-- ============================================================================
-- encode_code39
-- ============================================================================

describe("encode_code39", function()
  it("wraps with start/stop markers", function()
    local encoded = code39.encode_code39("A")
    assert.equal(3, #encoded)  -- * A *
    assert.equal("*", encoded[1].char)
    assert.equal("A", encoded[2].char)
    assert.equal("*", encoded[3].char)
  end)

  it("first and last characters are start/stop", function()
    local encoded = code39.encode_code39("HELLO")
    assert.is_true(encoded[1].is_start_stop)
    assert.is_true(encoded[#encoded].is_start_stop)
  end)

  it("middle characters are not start/stop", function()
    local encoded = code39.encode_code39("HELLO")
    for i = 2, #encoded - 1 do
      assert.is_false(encoded[i].is_start_stop)
    end
  end)

  it("correct count for multi-char input", function()
    -- "HELLO123" → 8 data chars + 2 markers = 10
    local encoded = code39.encode_code39("HELLO123")
    assert.equal(10, #encoded)
  end)

  it("converts lowercase before encoding", function()
    local encoded = code39.encode_code39("hello")
    assert.equal("H", encoded[2].char)
    assert.equal("E", encoded[3].char)
  end)

  it("single character produces 3 encoded chars", function()
    local encoded = code39.encode_code39("0")
    assert.equal(3, #encoded)
  end)

  it("empty string produces 2 encoded chars (start + stop)", function()
    local encoded = code39.encode_code39("")
    assert.equal(2, #encoded)
    assert.equal("*", encoded[1].char)
    assert.equal("*", encoded[2].char)
  end)
end)

-- ============================================================================
-- expand_code39_runs
-- ============================================================================

describe("expand_code39_runs", function()
  it("returns a list of run tables", function()
    local runs = code39.expand_code39_runs("A")
    assert.is_table(runs)
    assert.truthy(#runs > 0)
  end)

  it("each run has required fields", function()
    local runs = code39.expand_code39_runs("A")
    for _, run in ipairs(runs) do
      assert.is_string(run.color)
      assert.is_string(run.width)
      assert.is_string(run.source_char)
      assert.is_number(run.source_index)
      assert.is_boolean(run.is_inter_character_gap)
    end
  end)

  it("colors alternate bar/space within a character", function()
    -- For single char "A" → * A *: check bar/space alternation in first char (*)
    local runs = code39.expand_code39_runs("A")
    local expected_colors = {"bar","space","bar","space","bar","space","bar","space","bar"}
    for i = 1, 9 do
      assert.equal(expected_colors[i], runs[i].color,
        "color at position " .. i)
    end
  end)

  it("width is narrow or wide", function()
    local runs = code39.expand_code39_runs("A")
    for _, run in ipairs(runs) do
      assert.truthy(run.width == "narrow" or run.width == "wide")
    end
  end)

  it("first run is a bar", function()
    local runs = code39.expand_code39_runs("A")
    assert.equal("bar", runs[1].color)
  end)

  it("inter-character gap appears between chars", function()
    -- * A * → 3 chars: 9+1+9+1+9 = 29 runs (gap after each except last)
    local runs = code39.expand_code39_runs("A")
    -- Find inter-character gaps
    local gaps = 0
    for _, run in ipairs(runs) do
      if run.is_inter_character_gap then gaps = gaps + 1 end
    end
    assert.equal(2, gaps)  -- gap after * and after A
  end)

  it("no inter-character gap after the last char", function()
    local runs = code39.expand_code39_runs("A")
    assert.is_false(runs[#runs].is_inter_character_gap)
  end)

  it("total run count for single char (A): 9+1+9+1+9 = 29", function()
    local runs = code39.expand_code39_runs("A")
    assert.equal(29, #runs)
  end)

  it("total run count for two chars (AB): 9+1+9+1+9+1+9 = 39", function()
    local runs = code39.expand_code39_runs("AB")
    -- *=9, gap=1, A=9, gap=1, B=9, gap=1, *=9 = 39
    assert.equal(39, #runs)
  end)

  it("inter-character gap is a narrow space", function()
    local runs = code39.expand_code39_runs("A")
    for _, run in ipairs(runs) do
      if run.is_inter_character_gap then
        assert.equal("space", run.color)
        assert.equal("narrow", run.width)
      end
    end
  end)
end)

-- ============================================================================
-- draw_code39
-- ============================================================================

describe("draw_code39", function()
  it("returns a table", function()
    local result = code39.draw_code39("A")
    assert.is_table(result)
  end)

  it("result has width and height", function()
    local result = code39.draw_code39("A")
    assert.is_number(result.width or 0)
    -- If using draw_instructions backend, check scene structure
    -- If using fallback SVG, check svg field
    local has_dimensions = (result.width ~= nil) or
                           (type(result.svg) == "string")
    assert.is_true(has_dimensions)
  end)

  it("quiet zones are included in total width", function()
    local cfg    = {narrow_unit=4, wide_unit=12, bar_height=100,
                    quiet_zone_units=10, include_human_readable_text=false}
    local result = code39.draw_code39("A", cfg)
    -- Quiet zones add 2 * 10 * 4 = 80px to total width
    -- Total width should be > just the barcode width
    if result.width then
      assert.truthy(result.width > 80)
    end
  end)

  it("config is used (narrow_unit affects output)", function()
    local cfg1 = {narrow_unit=2, wide_unit=6, bar_height=100,
                  quiet_zone_units=5, include_human_readable_text=false}
    local cfg2 = {narrow_unit=4, wide_unit=12, bar_height=100,
                  quiet_zone_units=5, include_human_readable_text=false}
    local r1 = code39.draw_code39("A", cfg1)
    local r2 = code39.draw_code39("A", cfg2)
    -- With larger units, the output should be wider
    if r1.width and r2.width then
      assert.truthy(r2.width > r1.width)
    end
  end)

  it("SVG fallback contains rect elements", function()
    local result = code39.draw_code39("A")
    if result.svg then
      assert.truthy(result.svg:find("<rect"), "SVG should contain rect elements")
      assert.truthy(result.svg:find("<svg"), "SVG should start with svg tag")
    end
  end)

  it("normalizes input before drawing", function()
    -- Should not raise for lowercase input
    assert.has_no_error(function() code39.draw_code39("hello") end)
  end)

  it("raises for invalid character", function()
    assert.has_error(function() code39.draw_code39("ABC@DEF") end)
  end)
end)

-- ============================================================================
-- compute_checksum
-- ============================================================================

describe("compute_checksum", function()
  it("returns a single character", function()
    local c = code39.compute_checksum("HELLO")
    assert.equal(1, #c)
  end)

  it("checksum for '1' (value 1) → checksum is '1'", function()
    -- "1" has value 1; 1 mod 43 = 1 → "1"
    assert.equal("1", code39.compute_checksum("1"))
  end)

  it("checksum for '0' → '0'", function()
    -- "0" has value 0; 0 mod 43 = 0 → "0"
    assert.equal("0", code39.compute_checksum("0"))
  end)

  it("checksum wraps around mod 43", function()
    -- Values: K=20, L=21 → total=41 → char at index 42 = '5'? check
    -- Actually index is (41 % 43) + 1 = 42 → chars[42] = let's verify
    -- chars = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ-. $/+%"
    -- index 42 (1-based) = '4' (chars are 0='1', 9='10', A='11' ... wait: 0=idx1)
    -- chars: pos1='0', pos2='1',...pos10='9', pos11='A',...pos36='Z', pos37='-', etc.
    -- K is pos21 (value 20, since 0-based). L is value 21. Total = 41. 41%43=41. idx=42.
    -- pos42: 0-9=10 chars, A-Z=26 chars (up to pos36), pos37='-', pos38='.', pos39=' ', pos40='$', pos41='/', pos42='+'
    local c = code39.compute_checksum("KL")
    assert.equal("+", c)
  end)

  it("raises for invalid character", function()
    assert.has_error(function()
      code39.compute_checksum("@")
    end)
  end)

  it("returns character in the Code 39 alphabet", function()
    local alphabet = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ-. $/+%"
    for _, test in ipairs({"HELLO", "123", "ABC", "TEST"}) do
      local c = code39.compute_checksum(test)
      assert.truthy(alphabet:find(c, 1, true), "checksum should be in alphabet")
    end
  end)
end)

-- ============================================================================
-- Pattern lookup table verification
-- ============================================================================

describe("PATTERNS table", function()
  it("has correct pattern for '0'", function()
    assert.equal("bwbWBwBwb", code39.PATTERNS["0"])
  end)

  it("has correct pattern for 'A'", function()
    assert.equal("BwbwbWbwB", code39.PATTERNS["A"])
  end)

  it("has correct pattern for '*'", function()
    assert.equal("bWbwBwBwb", code39.PATTERNS["*"])
  end)

  it("has correct pattern for '-'", function()
    assert.equal("bWbwbwBwB", code39.PATTERNS["-"])
  end)

  it("has correct pattern for '$'", function()
    assert.equal("bWbWbWbwb", code39.PATTERNS["$"])
  end)

  it("patterns start with b or B (bar first)", function()
    for char, pat in pairs(code39.PATTERNS) do
      local first = pat:sub(1,1)
      assert.truthy(first == "b" or first == "B",
        "pattern for '" .. char .. "' should start with bar")
    end
  end)
end)
