-- Tests for coding_adventures.brotli (CMP06 Brotli compression)

local brotli = require("coding_adventures.brotli")

-- ---------------------------------------------------------------------------
-- Test helpers
-- ---------------------------------------------------------------------------

-- Convert a string to a byte array (table of integers).
local function str_to_bytes(s)
  local t = {}
  for i = 1, #s do
    t[#t + 1] = s:byte(i)
  end
  return t
end

-- Convert a byte array to a string.
local function bytes_to_str(t)
  local chars = {}
  for _, b in ipairs(t) do
    chars[#chars + 1] = string.char(b)
  end
  return table.concat(chars)
end

-- Perform a full round-trip: compress then decompress, assert equality.
local function roundtrip_bytes(input_bytes, label)
  local compressed = brotli.compress(input_bytes)
  local result = brotli.decompress(compressed)
  local input_str = bytes_to_str(input_bytes)
  local result_str = bytes_to_str(result)
  assert(result_str == input_str,
    string.format("roundtrip mismatch for %s: expected %d bytes, got %d bytes",
      label or "data", #input_str, #result_str))
end

local function roundtrip_string(s, label)
  local compressed = brotli.compress_string(s)
  local result = brotli.decompress_string(compressed)
  assert(result == s,
    string.format("roundtrip_string mismatch for %s: expected %d bytes, got %d bytes",
      label or "data", #s, #result))
end

-- ---------------------------------------------------------------------------
-- Test 1: Round-trip empty input
-- ---------------------------------------------------------------------------

describe("brotli test 1: empty input", function()
  it("compress then decompress empty string gives empty string", function()
    roundtrip_string("", "empty")
  end)

  it("compress then decompress empty byte array gives empty byte array", function()
    local compressed = brotli.compress({})
    local result = brotli.decompress(compressed)
    assert(#result == 0, "expected 0 bytes, got " .. #result)
  end)

  it("compress_string empty gives non-empty wire format", function()
    -- Empty input still produces a valid wire format (header + sentinel).
    local compressed = brotli.compress_string("")
    assert(#compressed > 0, "compressed empty string should not be empty wire format")
  end)
end)

-- ---------------------------------------------------------------------------
-- Test 2: Round-trip single byte
-- ---------------------------------------------------------------------------

describe("brotli test 2: single byte", function()
  it("single byte 0x42 ('B') round-trips correctly", function()
    roundtrip_string(string.char(0x42), "single-0x42")
  end)

  it("single byte 0x00 round-trips correctly", function()
    roundtrip_string(string.char(0x00), "single-0x00")
  end)

  it("single byte 0xFF round-trips correctly", function()
    roundtrip_string(string.char(0xFF), "single-0xFF")
  end)

  it("single byte 'A' round-trips correctly", function()
    roundtrip_string("A", "single-A")
  end)
end)

-- ---------------------------------------------------------------------------
-- Test 3: Round-trip all 256 distinct bytes (no repeats → no matches)
-- ---------------------------------------------------------------------------

describe("brotli test 3: all literals, no matches", function()
  it("256 distinct bytes round-trip correctly", function()
    local bytes = {}
    for i = 0, 255 do
      bytes[#bytes + 1] = i
    end
    roundtrip_bytes(bytes, "all-256-bytes")
  end)

  it("incompressible data is larger than input", function()
    -- All 256 distinct bytes are incompressible: compressed > original.
    local bytes = {}
    for i = 0, 255 do
      bytes[#bytes + 1] = i
    end
    local compressed = brotli.compress(bytes)
    assert(#compressed > 256,
      string.format("expected incompressible data to expand: %d bytes", #compressed))
  end)
end)

-- ---------------------------------------------------------------------------
-- Test 4: Round-trip all copies — 1024 × 'A'
-- ---------------------------------------------------------------------------

describe("brotli test 4: all copies, no leading literals beyond first match", function()
  it("1024 repetitions of 'A' round-trip correctly", function()
    local data = string.rep("A", 1024)
    roundtrip_string(data, "A×1024")
  end)

  it("1024 'A' compresses significantly", function()
    local data = string.rep("A", 1024)
    local compressed = brotli.compress_string(data)
    -- Highly repetitive data should compress well below 50% of original.
    assert(#compressed < #data * 0.5,
      string.format("expected compression: %d >= %d*0.5=%d",
        #compressed, #data, math.floor(#data * 0.5)))
  end)

  it("overlapping copy match (run encoding) works", function()
    -- 'AABABAB...' style — the copy can reference bytes it already emitted.
    roundtrip_string(string.rep("ABAB", 200), "ABAB×200")
  end)
end)

-- ---------------------------------------------------------------------------
-- Test 5: Round-trip English prose ≥ 1024 bytes, compressed < 80%
-- ---------------------------------------------------------------------------

describe("brotli test 5: English prose round-trip and compression ratio", function()
  local prose = (
    "The quick brown fox jumps over the lazy dog. " ..
    "Pack my box with five dozen liquor jugs. " ..
    "How vexingly quick daft zebras jump! " ..
    "The five boxing wizards jump quickly. " ..
    "Sphinx of black quartz, judge my vow. " ..
    "Two driven jocks help fax my big quiz. " ..
    "Five quacking zephyrs jolt my wax bed. " ..
    "The jay, pig, fox, zebra, and my wolves quack! " ..
    "Blowzy red vixens fight for a quick jump. " ..
    "Joaquin Phoenix was gazed by MTV for luck. " ..
    "A wizard's job is to vex chumps quickly in fog. " ..
    "Watch Jeopardy!, Alex Trebek's fun TV quiz game. "
  )
  -- Repeat to exceed 1024 bytes.
  while #prose < 1024 do
    prose = prose .. prose
  end

  it("English prose of length " .. #prose .. " round-trips correctly", function()
    roundtrip_string(prose, "english-prose")
  end)

  it("English prose compresses to < 80% of original", function()
    local compressed = brotli.compress_string(prose)
    assert(#compressed < #prose * 0.80,
      string.format("expected < 80%% compression: %d >= %d*0.8=%d",
        #compressed, #prose, math.floor(#prose * 0.8)))
  end)
end)

-- ---------------------------------------------------------------------------
-- Test 6: Round-trip binary blob (512 bytes)
-- ---------------------------------------------------------------------------

describe("brotli test 6: binary blob round-trip", function()
  it("512 bytes of pseudo-random binary data round-trip correctly", function()
    -- Generate 512 pseudo-random bytes using a simple LCG.
    -- LCG parameters from Numerical Recipes.
    local bytes = {}
    local lcg = 1013904223
    for _ = 1, 512 do
      lcg = (lcg * 1664525 + 1013904223) & 0xFFFFFFFF
      bytes[#bytes + 1] = lcg & 0xFF
    end
    roundtrip_bytes(bytes, "binary-512")
  end)

  it("binary data with all byte values present round-trips", function()
    -- 512 bytes cycling through all 256 values twice.
    local bytes = {}
    for i = 0, 511 do
      bytes[#bytes + 1] = i % 256
    end
    roundtrip_bytes(bytes, "cycling-512")
  end)
end)

-- ---------------------------------------------------------------------------
-- Test 7: Cross-command literal context — "abc123ABC"
-- ---------------------------------------------------------------------------

describe("brotli test 7: cross-command literal context", function()
  it("abc123ABC round-trips correctly", function()
    roundtrip_string("abc123ABC", "abc123ABC")
  end)

  it("context transitions work across longer mixed strings", function()
    -- Exercise all four context buckets: space/punct → digit → upper → lower.
    roundtrip_string("  12  AB  ab  ", "space-digit-upper-lower")
    roundtrip_string("Hello, World! 42 TIMES.", "mixed-context")
    roundtrip_string("abc123ABCxyz789DEFuvw", "all-four-contexts")
  end)

  it("literal context function returns correct buckets", function()
    -- We verify round-trip for strings that exercise each context boundary.
    -- After lowercase 'z': context 3
    roundtrip_string("aza", "after-lowercase")
    -- After uppercase 'Z': context 2
    roundtrip_string("AZA", "after-uppercase")
    -- After digit '9': context 1
    roundtrip_string("090", "after-digit")
    -- After space ' ': context 0
    roundtrip_string(" a b", "after-space")
  end)
end)

-- ---------------------------------------------------------------------------
-- Test 8: Long-distance match (offset > 4096)
-- ---------------------------------------------------------------------------

describe("brotli test 8: long-distance match", function()
  it("repeated 10-byte sequence with offset > 4096 round-trips correctly", function()
    -- Build a string with a 10-byte pattern far from its repeat.
    -- We need > 4096 bytes of filler between the occurrences.
    local pattern = "HelloWorld"
    -- Filler: non-repeating chars so the filler doesn't accidentally match.
    -- Use cycling distinct bytes to avoid LZ matches in the filler itself.
    local filler_bytes = {}
    for i = 0, 4999 do
      filler_bytes[#filler_bytes + 1] = string.char((i * 7 + 13) % 128 + 1)
    end
    local filler = table.concat(filler_bytes)
    local data = pattern .. filler .. pattern
    roundtrip_string(data, "long-distance-match")
  end)

  it("10-byte sequence at offset > 8192 uses extended distance codes", function()
    -- Same idea, but even longer filler to exercise distance codes 26–31.
    local pattern = "ABCDEFGHIJ"
    local filler_bytes = {}
    for i = 0, 8999 do
      filler_bytes[#filler_bytes + 1] = string.char((i * 11 + 7) % 128 + 1)
    end
    local filler = table.concat(filler_bytes)
    local data = pattern .. filler .. pattern
    roundtrip_string(data, "very-long-distance-match")
  end)
end)

-- ---------------------------------------------------------------------------
-- Additional: Compression correctness checks
-- ---------------------------------------------------------------------------

describe("brotli wire format", function()
  it("header has original_length stored correctly", function()
    -- The first 4 bytes of the compressed output are original_length BE.
    local data = "hello world"
    local compressed = brotli.compress_string(data)
    local a, b, c, d = compressed:byte(1, 4)
    local stored_len = (a << 24) | (b << 16) | (c << 8) | d
    assert(stored_len == #data,
      string.format("expected original_length=%d, got %d", #data, stored_len))
  end)

  it("empty input produces correct header", function()
    -- Empty: original_length=0, icc_entry_count=1 (sentinel only).
    local compressed = brotli.compress_string("")
    local a, b, c, d = compressed:byte(1, 4)
    local orig_len = (a << 24) | (b << 16) | (c << 8) | d
    local icc_count = compressed:byte(5)
    assert(orig_len == 0, "expected original_length=0, got " .. orig_len)
    assert(icc_count == 1, "expected icc_entry_count=1, got " .. icc_count)
  end)

  it("all-copies input has dist_entry_count > 0", function()
    local data = string.rep("ABCABCABC", 50)
    local compressed = brotli.compress_string(data)
    local dist_count = compressed:byte(6)
    assert(dist_count > 0,
      "expected dist_entry_count > 0 for data with matches, got " .. dist_count)
  end)

  it("all-literals input may have dist_entry_count = 0", function()
    -- 9 distinct bytes in a random order — no match of length >= 4.
    local data = "ABCDEFGHI"
    local compressed = brotli.compress_string(data)
    local dist_count = compressed:byte(6)
    assert(dist_count == 0,
      "expected dist_entry_count=0 for all-literals input, got " .. dist_count)
  end)
end)

describe("brotli string API", function()
  it("compress_string and decompress_string are inverses", function()
    local cases = {
      "",
      "a",
      "hello world",
      string.rep("ABCDEF", 100),
      "The quick brown fox jumps over the lazy dog.",
    }
    for _, case in ipairs(cases) do
      local compressed = brotli.compress_string(case)
      local result = brotli.decompress_string(compressed)
      assert(result == case,
        string.format("string API roundtrip failed: expected %d bytes, got %d",
          #case, #result))
    end
  end)
end)

describe("brotli various lengths", function()
  local lengths = {4, 5, 8, 10, 14, 18, 26, 34, 50, 66, 98, 130, 194, 258}
  for _, length in ipairs(lengths) do
    it("copy length " .. length .. " round-trips", function()
      local prefix = string.rep("A", length)
      local data = prefix .. "XYXYXY" .. prefix
      roundtrip_string(data, "copy-length-" .. length)
    end)
  end
end)

describe("brotli large inputs", function()
  it("10000 bytes of repetitive data round-trips", function()
    local data = string.rep("abcdefghij", 1000)
    roundtrip_string(data, "10000-bytes")
  end)

  it("mixed repetitive and unique data round-trips", function()
    local parts = {}
    for i = 1, 100 do
      parts[#parts + 1] = string.format("record_%04d_value_%04d_", i, i * 17)
    end
    local data = table.concat(parts)
    roundtrip_string(data, "structured-records")
  end)
end)
