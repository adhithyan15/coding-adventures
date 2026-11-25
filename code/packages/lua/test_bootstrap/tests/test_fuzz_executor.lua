-- Execution isolation in a single-threaded VM (VER06p section 5): one coroutine per
-- execution under a count hook, sentinels, sticky exhaustion judged after
-- retirement, protocol failures, type-only classification.
--
-- Every target here either terminates by construction or is stopped by the engine's
-- own budgets (small `timeout_ms` / `instruction_budget`), and every test runs under
-- an outer instruction bound.  No test contains a target only an outside timeout
-- could stop (an unbounded `pcall` loop): that limit is documented, not tested.
package.path = "../src/?.lua;../src/?/init.lua;" .. package.path
local H = require("helper")
local tb = H.tb
local Fuzz = tb.Fuzz
local Executor, Handle, Outcome, Options, Signature = Fuzz.Executor, Fuzz.Handle, Fuzz.Outcome, Fuzz.Options, Fuzz.Signature

-- Small budgets: a runaway loop ends in well under a second.
local function opts(over)
  local t = { timeout_ms = 5000, instruction_budget = 2000000, step_budget = 1000, mem_budget = 4 << 20 }
  for k, v in pairs(over or {}) do t[k] = v end
  return Options.new(t)
end

local epoch = 0
local function run(target, data, over)
  epoch = epoch + 1
  return Executor.run(target, data or "x", opts(over), epoch)
end

local function which_of(sentinel)
  local rec = Handle.sentinel_record(sentinel)
  return rec and rec.which
end

describe("a normal execution", function()
  it("passes, reports its edges ascending and counts every edge call", function()
    local out, edges, calls = run(function(data, h)
      h:edge(9)
      h:edge(3)
      h:edge(9)
      assert(data == "hello")
    end, "hello")
    assert.are.equal("pass", out.kind)
    assert.is_false(out.escaped)
    assert.are.same({ 3, 9 }, edges)
    assert.are.equal(3, calls)
  end)

  it("the target gets the data string and a handle with the three methods", function()
    run(function(data, h)
      assert.are.equal("abc\0def", data)
      assert.are.equal("function", type(h.edge))
      assert.are.equal("function", type(h.tick))
      assert.are.equal("function", type(h.reserve))
    end, "abc\0def")
  end)

  it("refuses a target that is not a function and data that is not a string", function()
    assert.has_error(function() Executor.run(5, "x", opts(), 1) end)
    assert.has_error(function() Executor.run(function() end, 5, opts(), 1) end)
    assert.has_error(function() Executor.make(5, opts()) end)
  end)

  it("Executor.make gives an exec that numbers executions and counts edge calls", function()
    local exec, calls = Executor.make(function(_, h) h:edge(1) h:edge(2) end, opts())
    assert.are.equal(0, calls())
    local out, edges = exec("a")
    assert.are.equal("pass", out.kind)
    assert.are.same({ 1, 2 }, edges)
    exec("b")
    assert.are.equal(4, calls())
  end)
end)

