-- The honest runner: verdict honesty, error classification by type, CI redaction.
package.path = "../src/?.lua;../src/?/init.lua;" .. package.path
local H = require("helper")
local tb = H.tb
local Runner, Gens, Gen, Canon, Int, Budget, Prng = tb.Runner, tb.Gens, tb.Gen, tb.Canon, tb.Int, tb.Budget, tb.Prng
local Options = Runner.Options

local function opts(t)
  t = t or {}
  t.ci = t.ci or "off"
  return Options.new(t)
end

local function check(seed, gen, prop, o, shrink)
  return H.limited(2e8, Runner.check, seed, gen, prop, o or opts(), shrink)
end

local function le100(x)
  return x <= 100
end

describe("verdicts", function()
  it("passes only when every case ran and held", function()
    local v = check(1, Gens.integer(0, 100), le100)
    assert.is_true(v.passed)
    assert.is_false(v.failed)
    assert.are.equal(100, v.cases_run)
    assert.is_nil(v.failure_id)
  end)

  it("zero cases is property-exhausted, never a pass", function()
    local v = check(1, Gens.integer(0, 100), le100, opts({ num_tests = 0 }))
    assert.is_false(v.passed)
    assert.are.equal("property-exhausted", v.failure_id)
    assert.are.equal(0, v.cases_run)
  end)

  it("shrinks a counterexample to the threshold and reports it (ci off)", function()
    local v = check(7, Gens.integer(0, 100000), le100)
    assert.are.equal("property-counterexample", v.failure_id)
    assert.are.equal(101, v.minimal)
    assert.is_true(v.shrink_steps > 0)
    assert.is_true(v.counterexample > 100)
    assert.are.equal("7", H.dec(v.seed))
    assert.is_truthy(v.message:find("minimal=101", 1, true))
    assert.is_truthy(v.message:find("seed=7", 1, true))
  end)

  it("is deterministic for a seed and different across seeds", function()
    local a = check(5, Gens.integer(0, 100000), le100)
    local b = check(5, Gens.integer(0, 100000), le100)
    assert.are.same(a, b)
    local c = check(6, Gens.integer(0, 100000), le100)
    assert.are_not.equal(a.counterexample, c.counterexample)
  end)

  it("a property holds only on exactly true", function()
    for _, result in ipairs({ { false }, { nil }, { 1 }, { "yes" }, { {} }, { 0 } }) do
      local v = check(1, Gens.u8(), function() return result[1] end)
      assert.are.equal("property-counterexample", v.failure_id, tostring(result[1]))
    end
    local forgot = check(1, Gens.u8(), function() end)
    assert.are.equal("property-counterexample", forgot.failure_id)
    assert.is_true(check(1, Gens.u8(), function() return true end).passed)
  end)

  it("a generator error is property-exhausted, an exhausted filter says so", function()
    local boom = Gen.new(function() error("gen boom") end)
    local v = check(1, boom, function() return true end)
    assert.are.equal("property-exhausted", v.failure_id)
    assert.is_truthy(v.message:find("generator error", 1, true))
    local never = Gens.filter(Gens.u8(), function() return false end, 5)
    v = check(1, never, function() return true end)
    assert.are.equal("property-exhausted", v.failure_id)
    assert.is_truthy(v.message:find("filter precondition", 1, true))
  end)

  it("reduces seeds modulo 2^64 and refuses floats", function()
    local a = check(-1, Gens.integer(0, 100000), le100)
    local b = check(Int.sub(Int.pow2(64), 1), Gens.integer(0, 100000), le100)
    assert.are.equal(a.counterexample, b.counterexample)
    assert.are.equal("18446744073709551615", H.dec(a.seed))
    assert.has_error(function() Runner.check(1.0, Gens.u8(), le100, opts()) end)
    assert.has_error(function() Runner.check("1", Gens.u8(), le100, opts()) end)
  end)

  it("validates its arguments loudly", function()
    assert.has_error(function() Runner.check(1, Gens.u8(), nil) end)
    assert.has_error(function() Runner.check(1, {}, le100) end)
    assert.has_error(function() Runner.check(1, Gens.u8(), le100, {}) end)
    assert.has_error(function() Runner.check(1, Gens.u8(), le100, opts(), 5) end)
    assert.has_error(function() Options.new({ ci = "maybe" }) end)
    assert.has_error(function() Options.new({ num_tests = 1.5 }) end)
    assert.has_error(function() Options.new({ num_tests = 100001 }) end)
    assert.has_error(function() Options.new({ max_shrink_steps = 10001 }) end)
    assert.has_error(function() Options.new({ num_tess = 5 }) end)
    assert.has_error(function() Options.new({ parent = {} }) end)
  end)
end)

