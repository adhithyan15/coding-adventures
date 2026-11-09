-- test_hostile_headers.lua — security regression tests
--
-- Each case is a few bytes whose header claims an enormous count or length.  An
-- implementation that looped or allocated from the claim would run for minutes
-- or exhaust memory; these tests assert bounded time and sane results, plus
-- round trips over every input of length <= 2 and a seeded random sample.

package.path = "../src/?.lua;../src/?/init.lua;" .. package.path

local codec = require("coding_adventures.lz77")

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

describe("lz77 hostile headers", function()
  it("max count header only", function()
    local ok, r = quick(function() return codec.decompress(bin(255, 255, 255, 255)) end)
    assert.is_true(ok)
    assert.are.equal("", r)
  end)

  it("count larger than remaining bytes yields the present tokens", function()
    local d = bin(255, 255, 255, 255, 0, 0, 0, 104, 0, 0, 0, 105)
    local ok, r = quick(function() return codec.decompress(d) end)
    assert.is_true(ok)
    assert.are.equal("hi", r)
    assert.are.equal("hi", codec.decompress(d .. bin(0, 0, 0)))
  end)

  it("count smaller than remaining bytes stops at the count", function()
    assert.are.equal("h", codec.decompress(bin(0, 0, 0, 1, 0, 0, 0, 104, 0, 0, 0, 105)))
  end)

  it("invalid offsets stop decoding", function()
    assert.are.equal("", codec.decompress(bin(0, 0, 0, 1, 0, 5, 3, 120)))
    assert.are.equal("", codec.decompress(bin(0, 0, 0, 1, 0, 0, 3, 120)))
    assert.are.equal("a", codec.decompress(bin(0, 0, 0, 2, 0, 0, 0, 97, 0, 9, 2, 122)))
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