describe("crash classification by type() alone", function()
  it("an error string is a crash of kind string, with the frame of the raising line", function()
    local line
    local out = run(function()
      line = debug.getinfo(1, "l").currentline + 1
      error("boom")
    end)
    assert.are.equal("crash", out.kind)
    assert.are.equal("string", out.crash_kind)
    assert.are.same({ "test_fuzz_executor.lua#" .. line }, out.frames)
    assert.is_false(out.escaped)
  end)

  it("every error type has its own kind, error(nil) included", function()
    local cases = {
      { function() error("s") end, "string" },
      { function() error({}) end, "table" },
      { function() error(nil) end, "nil" },
      { function() error(42) end, "number" },
      { function() error(1.5) end, "number" },
      { function() error(true) end, "boolean" },
      { function() error(print) end, "function" },
      { function() error(coroutine.create(print)) end, "thread" },
      { function() local x = nil; return x.field end, "string" }, -- a runtime error is a string
    }
    for i, c in ipairs(cases) do
      local out = run(c[1])
      assert.are.equal("crash", out.kind, "case " .. i)
      assert.are.equal(c[2], out.crash_kind, "case " .. i)
    end
  end)

  it("the same error at two sites is two crashes; at one site, one", function()
    local function a() error("x") end
    local function b() error("x") end
    local oa = run(a)
    local ob = run(b)
    local oa2 = run(a)
    assert.are_not.equal(Signature.crash_signature(oa.crash_kind, oa.frames), Signature.crash_signature(ob.crash_kind, ob.frames))
    assert.are.equal(Signature.crash_signature(oa.crash_kind, oa.frames), Signature.crash_signature(oa2.crash_kind, oa2.frames))
  end)

  it("a hostile error value never has a metamethod run, a field read or a message read", function()
    local log = {}
    local hostile = setmetatable({}, H.trap(log))
    local out = run(function() error(hostile) end)
    assert.are.equal("crash", out.kind)
    assert.are.equal("table", out.crash_kind)
    assert.are.same({}, log)
    -- and the outcome's printable form carries no message either way
    assert.are.equal("#<Fuzz.Outcome crash table>", tostring(out))
    assert.are.same({}, log)
  end)

  it("an error value that is a huge string costs nothing and is not read", function()
    local out = run(function() error(string.rep("x", 1 << 20)) end)
    assert.are.equal("string", out.crash_kind)
    assert.is_true(#table.concat(out.frames) < 200)
  end)

  it("frames walk inner to outer, drop C frames and stop at the body", function()
    local function inner() error("x") end
    local function middle() inner() end
    local out = run(function() middle() end)
    assert.are.equal(3, #out.frames)
    for _, f in ipairs(out.frames) do
      assert.is_truthy(f:match("^test_fuzz_executor%.lua#%d+$"), f)
    end
  end)

  it("a deep recursion is a bounded, collapsed crash (a stack overflow is an ordinary string error)", function()
    local function deep() return 1 + deep() end
    local out = H.limited(5e8, run, deep, "x", { mem_budget = 256 << 20, instruction_budget = 200000000 })
    assert.are.equal("crash", out.kind)
    assert.are.equal("string", out.crash_kind)
    assert.is_true(#out.frames <= Fuzz.Limits.MAX_FRAMES)
    assert.are.equal(1, #out.frames) -- every level is the same frame: collapsed
  end)

  it("a hostile chunk name cannot put a colon, a newline or text longer than the cap in a frame", function()
    local chunk = assert(load("error('x')", "=bad:name\nwith  stuff" .. string.rep("z", 500)))
    local out = run(function() chunk() end)
    assert.are.equal("crash", out.kind)
    for _, f in ipairs(out.frames) do
      assert.is_nil(f:find("[:\n ]"), f)
      assert.is_true(#f <= Fuzz.Limits.MAX_BASENAME + 12)
    end
    assert.is_true(#out.frames >= 1)
  end)

  it("a library patched by the target cannot stop the engine from classifying", function()
    -- luacov's own line hook calls string methods: while measuring coverage, a target that
    -- deletes them breaks the MEASURING tool, not the engine, so this test is skipped then
    if os.getenv("LUA_COVERAGE") then return end
    local saved = { sub = string.sub, gsub = string.gsub, find = string.find, concat = table.concat, floor = math.floor, tostring = tostring }
    local mt = getmetatable("")
    local saved_index = mt.__index
    local out = run(function()
      string.sub, string.gsub, string.find, table.concat, math.floor = nil, nil, nil, nil, nil
      mt.__index = nil
      error("late")
    end)
    -- restore BEFORE asserting (assert uses these)
    string.sub, string.gsub, string.find, table.concat, math.floor = saved.sub, saved.gsub, saved.find, saved.concat, saved.floor
    mt.__index = saved_index
    assert.are.equal("crash", out.kind)
    assert.are.equal("string", out.crash_kind)
  end)
end)

describe("the count hook: instructions and the wall clock", function()
  it("a runaway loop is a timeout by the INSTRUCTION budget", function()
    local out = run(function() while true do end end, "x", { timeout_ms = 60000, instruction_budget = 100000 })
    assert.are.equal("timeout", out.kind)
    assert.is_false(out.escaped)
  end)

  it("a runaway loop is a timeout by the WALL clock", function()
    local started = os.clock()
    local out = run(function() while true do end end, "x", { timeout_ms = 80, instruction_budget = 10000000000 })
    assert.are.equal("timeout", out.kind)
    assert.is_true(os.clock() - started < 5)
  end)

  it("a loop of calls (no allocation) is stopped too", function()
    local function f() return 1 end
    local out = run(function() while true do f() end end, "x", { instruction_budget = 100000 })
    assert.are.equal("timeout", out.kind)
  end)

  it("a target that finishes inside the budget is a pass", function()
    local out = run(function() for _ = 1, 1000 do end end, "x", { instruction_budget = 100000 })
    assert.are.equal("pass", out.kind)
  end)

  it("a timeout has no frames and is not a crash", function()
    local out = run(function() while true do end end, "x", { instruction_budget = 10000 })
    assert.are.equal("timeout", out.kind)
    assert.is_nil(out.crash_kind)
    assert.are.same({}, out.frames)
  end)

  it("a budget of zero instructions times out at the first poll", function()
    local out = run(function() while true do end end, "x", { instruction_budget = 0 })
    assert.are.equal("timeout", out.kind)
  end)

  it("the hook is a closure over ITS execution: a budget breach of one never leaks into the next", function()
    local o1 = run(function() while true do end end, "x", { instruction_budget = 10000 })
    local o2 = run(function() end)
    assert.are.equal("timeout", o1.kind)
    assert.are.equal("pass", o2.kind)
  end)
end)

describe("a target that swallows the budget error is still judged after retirement", function()
  it("catching the instruction-budget error with pcall and returning is a timeout, not a pass", function()
    local out = run(function()
      pcall(function() while true do end end)
    end, "x", { instruction_budget = 20000 })
    assert.are.equal("timeout", out.kind)
  end)

  it("... and raising something ordinary afterwards is still a timeout, not a crash", function()
    local out = run(function()
      pcall(function() while true do end end)
      error("something ordinary")
    end, "x", { instruction_budget = 20000 })
    assert.are.equal("timeout", out.kind)
  end)

  it("catching a tick breach and returning is a timeout; the sticky marker rethrows on every later call", function()
    local first, again, reserve0, reserve1
    local out = run(function(_, h)
      local ok, err = pcall(function() while true do h:tick() end end)
      first = err
      _, again = pcall(h.tick, h)
      _, reserve0 = pcall(h.reserve, h, 0)
      _, reserve1 = pcall(h.reserve, h, 1)
    end, "x", { step_budget = 50 })
    assert.are.equal("timeout", out.kind)
    assert.are.equal("steps", which_of(first))
    assert.is_true(rawequal(first, again))
    assert.is_true(rawequal(first, reserve0))
    assert.is_true(rawequal(first, reserve1))
  end)

  it("a reserve breach is an oom; every later tick raises it, and swallowing it is still an oom", function()
    local first, tick0, tick1
    local out = run(function(_, h)
      local _, err = pcall(h.reserve, h, 1 << 40)
      first = err
      _, tick0 = pcall(h.tick, h, 0)
      _, tick1 = pcall(h.tick, h)
    end, "x", { mem_budget = 1 << 20 })
    assert.are.equal("oom", out.kind)
    assert.are.equal("memory", which_of(first))
    assert.is_true(rawequal(first, tick0))
    assert.is_true(rawequal(first, tick1))
  end)

  it("a counter left over its budget with the marker cleared is still caught by the cross-check", function()
    -- the target reaches the state through the module's own upvalues (the documented
    -- in-process limit) and wipes the marker; the counters still say steps > budget
    local out = run(function(_, h)
      pcall(function() while true do h:tick() end end)
      local i = 1
      while true do
        local name, val = debug.getupvalue(Handle.tick, i)
        if name == nil then break end
        if name == "STATES" then
          local st = val[h]
          st.exhausted, st.which = nil, nil
          st.steps = st.step_budget + 1
        end
        i = i + 1
      end
    end, "x", { step_budget = 10 })
    assert.are.equal("timeout", out.kind)
  end)

  it("a target that returns normally after a clean tick budget is a pass", function()
    local out = run(function(_, h) for _ = 1, 10 do h:tick() end end, "x", { step_budget = 10 })
    assert.are.equal("pass", out.kind)
  end)
end)

describe("memory (best effort, polled)", function()
  it("an allocation loop that retains what it allocates is an oom", function()
    local out = run(function()
      local keep = {}
      for i = 1, 1000000 do keep[i] = string.rep("y", 50000) .. i end
    end, "x", { mem_budget = 4 << 20 })
    assert.are.equal("oom", out.kind)
    assert.is_false(out.escaped)
  end)

  it("garbage that is NOT retained is not an oom: the budget judges live growth after a collection", function()
    local out = run(function()
      for i = 1, 3000 do local _ = string.rep("z", 20000) .. i end -- 60 MB of garbage, none live
    end, "x", { mem_budget = 8 << 20, instruction_budget = 100000000 })
    assert.are.equal("pass", out.kind)
  end)

  it("memory is judged against the budget floored at 1 MiB", function()
    local out = run(function()
      local keep = {}
      for i = 1, 50 do keep[i] = string.rep("y", 4000) .. i end -- 200 KB: under the floor
    end, "x", { mem_budget = 0 })
    assert.are.equal("pass", out.kind)
  end)
end)

describe("forged, stale and retained sentinels and handles", function()
  it("a sentinel-shaped table the target builds is an ordinary crash of kind table", function()
    local out = run(function() error(setmetatable({}, { __index = function() return "steps" end }), 0) end)
    assert.are.equal("crash", out.kind)
    assert.are.equal("table", out.crash_kind)
    out = run(function() error({ which = "steps" }, 0) end)
    assert.are.equal("table", out.crash_kind)
  end)

  it("a sentinel retained from ANOTHER execution and re-raised later is a stale-sentinel crash", function()
    local stash
    local o1 = run(function(_, h)
      local _, err = pcall(function() while true do h:tick() end end)
      stash = err
    end, "x", { step_budget = 5 })
    assert.are.equal("timeout", o1.kind)
    local o2 = run(function() error(stash, 0) end)
    assert.are.equal("crash", o2.kind)
    assert.are.equal(Executor.STALE, o2.crash_kind)
  end)

  it("a RETAINED handle used in a later execution is inert and raises the retired sentinel", function()
    local retained
    run(function(_, h) retained = h h:edge(1) end)
    local seen, ticked
    local out, edges = run(function(_, h)
      retained:edge(77)
      local ok, err = pcall(retained.tick, retained)
      ticked = ok
      seen = err
      h:edge(5)
    end)
    assert.are.equal("pass", out.kind)
    assert.are.same({ 5 }, edges)
    assert.is_false(ticked)
    assert.are.equal("retired", which_of(seen))
  end)

  it("a retained handle that is re-raised from the retired sentinel is a stale-sentinel crash", function()
    local retained
    run(function(_, h) retained = h end)
    local out = run(function() retained:tick() end)
    assert.are.equal("crash", out.kind)
    assert.are.equal(Executor.STALE, out.crash_kind)
  end)

  it("a FORWARDED handle held by a coroutine that outlives the execution is inert afterwards", function()
    local later
    local out, edges = run(function(_, h)
      later = coroutine.wrap(function()
        coroutine.yield()
        h:edge(99)
        return pcall(h.tick, h)
      end)
      later()
      h:edge(1)
    end)
    assert.are.equal("pass", out.kind)
    assert.are.same({ 1 }, edges)
    local ok = later() -- resumes AFTER the execution ended: its hook is the retired one's
    assert.is_false(ok) -- the retired handle raised the retired sentinel inside it
  end)

  it("a coroutine created by the target inherits the hook: its loop is bounded by the SAME budget", function()
    local out = run(function()
      coroutine.wrap(function() while true do end end)()
    end, "x", { instruction_budget = 50000 })
    assert.are.equal("timeout", out.kind)
  end)

  it("a coroutine that outlives its execution runs a hook that finds the state RETIRED", function()
    local stray
    run(function() stray = coroutine.create(function() while true do end end) end)
    local ok, err = coroutine.resume(stray) -- bounded: the stale hook raises at its first poll
    assert.is_false(ok)
    assert.are.equal("retired", which_of(err))
  end)
end)

describe("protocol failures: yield and a tampered hook", function()
  it("a yield out of the engine's coroutine is a crash of kind yielded that counts as LEAKED", function()
    local co
    local out = run(function()
      co = coroutine.running()
      coroutine.yield()
      error("never reached")
    end)
    assert.are.equal("crash", out.kind)
    assert.are.equal(Executor.YIELDED, out.crash_kind)
    assert.is_true(out.escaped)
    assert.are.equal("dead", coroutine.status(co)) -- closed: nothing of it can be resumed later
  end)

  it("a yield with values, and from inside a pcall, is the same failure", function()
    local out = run(function() pcall(coroutine.yield, 1, 2, 3) end)
    assert.are.equal(Executor.YIELDED, out.crash_kind)
    assert.is_true(out.escaped)
  end)

  it("clearing the hook with debug.sethook() is a crash of kind hook-cleared, leaked", function()
    local out = run(function() debug.sethook() end)
    assert.are.equal("crash", out.kind)
    assert.are.equal(Executor.HOOK_CLEARED, out.crash_kind)
    assert.is_true(out.escaped)
  end)

  it("replacing the hook with the target's own is the same failure", function()
    local out = run(function() debug.sethook(function() end, "", 100) end)
    assert.are.equal(Executor.HOOK_CLEARED, out.crash_kind)
    assert.is_true(out.escaped)
  end)

  it("a hook cleared by a target that then errors is still the protocol failure", function()
    local out = run(function() debug.sethook() error("after") end)
    assert.are.equal(Executor.HOOK_CLEARED, out.crash_kind)
    assert.is_true(out.escaped)
  end)

  it("a budget breach still wins, and still counts as escaped, when the target also yielded", function()
    local out = run(function(_, h)
      pcall(function() while true do h:tick() end end)
      coroutine.yield()
    end, "x", { step_budget = 5 })
    assert.are.equal("timeout", out.kind)
    assert.is_true(out.escaped)
  end)

  it("a hostile __close on a yielded coroutine is bounded by the same hook", function()
    local out = run(function()
      local trap <close> = setmetatable({}, { __close = function() while true do end end })
      coroutine.yield()
    end, "x", { instruction_budget = 30000 })
    assert.are.equal("timeout", out.kind)
    assert.is_true(out.escaped)
  end)

  it("an ordinary run is never marked escaped", function()
    for _, target in ipairs({ function() end, function() error("x") end, function() while true do end end }) do
      assert.is_false((run(target, "x", { instruction_budget = 20000 })).escaped)
    end
  end)
end)

describe("hook tampering beyond replacing the function (M1)", function()
  it("keeping the hook function with a huge count is hook-cleared and leaked, not a pass", function()
    local out = run(function()
      local f = debug.gethook()
      debug.sethook(f, "", 1 << 30)
    end)
    assert.are.equal(Executor.HOOK_CLEARED, out.crash_kind)
    assert.is_true(out.escaped)
  end)

  it("changing only the mask is the same failure", function()
    local out = run(function()
      local f, _, c = debug.gethook()
      debug.sethook(f, "r", c)
    end)
    -- the hook's own check can catch it first (a breach), or the post-run check (hook-cleared)
    assert.are_not.equal("pass", out.kind)
    assert.is_true(out.escaped)
  end)

  it("a tampered hook on a coroutine the target made, even one still suspended, is detected", function()
    local out = run(function()
      local co = coroutine.create(function() end)
      debug.sethook(co, function() end, "", 5)
    end)
    assert.are.equal(Executor.HOOK_CLEARED, out.crash_kind)
    assert.is_true(out.escaped)
    local wrapped = run(function()
      local co = coroutine.create(function() coroutine.yield() end)
      coroutine.resume(co)
      debug.sethook(co)
    end)
    assert.are.equal(Executor.HOOK_CLEARED, wrapped.crash_kind)
  end)

  it("a hook that finds its own count changed breaches instead of trusting it", function()
    local out = run(function()
      local f = debug.gethook()
      debug.sethook(f, "", 10) -- fires sooner, so the in-hook check sees the change
      while true do end
    end, "x", { instruction_budget = 100000000 })
    assert.are.equal("timeout", out.kind)
    assert.is_true(out.escaped)
  end)
end)

describe("poisoning the engine from the target (M2)", function()
  local function poison_then_clean(poison)
    local o1 = run(poison)
    local seen
    local o2 = run(function(_, h) seen = { h.tick, h.edge } end)
    return o1, o2, seen
  end

  it("the metatable cannot be reached or rewritten through a handle", function()
    local out = run(function(_, h) getmetatable(h).__index = { tick = function() end } end)
    assert.are.equal("crash", out.kind) -- an ordinary error: getmetatable(h) is false
    assert.is_false(out.escaped)
  end)

  it("replacing Handle.tick through the module is detected, repaired, and does not reach the next execution", function()
    local real = Handle.tick
    local o1, o2, seen = poison_then_clean(function() Handle.tick = function() end end)
    assert.are.equal(Executor.ENGINE_TAMPERED, o1.crash_kind)
    assert.is_true(o1.escaped)
    assert.are.equal("pass", o2.kind)
    assert.are.equal(real, Handle.tick)
    assert.are.equal(real, seen[1])
    -- and re-verification (a plain re-run) is unaffected
    assert.are.equal("pass", (run(function(_, h) h:tick() end)).kind)
  end)

  it("adding a field to the module is detected too", function()
    local out = run(function() Handle.backdoor = true end)
    assert.are.equal(Executor.ENGINE_TAMPERED, out.crash_kind)
    assert.is_nil(Handle.backdoor)
  end)
end)

describe("rewriting the engine through require (round 2)", function()
  local Limits = Fuzz.Limits

  it("replacing Handle.intact and Handle.tick is detected, repaired, and the next execution is bounded", function()
    local real_intact, real_tick = Handle.intact, Handle.tick
    local out = run(function()
      Handle.intact = function() return true end
      Handle.tick = function() end
    end)
    assert.are_not.equal("pass", out.kind)
    assert.is_true(out.escaped)
    assert.are.equal(real_intact, Handle.intact)
    assert.are.equal(real_tick, Handle.tick)
    local next_out = run(function() while true do end end, "x", { instruction_budget = 50000 })
    assert.are.equal("timeout", next_out.kind)
  end)

  it("a huge-count sethook (the engine reads its constants from locals) is still not a pass", function()
    local out = run(function()
      local f = debug.gethook()
      debug.sethook(f, "", 2147483647)
    end)
    assert.are_not.equal("pass", out.kind)
    assert.is_true(out.escaped)
    local next_out = run(function() while true do end end, "x", { instruction_budget = 50000 })
    assert.are.equal("timeout", next_out.kind)
  end)

  it("replacing Executor.make_hook changes nothing: the next long loop is still stopped", function()
    local real = Executor.make_hook
    run(function() Executor.make_hook = function() return function() end end end)
    Executor.make_hook = real
    local out = run(function() while true do end end, "x", { instruction_budget = 50000 })
    assert.are.equal("timeout", out.kind)
  end)

  it("a metatable reached through debug.getmetatable and given a key is detected", function()
    local out = run(function(_, h) debug.getmetatable(h).__newindex = function() end end)
    assert.are.equal(Executor.ENGINE_TAMPERED, out.crash_kind)
    assert.is_true(out.escaped)
    assert.is_true(Handle.intact())
  end)

  it("a nested coroutine that clears its hook, finishes and is collected is still caught", function()
    local out = run(function()
      do
        local co = coroutine.create(function() debug.sethook() for _ = 1, 10 do end end)
        coroutine.resume(co)
      end
      collectgarbage()
      collectgarbage()
    end)
    assert.are_not.equal("pass", out.kind)
    assert.is_true(out.escaped)
  end)

  it("10,000 sequential short-lived wrap generators are an ordinary pass, not a leak", function()
    local out = run(function()
      for i = 1, 10000 do
        local g = coroutine.wrap(function() coroutine.yield(i) end)
        g()
        g()
      end
    end, "x", { instruction_budget = 100000000, mem_budget = 64 << 20 })
    assert.are.equal("pass", out.kind)
    assert.is_false(out.escaped)
  end)

  it("4,097 simultaneously suspended coroutines are a too-many-coroutines crash, not a leak", function()
    local keep = {}
    local out = run(function()
      for i = 1, 4097 do
        keep[i] = coroutine.create(function() coroutine.yield() end)
        coroutine.resume(keep[i])
      end
    end, "x", { instruction_budget = 100000000 })
    assert.are.equal("crash", out.kind)
    assert.are.equal(Executor.TOO_MANY, out.crash_kind)
    assert.is_false(out.escaped)
    assert.are.equal("pass", (run(function() coroutine.create(function() end) end)).kind)
  end)

  it("a dead nested coroutine that cleared its hook is flagged even when dropped by compaction", function()
    local out = run(function()
      local bad = coroutine.create(function() debug.sethook() end)
      coroutine.resume(bad)
      for _ = 1, 4200 do
        local g = coroutine.create(function() end)
        coroutine.resume(g)
      end
    end, "x", { instruction_budget = 100000000 })
    assert.are_not.equal("pass", out.kind)
    assert.is_true(out.escaped)
  end)

  it("a wrapped coroutine that tampers is recorded as it happens", function()
    local out = run(function()
      local w = coroutine.wrap(function() debug.sethook() end)
      w()
    end)
    assert.are_not.equal("pass", out.kind)
    assert.is_true(out.escaped)
  end)
end)

describe("the nested create/wrap replacements after the execution (L1)", function()
  it("a function captured during an execution and called later makes a plain working coroutine", function()
    local captured_create, captured_wrap
    run(function()
      captured_create, captured_wrap = coroutine.create, coroutine.wrap
    end)
    assert.are_not.equal(coroutine.create, captured_create) -- restored globally
    local co = captured_create(function(a) return coroutine.yield(a + 1) end)
    local ok, v = coroutine.resume(co, 1)
    assert.is_true(ok)
    assert.are.equal(2, v)
    if not os.getenv("LUA_COVERAGE") then assert.is_nil(debug.gethook(co)) end -- no hook (luacov gives its own)
    local w = captured_wrap(function() return "fine" end)
    assert.are.equal("fine", w())
  end)

  it("coroutine.create and wrap are restored even after an erroring target", function()
    local before_c, before_w = coroutine.create, coroutine.wrap
    run(function() error("x") end)
    assert.are.equal(before_c, coroutine.create)
    assert.are.equal(before_w, coroutine.wrap)
  end)
end)

describe("make_hook", function()
  local function state(over)
    local _, st = Handle.new(10, 10)
    return st, Executor.make_hook(st, {
      mem_budget = over and over.mem or (1 << 20), timeout_ms = over and over.ms or 60000,
      instruction_budget = over and over.instr or 5000,
    })
  end

  it("counts 1,000 instructions per call and breaches when the total passes the budget", function()
    local st, hook = state({ instr = 3000 })
    hook() hook() hook()
    local ok, err = pcall(hook)
    assert.is_false(ok)
    assert.are.equal("steps", which_of(err))
    assert.are.equal("steps", Handle.exhausted_which(st))
  end)

  it("is sticky: after a breach every call raises the same sentinel", function()
    local _, hook = state({ instr = 0 })
    local _, e1 = pcall(hook)
    local _, e2 = pcall(hook)
    assert.is_true(rawequal(e1, e2))
  end)

  it("a retired state raises the retired sentinel, exhausted or not", function()
    local st, hook = state()
    Handle.retire(st)
    local _, err = pcall(hook)
    assert.are.equal("retired", which_of(err))
  end)

  it("the wall clock breaches as steps", function()
    local _, hook = state({ ms = 0, instr = 1000000000 })
    local t = os.clock()
    while os.clock() == t do end -- guarantee the clock moved
    local ok, err = pcall(hook)
    assert.is_false(ok)
    assert.are.equal("steps", which_of(err))
  end)

  it("live growth over the memory budget breaches as memory; garbage does not", function()
    local keep = {}
    collectgarbage() -- the baseline is taken when the hook is made
    local st, hook = state({ mem = 1 << 20 })
    hook() -- under budget
    for i = 1, 40 do keep[i] = string.rep("m", 100000) .. i end -- 4 MB live
    local ok, err = pcall(hook)
    assert.is_false(ok)
    assert.are.equal("memory", which_of(err))
    assert.are.equal("memory", Handle.exhausted_which(st))
    assert.is_true(#keep == 40)
  end)
end)

describe("the executor refuses to be confused by what a target does with the engine", function()
  it("a target that calls Executor.run recursively gets its own execution and cannot disturb the outer", function()
    local inner
    local out = run(function()
      inner = Executor.run(function() error("inner") end, "y", opts(), 999)
    end)
    assert.are.equal("pass", out.kind)
    assert.are.equal("crash", inner.kind)
  end)

  it("outcomes are recognised by identity: a look-alike is not an Outcome", function()
    local out = run(function() end)
    assert.is_true(Outcome.is(out))
    assert.is_false(Outcome.is({ kind = "pass", frames = {} }))
    assert.is_false(Outcome.is(setmetatable({ kind = "pass" }, getmetatable(out))))
    assert.is_false(Outcome.is(nil))
    assert.is_true(Outcome.is_pass(out))
    assert.is_false(Outcome.is_crash(out))
    assert.is_true(Outcome.is_crash(Outcome.crash("k", {})))
    assert.are.equal("#<Fuzz.Outcome pass>", tostring(out))
    assert.are.equal("#<Fuzz.Outcome timeout>", tostring(Outcome.timeout()))
    assert.are.equal("#<Fuzz.Outcome oom>", tostring(Outcome.oom()))
    assert.are.equal("oom", Outcome.oom(true).kind)
    assert.is_true(Outcome.pass(true).escaped)
  end)
end)
