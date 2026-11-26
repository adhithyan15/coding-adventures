-- The generator algebra: draw order, ranges, shrinking inside refinements.
package.path = "../src/?.lua;../src/?/init.lua;" .. package.path
local H = require("helper")
local tb = H.tb
local Gens, Gen, Prng, Int, Limits = tb.Gens, tb.Gen, tb.Prng, tb.Int, tb.Limits

local function sample_n(gen, seed, n)
  local rng = Prng.new(seed)
  local out = {}
  for i = 1, n do
    out[i] = gen:sample(rng)
  end
  return out
end

local function shrink(gen, v, falsifies, steps)
  return gen:shrink(v, steps or 1000, falsifies)
end

local function yes()
  return true
end

describe("primitives", function()
  it("draw exactly as the corpus pins (bool, u8, u64, i128)", function()
    assert.are.same({ true, false, true, false }, Gens.gen_primitive(0, "bool", 4))
    assert.are.same({ 175, 244, 79 }, Gens.gen_primitive(0, "u8", 3))
    assert.are.equal("16294208416658607535", H.dec(Gens.gen_primitive(0, "u64", 1)[1]))
    assert.are.equal("-87940914747023602177814525588822101757", H.dec(Gens.gen_primitive(42, "i128", 1)[1]))
    assert.has_error(function() Gens.gen_primitive(0, "u16", 1) end)
    assert.has_error(function() Gens.gen_primitive(0, "bool", -1) end)
    assert.has_error(function() Gens.gen_primitive(0, "bool", Limits.MAX_GEN_COUNT + 1) end)
    assert.has_error(function() Gens.gen_primitive(0, "bool", 1.0) end)
    assert.has_error(function() Gens.gen_primitive(0.0, "bool", 1) end)
  end)

  it("i128 reads two words as two's complement", function()
    local function forced(list)
      local rng = Prng.new(0)
      local i = 0
      function rng:next_word()
        i = i + 1
        return list[i]
      end
      return rng
    end
    assert.are.equal("-170141183460469231731687303715884105728", H.dec(Gens.sample_i128(forced({ math.mininteger, 0 }))))
    assert.are.equal("170141183460469231731687303715884105727", H.dec(Gens.sample_i128(forced({ math.maxinteger, -1 }))))
    assert.are.equal("0", H.dec(Gens.sample_i128(forced({ 0, 0 }))))
    assert.are.equal("-1", H.dec(Gens.sample_i128(forced({ -1, -1 }))))
  end)

  it("bool shrinks true to false", function()
    local g = Gens.bool()
    assert.are.same({ false, 1 }, { shrink(g, true, yes) })
    assert.are.same({ true, 0 }, { shrink(g, true, function() return false end) })
    assert.are.same({ false, 0 }, { shrink(g, false, yes) })
    assert.are.same({ true, 0 }, { shrink(g, true, yes, 0) })
  end)

  it("u8 and u64 stay in range and shrink toward 0", function()
    for _, v in ipairs(sample_n(Gens.u8(), 4, 200)) do
      assert.is_true(v >= 0 and v <= 255)
    end
    assert.are.equal(7, (shrink(Gens.u8(), 200, function(x) return x >= 7 end)))
    local big = H.int("18446744073709551615")
    -- a local minimum within the 1000-step budget, never below the threshold
    local m = shrink(Gens.u64(), big, function(x) return Int.cmp(x, 5000) >= 0 end)
    assert.is_true(Int.cmp(m, 5000) >= 0 and Int.cmp(m, 10000) < 0)
  end)

  it("constant draws nothing", function()
    local rng = Prng.new(1)
    local before = rng.state
    assert.are.equal(9, Gens.constant(9):sample(rng))
    assert.are.equal(before, rng.state)
  end)
end)

