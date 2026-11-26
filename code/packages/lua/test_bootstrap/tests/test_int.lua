-- Hazard 1: 64-bit arithmetic.  The Int module is checked against rows computed
-- by an independent Python reference (tests/int_vectors.lua).
package.path = "../src/?.lua;../src/?/init.lua;" .. package.path
local H = require("helper")
local Int = H.tb.Int
local vectors = require("int_vectors")
local udiv = require("udivmod_vectors")
local Prng = H.tb.Prng

local function I(t)
  return Int.parse(t)
end

describe("Int against the Python reference", function()
  it("agrees on every vector row", function()
    local checked = 0
    for _, row in ipairs(vectors) do
      local op, a, b = row[1], row[2], row[3]
      local x, y = I(a), I(b)
      local got
      if op == "add" then
        got = Int.tostring(Int.add(x, y))
      elseif op == "sub" then
        got = Int.tostring(Int.sub(x, y))
      elseif op == "mul" then
        got = Int.tostring(Int.mul(x, y))
      elseif op == "cmp" then
        got = tostring(Int.cmp(x, y))
      elseif op == "divmod" then
        local q, m = Int.divmod(x, y)
        assert.are.equal(row[5], Int.tostring(m), a .. " mod " .. b)
        got = Int.tostring(q)
      elseif op == "floor_half" then
        got = Int.tostring(Int.floor_half(x))
      elseif op == "bit_length" then
        got = tostring(Int.bit_length(x))
      elseif op == "neg" then
        got = Int.tostring(Int.neg(x))
      elseif op == "abs" then
        got = Int.tostring(Int.abs(x))
      elseif op == "low64" then
        got = Int.tostring(Int.from_u64(Int.to_u64(x)))
      elseif op == "shl" then
        got = Int.tostring(Int.shl(x, tonumber(b)))
      else
        error("unknown vector op " .. op)
      end
      local want = (op == "shl" or op == "divmod") and row[4] or row[4]
      assert.are.equal(want, got, op .. " " .. a .. " " .. b)
      checked = checked + 1
    end
    assert.is_true(checked > 2000)
  end)

  it("parse and tostring round-trip", function()
    for _, row in ipairs(vectors) do
      assert.are.equal(row[2], Int.tostring(I(row[2])))
    end
  end)
end)

describe("normalization", function()
  it("keeps every int64 value native and everything else Big", function()
    assert.are.equal("integer", math.type(I("9223372036854775807")))
    assert.are.equal("integer", math.type(I("-9223372036854775808")))
    assert.is_true(Int.is_big(I("9223372036854775808")))
    assert.is_true(Int.is_big(I("-9223372036854775809")))
    assert.are.equal(math.maxinteger, Int.sub(I("9223372036854775808"), 1))
    assert.are.equal(math.mininteger, Int.add(I("-9223372036854775809"), 1))
    assert.is_false(Int.is_big(Int.sub(Int.pow2(70), Int.pow2(70))))
  end)

  it("negating the minimum integer does not wrap", function()
    assert.are.equal("9223372036854775808", Int.tostring(Int.neg(math.mininteger)))
    assert.are.equal("9223372036854775808", Int.tostring(Int.abs(math.mininteger)))
    assert.are.equal(math.mininteger, Int.neg(I("9223372036854775808")))
  end)

  it("divides the minimum integer by -1 without wrapping", function()
    local q, r = Int.divmod(math.mininteger, -1)
    assert.are.equal("9223372036854775808", Int.tostring(q))
    assert.are.equal(0, r)
  end)

  it("floors a negative half (the bisection step)", function()
    assert.are.equal(-41, Int.floor_half(-81))
    assert.are.equal(-2, Int.floor_half(-3))
    assert.are.equal(-1, Int.floor_half(-1))
    assert.are.equal(0, Int.floor_half(1))
    assert.are.equal("-4611686018427387905", Int.tostring(Int.floor_half(I("-9223372036854775809"))))
  end)
end)

describe("floats are never Ints", function()
  it("refuses every float, integral or not", function()
    assert.is_false(Int.is(3.0))
    assert.is_false(Int.is(0.5))
    assert.is_false(Int.is(1 / 0))
    assert.is_false(Int.is("3"))
    assert.is_false(Int.is(nil))
    assert.is_false(Int.is({}))
    assert.has_error(function() Int.add(3.0, 1) end)
    assert.has_error(function() Int.cmp(1, 2.0) end)
    assert.has_error(function() Int.check(3.0, "seed") end)
    assert.has_error(function() Int.shl(2.0, 1) end)
  end)

  it("does not mistake a user table for a Big", function()
    local forged = { neg = false, mag = { 1 } }
    assert.is_false(Int.is(forged))
    assert.has_error(function() Int.add(forged, 1) end)
  end)

  it("keeps 3 and 3.0 apart in Lua itself (what the engine guards against)", function()
    local t = {}
    t[3.0] = "x"
    assert.are.equal("x", t[3]) -- Lua folds float keys: never use an Int as a key
    assert.is_true(Int.is(3))
    assert.is_false(Int.is(3.0))
  end)
end)

describe("decimal text", function()
  it("parses and prints 1,000-digit integers", function()
    local text = "1" .. string.rep("0", 999)
    assert.are.equal(text, Int.tostring(I(text)))
    assert.are.equal("-" .. text, Int.tostring(I("-" .. text)))
  end)

  it("refuses malformed text", function()
    assert.has_error(function() Int.parse("") end)
    assert.has_error(function() Int.parse("-") end)
    assert.has_error(function() Int.parse("1.5") end)
    assert.has_error(function() Int.parse("12a") end)
  end)
end)

describe("udivmod on unsigned 64-bit words", function()
  it("agrees with Python on every vector", function()
    for _, row in ipairs(udiv) do
      local a, b = Int.to_u64(I(row[1])), Int.to_u64(I(row[2]))
      local q, r = Prng.udivmod(a, b)
      assert.are.equal(row[3], Int.tostring(Int.from_u64(q)), row[1] .. " / " .. row[2])
      assert.are.equal(row[4], Int.tostring(Int.from_u64(r)), row[1] .. " % " .. row[2])
    end
  end)

  it("is exact at the boundaries n = 1, 2, 2^63, 2^63+1, 2^64-1", function()
    local function t(n)
      return Prng.threshold(Int.to_u64(n))
    end
    -- t = (2^64 - n) mod n
    assert.are.equal(0, t(1))
    assert.are.equal(0, t(2))
    assert.are.equal(1, t(3))
    assert.are.equal(0, t(Int.pow2(63))) -- 2^64 - 2^63 = 2^63, mod 2^63 = 0
    assert.are.equal(Int.to_u64(Int.sub(Int.pow2(63), 1)), t(Int.add(Int.pow2(63), 1))) -- 2^63 - 1
    assert.are.equal(1, t(Int.sub(Int.pow2(64), 1))) -- 2^64 - n = 1
    -- the same expression through SIGNED floored modulo is wrong:
    assert.are_not.equal(1, (0 - 3) % 3)
    assert.are.equal(0, (0 - 3) % 3)
  end)

  it("compares unsigned only through math.ult", function()
    local big = Int.to_u64(Int.pow2(63))
    assert.is_true(big < 0) -- a signed comparison would call it negative
    assert.is_true(math.ult(1, big))
  end)
end)
