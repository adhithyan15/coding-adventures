-- The campaign, its caps, the leak cap in every path, verdicts and CI mode
-- (VER06 section 9.1, VER06p sections 5.4 and 6).
package.path = "../src/?.lua;../src/?/init.lua;" .. package.path
local H = require("helper")
local tb = H.tb
local Fuzz = tb.Fuzz
local Engine, Outcome, Options, Report, Signature, Limits = Fuzz.Engine, Fuzz.Outcome, Fuzz.Options, Fuzz.Report, Fuzz.Signature, Fuzz.Limits

local function opts(over)
  local t = { max_iterations = 200, seed = 42, timeout_ms = 5000, instruction_budget = 5000000 }
  for k, v in pairs(over or {}) do t[k] = v end
  return Options.new(t)
end

local function campaign(corpus, o, exec, extras)
  return H.limited(5e8, Fuzz.run_outcomes, corpus, o, exec, extras)
end

local PASS = function() return Outcome.pass(), {} end

describe("Options", function()
  it("has the documented defaults and prints without the seed", function()
    local o = Options.new()
    assert.are.equal(1000, o.max_iterations)
    assert.are.equal(4096, o.max_input_len)
    assert.are.equal(10, o.base_energy)
    assert.are.equal(2000, o.minimize_budget)
    assert.are.equal(200000, o.step_budget)
    assert.are.equal(64 << 20, o.mem_budget)
    assert.are.equal(5000, o.timeout_ms)
    assert.are.equal(100000000, o.instruction_budget)
    assert.are.equal("unset", o.ci)
    assert.are.equal(42, o.seed)
    assert.are.equal("#<Fuzz.Options max_iterations=1000>", tostring(Options.new({ seed = 987654321 })))
  end)

  it("refuses unknown fields, out-of-range values, floats, strings and a bad ci mode", function()
    local bad = {
      { bogus = 1 }, { max_iterations = -1 }, { max_iterations = Limits.MAX_ITERATIONS + 1 },
      { max_iterations = 1.5 }, { max_iterations = "10" }, { max_input_len = Limits.MAX_INPUT_BYTES + 1 },
      { base_energy = -1 }, { minimize_budget = Limits.MAX_MINIMIZE_BUDGET + 1 }, { step_budget = -1 },
      { mem_budget = Limits.MAX_MEM_BUDGET + 1 }, { timeout_ms = Limits.MAX_TIMEOUT_MS + 1 },
      { instruction_budget = Limits.MAX_INSTRUCTION_BUDGET + 1 }, { seed = 1.5 }, { seed = "1" },
      { ci = "maybe" }, { ci = true },
    }
    for i, t in ipairs(bad) do
      assert.is_false(pcall(Options.new, t), "case " .. i)
    end
    assert.has_error(function() Options.new({ [{}] = 1 }) end)
  end)

  it("accepts a Big seed and every ci mode, and is read without metamethods", function()
    assert.is_true(tb.Int.eq(tb.Int.parse("100000000000000000000"), Options.new({ seed = tb.Int.parse("100000000000000000000") }).seed))
    for _, ci in ipairs({ "on", "off", "unset" }) do assert.are.equal(ci, Options.new({ ci = ci }).ci) end
    local log = {}
    local t = setmetatable({}, H.trap(log))
    Options.new(t)
    assert.are.same({}, log)
    assert.is_true(Options.is(Options.new()))
    assert.is_false(Options.is({}))
    assert.is_false(Options.is(nil))
  end)
end)