describe("integer(min, max)", function()
  it("is uniform over the inclusive range, extremes included", function()
    local seen = {}
    for _, v in ipairs(sample_n(Gens.integer(-2, 2), 1, 400)) do
      seen[v] = (seen[v] or 0) + 1
    end
    for v = -2, 2 do
      assert.is_truthy(seen[v], "missing " .. v)
    end
    assert.is_nil(seen[3])
    assert.are.equal(7, sample_n(Gens.integer(7, 7), 1, 3)[2])
  end)

  it("covers the full i128 range and spans beyond 2^64", function()
    local g = Gens.integer(Gens.MIN_I128, Gens.MAX_I128)
    for _, v in ipairs(sample_n(g, 2, 30)) do
      assert.is_true(Int.cmp(v, Gens.MIN_I128) >= 0 and Int.cmp(v, Gens.MAX_I128) <= 0)
    end
    local wide = Gens.integer(0, Int.pow2(200))
    local sawbig = false
    for _, v in ipairs(sample_n(wide, 3, 20)) do
      assert.is_true(Int.cmp(v, Int.pow2(200)) <= 0)
      sawbig = sawbig or Int.bit_length(v) > 64
    end
    assert.is_true(sawbig)
  end)

  it("never shrinks outside [min, max] (an integrated refinement)", function()
    assert.are.equal(5, (shrink(Gens.integer(5, 10), 9, yes)))
    assert.are.equal(-3, (shrink(Gens.integer(-10, -3), -9, yes)))
    assert.are.equal(0, (shrink(Gens.integer(-10, 10), 7, yes)))
  end)

  it("refuses bad bounds, floats and spans over 16,384 bits", function()
    assert.has_error(function() Gens.integer(5, 4) end)
    assert.has_error(function() Gens.integer(1.0, 4) end)
    assert.has_error(function() Gens.integer(1, "4") end)
    assert.has_error(function() Gens.integer(0, Int.pow2(16400)) end)
  end)
end)

