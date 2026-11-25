-- The shrink-budget tree (VER05e 4.1 via VER05n 5; the required test set of
-- ShrinkBudgetTest, with the Lua carrier: an explicit parameter).
package.path = "../src/?.lua;../src/?/init.lua;" .. package.path
local H = require("helper")
local tb = H.tb
local Budget, Runner, Gens, Gen, Limits, Shrink = tb.Budget, tb.Runner, tb.Gens, tb.Gen, tb.Limits, tb.Shrink
local Options = Runner.Options

local function yes()
  return true
end

local function run(fn, ...)
  return H.limited(5e8, fn, ...)
end

describe("charging", function()
  it("is all or nothing: a refused charge leaves every counter unchanged", function()
    local root = Budget.root(10, 100)
    local child = Budget.child(root, 3, 100)
    assert.is_true(child:charge(5))
    assert.is_true(child:charge(5))
    assert.is_true(child:charge(5))
    -- the child's attempt cap (3) is reached: refused, and nothing moved anywhere
    assert.is_false(child:charge(1))
    assert.is_false(child:charge_copy(1)) -- attempts >= max refuses work-only charges too
    assert.are.equal(3, child:attempts())
    assert.are.equal(15, child:work())
    assert.are.equal(3, root:attempts())
    assert.are.equal(15, root:work())
    assert.is_true(root:refused())
  end)

  it("a refusal higher up (the root's work) leaves the child untouched too", function()
    local root = Budget.root(100, 10)
    local child = Budget.child(root, 100, 1000)
    assert.is_true(child:charge(6))
    assert.is_false(child:charge(6))
    assert.are.equal(1, child:attempts())
    assert.are.equal(6, child:work())
    assert.are.equal(6, root:work())
  end)

  it("charge_copy is work only; zero elements still cost one", function()
    local root = Budget.root(10, 100)
    assert.is_true(root:charge_copy(7))
    assert.are.equal(0, root:attempts())
    assert.are.equal(7, root:work())
    assert.is_true(root:charge_copy(0))
    assert.are.equal(8, root:work())
    assert.is_true(root:charge(0))
    assert.are.equal(1, root:attempts())
    assert.are.equal(9, root:work())
  end)

  it("a huge, negative, float or NaN cost is refused (fail closed) without overflow", function()
    local root = Budget.root(10, 100)
    assert.is_false(root:charge(math.maxinteger))
    assert.is_false(root:charge_copy(math.maxinteger))
    assert.is_false(root:charge(1e300))
    assert.is_false(root:charge(0 / 0))
    assert.is_false(root:charge(-1))
    assert.is_false(root:charge(1.5))
    assert.is_false(root:charge("3"))
    assert.is_false(root:charge(nil))
    assert.are.equal(0, root:attempts())
    assert.are.equal(0, root:work())
    local used = Budget.root(10, math.maxinteger)
    assert.is_true(used:charge(math.maxinteger - 1))
    assert.is_false(used:charge(2)) -- cost > max - used cannot wrap
    assert.is_true(used:charge(1))
    assert.is_false(used:charge(1))
  end)

  it("a joined handle charges its owner and closing it closes nothing", function()
    local root = Budget.root(10, 100)
    local j = Budget.join(root)
    assert.is_true(j:charge(4))
    assert.are.equal(4, root:work())
    assert.are.equal(1, j:attempts())
    j:close()
    assert.is_false(root:is_spent())
    assert.is_true(root:charge(1))
  end)

  it("charged() wraps an evaluation with one attempt and refuses once spent", function()
    local root = Budget.root(2, 100)
    local f = root:charged(yes)
    assert.is_true(f(1))
    assert.is_true(f(1))
    assert.is_false(f(1))
    assert.are.equal(2, root:attempts())
  end)

  it("room() reflects every node on the path", function()
    local root = Budget.root(2, 100)
    local child = Budget.child(root, 100, 100)
    assert.is_true(child:room())
    child:charge(1)
    child:charge(1)
    assert.is_false(child:room()) -- the ROOT is out of attempts
  end)
end)

