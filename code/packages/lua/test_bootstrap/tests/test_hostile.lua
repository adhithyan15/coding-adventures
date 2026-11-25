-- Hazards 3, 4, 5 and 8 end to end: hostile values and properties flowing through
-- the runner.  Every test is bounded: a debug hook raises instead of hanging.
package.path = "../src/?.lua;../src/?/init.lua;" .. package.path
local H = require("helper")
local tb = H.tb
local Runner, Gens, Gen, Canon = tb.Runner, tb.Gens, tb.Gen, tb.Canon
local Options = Runner.Options

-- Under luacov every executed line costs a hook call, so the very large values are scaled
-- down (still over every cap: 10,000 nodes, 1 MiB) when LUA_COVERAGE is set.
local COVERAGE = os.getenv("LUA_COVERAGE") ~= nil
local WIDE = COVERAGE and 30000 or 1000000
local BIG_STRING = (COVERAGE and 2 or 10) * 1024 * 1024

local function check(gen, prop, o, shrink)
  return H.limited(3e8, Runner.check, 1, gen, prop, o or Options.new({ ci = "off", num_tests = 3 }), shrink)
end

local function constant_gen(make)
  return Gen.new(function() return make() end)
end

describe("a runaway property is stopped by an instruction-count hook (hazard 8)", function()
  it("is reported as property-panic, never a hang", function()
    local v = H.hard_limited(2e6, function()
      return Runner.check(1, Gens.u8(), function()
        while true do end
      end, Options.new({ ci = "off", num_tests = 1 }))
    end)
    assert.are.equal("property-panic", v.failure_id)
    assert.is_truthy(v.message:find("instruction limit", 1, true))
  end)

  it("H.limited itself raises when the code does not terminate", function()
    local before = debug.gethook()
    local ok, err = pcall(H.hard_limited, 1e5, function()
      while true do end
    end)
    assert.is_false(ok)
    assert.is_truthy(tostring(err):find("instruction limit", 1, true))
    assert.are.equal(before, (debug.gethook())) -- restored on every exit
  end)
end)

