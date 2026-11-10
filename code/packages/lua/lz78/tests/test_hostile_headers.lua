-- test_hostile_headers.lua — security regression tests
--
-- Each case is a few bytes whose header claims an enormous count or length.  An
-- implementation that looped or allocated from the claim would run for minutes
-- or exhaust memory; these tests assert bounded time and sane results, plus
-- round trips over every input of length <= 2 and a seeded random sample.

package.path = "../src/?.lua;../src/?/init.lua;" .. package.path

local codec = require("coding_adventures.lz78")

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

describe("lz78 hostile headers", function()
  local function hdr(len, count) return u32(len) .. u32(count) end

  it("max header only", function()
    local ok, r = quick(function() return codec.decompress(string.rep("\255", 8)) end)
    assert.is_true(ok)
    assert.are.equal("", r)
  end)

  it("count larger than remaining bytes yields the present tokens", function()
    local d = hdr(2, 0xFFFFFFFF) .. bin(0, 0, 104, 0, 0, 0, 105, 0)
    local ok, r = quick(function() return codec.decompress(d) end)
    assert.is_true(ok)
    assert.are.equal("hi", r)
  end)

  it("length larger than the tokens supply", function()
    local d = hdr(0xFFFFFFFF, 1) .. bin(0, 0, 104, 0)
    local ok, r = quick(function() return codec.decompress(d) end)
    assert.is_true(ok)
    assert.are.equal("h", r)
  end)

  it("dangling dictionary index does not raise", function()
    local d = hdr(10, 2) .. bin(0, 0, 97, 0, 0x27, 0x0F, 98, 0)
    local ok, r = quick(function() return codec.decompress(d) end)
    assert.is_true(ok)
    assert.are.equal("a", r)
  end)

  it("chain tokens stop at the declared length", function()
    local n = 3000
    local parts = {hdr(1000, n)}
    for i = 0, n - 1 do parts[#parts + 1] = bin(i >> 8, i & 255, 97, 0) end
    local ok, r = quick(function() return codec.decompress(table.concat(parts)) end)
    assert.is_true(ok)
    assert.are.equal(1000, #r)
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
