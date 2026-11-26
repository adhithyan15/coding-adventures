-- Hazard 6: coroutines.  The budget carrier is an explicit parameter, so a
-- yield, a killed coroutine or a captured handle cannot corrupt another check.
package.path = "../src/?.lua;../src/?/init.lua;" .. package.path
local H = require("helper")
local tb = H.tb
local Budget, Runner, Gens = tb.Budget, tb.Runner, tb.Gens
local Options = Runner.Options

local function off(t)
  t = t or {}
  t.ci = "off"
  return Options.new(t)
end

-- Resumes `co` until it finishes, returning the yielded values and the result.
local function drive(co, limit)
  local yields = {}
  for _ = 1, limit or 1000 do
    local ok, a = coroutine.resume(co)
    assert.is_true(ok, tostring(a))
    if coroutine.status(co) == "dead" then
      return yields, a
    end
    yields[#yields + 1] = a
  end
  error("coroutine did not finish")
end

describe("a property that yields across the pcall boundary", function()
  it("suspends the whole check and resumes it intact", function()
    local co = coroutine.create(function()
      return Runner.check(1, Gens.u8(), function()
        coroutine.yield("tick")
        return true
      end, off({ num_tests = 3 }))
    end)
    local yields, verdict = drive(co)
    assert.are.same({ "tick", "tick", "tick" }, yields)
    assert.is_true(verdict.passed)
  end)

  it("yields inside shrinking too, and the minimal witness is unchanged", function()
    local evals = 0
    local co = coroutine.create(function()
      return Runner.check(7, Gens.integer(0, 100000), function(x)
        evals = evals + 1
        coroutine.yield()
        return x <= 100
      end, off())
    end)
    local _, v = drive(co, 100000)
    assert.are.equal(101, v.minimal)
    assert.is_true(evals > 10)
    local plain = Runner.check(7, Gens.integer(0, 100000), function(x) return x <= 100 end, off())
    assert.are.equal(plain.minimal, v.minimal)
    assert.are.equal(plain.shrink_steps, v.shrink_steps)
  end)

  it("a yield on the main thread is a property panic, not an engine crash", function()
    local v = Runner.check(1, Gens.u8(), function()
      coroutine.yield()
      return true
    end, off({ num_tests = 1 }))
    assert.are.equal("property-panic", v.failure_id)
  end)

  it("a generator that yields works the same way", function()
    local g = tb.Gen.new(function(rng)
      coroutine.yield("sampling")
      return rng:bounded(10)
    end)
    local co = coroutine.create(function()
      return Runner.check(1, g, function() return true end, off({ num_tests = 2 }))
    end)
    local yields, v = drive(co)
    assert.are.same({ "sampling", "sampling" }, yields)
    assert.is_true(v.passed)
  end)
end)

describe("a coroutine killed mid-check", function()
  it("closes the check's scope (the to-be-closed variable runs)", function()
    local captured
    local co = coroutine.create(function()
      return Runner.check(1, Gens.u8(), function(_, scope)
        captured = scope
        coroutine.yield("paused")
        return true
      end, off())
    end)
    assert.is_true(coroutine.resume(co))
    assert.is_false(captured:is_spent())
    assert.is_true(captured:charge(1))
    assert.is_true(coroutine.close(co))
    assert.are.equal("dead", coroutine.status(co))
    assert.is_true(captured:is_spent())
    assert.is_false(captured:charge(1)) -- refuses forever; never a fresh root
  end)

  it("closes shrink scopes opened inside the dead check as well", function()
    local seen = {}
    local co = coroutine.create(function()
      return Runner.check(7, Gens.integer(0, 100000), function(x, scope)
        seen[#seen + 1] = scope
        if x <= 100 then
          return true
        end
        if #seen > 2 then
          coroutine.yield("in the shrink")
        end
        return false
      end, off())
    end)
    assert.is_true(coroutine.resume(co))
    assert.are.equal("suspended", coroutine.status(co))
    assert.is_true(coroutine.close(co))
    for _, scope in ipairs(seen) do
      assert.is_true(scope:is_spent())
    end
  end)

  it("leaves unrelated checks untouched: the killed check shares nothing", function()
    local doomed = coroutine.create(function()
      return Runner.check(1, Gens.u8(), function() coroutine.yield() return true end, off())
    end)
    coroutine.resume(doomed)
    coroutine.close(doomed)
    local v = Runner.check(7, Gens.integer(0, 100000), function(x) return x <= 100 end, off())
    assert.are.equal(101, v.minimal)
  end)

  it("an abandoned, never-closed coroutine is reclaimed without effect", function()
    do
      local co = coroutine.create(function()
        Runner.check(1, Gens.u8(), function() coroutine.yield() return true end, off())
      end)
      coroutine.resume(co)
    end
    collectgarbage()
    collectgarbage()
    assert.is_true(Runner.check(1, Gens.u8(), function() return true end, off()).passed)
  end)
end)

describe("interleaved checks are independent", function()
  it("two coroutines each running a check give the same verdicts as running alone", function()
    local function job(seed)
      return coroutine.create(function()
        return Runner.check(seed, Gens.integer(0, 100000), function(x)
          coroutine.yield()
          return x <= 100
        end, off())
      end)
    end
    local a, b = job(7), job(8)
    local ra, rb
    for _ = 1, 100000 do
      if coroutine.status(a) ~= "dead" then
        local ok, r = coroutine.resume(a)
        assert.is_true(ok, tostring(r))
        if coroutine.status(a) == "dead" then ra = r end
      end
      if coroutine.status(b) ~= "dead" then
        local ok, r = coroutine.resume(b)
        assert.is_true(ok, tostring(r))
        if coroutine.status(b) == "dead" then rb = r end
      end
      if ra and rb then break end
    end
    local sa = Runner.check(7, Gens.integer(0, 100000), function(x) return x <= 100 end, off())
    local sb = Runner.check(8, Gens.integer(0, 100000), function(x) return x <= 100 end, off())
    assert.are.same(sa, ra)
    assert.are.same(sb, rb)
  end)

  it("coroutines charging one shared tree never exceed its caps", function()
    local root = Budget.root(50, 1000)
    local function worker()
      return coroutine.create(function()
        for _ = 1, 100 do
          root:charge(3)
          coroutine.yield()
        end
      end)
    end
    local ws = { worker(), worker(), worker() }
    for _ = 1, 100 do
      for _, w in ipairs(ws) do
        if coroutine.status(w) ~= "dead" then coroutine.resume(w) end
      end
    end
    assert.are.equal(50, root:attempts())
    assert.are.equal(150, root:work())
    assert.is_true(root:refused())
  end)
end)

describe("a coroutine that captured a handle", function()
  it("charges the same tree through the handle", function()
    local captured
    local v = Runner.check(1, Gens.u8(), function(_, scope)
      captured = captured or scope
      local co = coroutine.create(function()
        return Runner.check(2, Gens.u8(), function() return true end, off({ num_tests = 3, parent = scope }))
      end)
      local ok, inner = coroutine.resume(co)
      assert.is_true(ok)
      return inner.passed
    end, off({ num_tests = 4 }))
    assert.is_true(v.passed)
    assert.are.equal(4 * 3, captured:attempts())
  end)

  it("is refused every charge once the scope it captured has closed (spent, never a fresh root)", function()
    local held
    local co
    local v = Runner.check(1, Gens.u8(), function(_, scope)
      held = scope
      co = co or coroutine.create(function()
        coroutine.yield()
        return Runner.check(2, Gens.u8(), function() return true end, off({ parent = held }))
      end)
      coroutine.resume(co) -- parks at the first yield
      return true
    end, off({ num_tests = 1 }))
    assert.is_true(v.passed)
    -- the scope has closed; the coroutine wakes up and tries to use its handle
    local ok, inner = coroutine.resume(co)
    assert.is_true(ok)
    assert.are.equal("property-exhausted", inner.failure_id)
    assert.is_false(inner.passed)
  end)

  it("a coroutine WITHOUT a handle opens a fresh root and cannot touch ours", function()
    local captured
    Runner.check(1, Gens.u8(), function(_, scope)
      captured = captured or scope
      local co = coroutine.wrap(function()
        return Runner.check(2, Gens.u8(), function() return true end, off({ num_tests = 5 }))
      end)
      assert.is_true(co().passed)
      return true
    end, off({ num_tests = 2 }))
    assert.are.equal(0, captured:attempts()) -- nothing was charged to it
  end)
end)