describe("campaign contracts (VER06 9.1)", function()
  local function staircase(data, edges)
    -- edges are reported per stage reached: the bytes "A", "B", "C" in order
    edges[#edges + 1] = 1
    if data:sub(1, 1) == "A" then
      edges[#edges + 1] = 2
      if data:sub(2, 2) == "B" then
        edges[#edges + 1] = 3
        if data:sub(3, 3) == "C" then
          return Outcome.crash("staircase", {})
        end
      end
    end
    return Outcome.pass()
  end

  it("is deterministic: the same inputs give the same report, a different seed another", function()
    local function exec(data)
      local edges = {}
      local out = staircase(data, edges)
      return out, edges
    end
    local function key(r)
      return table.concat({ r.iterations, r.admitted_seeds, r.total_edges, #r.crashes, table.concat(r.corpus, "|") }, ";")
    end
    local a = campaign({ "AAA" }, opts({ max_iterations = 300 }), exec)
    local b = campaign({ "AAA" }, opts({ max_iterations = 300 }), exec)
    local c = campaign({ "AAA" }, opts({ max_iterations = 300, seed = 43 }), exec)
    assert.are.equal(key(a), key(b))
    assert.are_not.equal(key(a), key(c))
  end)

  it("runs exactly max_iterations mutations with a live corpus, and no candidate exceeds max_input_len", function()
    local longest, count = 0, 0
    local function exec(data)
      count = count + 1
      if count > 1 then longest = math.max(longest, #data) end -- the first run is the seed, as given
      return Outcome.pass(), {}
    end
    local r = campaign({ "0123456789" }, opts({ max_iterations = 150, max_input_len = 12 }), exec)
    assert.are.equal(150, r.iterations)
    assert.are.equal(151, count)
    assert.is_true(longest <= 12, "longest " .. longest)
  end)

  it("initial seeds run as given, even beyond max_input_len", function()
    local seen
    campaign({ string.rep("s", 100) }, opts({ max_iterations = 0 }), function(d) seen = #d return Outcome.pass(), {} end)
    assert.are.equal(100, seen)
  end)

  it("an empty corpus is replaced by one 00 seed", function()
    local first
    local r = campaign({}, opts({ max_iterations = 5 }), function(d)
      first = first or d
      return Outcome.pass(), {}
    end)
    assert.are.equal("\0", first)
    assert.are.equal(5, r.iterations)
  end)

  it("seeds that crash, time out or exhaust memory are never mutated: a corpus of failures fuzzes nothing", function()
    local outs = { a = Outcome.crash("k", {}), b = Outcome.timeout(), c = Outcome.oom() }
    local r = campaign({ "a", "b", "c" }, opts({ max_iterations = 100 }), function(d)
      return outs[d] or Outcome.pass(), {}
    end)
    assert.are.equal(0, r.iterations)
    assert.is_false(Report.clean(r))
    assert.are.equal(1, r.timeouts)
    assert.are.equal(1, r.ooms)
    assert.are.equal(1, #r.crashes)
  end)

  it("every execution lands somewhere and clean needs a mutation iteration and nothing else", function()
    local clean = campaign({ "x" }, opts({ max_iterations = 20 }), PASS)
    assert.is_true(Report.clean(clean))
    assert.are.equal(clean.admitted_seeds, 1) -- the first passing seed is admitted without coverage
    assert.are.equal(Report.TIER_BLACK_BOX, clean.coverage_tier)
    assert.is_false(Report.clean(campaign({ "x" }, opts({ max_iterations = 0 }), PASS))) -- zero executions never pass
    local timeouts = campaign({ "x" }, opts({ max_iterations = 20 }), function(d) return #d > 5 and Outcome.timeout() or Outcome.pass(), {} end)
    assert.is_true(timeouts.timeouts == 0 or not Report.clean(timeouts))
    local always_oom = campaign({ "x", "y" }, opts({ max_iterations = 20 }), function() return Outcome.oom(), {} end)
    assert.is_false(Report.clean(always_oom))
    assert.are.equal(2, always_oom.ooms)
  end)

  it("finds the staircase crash, deduplicates it, minimises it to its 3 bytes and counts hits", function()
    local function exec(data)
      local edges = {}
      return staircase(data, edges), edges
    end
    local r = campaign({ "AAAA" }, opts({ max_iterations = 4000, seed = 5 }), exec)
    assert.are.equal(1, #r.crashes)
    local rec = r.crashes[1]
    assert.are.equal("staircase", rec.kind)
    assert.are.equal("ABC", rec.minimal_input)
    assert.is_true(rec.minimized)
    assert.is_false(rec.input_dropped)
    assert.is_true(rec.hits >= 1)
    assert.are.equal(Signature.crash_signature("staircase", {}), rec.signature)
    assert.are.equal(Report.TIER_REPORTED, r.coverage_tier)
    assert.is_truthy(tostring(rec):find("staircase", 1, true))
    assert.is_nil(tostring(rec):find("ABC", 1, true)) -- the witness never reaches a printed form
  end)

  it("a report and an options object print without the seed, the corpus or any input", function()
    local r = campaign({ "SECRETSEED" }, opts({ max_iterations = 5, seed = 987654321 }), PASS)
    local text = tostring(r)
    assert.is_nil(text:find("SECRET", 1, true))
    assert.is_nil(text:find("987654321", 1, true))
    assert.is_truthy(text:find("iterations=5", 1, true))
  end)

  it("one crash record per signature, for seeds as well as mutants, in first-seen order", function()
    local r = campaign({ "a", "b" }, opts({ max_iterations = 0 }), function(d)
      return Outcome.crash("same", { "f" }), {}
    end)
    assert.are.equal(1, #r.crashes)
    assert.are.equal(2, r.crashes[1].hits)
  end)

  it("a shorter witness of a known crash replaces the retained one", function()
    local n = 0
    local r = campaign({ "abcdef", "ab" }, opts({ max_iterations = 0, minimize_budget = 0 }), function()
      n = n + 1
      return Outcome.crash("same", {}), {}
    end)
    assert.are.equal("ab", r.crashes[1].minimal_input)
    assert.are.equal(2, r.crashes[1].hits)
  end)

  it("sorts a seed's edges into the corpus only when they are new, and reports the tier by evidence", function()
    local r = campaign({ "a" }, opts({ max_iterations = 50 }), function(d) return Outcome.pass(), { #d % 3 } end)
    assert.are.equal(Report.TIER_REPORTED, r.coverage_tier)
    assert.are.equal(3, r.total_edges)
    assert.is_true(r.admitted_seeds <= 3 + 0)
    local labelled = campaign({ "a" }, opts({ max_iterations = 3 }), PASS, { tier = function(total) return "custom:" .. total end })
    assert.are.equal("custom:0", labelled.coverage_tier)
  end)
end)

describe("caps (CWE-400), checked before the allocation they bound", function()
  it("refuses a bad corpus before running anything", function()
    local ran = false
    local function exec() ran = true return Outcome.pass(), {} end
    local o = opts()
    assert.has_error(function() Fuzz.run_outcomes("x", o, exec) end)
    assert.has_error(function() Fuzz.run_outcomes({ 1 }, o, exec) end)
    assert.has_error(function() Fuzz.run_outcomes({ string.rep("x", Limits.MAX_INPUT_BYTES + 1) }, o, exec) end)
    local many = {}
    for i = 1, Limits.MAX_INITIAL_CORPUS + 1 do many[i] = "x" end
    assert.has_error(function() Fuzz.run_outcomes(many, o, exec) end)
    assert.has_error(function() Fuzz.run_outcomes({}, {}, exec) end)
    assert.has_error(function() Fuzz.run_outcomes({}, o, "exec") end)
    assert.is_false(ran)
  end)

  it("a custom exec that returns a non-Outcome is refused", function()
    assert.has_error(function() Fuzz.run_outcomes({ "x" }, opts(), function() return { kind = "pass" }, {} end) end)
    assert.has_error(function() Fuzz.run_outcomes({ "x" }, opts(), function() return "pass", {} end) end)
  end)

  it("the retained-bytes cap drops an over-cap admission and counts it", function()
    local r = campaign({ "aaaa" }, opts({ max_iterations = 100 }), function(d) return Outcome.pass(), { #d } end, { retained_cap = 6 })
    assert.is_true(r.admissions_dropped > 0)
    local total = 0
    for _, s in ipairs(r.corpus) do total = total + #s end
    assert.is_true(total <= 6, "retained " .. total)
  end)

  it("an over-cap crash record keeps no input and says so", function()
    local r = campaign({ "abcdefgh" }, opts({ max_iterations = 0 }), function() return Outcome.crash("k", {}), {} end, { retained_cap = 3 })
    assert.are.equal(1, #r.crashes)
    assert.is_true(r.crashes[1].input_dropped)
    assert.are.equal("", r.crashes[1].minimal_input)
    assert.is_false(r.crashes[1].minimized)
  end)

  it("a smaller witness releases its retained bytes", function()
    -- cap 10: the first crash keeps 5 bytes; a repeat with 1 byte frees 4; then a seed of 9 fits
    local r = campaign({ "abcde", "a" }, opts({ max_iterations = 0, minimize_budget = 0 }), function(d)
      if #d == 5 or #d == 1 then return Outcome.crash("k", {}), {} end
      return Outcome.pass(), {}
    end, { retained_cap = 10 })
    assert.are.equal("a", r.crashes[1].minimal_input)
  end)

  it("at most 1,000 distinct crash signatures are kept; the rest are counted", function()
    local n = 0
    local r = H.limited(8e8, Fuzz.run_outcomes, { "a" }, opts({ max_iterations = 1100, minimize_budget = 0 }), function()
      n = n + 1
      return Outcome.crash("kind" .. n, {}), {}
    end)
    -- the seed itself is crash 1 and is never mutated: no iterations; use many seeds instead
    assert.are.equal(1, #r.crashes)
    local seeds = {}
    for i = 1, 1100 do seeds[i] = tostring(i) end
    local m = 0
    r = H.limited(8e8, Fuzz.run_outcomes, seeds, opts({ max_iterations = 0, minimize_budget = 0 }), function()
      m = m + 1
      return Outcome.crash("kind" .. m, {}), {}
    end)
    assert.are.equal(Limits.MAX_CRASHES, #r.crashes)
    assert.are.equal(100, r.crashes_dropped)
    assert.are.equal(1100, Report.unique_crashes(r))
    assert.is_false(Report.clean(r))
  end)

  it("the minimisation budget is campaign-wide: it counts REJECTED candidates and never exceeds max_iterations", function()
    local calls = 0
    local seeds = {}
    for i = 1, 30 do seeds[i] = string.rep(string.char(64 + (i % 26)), 40) end
    campaign(seeds, opts({ max_iterations = 25, minimize_budget = 1000 }), function()
      calls = calls + 1
      -- every execution is a DIFFERENT crash: every minimisation candidate is rejected
      return Outcome.crash("distinct" .. calls, {}), {}
    end)
    -- 30 triage executions + at most 25 minimisation executions in total
    assert.is_true(calls <= 30 + 25, "calls " .. calls)
    assert.is_true(calls >= 30 + 25 - 1, "calls " .. calls)
  end)

  it("minimize_budget caps one crash", function()
    local calls = 0
    local r = campaign({ string.rep("z", 40) }, opts({ max_iterations = 500, minimize_budget = 7 }), function()
      calls = calls + 1
      return Outcome.crash("k" .. calls, {}), {}
    end)
    assert.is_true(calls <= 1 + 7)
    assert.is_false(r.crashes[1].minimized)
  end)
end)

describe("the leak cap holds in EVERY path (VER06p 5.4)", function()
  local function leaky() return Outcome.crash("yielded", {}, true), {} end

  it("the guard counts escaped executions and runs nothing once the cap is reached", function()
    local leaks = Engine.new_leaks()
    local calls = 0
    local g = Engine.guard(function() calls = calls + 1 return leaky() end, leaks)
    for _ = 1, Limits.MAX_LEAKED do assert.is_not_nil(g("x")) end
    assert.are.equal(Limits.MAX_LEAKED, leaks.count)
    assert.is_true(Engine.leak_cap_reached(leaks))
    assert.is_nil(g("x"))
    assert.are.equal(Limits.MAX_LEAKED, calls)
    assert.has_error(function() Engine.guard(function() return 1 end, Engine.new_leaks())("x") end)
  end)

  it("triage stops at the cap: later seeds never run, the report says aborted and is never clean", function()
    local calls = 0
    local seeds = {}
    for i = 1, 10 do seeds[i] = tostring(i) end
    local r = campaign(seeds, opts(), function() calls = calls + 1 return leaky() end)
    assert.are.equal(4, calls)
    assert.are.equal(4, r.leaked)
    assert.is_true(r.aborted)
    assert.are.equal(0, r.iterations)
    assert.is_false(Report.clean(r))
  end)

  it("the mutation loop stops at the cap", function()
    local calls = 0
    local r = campaign({ "seed" }, opts({ max_iterations = 1000 }), function()
      calls = calls + 1
      if calls == 1 then return Outcome.pass(), {} end
      return leaky()
    end)
    -- the seed, one leaked mutant, and the three probes its minimisation ran before the cap
    assert.are.equal(5, calls)
    assert.are.equal(4, r.leaked)
    assert.is_true(r.aborted)
    assert.are.equal(1, r.iterations) -- the loop then found the cap reached and ran nothing more
    assert.is_false(Report.clean(r))
  end)

  it("minimisation stops at the cap: the crash keeps the input it has, unminimised", function()
    local calls = 0
    local r = campaign({ string.rep("m", 64) }, opts({ max_iterations = 100 }), function()
      calls = calls + 1
      if calls == 1 then return Outcome.crash("real", {}), {} end
      return leaky() -- every minimisation candidate escapes
    end)
    assert.are.equal(1 + 4, calls)
    assert.are.equal(1, #r.crashes)
    assert.is_false(r.crashes[1].minimized)
    assert.is_true(#r.crashes[1].minimal_input >= 1)
    assert.is_true(r.aborted)
  end)

  it("re-verification: a report that already leaked four executions re-verifies nothing", function()
    local calls = 0
    -- a report with leaked == 4 and one crash
    local seeds = { "a", "b", "c", "d", "e" }
    local r = campaign(seeds, opts(), function(d) return d == "a" and Outcome.crash("real", {}) or leaky(), {} end)
    assert.are.equal(4, r.leaked)
    assert.are.equal(2, #r.crashes) -- "real", and the escaped executions' own signature
    local v = Fuzz.verdict(r, Options.new({ ci = "off" }), function() calls = calls + 1 return Outcome.crash("real", {}), {} end)
    assert.are.equal(0, calls)
    assert.are.equal("unverified", v.crashes[1].status)
    assert.are.equal("unverified", v.crashes[2].status)
    assert.are.equal("executions-leaked", v.reason)
    assert.is_false(v.passed)
  end)

  it("re-verification itself stops at the cap: unverified for the rest", function()
    local seeds = {}
    for i = 1, 8 do seeds[i] = "s" .. i end
    -- 8 distinct crashes (no leaks at all in the campaign)
    local n = 0
    local r = campaign(seeds, opts({ max_iterations = 0, minimize_budget = 0 }), function() n = n + 1 return Outcome.crash("c" .. n, {}), {} end)
    assert.are.equal(8, #r.crashes)
    local calls = 0
    local v = Fuzz.verdict(r, Options.new({ ci = "off" }), function()
      calls = calls + 1
      return Outcome.crash("c1", {}, true), {} -- every re-run escapes
    end)
    assert.are.equal(4, calls)
    local statuses = {}
    for i, c in ipairs(v.crashes) do statuses[i] = c.status end
    -- the re-run returns crash "c1": the first record's signature comes back, the rest do not
    assert.are.same({ "reproduced", "flaky", "flaky", "flaky", "unverified", "unverified", "unverified", "unverified" }, statuses)
    assert.are.equal(4, v.leaked)
  end)

  it("verdict_for shares ONE tracker between the campaign and the re-verification", function()
    local runs = 0
    local function target(data)
      runs = runs + 1
      coroutine.yield() -- every execution yields: leaked
    end
    local leaks = Fuzz.new_leaks()
    local o = opts({ ci = "off", max_iterations = 50 })
    local report = Fuzz.run({ "a", "b", "c", "d", "e", "f" }, o, target, { leaks = leaks })
    assert.are.equal(4, runs) -- the campaign stopped after four leaked executions
    assert.are.equal(4, leaks.count)
    assert.is_true(report.aborted)
    local before = runs
    local v = Fuzz.verdict_for(target, report, o, "t", leaks)
    assert.are.equal(before, runs) -- the cap was already reached: nothing re-ran
    assert.are.equal("executions-leaked", v.reason)
  end)

  it("verdict_for without a tracker is seeded from the report, so the cap still spans both", function()
    local runs = 0
    local function target() runs = runs + 1 coroutine.yield() end
    local o = opts({ ci = "off", max_iterations = 50 })
    local report = Fuzz.run({ "a", "b", "c", "d", "e" }, o, target)
    assert.are.equal(4, report.leaked)
    local before = runs
    Fuzz.verdict_for(target, report, o)
    assert.are.equal(before, runs)
  end)

  it("a campaign with a real yielding target is aborted at four, with the public line fuzz-timeout", function()
    local report = Fuzz.run({ "a", "b", "c", "d", "e", "f" }, opts({ max_iterations = 50 }), function() coroutine.yield() end)
    local lines = Signature.tap_failures(report, "tgt", 1)
    local want = "not ok " .. #lines .. " - tgt [fuzz-timeout:" .. Signature.crash_signature("executions-leaked", {}) .. "]"
    assert.are.equal(want, lines[#lines])
  end)
end)

describe("verdicts, CI mode and re-verification", function()
  local function crashy(data)
    if data:sub(1, 1) == "!" then error("boom") end
  end

  local function crash_report(o)
    return Fuzz.run({ "!seed" }, o, crashy)
  end

  it("a clean report passes; the same campaign with zero iterations does not", function()
    local o = opts({ ci = "off" })
    local good = Fuzz.run({ "ok" }, o, function() end)
    local v = Fuzz.verdict_for(function() end, good, o, "t")
    assert.is_true(v.passed)
    assert.is_nil(v.reason)
    assert.are.same({}, v.lines)
    local none = Fuzz.run({ "ok" }, opts({ ci = "off", max_iterations = 0 }), function() end)
    local v0 = Fuzz.verdict_for(function() end, none, o, "t")
    assert.is_false(v0.passed)
    assert.are.equal("no-iterations", v0.reason)
  end)

  it("a crash is re-verified: reproduced, with its witness and seed only when ci is off", function()
    local o = opts({ ci = "off" })
    local report = crash_report(o)
    local v = Fuzz.verdict_for(crashy, report, o, "my/target")
    assert.is_false(v.passed)
    assert.are.equal("crash", v.reason)
    assert.are.equal(1, #v.crashes)
    assert.are.equal("string", v.crashes[1].kind)
    assert.are.equal("reproduced", v.crashes[1].status)
    assert.are.equal(42, v.seed)
    assert.are.equal("!", v.witnesses[1].input)
    assert.are.equal("not ok 1 - my/target [fuzz-crash:" .. v.crashes[1].signature .. "]", v.lines[1])
  end)

  it("ci = on and ci = unset carry NO seed and NO witness (an unset mode is CI)", function()
    for _, ci in ipairs({ "on", "unset" }) do
      local o = opts({ ci = ci })
      local report = crash_report(o)
      local v = Fuzz.verdict_for(crashy, report, o, "t")
      assert.is_nil(v.seed, ci)
      assert.is_nil(v.witnesses, ci)
      assert.is_false(v.passed)
      assert.are.equal("reproduced", v.crashes[1].status)
    end
  end)

  it("a crash that does not come back is flaky (still failing, fail closed), never passed", function()
    local o = opts({ ci = "off" })
    local report = crash_report(o)
    local v = Fuzz.verdict_for(function() end, report, o, "t") -- the target no longer crashes
    assert.are.equal("flaky", v.crashes[1].status)
    assert.is_false(v.passed)
    -- ... and one that crashes differently is flaky too
    local v2 = Fuzz.verdict(report, o, function() return Outcome.crash("other", {}), {} end, "t")
    assert.are.equal("flaky", v2.crashes[1].status)
  end)

  it("a crash whose input was dropped is unverified, and has no witness", function()
    local o = opts({ ci = "off" })
    -- the crash needs 3 bytes, so the minimal witness (3 bytes) cannot be kept under a cap of 0
    local report = Fuzz.run_outcomes({ "abcdefgh" }, o, function(d) return #d >= 3 and Outcome.crash("k", {}) or Outcome.pass(), {} end, { retained_cap = 0 })
    local ran = false
    local v = Fuzz.verdict(report, o, function() ran = true return Outcome.crash("k", {}), {} end, "t")
    assert.is_false(ran)
    assert.are.equal("unverified", v.crashes[1].status)
    assert.are.same({}, v.witnesses)
  end)

  it("the reason names the most specific failure: crash, timeout, oom, drops", function()
    local o = opts({ ci = "off" })
    local function verdict_of(exec, corpus, over)
      local r = Fuzz.run_outcomes(corpus or { "a", "b" }, over or o, exec)
      return Fuzz.verdict(r, o, function() return Outcome.pass(), {} end, "t")
    end
    assert.are.equal("timeout", verdict_of(function() return Outcome.timeout(), {} end).reason)
    assert.are.equal("oom", verdict_of(function() return Outcome.oom(), {} end).reason)
    local r = Fuzz.run_outcomes({ "aaaa" }, o, function(d) return Outcome.pass(), { #d } end, { retained_cap = 5 })
    assert.is_true(r.admissions_dropped > 0)
    assert.is_false(Report.clean(r)) -- a retention cap that cut exploration short is not "found nothing"
    assert.are.equal("admissions-dropped", Fuzz.verdict(r, o, PASS, "t").reason)
    r.admissions_dropped = 0
    r.crashes_dropped = 1
    assert.are.equal("crashes-dropped", Fuzz.verdict(r, o, PASS, "t").reason)
  end)

  it("timeouts and ooms have public lines hashed from the bare kind", function()
    local o = opts({ ci = "off" })
    local r = Fuzz.run_outcomes({ "a", "b" }, o, function(d) return d == "a" and Outcome.timeout() or Outcome.oom(), {} end)
    local lines = Signature.tap_failures(r, "t", 5)
    assert.are.same({
      "not ok 5 - t [fuzz-timeout:" .. Signature.crash_signature("timeout", {}) .. "]",
      "not ok 6 - t [fuzz-oom:" .. Signature.crash_signature("oom", {}) .. "]",
    }, lines)
  end)

  it("validates its arguments", function()
    local o = opts()
    local report = Fuzz.run({ "a" }, opts({ max_iterations = 1 }), function() end)
    assert.has_error(function() Fuzz.verdict(report, {}, PASS) end)
    assert.has_error(function() Fuzz.verdict(report, o, "exec") end)
    assert.has_error(function() Fuzz.verdict(report, o, PASS, 5) end)
    assert.has_error(function() Fuzz.run({ "a" }, {}, function() end) end)
    assert.has_error(function() Fuzz.run({ "a" }, o, "target") end)
    assert.is_true(Report.clean(Fuzz.run({ "a" }, nil, function() end)) == (Options.new().max_iterations > 0))
  end)
end)

describe("running real targets", function()
  it("a runaway target is a timeout in the campaign report, and the campaign still finishes", function()
    local report = Fuzz.run({ "a" }, opts({ max_iterations = 3, instruction_budget = 20000 }), function() while true do end end)
    assert.are.equal(1, report.timeouts)
    assert.are.equal(0, report.iterations)
    assert.is_false(Report.clean(report))
  end)

  it("a target with an input-dependent loop times out only on the inputs that make it loop", function()
    local report = Fuzz.run({ "a" }, opts({ max_iterations = 200, instruction_budget = 30000, seed = 3 }), function(data)
      if data:find("\0", 1, true) then while true do end end
    end)
    assert.are.equal(200, report.iterations)
    assert.is_true(report.timeouts >= 1)
  end)

  it("reports edges through the handle and labels the tier", function()
    local report = Fuzz.run({ "a" }, opts({ max_iterations = 30 }), function(data, h) h:edge(#data) end)
    assert.are.equal(Report.TIER_REPORTED, report.coverage_tier)
    assert.is_true(report.total_edges >= 1)
    local blind = Fuzz.run({ "a" }, opts({ max_iterations = 5 }), function() end)
    assert.are.equal(Report.TIER_BLACK_BOX, blind.coverage_tier)
  end)

  it("tick and reserve make a cooperative target a timeout or an oom", function()
    local t = Fuzz.run({ "a" }, opts({ max_iterations = 3, step_budget = 10 }), function(_, h) for _ = 1, 100 do h:tick() end end)
    assert.are.equal(1, t.timeouts)
    local m = Fuzz.run({ "a" }, opts({ max_iterations = 3, mem_budget = 100 }), function(_, h) h:reserve(101) end)
    assert.are.equal(1, m.ooms)
  end)

  it("the executor facade re-runs one input and counts edge calls", function()
    local exec, calls = Fuzz.executor(function(d, h) h:edge(1) if d == "boom" then error("x") end end)
    local out, edges = exec("fine")
    assert.are.equal("pass", out.kind)
    assert.are.same({ 1 }, edges)
    assert.are.equal("crash", (exec("boom")).kind)
    assert.are.equal(2, calls())
    assert.has_error(function() Fuzz.executor(5) end)
  end)

  it("exports the facade's helpers", function()
    assert.is_function(Fuzz.corpus_admit)
    assert.are.equal(8, Fuzz.calculate_energy(10, 4, 3, 6))
    assert.are.equal(Fuzz.Mutate.mutate_sequence, Fuzz.mutate_sequence)
    assert.are.equal(Signature.crash_signature("panic", { "eval", "step" }), Fuzz.crash_signature("panic", { "eval", "step" }))
    assert.are.equal("not ok 1 - t [fuzz-oom:7146a410]", Fuzz.format_tap_failure(1, "t", "fuzz-oom", "7146a410"))
    assert.is_function(Fuzz.minimize_crashing_input)
    assert.is_true(Fuzz.FailureIds.is_fuzz_failure_id("fuzz-leak"))
    assert.is_false(Fuzz.FailureIds.is_fuzz_failure_id(nil))
    assert.is_false(Fuzz.FailureIds.is_fuzz_failure_id(5))
    assert.are.equal(tb.Fuzz, require("coding_adventures.test_bootstrap.fuzz"))
  end)

  it("a target that patches the engine's own library functions still gets an honest report", function()
    if os.getenv("LUA_COVERAGE") then return end -- see test_fuzz_executor.lua: luacov needs string methods
    local saved = { gsub = string.gsub, sub = string.sub, concat = table.concat, format = string.format, sort = table.sort }
    local report = Fuzz.run({ "a" }, opts({ max_iterations = 5 }), function()
      string.gsub, string.sub, table.concat, string.format, table.sort = nil, nil, nil, nil, nil
      error("patched")
    end)
    string.gsub, string.sub, table.concat, string.format, table.sort = saved.gsub, saved.sub, saved.concat, saved.format, saved.sort
    assert.are.equal(1, #report.crashes)
  end)
end)
