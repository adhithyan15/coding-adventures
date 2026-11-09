-- test_hostile_headers.lua — security regression tests
--
-- Each case is a few bytes whose header claims an enormous count or length.  An
-- implementation that looped or allocated from the claim would run for minutes
-- or exhaust memory; these tests assert bounded time and sane results, plus
-- round trips over every input of length <= 2 and a seeded random sample.

package.path = "../src/?.lua;../src/?/init.lua;" .. package.path

local codec = require("coding_adventures.lzw")

local function bin(...)
  return string.char(...)
end

local function u32(n)
  return string.char((n >> 24) & 255, (n >> 16) & 255, (n >> 8) & 255, n & 255)
end

local function quick(f)
  local start = os.clock()
  local ok, result = pcall(f)
  assert.is_true(os.clock() - start < 10, "hostile input took too long")
  return ok, result
end

describe("lzw hostile headers", function()
  it("garbage headers do not hang", function()
    quick(function() return codec.decompress(string.rep("\255", 4)) end)
    quick(function() return codec.decompress(string.rep("\255", 64)) end)
  end)

  it("declared length larger than the stream", function()
    local c = codec.compress("hello hello hello")
    local ok, r = quick(function() return codec.decompress(u32(0xFFFFFFFF) .. c:sub(5)) end)
    assert.is_true(ok)
    assert.are.equal("hello hello hello", r)
  end)

  it("declared length truncates output", function()
    local c = codec.compress("hello hello hello")
    local ok, r = quick(function() return codec.decompress(u32(5) .. c:sub(5)) end)
    assert.is_true(ok)
    assert.are.equal("hello", r)
  end)

  it("round-trips every input of length <= 2", function()
    local inputs = {""}
    for a = 0, 255 do
      inputs[#inputs + 1] = bin(a)
      for b = 0, 255 do inputs[#inputs + 1] = bin(a, b) end
    end
    for _, d in ipairs(inputs) do
      assert.are.equal(d, codec.decompress(codec.compress(d)))
    end
  end)

  it("round-trips a seeded random sample", function()
    math.randomseed(0x5EED)
    local alphabets = {2, 4, 16, 256}
    for i = 0, 299 do
      local n = 3 + math.random(0, 596)
      local t = {}
      for j = 1, n do t[j] = math.random(0, alphabets[(i % 4) + 1] - 1) end
      local d = string.char(table.unpack(t))
      assert.are.equal(d, codec.decompress(codec.compress(d)))
    end
  end)
end)
