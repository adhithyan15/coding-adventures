-- test_lz78.lua — Tests for CodingAdventures.LZ78

local lz78 = require("coding_adventures.lz78")
local describe = describe
local it       = it
local assert   = assert

-- ─── Helpers ──────────────────────────────────────────────────────────────────

local function bytes(s)
  local t = {}
  for i = 1, #s do t[i] = s:byte(i) end
  return t
end

local function tokens_equal(a, b)
  if #a ~= #b then return false end
  for i = 1, #a do
    if a[i].dict_index ~= b[i].dict_index then return false end
    if a[i].next_char  ~= b[i].next_char  then return false end
  end
  return true
end

local function rt(s)
  return lz78.decompress(lz78.compress(s))
end

-- ─── TrieCursor ───────────────────────────────────────────────────────────────

describe("TrieCursor", function()
  it("new cursor starts at root with dict_id 0", function()
    local c = lz78.TrieCursor.new()
    assert.is_true(c:at_root())
    assert.equals(0, c:dict_id())
  end)

  it("step returns false on empty trie", function()
    local c = lz78.TrieCursor.new()
    assert.is_false(c:step(65))
  end)

  it("insert then step finds the child", function()
    local c = lz78.TrieCursor.new()
    c:insert(65, 1)
    assert.is_true(c:step(65))
    assert.equals(1, c:dict_id())
  end)

  it("insert does not advance cursor", function()
    local c = lz78.TrieCursor.new()
    c:insert(65, 1)
    assert.is_true(c:at_root())
  end)

  it("reset returns cursor to root", function()
    local c = lz78.TrieCursor.new()
    c:insert(65, 1)
    c:step(65)
    c:reset()
    assert.is_true(c:at_root())
  end)

  it("step misses on unknown byte after insert", function()
    local c = lz78.TrieCursor.new()
    c:insert(65, 1)
    assert.is_false(c:step(66))
  end)

  it("LZ78 simulation: AABCBBABC", function()
    local cursor  = lz78.TrieCursor.new()
    local next_id = 1
    local got     = {}
    local input   = "AABCBBABC"
    for i = 1, #input do
      local byte = input:byte(i)
      if not cursor:step(byte) then
        got[#got + 1] = {cursor:dict_id(), byte}
        cursor:insert(byte, next_id)
        next_id = next_id + 1
        cursor:reset()
      end
    end
    local want = {
      {0, 65}, {1, 66}, {0, 67}, {0, 66}, {4, 65}, {4, 67}
    }
    assert.equals(#want, #got)
    for i = 1, #want do
      assert.equals(want[i][1], got[i][1])
      assert.equals(want[i][2], got[i][2])
    end
  end)
end)

-- ─── encode ───────────────────────────────────────────────────────────────────

describe("encode", function()
  it("empty input produces no tokens", function()
    assert.equals(0, #lz78.encode(""))
  end)

  it("single byte produces one literal token", function()
    local tokens = lz78.encode("A")
    assert.equals(1, #tokens)
    assert.equals(0, tokens[1].dict_index)
    assert.equals(65, tokens[1].next_char)
  end)

  it("no repetition: all literals", function()
    local tokens = lz78.encode("ABCDE")
    assert.equals(5, #tokens)
    for _, tok in ipairs(tokens) do
      assert.equals(0, tok.dict_index)
    end
  end)

  it("AABCBBABC produces correct token sequence", function()
    local want = {
      {dict_index = 0, next_char = 65},
      {dict_index = 1, next_char = 66},
      {dict_index = 0, next_char = 67},
      {dict_index = 0, next_char = 66},
      {dict_index = 4, next_char = 65},
      {dict_index = 4, next_char = 67},
    }
    assert.is_true(tokens_equal(lz78.encode("AABCBBABC"), want))
  end)

  it("ABABAB ends with flush token", function()
    local want = {
      {dict_index = 0, next_char = 65},
      {dict_index = 0, next_char = 66},
      {dict_index = 1, next_char = 66},
      {dict_index = 3, next_char = 0},
    }
    assert.is_true(tokens_equal(lz78.encode("ABABAB"), want))
  end)

  it("all identical bytes produces 4 tokens for AAAAAAA", function()
    assert.equals(4, #lz78.encode("AAAAAAA"))
  end)
end)

-- ─── decode ───────────────────────────────────────────────────────────────────

describe("decode", function()
  it("empty tokens returns empty string", function()
    assert.equals("", lz78.decode({}))
  end)

  it("single literal token", function()
    assert.equals("A", lz78.decode({{dict_index=0, next_char=65}}, 1))
  end)

  it("AABCBBABC round-trips", function()
    local tokens = lz78.encode("AABCBBABC")
    assert.equals("AABCBBABC", lz78.decode(tokens, 9))
  end)

  it("ABABAB round-trips with original_length", function()
    local tokens = lz78.encode("ABABAB")
    assert.equals("ABABAB", lz78.decode(tokens, 6))
  end)
end)

-- ─── compress / decompress ────────────────────────────────────────────────────

describe("compress/decompress round-trip", function()
  local cases = {
    {"empty",    ""},
    {"single",   "A"},
    {"no rep",   "ABCDE"},
    {"identical","AAAAAAA"},
    {"AABCBBABC","AABCBBABC"},
    {"ABABAB",   "ABABAB"},
    {"hello",    "hello world"},
    {"repeated", string.rep("ABC", 100)},
  }

  for _, case_ in ipairs(cases) do
    local label, data = case_[1], case_[2]
    it("round-trip: " .. label, function()
      assert.equals(data, rt(data))
    end)
  end

  it("round-trip: binary with null bytes", function()
    local data = "\0\0\0\255\255"
    assert.equals(data, rt(data))
  end)

  it("round-trip: full byte range 0-255", function()
    local chars = {}
    for i = 0, 255 do chars[i + 1] = string.char(i) end
    local data = table.concat(chars)
    assert.equals(data, rt(data))
  end)
end)

-- ─── Parameters ───────────────────────────────────────────────────────────────

describe("max_dict_size", function()
  it("dict indices never exceed max_dict_size", function()
    local tokens = lz78.encode("ABCABCABCABCABC", 10)
    for _, tok in ipairs(tokens) do
      assert.is_true(tok.dict_index < 10)
    end
  end)

  it("max_dict_size=1 means no dictionary entries added", function()
    local tokens = lz78.encode("AAAA", 1)
    for _, tok in ipairs(tokens) do
      assert.equals(0, tok.dict_index)
    end
  end)
end)

-- ─── Wire format ──────────────────────────────────────────────────────────────

describe("wire format", function()
  it("compressed size matches expected format", function()
    local data = "AB"
    local compressed = lz78.compress(data)
    local tokens = lz78.encode(data)
    assert.equals(8 + #tokens * 4, #compressed)
  end)

  it("compress is deterministic", function()
    local data = "hello world test"
    assert.equals(lz78.compress(data), lz78.compress(data))
  end)
end)

-- ─── Compression effectiveness ────────────────────────────────────────────────

describe("compression effectiveness", function()
  it("repetitive data compresses smaller than original", function()
    local data = string.rep("ABC", 1000)
    assert.is_true(#lz78.compress(data) < #data)
  end)

  it("all same byte compresses significantly", function()
    local data = string.rep("A", 10000)
    local compressed = lz78.compress(data)
    assert.is_true(#compressed < #data)
    assert.equals(data, lz78.decompress(compressed))
  end)
end)