describe("a closed or stale handle is spent, never a fresh root", function()
  it("refuses every charge once closed, and so does a child of a closed node", function()
    local root = Budget.root(100, 1000)
    local child = Budget.child(root, 100, 1000)
    root:close()
    assert.is_true(root:is_spent())
    assert.is_true(child:is_spent())
    assert.is_false(child:charge(1))
    assert.is_false(child:charge_copy(1))
    assert.is_false(root:charge(1))
    assert.is_false(child:room())
    local grandchild = Budget.open_child(child, 10)
    assert.is_false(grandchild:charge(1))
  end)

  it("a scope closes on every exit: return, error", function()
    local captured
    local ok = pcall(function()
      local scope <close> = Budget.open_child(nil, 10)
      captured = scope
      error("boom")
    end)
    assert.is_false(ok)
    assert.is_true(captured:is_spent())
    local again
    do
      local scope <close> = Budget.open_child(nil, 10)
      again = scope
      assert.is_false(scope:is_spent())
    end
    assert.is_true(again:is_spent())
  end)

  it("a check run under a spent parent is exhausted, never a pass and never a fresh root", function()
    local captured
    local v = run(Runner.check, 1, Gens.u8(), function(_, scope)
      captured = scope
      return true
    end, Options.new({ ci = "off", num_tests = 2 }))
    assert.is_true(v.passed)
    assert.is_true(captured:is_spent())
    assert.is_false(captured:charge(1))
    local nested = run(Runner.check, 1, Gens.u8(), yes, Options.new({ ci = "off", parent = captured }))
    assert.is_false(nested.passed)
    assert.are.equal("property-exhausted", nested.failure_id)
    assert.is_truthy(nested.message:find("spent", 1, true))
  end)

  it("a shrinker handed a spent parent returns its input and shrinks nothing", function()
    local root = Budget.root(100, 1000)
    root:close()
    local m, s = Shrink.shrink_int(50, 0, 0, 100, function() return false end, 1000, root)
    assert.are.same({ 50, 0 }, { m, s })
    m, s = Gens.integer(0, 100):shrink(50, 1000, yes, root)
    assert.are.same({ 50, 0 }, { m, s })
  end)
end)