describe("bytes and list", function()
  it("bytes lengths are inclusive and values are bytes", function()
    local lens = {}
    for _, v in ipairs(sample_n(Gens.bytes(2, 4), 6, 200)) do
      lens[#v] = true
    end
    assert.is_true(lens[2] and lens[3] and lens[4])
    assert.is_nil(lens[1] or lens[5])
    assert.are.equal(0, #sample_n(Gens.bytes(0, 0), 1, 1)[1])
  end)

  it("bytes shrinking respects min_len", function()
    local m = shrink(Gens.bytes(3, 8), "abcdefgh", yes)
    assert.are.equal("\0\0\0", m)
  end)

  it("list lengths are inclusive and shrinking respects min_len", function()
    local lens = {}
    for _, v in ipairs(sample_n(Gens.list(Gens.u8(), 1, 3), 7, 200)) do
      lens[#v] = true
    end
    assert.is_true(lens[1] and lens[2] and lens[3])
    assert.are.same({ 0, 0 }, (shrink(Gens.list(Gens.integer(0, 9), 2, 5), { 9, 8, 7, 6, 5 }, yes)))
  end)

  it("refuses bad lengths, floats and oversized maxima", function()
    assert.has_error(function() Gens.bytes(-1, 3) end)
    assert.has_error(function() Gens.bytes(4, 3) end)
    assert.has_error(function() Gens.bytes(0, 1.0) end)
    assert.has_error(function() Gens.bytes(0, Limits.MAX_INPUT_BYTES + 1) end)
    assert.has_error(function() Gens.list(Gens.bool(), 0, Limits.MAX_LIST_LEN + 1) end)
  end)

  it("gen_bytes and gen_bounded enforce their generation caps", function()
    assert.has_error(function() Gens.gen_bytes(0, 0, 1 << 20, 2) end)
    assert.has_error(function() Gens.gen_bytes(0, 0, 4, Limits.MAX_GEN_COUNT + 1) end)
    assert.are.equal(0, #Gens.gen_bytes(0, 0, 0, 0))
    assert.has_error(function() Gens.gen_bounded(0, 0, Int.pow2(2000), 1000000) end)
    assert.has_error(function() Gens.gen_bounded(0, 5, 4, 1) end)
    assert.are.equal(3, #Gens.gen_bounded(0, 0, 9, 3))
  end)
end)

describe("combinators", function()
  it("map transforms and drops shrinking", function()
    local g = Gens.map(Gens.u8(), function(x) return x * 2 end)
    assert.are.equal(0, g:sample(Prng.new(1)) % 2)
    assert.are.same({ 200, 0 }, { g:shrink(200, 100, yes) })
  end)

  it("filter retries exactly max_retries times, then the sample fails as exhausted", function()
    local calls = 0
    local g = Gens.filter(Gens.u8(), function()
      calls = calls + 1
      return false
    end, 7)
    local ok, err = pcall(g.sample, g, Prng.new(1))
    assert.is_false(ok)
    assert.is_true(Gen.is_exhausted_error(err))
    assert.are.equal(7, calls)
    assert.has_error(function() Gens.filter(Gens.u8(), yes, 0) end)
    assert.has_error(function() Gens.filter(Gens.u8(), yes, Limits.MAX_RETRIES + 1) end)
    assert.has_error(function() Gens.filter(Gens.u8(), yes, 2.0) end)
  end)

  it("filter accepts only exactly true and keeps shrinking inside the predicate", function()
    local g = Gens.filter(Gens.integer(0, 100), function(x) return x % 2 == 0 end)
    for _, v in ipairs(sample_n(g, 5, 50)) do
      assert.are.equal(0, v % 2)
    end
    -- the bisection halves 80 -> 40 -> 20; 10 holds, and the odd unit step 19 fails the
    -- predicate, so it is never accepted: 20 is the (even) local minimum
    assert.are.equal(20, (shrink(g, 80, function(x) return x >= 11 end)))
    local sloppy = Gens.filter(Gens.u8(), function() return 1 end, 3)
    assert.is_false((pcall(sloppy.sample, sloppy, Prng.new(1))))
  end)

  it("one_of is uniform and flat_map is dependent", function()
    local g = Gens.one_of({ Gens.constant("a"), Gens.constant("b") })
    local n = { a = 0, b = 0 }
    for _, v in ipairs(sample_n(g, 8, 400)) do
      n[v] = n[v] + 1
    end
    assert.is_true(n.a > 120 and n.b > 120)
    local dep = Gens.flat_map(Gens.integer(1, 3), function(k) return Gens.bytes(k, k) end)
    for _, v in ipairs(sample_n(dep, 9, 30)) do
      assert.is_true(#v >= 1 and #v <= 3)
    end
    assert.has_error(function() Gens.one_of({}) end)
    assert.has_error(function() Gens.one_of(nil) end)
  end)

  it("frequency never selects a zero-weight arm and follows its weights", function()
    local g = Gens.frequency({ { 0, Gens.constant("never") }, { 3, Gens.constant("x") }, { 1, Gens.constant("y") } })
    local n = { x = 0, y = 0 }
    for _, v in ipairs(sample_n(g, 3, 800)) do
      assert.are_not.equal("never", v)
      n[v] = n[v] + 1
    end
    assert.is_true(n.x > 3 * n.y * 0.6 and n.x < 3 * n.y * 1.6)
    assert.has_error(function() Gens.frequency({}) end)
    assert.has_error(function() Gens.frequency({ { 0, Gens.bool() } }) end)
    assert.has_error(function() Gens.frequency({ { -1, Gens.bool() } }) end)
    assert.has_error(function() Gens.frequency({ { 1 << 32, Gens.bool() }, { 1, Gens.bool() } }) end)
    assert.has_error(function() Gens.frequency({ { 1.5, Gens.bool() } }) end)
  end)

  it("tuple samples left to right and shrinks one coordinate at a time", function()
    local g = Gens.tuple({ Gens.integer(0, 100), Gens.integer(0, 100) })
    local m, s = g:shrink({ 90, 80 }, 1000, function(t) return t[1] + t[2] >= 20 end)
    assert.are.equal(20, m[1] + m[2])
    assert.is_true(s > 0)
    assert.has_error(function() Gens.tuple({}) end)
    local many = {}
    for i = 1, Limits.MAX_TUPLE_ARITY + 1 do
      many[i] = Gens.bool()
    end
    assert.has_error(function() Gens.tuple(many) end)
  end)

  it("refuses non-functions", function()
    assert.has_error(function() Gen.new(5) end)
    assert.has_error(function() Gen.new(function() end, 5) end)
    assert.has_error(function() Gens.map(Gens.u8(), nil) end)
    assert.has_error(function() Gens.filter(Gens.u8(), nil) end)
    assert.has_error(function() Gens.flat_map(Gens.u8(), nil) end)
    assert.has_error(function() Gens.u8():shrink(1, 10, nil) end)
    assert.has_error(function() Gens.u8():shrink(1, 10001, yes) end)
  end)
end)