describe("hostile witnesses never run user code or hang", function()
  it("a witness with a metatable of traps: sampling, sizing, shrinking, rendering, digest", function()
    local log = {}
    local g = constant_gen(function()
      return setmetatable({ 1, 2, 3, name = "x" }, H.trap(log))
    end)
    local prop = function() return false end
    for _, mode in ipairs({ "off", "on", "unset" }) do
      local v = check(g, prop, Options.new({ ci = mode, num_tests = 2 }))
      assert.are.equal("property-counterexample", v.failure_id)
      assert.is_string(v.message)
    end
    assert.are.same({}, log)
  end)

  it("a witness that contains itself, at every level", function()
    local g = constant_gen(function()
      local t = { "a" }
      t[2] = t
      t.me = t
      return t
    end)
    for _, mode in ipairs({ "off", "on" }) do
      local v = check(g, function() return false end, Options.new({ ci = mode, num_tests = 1 }))
      assert.are.equal("property-counterexample", v.failure_id)
      assert.is_true(#v.message < 200000)
    end
  end)

  it("a witness nested 1,000 deep and one with a million entries", function()
    local deep = constant_gen(function()
      local t, cur = {}, nil
      cur = t
      for _ = 1, 1000 do
        local n = {}
        cur[1] = n
        cur = n
      end
      return t
    end)
    local v = check(deep, function() return false end, Options.new({ ci = "off", num_tests = 1 }))
    assert.are.equal("property-counterexample", v.failure_id)
    assert.is_true(#v.message < 4000)
    local wide = constant_gen(function()
      local t = {}
      for i = 1, WIDE do t[i] = i end
      return t
    end)
    v = check(wide, function() return false end, Options.new({ ci = "off", num_tests = 1 }))
    assert.is_true(#v.message < Canon.MAX_OUTPUT * 3)
    v = check(wide, function() return false end, Options.new({ ci = "on", num_tests = 1 }))
    assert.are.equal(16, #v.digest)
  end)

  it("a 10 MiB string witness is clipped and the verdict stays small", function()
    local g = constant_gen(function() return string.rep("\0", BIG_STRING) end)
    local v = check(g, function() return false end, Options.new({ ci = "off", num_tests = 1 }))
    -- two renderings (initial and minimal), each capped at 1 MiB
    assert.is_true(#v.message < 2 * Canon.MAX_OUTPUT + 1000)
  end)

  it("an error value that is a 10 MiB string is clipped in the message", function()
    local v = check(Gens.u8(), function() error(string.rep("e", BIG_STRING), 0) end)
    assert.are.equal("property-panic", v.failure_id)
    assert.is_true(#v.message < 1000)
  end)

  it("invalid UTF-8 witnesses are hex, and a lone NUL is escaped, never raw", function()
    local g = constant_gen(function() return "bad\255\0" end)
    local v = check(g, function() return false end, Options.new({ ci = "off", num_tests = 1 }))
    assert.is_truthy(v.message:find("0x626164ff00", 1, true))
    assert.is_nil(v.message:find("\0", 1, true))
  end)

  it("a witness with a user __close or __gc metamethod is never closed or collected by the engine", function()
    collectgarbage("stop") -- a finalizer would otherwise fire whenever the collector happens to run
    local log = {}
    local g = constant_gen(function()
      return setmetatable({}, {
        __gc = function() log[#log + 1] = "gc" end,
        __close = function() log[#log + 1] = "close" end,
      })
    end)
    check(g, function() return false end)
    local seen = #log
    collectgarbage("restart")
    assert.are.equal(0, seen)
  end)

  it("a custom shrinker that returns a hostile value cannot break the verdict", function()
    local log = {}
    local v = check(Gens.integer(0, 1000), function(x) return type(x) ~= "number" or x < 5 end, Options.new({ ci = "off", num_tests = 20 }), function()
      return setmetatable({}, H.trap(log)), 3
    end)
    assert.are.equal("property-counterexample", v.failure_id)
    assert.are.equal(0, v.shrink_steps)
    assert.are.same({}, log)
  end)
end)

describe("hostile properties", function()
  it("a property that mutates its candidate does not corrupt the report", function()
    local g = Gens.list(Gens.integer(0, 9), 3, 5)
    local v = check(g, function(l)
      table.sort(l, function(a, b) return a > b end)
      l[#l + 1] = 777
      return false
    end)
    assert.are.equal("property-counterexample", v.failure_id)
  end)

  it("a property that spawns a coroutine, errors inside it, and propagates is a panic", function()
    local v = check(Gens.u8(), function()
      local co = coroutine.wrap(function() error({ code = 7 }, 0) end)
      co()
      return true
    end)
    assert.are.equal("property-panic", v.failure_id)
  end)

  it("a property that returns multiple values is judged by the first", function()
    assert.is_true(check(Gens.u8(), function() return true, false end).passed)
    assert.is_false(check(Gens.u8(), function() return false, true end).passed)
  end)

  it("a property that calls the engine reentrantly with bad arguments gets an ordinary error", function()
    local v = check(Gens.u8(), function() Runner.check("nope", Gens.u8(), function() return true end) return true end)
    assert.are.equal("property-panic", v.failure_id)
  end)

  it("deep recursion inside a property is a stack-overflow panic, not a crash", function()
    local function recurse(n) return 1 + recurse(n + 1) end
    local v = H.limited(5e8, Runner.check, 1, Gens.u8(), function() return recurse(1) end, Options.new({ ci = "on", num_tests = 1 }))
    assert.are.equal("property-panic", v.failure_id)
  end)

  it("string errors from the engine itself never leak the seed in redacted mode", function()
    local v = check(Gens.u8(), function() error("seed is 12345") end, Options.new({ ci = "on", num_tests = 1 }))
    assert.is_nil(v.message:find("12345", 1, true))
  end)
end)
