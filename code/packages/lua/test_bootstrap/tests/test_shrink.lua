-- Integrated shrinking: exact step counts, floored negative bisection, both
-- bisection halves, 1-minimality (VER05 4, VER05o sections 1 and 10).
package.path = "../src/?.lua;../src/?/init.lua;" .. package.path
local H = require("helper")
local S, Int, Hooks, Gens = H.tb.Shrink, H.tb.Int, H.tb.Hooks, H.tb.Gens

local function str(list)
  local t = {}
  for i, v in ipairs(list) do
    t[i] = H.dec(v)
  end
  return "[" .. table.concat(t, ",") .. "]"
end

local function hex(s)
  return (s:gsub(".", function(c)
    return string.format("%02x", c:byte())
  end))
end

local function contains(list, x)
  for _, v in ipairs(list) do
    if v == x then
      return true
    end
  end
  return false
end

describe("shrink_int", function()
  it("pins the step counts", function()
    local function le100(x) return x <= 100 end
    local m, s = S.shrink_int(54321, 0, 0, 100000, le100)
    assert.are.same({ 101, 15 }, { m, s })
    m, s = S.shrink_int(77, 0, -1000, 1000, function() return false end)
    assert.are.same({ 0, 1 }, { m, s }) -- everything fails: the target probe, one step
    m, s = S.shrink_int(100, 0, -100, 500, function(x) return x >= -10 and x <= 50 end)
    assert.are.same({ 51, 49 }, { m, s })
    m, s = S.shrink_int(99, 10, 10, 500, function(x) return x < 5 end)
    assert.are.same({ 10, 1 }, { m, s })
  end)

  it("honours the step budget exactly", function()
    local function le100(x) return x <= 100 end
    local m, s = S.shrink_int(54321, 0, 0, 100000, le100, 3)
    assert.are.same({ 6791, 3 }, { m, s })
    m, s = S.shrink_int(54321, 0, 0, 100000, le100, 0)
    assert.are.same({ 54321, 0 }, { m, s })
  end)

  it("bisects negatives with a FLOORED half: -81 tries -40, not -41", function()
    local seen = {}
    S.shrink_int(-81, 0, -1000, 1000, function(x)
      seen[#seen + 1] = x
      return x > -50
    end)
    assert.are.equal(-81, seen[1]) -- the initial evaluation
    assert.are.equal(0, seen[2]) -- the target probe
    assert.are.equal(-40, seen[3]) -- floor: -81 - floor(-81/2) = -81 + 41
    assert.are.equal(-80, seen[4]) -- then the unit step
    assert.are.equal(-40, Hooks.default_bisect(-81, -81))
    assert.are.equal(-40, Hooks.default_bisect(-80, -80))
    assert.are.equal(-1, Hooks.default_bisect(-2, -2) + 0 - 0 == -1 and -1 or Hooks.default_bisect(-2, -2))
  end)

  it("is exactly the threshold for a monotone property from every start, both signs", function()
    for t = 1, 40 do
      for start = t, t + 60 do
        local m = S.shrink_int(start, 0, 0, 1000, function(x) return x < t end)
        assert.are.equal(t, m)
      end
      for start = -t - 60, -t do
        local m = S.shrink_int(start, 0, -1000, 1000, function(x) return x > -t end)
        assert.are.equal(-t, m)
      end
    end
  end)

  it("never leaves [min, max] and never moves farther from the target", function()
    for lo = -5, 5 do
      for hi = lo, lo + 8 do
        local target = Gens.clamp_zero(lo, hi)
        for start = lo, hi do
          if start % 2 == 0 then
            local ok, m = pcall(S.shrink_int, start, target, lo, hi, function(x) return x % 2 ~= 0 end)
            assert.is_true(ok)
            assert.is_true(m >= lo and m <= hi)
            assert.is_true(math.abs(m - target) <= math.abs(start - target))
            assert.are.equal(0, m % 2)
          end
        end
      end
    end
  end)

  it("works on 128-bit values and extreme targets", function()
    local lo, hi = Gens.MIN_I128, Gens.MAX_I128
    local m, s = S.shrink_int(hi, 0, lo, hi, function(x) return Int.le(x, 1000) end)
    assert.are.equal(1001, m)
    assert.is_true(s > 100 and s <= 1000)
    m = S.shrink_int(lo, hi, lo, hi, function(x) return Int.cmp(x, 0) >= 0 end) -- opposite-extreme target
    assert.is_true(Int.cmp(m, 0) < 0)
    local m2 = S.shrink_int(math.mininteger, 0, Gens.MIN_I128, Gens.MAX_I128, function(x) return Int.cmp(x, -5) >= 0 end)
    assert.are.equal(-6, m2)
  end)

  it("refuses bad arguments loudly", function()
    local always = function() return false end
    assert.has_error(function() S.shrink_int(5, 0, 0, 10, function() return true end) end, S.NOT_FALSIFYING)
    assert.has_error(function() S.shrink_int(5.0, 0, 0, 10, always) end)
    assert.has_error(function() S.shrink_int(5, 0, 10, 0, always) end)
    assert.has_error(function() S.shrink_int(50, 0, 0, 10, always) end)
    assert.has_error(function() S.shrink_int(5, 0, 0, 10, always, 10001) end)
    assert.has_error(function() S.shrink_int(5, 0, 0, 10, always, -1) end)
    assert.has_error(function() S.shrink_int(5, 0, 0, 10, nil) end)
  end)

  it("treats only exactly true as 'holds'", function()
    -- a property that forgets to return falsifies everything: the target wins
    local m = S.shrink_int(50, 0, 0, 100, function() end)
    assert.are.equal(0, m)
    m = S.shrink_int(50, 0, 0, 100, function() return 1 end)
    assert.are.equal(0, m)
  end)
end)

describe("shrink_list", function()
  local function no_five(l)
    return not contains(l, 5)
  end

  it("evaluates in exactly the documented order", function()
    local seen = {}
    local m, s = S.shrink_list({ 0, 0, 0, 0, 5, 0, 0, 0 }, 0, function(l)
      seen[#seen + 1] = str(l)
      return no_five(l)
    end)
    assert.are.same({ 5 }, m)
    assert.are.equal(3, s)
    assert.are.same({ "[0,0,0,0,5,0,0,0]", "[]", "[0,0,0,0]", "[5,0,0,0]", "[5,0]", "[5]", "[3]", "[4]" }, seen)
  end)

  it("takes both bisection halves: the left and the right", function()
    local m, s = S.shrink_list({ 5, 0, 0, 0, 0, 0, 0, 0 }, 0, no_five)
    assert.are.same({ 5 }, m)
    assert.are.equal(3, s)
    m, s = S.shrink_list({ 0, 0, 0, 0, 0, 0, 0, 5 }, 0, no_five)
    assert.are.same({ 5 }, m)
    assert.are.equal(3, s)
  end)

  it("deletes single elements right to left and shrinks elements left to right", function()
    local m, s = S.shrink_list({ 10, 5, 8, 4 }, 0, function(l)
      local sum = 0
      for _, v in ipairs(l) do sum = sum + v end
      return sum <= 20
    end)
    assert.are.same({ 8, 5, 8 }, m)
    assert.are.equal(3, s) -- as the corpus pins
  end)

  it("is 1-minimal by deletion over every list of length 0..4 over {0,1,2,3}", function()
    local props = {
      function(l) return #l < 2 end,
      function(l) return not contains(l, 3) end,
      function(l)
        local sum = 0
        for _, v in ipairs(l) do sum = sum + v end
        return sum < 4
      end,
      function(l)
        for i = 1, #l - 1 do
          if l[i] == l[i + 1] then return false end
        end
        return true
      end,
      function(l) return not (#l >= 2 and l[#l] > l[1]) end,
      function(l) return #l == 0 or l[1] ~= 2 end,
    }
    local count = 0
    local function each(prefix, n)
      if n == 0 then
        for _, prop in ipairs(props) do
          if not prop(prefix) then
            local m = S.shrink_list(prefix, 0, prop)
            assert.is_false(prop(m)) -- still falsifies
            assert.is_true(#m <= #prefix)
            for i = 1, #m do -- removing any single element makes the property hold
              local cand = {}
              for j = 1, #m do
                if j ~= i then cand[#cand + 1] = m[j] end
              end
              assert.is_true(prop(cand), str(prefix) .. " shrank to " .. str(m))
            end
            for _, v in ipairs(m) do
              assert.is_true(v >= 0 and v <= 3)
            end
          end
        end
        count = count + 1
        return
      end
      for v = 0, 3 do
        local nxt = { table.unpack(prefix) }
        nxt[#nxt + 1] = v
        each(nxt, n - 1)
      end
    end
    for len = 0, 4 do
      each({}, len)
    end
    assert.are.equal(341, count)
  end)

  it("does not reach the global minimum for 'no adjacent equal' (documented local minimum)", function()
    local m = S.shrink_list({ 1, 2, 3, 3, 4, 5 }, 0, function(l)
      for i = 1, #l - 1 do
        if l[i] == l[i + 1] then return false end
      end
      return true
    end)
    assert.are.same({ 3, 3 }, m)
  end)

  it("refuses a start that does not falsify and non-integer elements", function()
    assert.has_error(function() S.shrink_list({ 1 }, 0, function() return true end) end, S.NOT_FALSIFYING)
    assert.has_error(function() S.shrink_list({ 1.5 }, 0, function() return false end) end)
    assert.has_error(function() S.shrink_list({ 1 }, 0.0, function() return false end) end)
    assert.has_error(function() S.shrink_list("x", 0, function() return false end) end)
  end)

  it("with a zero budget returns the start unchanged", function()
    local start = { 1, 2, 3 }
    local m, s = S.shrink_list(start, 0, function() return false end, 0)
    assert.are.equal(0, s)
    assert.are.same(start, m)
  end)
end)

describe("shrink_bytes and replay", function()
  it("pins the byte shrink: 0102030405060708 (len < 5) is five zero bytes in 16 steps", function()
    local m, s = S.shrink_bytes("\1\2\3\4\5\6\7\8", function(b) return #b < 5 end)
    assert.are.equal("0000000000", hex(m))
    assert.are.equal(16, s)
  end)

  it("never returns a value that holds", function()
    local prop = function(b) return not b:find("\0", 1, true) == false and false or not b:find("\0", 1, true) end
    local m = S.shrink_bytes("ab\0cd", prop)
    assert.is_truthy(m:find("\0", 1, true))
    assert.are.equal("\0", m)
    assert.has_error(function() S.shrink_bytes("abc", function() return true end) end, S.NOT_FALSIFYING)
    assert.has_error(function() S.shrink_bytes(5, function() return false end) end)
  end)

  it("replays from nothing but the seed: seed 42, x <= 100 is (413, 101) forever", function()
    local a, b = S.simulate_replay(42, function(x) return x <= 100 end)
    assert.are.same({ 413, 101 }, { a, b })
    local c, d = S.simulate_replay(42, function(x) return x <= 100 end)
    assert.are.same({ a, b }, { c, d })
    assert.has_error(function() S.simulate_replay(1, function() return true end) end)
  end)
end)

describe("sabotage hooks make the pins fail (the pins are not vacuous)", function()
  it("a changed bisection, unit step or skipped deletion changes the answers", function()
    local function le100(x) return x <= 100 end
    local base = { S.shrink_int(54321, 0, 0, 100000, le100) }
    local function run(over)
      return { Hooks.with(over, S.shrink_int, 54321, 0, 0, 100000, le100) }
    end
    assert.are_not.same(base, run({ bisect = function(c, d) return Int.sub(c, 1) end }))
    assert.are_not.same(base, run({ unit = function(c, d) return Int.sub(c, 2) end }))
    local function sum20(l)
      local sum = 0
      for _, v in ipairs(l) do sum = sum + v end
      return sum <= 20
    end
    local seq = { S.shrink_list({ 10, 5, 8, 4 }, 0, sum20) }
    local sab = { Hooks.with({ skip_deletion = true }, S.shrink_list, { 10, 5, 8, 4 }, 0, sum20) }
    assert.are_not.same(seq, sab)
  end)
end)