describe("nesting", function()
  local function nester(levels, captured)
    return function(_, scope)
      captured[1] = captured[1] or scope
      if levels == 0 then
        return true
      end
      local v = Runner.check(1, Gens.u8(), nester(levels - 1, {}), Options.new({ num_tests = 3, ci = "off", parent = scope }))
      return v.passed
    end
  end

  it("a nested check charges exactly 1 + 2 attempts and 1 + 2 + 3 work per outer case", function()
    local captured = {}
    local v = run(Runner.check, 1, Gens.u8(), function(x, scope)
      captured[1] = captured[1] or scope
      local inner = Runner.check(2, Gens.u8(), yes, Options.new({ num_tests = 3, ci = "off", parent = scope }))
      return inner.passed
    end, Options.new({ num_tests = 6, ci = "off" }))
    assert.is_true(v.passed)
    -- entry (1) + cases 2 and 3 (1 each); work: entry 1 + two case charges + three sample charges
    assert.are.equal(6 * 3, captured[1]:attempts())
    assert.are.equal(6 * 6, captured[1]:work())
  end)

  it("depth 1 to 3 all pass and the root accounts for every level", function()
    local totals = {}
    for levels = 1, 3 do
      local captured = {}
      local v = run(Runner.check, 1, Gens.u8(), nester(levels, captured), Options.new({ num_tests = 2, ci = "off" }))
      assert.is_true(v.passed, "levels " .. levels)
      totals[levels] = captured[1]:attempts()
      assert.is_true(totals[levels] <= 2 + Limits.attempt_budget(1000))
    end
    assert.is_true(totals[1] < totals[2] and totals[2] < totals[3])
  end)

  it("branching does not multiply past the root's cap", function()
    local captured = {}
    local function branching(levels)
      return function(_, scope)
        captured[1] = captured[1] or scope
        if levels == 0 then
          return true
        end
        for _ = 1, 4 do
          Runner.check(1, Gens.u8(), branching(levels - 1), Options.new({ num_tests = 4, ci = "off", parent = scope }))
        end
        return true
      end
    end
    local v = run(Runner.check, 1, Gens.u8(), branching(4), Options.new({ num_tests = 4, ci = "off", max_shrink_steps = 10 }))
    -- the work is bounded by the root's cap (num_tests + 16*10 + 64 attempts) however it branches
    assert.is_true(captured[1]:attempts() <= 4 + Limits.attempt_budget(10))
    assert.is_false(v.passed) -- starved: never reported as a pass
    assert.are.equal("property-exhausted", v.failure_id)
  end)

  it("starvation is never a pass: a refused nested check turns a would-be pass into exhausted", function()
    local v = run(Runner.check, 1, Gens.u8(), function(_, scope)
      -- the property itself "passes" whatever the nested check says
      Runner.check(1, Gens.u8(), yes, Options.new({ num_tests = 5000, ci = "off", max_shrink_steps = 0, parent = scope }))
      return true
    end, Options.new({ num_tests = 3, ci = "off", max_shrink_steps = 0 }))
    assert.is_false(v.passed)
    assert.are.equal("property-exhausted", v.failure_id)
    assert.is_truthy(v.message:find("starved", 1, true))
  end)

  it("a falsified verdict is never overridden by starvation", function()
    local v = run(Runner.check, 1, Gens.u8(), function(x, scope)
      Runner.check(1, Gens.u8(), yes, Options.new({ num_tests = 5000, ci = "off", max_shrink_steps = 0, parent = scope }))
      return false
    end, Options.new({ num_tests = 3, ci = "off", max_shrink_steps = 0 }))
    assert.are.equal("property-counterexample", v.failure_id)
  end)

  it("nesting is capped at 64 enclosing checks: the 65th evaluates nothing", function()
    local innermost
    local function dive(level)
      return function(_, scope)
        if level >= 70 then
          return true
        end
        innermost = Runner.check(1, Gens.u8(), dive(level + 1), Options.new({ num_tests = 1, ci = "off", max_shrink_steps = 0, parent = scope }))
        return true
      end
    end
    local v = run(Runner.check, 1, Gens.u8(), dive(1), Options.new({ num_tests = 1, ci = "off", max_shrink_steps = 0 }))
    assert.is_false(v.passed)
    assert.are.equal("property-exhausted", v.failure_id)
    assert.is_truthy(innermost)
  end)

  it("depth-cap joining: scopes past depth 64 join and closing them closes nothing", function()
    local scopes = {}
    local owner = Budget.root(100000, 1000000)
    local cur = owner
    for _ = 1, 70 do
      cur = Budget.open_child(cur, 10)
      scopes[#scopes + 1] = cur
    end
    assert.is_true(scopes[70]:depth() <= Budget.MAX_DEPTH - 1)
    scopes[70]:close()
    scopes[69]:close()
    assert.is_false(scopes[1]:is_spent())
    assert.is_true(scopes[70]:charge(1))
    assert.is_true(scopes[1]:attempts() >= 1) -- the joined charge reached the ancestors
  end)

  it("re-entrancy equality: a nested check gives the same verdict as a top-level one", function()
    local top = run(Runner.check, 9, Gens.integer(0, 100000), function(x) return x <= 100 end, Options.new({ ci = "off" }))
    local nested
    run(Runner.check, 1, Gens.u8(), function(_, scope)
      nested = Runner.check(9, Gens.integer(0, 100000), function(x) return x <= 100 end, Options.new({ ci = "off", parent = scope }))
      return true
    end, Options.new({ ci = "off", num_tests = 1 }))
    assert.are.equal(top.failure_id, nested.failure_id)
    assert.are.equal(top.counterexample, nested.counterexample)
    assert.are.equal(top.minimal, nested.minimal)
    assert.are.equal(top.shrink_steps, nested.shrink_steps)
  end)
end)

describe("shrinkers and filters inside the tree", function()
  it("a recursing custom shrinker is bounded by the root however deep it recurses", function()
    local root = Budget.root(Limits.attempt_budget(1000) + 100, Limits.MAX_SHRINK_WORK)
    local evaluations, depth = 0, 0
    local gen
    gen = Gen.new(function() return 0 end, function(value, falsifies, _, scope)
      -- a hostile shrinker: every accepted evaluation recurses into another shrink of the
      -- same generator, which loops again; only the budget can end this
      depth = depth + 1
      for i = 1, 100000 do
        evaluations = evaluations + 1
        if depth > 40 or not falsifies(i) then
          break
        end
        gen:shrink(i, 1000, yes, scope)
      end
      depth = depth - 1
      return value, 0
    end)
    run(function() return gen:shrink(5, 1000, yes, root) end)
    assert.is_true(root:attempts() <= Limits.attempt_budget(1000) + 100)
    assert.is_true(evaluations < 100000)
  end)

  it("a filter predicate that itself shrinks is charged to the same tree", function()
    local root = Budget.root(500, Limits.MAX_SHRINK_WORK)
    local scope = Budget.open_child(root, 100)
    local inner = Gens.integer(0, 1000)
    local g = Gens.filter(Gens.integer(0, 1000), function(x)
      inner:shrink(x, 100, yes, scope) -- predicate does real shrinking work
      return x % 2 == 0
    end)
    local before = root:attempts()
    run(function() return g:shrink_nested(900, yes, 1000, scope) end)
    assert.is_true(root:attempts() > before)
    assert.is_true(root:attempts() <= 500)
  end)

  it("a filter sampler rejecting everything is bounded by its retries and by the budget", function()
    local calls = 0
    local g = Gens.filter(Gens.u8(), function() calls = calls + 1; return false end, 10000)
    local v = run(Runner.check, 1, g, yes, Options.new({ ci = "off" }))
    assert.are.equal("property-exhausted", v.failure_id)
    assert.are.equal(10000, calls)
    -- nested under a small budget the retries are charged as attempts and refused early
    calls = 0
    local captured
    run(Runner.check, 1, Gens.u8(), function(_, scope)
      captured = Runner.check(1, g, yes, Options.new({ ci = "off", parent = scope, max_shrink_steps = 0 }))
      return true
    end, Options.new({ ci = "off", num_tests = 1, max_shrink_steps = 0 }))
    assert.are.equal("property-exhausted", captured.failure_id)
    assert.is_true(calls <= 100, "retries are charged as attempts: saw " .. calls)
  end)

  it("top-level filter retries are charged as WORK, not attempts", function()
    local captured
    local g = Gens.filter(Gens.integer(0, 3), function(x) return x == 3 end, 1000)
    local v = run(Runner.check, 5, g, function(_, scope)
      captured = captured or scope
      return true
    end, Options.new({ ci = "off", num_tests = 1 }))
    assert.is_true(v.passed)
    assert.are.equal(0, captured:attempts())
    assert.is_true(captured:work() >= 1)
  end)

  it("a rare filter (1 in 200) is not starved at top level by the attempt cap", function()
    local g = Gens.filter(Gens.integer(0, 199), function(x) return x == 0 end, 10000)
    local v = run(Runner.check, 3, g, yes, Options.new({ ci = "off", num_tests = 20, max_shrink_steps = 0 }))
    assert.is_true(v.passed, v.message)
  end)
end)