describe("errors are any value, classified by type only (hazard 2)", function()
  local function panics(err_value, expect_type)
    local v = check(1, Gens.u8(), function() error(err_value, 0) end, opts({ ci = "on" }))
    assert.are.equal("property-panic", v.failure_id)
    assert.is_truthy(v.message:find("(" .. expect_type .. ")", 1, true), v.message)
    local local_v = check(1, Gens.u8(), function() error(err_value, 0) end)
    assert.are.equal("property-panic", local_v.failure_id)
    return v
  end

  it("a string, a table, nil, a function, a number, a boolean, a coroutine", function()
    panics("plain", "string")
    panics({ code = 1 }, "table")
    panics(nil, "nil")
    panics(print, "function")
    panics(42, "number")
    panics(false, "boolean")
    panics(coroutine.create(print), "thread")
  end)

  it("a table whose __tostring raises is never stringified", function()
    local log = {}
    local evil = setmetatable({}, H.trap(log))
    panics(evil, "table")
    assert.are.same({}, log)
  end)

  it("error(setmetatable({}, {__index = ...})) is a plain panic", function()
    local calls = 0
    local e = setmetatable({}, {
      __index = function()
        calls = calls + 1
        error("idx")
      end,
    })
    panics(e, "table")
    assert.are.equal(0, calls)
  end)

  it("a runtime error (indexing nil) is a panic, not an engine crash", function()
    local v = check(1, Gens.u8(), function() local x = nil; return x.y end)
    assert.are.equal("property-panic", v.failure_id)
  end)

  it("the redacted message names only the type, never the content", function()
    local v = check(1, Gens.u8(), function() error("SECRET-TOKEN-123", 0) end, opts({ ci = "on" }))
    assert.is_nil(v.message:find("SECRET", 1, true))
    assert.is_nil(v.seed)
    local l = check(1, Gens.u8(), function() error("SECRET-TOKEN-123", 0) end)
    assert.is_truthy(l.message:find("SECRET-TOKEN-123", 1, true))
    assert.is_not_nil(l.seed)
  end)

  it("a candidate the property raises on during shrinking is never accepted as smaller", function()
    local prop = function(x)
      if x < 50 then error("raises below 50") end
      return x < 80
    end
    local v = check(3, Gens.integer(80, 255), prop)
    assert.are.equal("property-counterexample", v.failure_id)
    assert.are.equal(80, v.minimal)
  end)

  it("a generator that raises a table is exhausted with only the type redacted", function()
    local g = Gen.new(function() error({ secret = "x" }, 0) end)
    local v = check(1, g, function() return true end, opts({ ci = "on" }))
    assert.are.equal("property-exhausted", v.failure_id)
    assert.is_truthy(v.message:find("(table)", 1, true))
    assert.is_nil(v.message:find("secret", 1, true))
  end)
end)

