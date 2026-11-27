-- Fixes from the security review: author-supplied tables are read raw, a hostile
-- string environment cannot make the engine raise, Int guards live in Int itself.
package.path = "../src/?.lua;../src/?/init.lua;" .. package.path
local H = require("helper")
local tb = H.tb
local Runner, Gens, Int, Hooks, J = tb.Runner, tb.Gens, tb.Int, tb.Hooks, tb.StrictJson
local Options = Runner.Options

describe("L1: author-supplied tables are read with raw access only", function()
  it("Options.new, one_of, frequency, tuple and Hooks.with fire no metamethod", function()
    local log = {}
    local trap = H.trap(log)
    local o = Options.new(setmetatable({ ci = "off", num_tests = 3 }, trap))
    assert.are.equal(3, o.num_tests)
    assert.are.equal("off", o.ci)
    local g = Gens.u8()
    Gens.one_of(setmetatable({ g, g }, trap))
    Gens.frequency(setmetatable({ setmetatable({ 1, g }, trap), setmetatable({ 2, g }, trap) }, trap))
    Gens.tuple(setmetatable({ g, g }, trap))
    assert.are.equal(7, Hooks.with(setmetatable({ skip_deletion = true }, trap), function() return 7 end))
    assert.are.same({}, log)
  end)

  it("an unknown option on a trapped table is still refused, without a metamethod", function()
    local log = {}
    assert.has_error(function() Options.new(setmetatable({ bogus = 1 }, H.trap(log))) end)
    assert.are.same({}, log)
  end)
end)

describe("L2: a hostile string environment cannot make Runner.check raise", function()
  it("a property that patches the string metatable and string/table/utf8 functions still yields a verdict", function()
    if os.getenv("LUA_COVERAGE") then
      return pending("luacov's own hook uses string methods, which this test deliberately breaks")
    end
    local smt = getmetatable("")
    local saved = { index = smt.__index, byte = string.byte, sub = string.sub, format = string.format, find = string.find,
      concat = table.concat, ulen = utf8.len, mtype = math.type, char = string.char }
    local boom = function() error("patched") end
    local patched = false
    local function patch()
      if patched then return end
      patched = true
      smt.__index = boom
      string.byte, string.sub, string.format, string.find, string.char = boom, boom, boom, boom, boom
      table.concat, utf8.len = boom, boom
    end
    local function restore()
      smt.__index = saved.index
      string.byte, string.sub, string.format, string.find, string.char = saved.byte, saved.sub, saved.format, saved.find, saved.char
      table.concat, utf8.len = saved.concat, saved.ulen
    end
    local results = {}
    local ok, err = pcall(function()
      for _, mode in ipairs({ "on", "off" }) do
        patched = false
        results[#results + 1] = Runner.check(7, Gens.integer(0, 100000), function(x)
          patch()
          return x <= 100
        end, Options.new({ ci = mode }))
      end
      patched = false
      results[#results + 1] = Runner.check(7, Gens.bytes(0, 8), function(b)
        patch()
        return #b < 3
      end, Options.new({ ci = "on" }))
    end)
    restore()
    assert.is_true(ok, tostring(err))
    for _, v in ipairs(results) do
      assert.are.equal("property-counterexample", v.failure_id)
    end
    assert.are.equal(saved.index, getmetatable("").__index) -- the engine restores nothing it does not own, and we did
    assert.are.equal(saved.byte, string.byte)
  end)

  it("an engine failure becomes an honest property-panic, never a thrown error or a pass", function()
    local v = Hooks.with({ new_rng = function() error("rng broke") end }, Runner.check, 1, Gens.u8(), function() return true end, Options.new({ ci = "on" }))
    assert.is_false(v.passed)
    assert.are.equal("property-panic", v.failure_id)
    assert.is_truthy(v.message:find("(string)", 1, true))
  end)
end)

describe("L4: Int guards live inside Int", function()
  it("parse caps digits itself (1000 ok, 1001 refused, parameter honoured)", function()
    assert.is_truthy(Int.parse(string.rep("9", 1000)))
    assert.has_error(function() Int.parse(string.rep("9", 1001)) end)
    assert.has_error(function() Int.parse("123456", 5) end)
    assert.are.equal(12345, Int.parse("12345", 5))
    assert.has_error(function() Int.parse(12) end)
  end)

  it("shl and pow2 refuse results wider than 2^20 bits", function()
    assert.is_truthy(Int.pow2((1 << 20) - 1))
    assert.has_error(function() Int.pow2(1 << 20) end)
    assert.has_error(function() Int.pow2(-1) end)
    assert.has_error(function() Int.pow2(2.0) end)
    assert.has_error(function() Int.shl(1, 1 << 40) end)
    assert.has_error(function() Int.shl(Int.pow2(1000), (1 << 20) - 10) end)
    assert.has_error(function() Int.shl(1, -1) end)
    assert.are.equal(8, Int.shl(1, 3))
  end)

  it("the runner hands a shrinker a COPY of a Big witness, never the original", function()
    local big = Int.pow2(100)
    local seen
    local g = tb.Gen.new(function() return big end)
    local v = Runner.check(1, g, function() return false end, Options.new({ ci = "off", num_tests = 1 }), function(c)
      seen = c
      return c, 0
    end)
    assert.are.equal("property-counterexample", v.failure_id)
    assert.is_true(Int.eq(seen, big))
    assert.is_false(rawequal(seen, big))
  end)
end)

describe("INFO fixes", function()
  it("Canon never queries the locale and sorts bytewise", function()
    local src = H.read_file("../src/coding_adventures/test_bootstrap/canon.lua")
    assert.is_nil(src:find("setlocale", 1, true))
    assert.are.equal("%{\"B\"=>1,\"a\"=>2}", tb.Canon.render({ a = 2, B = 1 }))
    local saved = os.setlocale
    local called = false
    os.setlocale = function() called = true end
    local key = "coding_adventures.test_bootstrap.canon"
    local original = package.loaded[key]
    package.loaded[key] = nil
    require(key)
    package.loaded[key] = original
    os.setlocale = saved
    assert.is_false(called)
  end)

  it("strict JSON parses 8 MiB of escapes without building a giant parts table", function()
    local doc = '"' .. string.rep("\\n", 4 * 1024 * 1024 - 2) .. '"'
    local v = H.limited(5e8, J.parse, doc)
    assert.are.equal(4 * 1024 * 1024 - 2, #v)
    assert.are.equal("\n", v:sub(1, 1))
  end)
end)
