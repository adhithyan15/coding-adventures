-- Tests for coding_adventures.deflate (CMP05 DEFLATE compression)

local deflate = require("coding_adventures.deflate")

local function roundtrip(data, label)
  local compressed = deflate.compress(data)
  local result = deflate.decompress(compressed)
  assert(result == data,
    string.format("roundtrip mismatch for %s: expected %d bytes, got %d bytes",
      label or "data", #data, #result))
end

-- Helper: unpack big-endian uint32 from string
local function unpack_uint32_be(s, pos)
  local a, b, c, d = s:byte(pos, pos + 3)
  return (a << 24) | (b << 16) | (c << 8) | d
end

-- Helper: unpack big-endian uint16 from string
local function unpack_uint16_be(s, pos)
  local a, b = s:byte(pos, pos + 1)
  return (a << 8) | b
end

-- ---------------------------------------------------------------------------
-- Edge cases
-- ---------------------------------------------------------------------------

describe("deflate edge cases", function()
  it("empty input", function()
    local compressed = deflate.compress("")
    local result = deflate.decompress(compressed)
    assert(result == "" or #result == 0, "empty input should decompress to empty")
  end)

  it("single byte 0x00", function()
    roundtrip(string.char(0), "0x00")
  end)

  it("single byte 0xFF", function()
    roundtrip(string.char(0xFF), "0xFF")
  end)

  it("single byte 'A'", function()
    roundtrip("A", "single A")
  end)

  it("single byte repeated (run)", function()
    roundtrip(string.rep("A", 20), "A×20")
    roundtrip(string.rep("\0", 100), "NUL×100")
  end)
end)

-- ---------------------------------------------------------------------------
-- Spec examples
-- ---------------------------------------------------------------------------

describe("deflate spec examples", function()
  it("AAABBC — all literals, no matches", function()
    local data = "AAABBC"
    roundtrip(data, "AAABBC")
    local compressed = deflate.compress(data)
    local dist_count = unpack_uint16_be(compressed, 7)
    assert(dist_count == 0, "expected dist_entry_count=0 for all-literals input, got " .. dist_count)
  end)

  it("AABCBBABC — one LZSS match", function()
    local data = "AABCBBABC"
    roundtrip(data, "AABCBBABC")
    local compressed = deflate.compress(data)
    local orig_len = unpack_uint32_be(compressed, 1)
    local dist_count = unpack_uint16_be(compressed, 7)
    assert(orig_len == 9, "expected original_length=9, got " .. orig_len)
    assert(dist_count > 0, "expected dist_entry_count>0 for input with a match")
  end)
end)

-- ---------------------------------------------------------------------------
-- Match tests
-- ---------------------------------------------------------------------------

describe("deflate match tests", function()
  it("overlapping match (run encoding)", function()
    roundtrip("AAAAAAA", "AAAAAAA")
    roundtrip("ABABABABABAB", "ABABAB...")
  end)

  it("multiple matches", function()
    roundtrip("ABCABCABCABC", "ABCABC×3")
    roundtrip("hello hello hello world", "hello×3")
  end)

  it("max match length ~255", function()
    roundtrip(string.rep("A", 300), "A×300")
  end)
end)

-- ---------------------------------------------------------------------------
-- Data variety
-- ---------------------------------------------------------------------------

describe("deflate data variety", function()
  it("all 256 byte values", function()
    local bytes = {}
    for i = 0, 255 do bytes[#bytes + 1] = string.char(i) end
    roundtrip(table.concat(bytes), "all-bytes")
  end)

  it("binary data 1000 bytes", function()
    local bytes = {}
    for i = 0, 999 do bytes[#bytes + 1] = string.char(i % 256) end
    roundtrip(table.concat(bytes), "binary-1000")
  end)

  it("longer text with repetition", function()
    local base = "the quick brown fox jumps over the lazy dog "
    local data = base:rep(10)
    roundtrip(data, "lorem×10")
  end)
end)

-- ---------------------------------------------------------------------------
-- Compression ratio
-- ---------------------------------------------------------------------------

describe("deflate compression ratio", function()
  it("highly repetitive data compresses to < 50%", function()
    local data = string.rep("ABCABC", 100)
    local compressed = deflate.compress(data)
    assert(#compressed < #data * 0.5,
      string.format("expected significant compression: %d >= %d×0.5=%d",
        #compressed, #data, math.floor(#data * 0.5)))
  end)
end)

-- ---------------------------------------------------------------------------
-- Various match lengths
-- ---------------------------------------------------------------------------

describe("deflate various lengths", function()
  local lengths = {3, 4, 10, 11, 13, 19, 35, 67, 131, 227, 255}
  for _, length in ipairs(lengths) do
    it("match length " .. length, function()
      local prefix = string.rep("A", length)
      local data = prefix .. "BBB" .. prefix
      roundtrip(data, "length=" .. length)
    end)
  end
end)