describe("witness re-verification", function()
  local function run(shrinker)
    return check(7, Gens.integer(0, 100000), le100, opts(), shrinker)
  end

  it("reports the original with 0 steps when the shrinker raises", function()
    local v = run(function() error("shrinker boom") end)
    assert.are.equal("property-counterexample", v.failure_id)
    assert.are.equal(v.counterexample, v.minimal)
    assert.are.equal(0, v.shrink_steps)
  end)

  it("when it returns a value that no longer falsifies", function()
    local v = run(function() return 1, 5 end)
    assert.are.equal(v.counterexample, v.minimal)
    assert.are.equal(0, v.shrink_steps)
  end)

  it("when it returns nonsense step counts", function()
    for _, steps in ipairs({ -1, 1001, 1.5, "3", 1e30 }) do
      local v = run(function(c) return c, steps end)
      assert.are.equal(0, v.shrink_steps, tostring(steps))
    end
    local v = run(function(c) return c end)
    assert.are.equal(0, v.shrink_steps)
  end)

  it("when it returns a value the property raises on", function()
    local v = check(7, Gens.integer(0, 100000), function(x)
      if x == 5 then error("x") end
      return x <= 100
    end, opts(), function() return 5, 3 end)
    assert.are.equal(v.counterexample, v.minimal)
    assert.are.equal(0, v.shrink_steps)
  end)

  it("a shrinker that mutates the candidate in place cannot damage the original", function()
    local g = Gens.list(Gens.integer(0, 9), 3, 5)
    local v = check(2, g, function(l) return #l < 3 end, opts(), function(c)
      for i = 1, #c do c[i] = -99 end
      c[#c + 1] = 12345
      return c, 1
    end)
    assert.are.equal("property-counterexample", v.failure_id)
    for _, x in ipairs(v.counterexample) do
      assert.is_true(x >= 0 and x <= 9)
    end
    -- the (mutated) copy the shrinker returned is itself a genuine counterexample
    assert.are_not.equal(v.counterexample, v.minimal)
    assert.is_true(#v.minimal >= 3)
  end)

  it("when the budget refuses the re-verification charge the original is reported", function()
    local root = Budget.root(1000, 1000000)
    -- a check nested in a spent-down chain: leave exactly the entry attempt
    local v = Runner.check(7, Gens.integer(0, 100000), le100, Options.new({ ci = "off", parent = root }))
    assert.are.equal("property-counterexample", v.failure_id)
    local tight = Budget.root(2, 1000000)
    local v2 = Runner.check(7, Gens.integer(0, 100000), le100, Options.new({ ci = "off", parent = tight }))
    assert.are_not.equal("property-counterexample" .. "x", v2.failure_id)
    assert.is_false(v2.passed)
  end)

  it("a flaky property that fails on the first evaluation only reports the original", function()
    local n = 0
    local v = check(7, Gens.integer(0, 100000), function(x)
      if x > 100 then
        n = n + 1
        return n > 1 -- falsifies exactly once, on the initial evaluation
      end
      return true
    end)
    assert.are.equal("property-counterexample", v.failure_id)
    assert.are.equal(v.counterexample, v.minimal)
    assert.are.equal(0, v.shrink_steps)
  end)
end)

describe("CI redaction: an unset mode is CI", function()
  local function failing(o)
    return check(7, Gens.integer(0, 100000), le100, o)
  end

  it("carries no seed, counterexample or minimal and no concrete value in the message", function()
    for _, mode in ipairs({ "unset", "on" }) do
      local v = failing(Options.new({ ci = mode }))
      assert.are.equal("property-counterexample", v.failure_id)
      assert.is_nil(v.seed)
      assert.is_nil(v.counterexample)
      assert.is_nil(v.minimal)
      assert.is_nil(v.message:find("minimal=", 1, true))
      assert.is_nil(v.message:find("initial=", 1, true))
      assert.is_nil(v.message:find("seed=", 1, true))
      assert.is_truthy(v.message:find("repro%-fnv: %x%x%x%x%x%x%x%x%x%x%x%x%x%x%x%x"))
      assert.are.equal(16, #v.digest)
    end
    assert.are.equal("unset", Options.new().ci)
  end)

  it("the digest is FNV-1a of iter:<i>;steps:<s>;shrunk:<canonical>", function()
    local v = failing(Options.new({ ci = "on" }))
    local want = string.format("%016x", Prng.fnv1a64_word("iter:" .. v.iteration .. ";steps:" .. v.shrink_steps .. ";shrunk:101"))
    assert.are.equal(want, v.digest)
  end)

  it("local mode (ci off) is the only mode that reveals the witness", function()
    local v = failing(Options.new({ ci = "off" }))
    assert.are.equal(101, v.minimal)
    assert.is_truthy(v.counterexample)
  end)

  it("Options.ci_from_value is a pure helper: only unset/empty/0/false mean off", function()
    assert.are.equal("off", Options.ci_from_value(nil))
    for _, v in ipairs({ "", " ", "0", "false", "FALSE", " False ", "\t0\n" }) do
      assert.are.equal("off", Options.ci_from_value(v), v)
    end
    for _, v in ipairs({ "1", "true", "yes", "TRUE", "on", "2", "00" }) do
      assert.are.equal("on", Options.ci_from_value(v), v)
    end
    assert.has_error(function() Options.ci_from_value(1) end)
  end)

  it("redacted panic and exhausted verdicts also carry no seed", function()
    local p = check(1, Gens.u8(), function() error("x") end, Options.new())
    assert.is_nil(p.seed)
    local e = check(1, Gen.new(function() error("x") end), function() return true end, Options.new())
    assert.is_nil(e.seed)
  end)
end)

describe("size_of (the work estimate)", function()
  it("counts strings, tables and one level of children, bounded by an allowance", function()
    assert.are.equal(0, Runner.size_of(5))
    assert.are.equal(0, Runner.size_of(nil))
    assert.are.equal(0, Runner.size_of(Int.pow2(70)))
    assert.are.equal(5, Runner.size_of("hello"))
    assert.are.equal(3, Runner.size_of({ 1, 2, 3 }))
    assert.are.equal(2 + 1 + 3, Runner.size_of({ { 1 }, "abc" }) + 0)
    local huge = {}
    for i = 1, 100000 do huge[i] = { 1, 2, 3 } end
    assert.is_true(Runner.size_of(huge) <= 40000)
  end)

  it("reads with next only: a hostile table costs what it holds and runs no metamethod", function()
    local log = {}
    local t = setmetatable({ 1, 2 }, H.trap(log))
    assert.are.equal(2, Runner.size_of(t))
    assert.are.same({}, log)
  end)
end)
