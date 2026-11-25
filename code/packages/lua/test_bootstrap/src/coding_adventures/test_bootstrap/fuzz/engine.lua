--- The deterministic, budgeted coverage-guided campaign (VER06 sections 5 and 9.1,
-- VER06p section 6).
--
-- A campaign is a pure function of (corpus, options, exec): triage every seed, then
-- repeatedly pick a seed, give it an energy-scaled number of mutated children, run
-- each child and either admit it (new coverage), record it (crash, timeout, OOM) or
-- drop it (redundant).  Nothing here reads a clock, a file or the environment and
-- nothing starts a coroutine: the executor does that.
--
-- EVERY execution goes through `guard`, the one choke point that (a) refuses to run
-- once the leak cap is reached and (b) counts leaked executions.  So the cap holds in
-- triage, the mutation loop, every minimisation candidate and re-verification alike.

-- Standard functions are captured at load so that a target that patches the string
-- metatable or a library table cannot make the engine raise (VER06p section 1).
local m_max, m_min, m_type = math.max, math.min, math.type
local s_sub = string.sub
local type, error, rawget, rawlen, ipairs = type, error, rawget, rawlen, ipairs

local Prng = require("coding_adventures.test_bootstrap.prng")
local Limits = require("coding_adventures.test_bootstrap.fuzz.limits")
local Mutate = require("coding_adventures.test_bootstrap.fuzz.mutate")
local Bitmap = require("coding_adventures.test_bootstrap.fuzz.bitmap")
local Signature = require("coding_adventures.test_bootstrap.fuzz.signature")
local Outcome = require("coding_adventures.test_bootstrap.fuzz.outcome")
local Options = require("coding_adventures.test_bootstrap.fuzz.options")
local Report = require("coding_adventures.test_bootstrap.fuzz.report")
-- captured at load: a target that rewrites Limits later changes nothing here
local limits_MAX_LEAKED, limits_MAX_MINIMIZE_BUDGET, limits_DEFAULT_MINIMIZE_BUDGET, limits_MAX_INPUT_BYTES, limits_MAX_INITIAL_CORPUS, limits_MAX_RETAINED_BYTES, limits_MAX_CRASHES, limits_MAX_SEQUENCE_WORK = Limits.MAX_LEAKED, Limits.MAX_MINIMIZE_BUDGET, Limits.DEFAULT_MINIMIZE_BUDGET, Limits.MAX_INPUT_BYTES, Limits.MAX_INITIAL_CORPUS, Limits.MAX_RETAINED_BYTES, Limits.MAX_CRASHES, Limits.MAX_SEQUENCE_WORK
-- captured at load: a target that rewrites Prng later changes nothing here
local prng_new_rng = Prng.new_rng
-- captured at load: a target that rewrites Report later changes nothing here
local report_crash_record, report_new, report_TIER_REPORTED, report_TIER_BLACK_BOX = Report.crash_record, Report.new, Report.TIER_REPORTED, Report.TIER_BLACK_BOX
-- captured at load: a target that rewrites Options later changes nothing here
local options_is = Options.is
-- captured at load: a target that rewrites Outcome later changes nothing here
local outcome_is, outcome_is_crash = Outcome.is, Outcome.is_crash
-- captured at load: a target that rewrites Signature later changes nothing here
local signature_crash_signature = Signature.crash_signature
-- captured at load: a target that rewrites Bitmap later changes nothing here
local bitmap_new, bitmap_calculate_energy, bitmap_admit, bitmap_count = Bitmap.new, Bitmap.calculate_energy, Bitmap.admit, Bitmap.count
-- captured at load: a target that rewrites Mutate later changes nothing here
local mutate_mutate_sequence_counted, mutate_splice = Mutate.mutate_sequence_counted, Mutate.splice

local engine_new_leaks, engine_leak_cap_reached, engine_guard, engine_same_crash, engine_minimize, engine_minimize_crashing_input, engine_run_outcomes
local Engine = {}

-- ---------------------------------------------------------------------------
-- The leak tracker and the guarded executor
-- ---------------------------------------------------------------------------

--- A tracker of leaked executions: `count` so far against `cap`.  One tracker is
-- shared by every path that runs a target (VER06p 5.4).
function engine_new_leaks(count, cap)
  return { count = count or 0, cap = cap or limits_MAX_LEAKED }
end

function engine_leak_cap_reached(leaks)
  return leaks.count >= leaks.cap
end

--- Wraps `exec(data) -> outcome, edges` so that it (1) returns nothing at all, running
-- nothing, once the leak cap is reached, (2) refuses anything that is not an Outcome
-- and (3) counts an escaped execution.  Returns `outcome, edges`, or `nil` when the
-- campaign is aborted.
function engine_guard(exec, leaks)
  return function(data)
    if engine_leak_cap_reached(leaks) then
      return nil
    end
    local out, edges = exec(data)
    if not outcome_is(out) then
      error("exec must return a Fuzz.Outcome", 2)
    end
    if out.escaped then
      leaks.count = leaks.count + 1
    end
    return out, edges
  end
end

-- ---------------------------------------------------------------------------
-- Minimisation (VER06 section 5.3)
-- ---------------------------------------------------------------------------

function engine_same_crash(outcome, sig)
  return outcome ~= nil and outcome_is_crash(outcome) and signature_crash_signature(outcome.crash_kind, outcome.frames) == sig
end

--- Shrinks `data` while it keeps the exact signature.  `test(candidate)` returns an
-- outcome, or nil when the campaign is aborted.  Returns `bytes, executions spent,
-- finished`.  EVERY candidate execution, accepted or rejected, is counted and the
-- budget is checked before each one, so no stream of rejected candidates can run past
-- it; an aborted `test` ends the minimisation with what it has.
function engine_minimize(data, sig, test, budget)
  local cur = data
  local spent = 0
  if cur == "" then
    return cur, 0, true
  end
  -- Pass 1: delete blocks, halving the block size.  Starting at max(1, len // 2) (not
  -- len // 2) is what makes the empty input reachable from a 1-byte crasher.
  local chunk = m_max(1, #cur // 2)
  while chunk > 0 do
    local offset = 0
    while offset < #cur do
      if spent >= budget then
        return cur, spent, false
      end
      local stop = offset + m_min(chunk, #cur - offset)
      local candidate = s_sub(cur, 1, offset) .. s_sub(cur, stop + 1)
      spent = spent + 1
      local out = test(candidate)
      if out == nil then
        return cur, spent, false
      end
      if engine_same_crash(out, sig) then
        cur = candidate -- same offset again: the tail slid left
      else
        offset = offset + chunk
      end
    end
    chunk = chunk // 2
  end
  -- Pass 2: zero individual non-essential bytes.
  local i = 1
  while i <= #cur do
    if s_sub(cur, i, i) ~= "\0" then
      if spent >= budget then
        return cur, spent, false
      end
      local candidate = s_sub(cur, 1, i - 1) .. "\0" .. s_sub(cur, i + 1)
      spent = spent + 1
      local out = test(candidate)
      if out == nil then
        return cur, spent, false
      end
      if engine_same_crash(out, sig) then
        cur = candidate
      end
    end
    i = i + 1
  end
  return cur, spent, true
end

--- Shrinks `data` while it keeps the EXACT crash signature (never trading one bug for
-- another).  `budget` caps executions of `test`, which returns an outcome.
function engine_minimize_crashing_input(data, sig, budget, test)
  if budget == nil then
    budget = limits_DEFAULT_MINIMIZE_BUDGET
  end
  if type(test) ~= "function" then
    error("a test function is required", 2)
  end
  if type(data) ~= "string" then
    error("data must be a string", 2)
  end
  if m_type(budget) ~= "integer" or budget < 0 or budget > limits_MAX_MINIMIZE_BUDGET then
    error("budget must be an integer in [0, " .. limits_MAX_MINIMIZE_BUDGET .. "]", 2)
  end
  if #data > limits_MAX_INPUT_BYTES then
    error("data exceeds " .. limits_MAX_INPUT_BYTES .. " bytes", 2)
  end
  return (engine_minimize(data, sig, test, budget))
end

-- ---------------------------------------------------------------------------
-- The campaign
-- ---------------------------------------------------------------------------

--- Runs a bounded coverage-guided campaign over `exec`, a function `(data) -> outcome,
-- edges` that runs the code under test and returns the edges it reported (a list of
-- Ints).  Contracts (VER06 section 9.1), each pinned by a test:
--
--   * a pure function of (corpus, options, exec): same inputs, same report;
--   * with a live corpus `iterations == max_iterations` exactly, and no mutated
--     candidate exceeds `max_input_len` (initial seeds run as given);
--   * an empty corpus is replaced by one 00 seed; seeds that crash, time out or exhaust
--     memory are never mutated, so a corpus of crashers fuzzes nothing;
--   * every execution lands in crashes, timeouts, ooms or the corpus; `clean` needs at
--     least one iteration and no failure at all;
--   * one crash record per signature; a novel crash is minimised.
--
-- Per iteration the stream draws: bounded(#corpus) for the seed; per energy slot
-- 1 + bounded(4) steps, one next_u64 sub-seed, then, if there is more than one seed,
-- bounded(4) and on 0 a donor index and two split draws.
--
-- `extras` (all optional): `tier(total) -> string` labels the coverage, `retained_cap`
-- lowers the retained-bytes cap (a test seam), `leaks` is the shared leak tracker.
function engine_run_outcomes(initial_corpus, options, exec, extras)
  if not options_is(options) then
    error("options must be Fuzz.Options", 2)
  end
  if type(exec) ~= "function" then
    error("exec must be a function", 2)
  end
  if type(initial_corpus) ~= "table" then
    error("initial_corpus must be a list of strings", 2)
  end
  local n_seeds = rawlen(initial_corpus)
  if n_seeds > limits_MAX_INITIAL_CORPUS then
    error("initial corpus exceeds " .. limits_MAX_INITIAL_CORPUS .. " seeds", 2)
  end
  for i = 1, n_seeds do
    local s = rawget(initial_corpus, i)
    if type(s) ~= "string" then
      error("initial corpus seed " .. i .. " is not a string", 2)
    end
    if #s > limits_MAX_INPUT_BYTES then
      error("initial corpus seed " .. i .. " exceeds " .. limits_MAX_INPUT_BYTES .. " bytes", 2)
    end
  end
  extras = extras or {}
  local retained_cap = rawget(extras, "retained_cap") or limits_MAX_RETAINED_BYTES
  local tier = rawget(extras, "tier")
  local leaks = rawget(extras, "leaks") or engine_new_leaks()
  local run_exec = engine_guard(exec, leaks)

  local seeds = {}
  if n_seeds == 0 then
    seeds[1] = "\0"
  else
    for i = 1, n_seeds do
      seeds[i] = rawget(initial_corpus, i)
    end
  end

  local rng = prng_new_rng(options.seed)
  local cumulative = bitmap_new()
  local corpus, crashes, by_sig = {}, {}, {}
  local timeouts, ooms, dropped, admissions_dropped = 0, 0, 0, 0
  local minimize_left = options.max_iterations
  local retained = 0

  -- Puts a passing input in the corpus iff it covered a new edge.  The first passing
  -- seed is always admitted, even with no coverage, so there is something to mutate in
  -- black-box mode.  The cap is checked BEFORE the bytes are stored.
  local function admit(data, generation, edges)
    local has_new, count_new = bitmap_admit(cumulative, edges)
    local first_seed = generation == 0 and #corpus == 0
    if not (has_new or first_seed) then
      return
    end
    if retained + #data > retained_cap then
      admissions_dropped = admissions_dropped + 1
      return
    end
    retained = retained + #data
    corpus[#corpus + 1] = { data = data, new_edges = count_new, generation = generation }
  end

  -- The minimiser's re-run: coverage is discarded and nothing is counted.
  local function probe(data)
    return (run_exec(data))
  end

  local function record_crash(out, data)
    local sig = signature_crash_signature(out.crash_kind, out.frames)
    local at = by_sig[sig]
    if at then
      local rec = crashes[at]
      rec.hits = rec.hits + 1
      if not rec.input_dropped and #data < #rec.minimal then
        retained = retained - (#rec.minimal - #data)
        rec.minimal = data
      end
      return
    end
    if #crashes >= limits_MAX_CRASHES then
      dropped = dropped + 1
      return
    end
    local allowance = m_min(options.minimize_budget, minimize_left)
    local kept, spent, finished = engine_minimize(data, sig, probe, allowance)
    minimize_left = minimize_left - spent
    local dropped_input = false
    if retained + #kept > retained_cap then
      kept = ""
      dropped_input = true
    else
      retained = retained + #kept
    end
    crashes[#crashes + 1] = {
      signature = sig, kind = out.crash_kind, frames = out.frames, minimal = kept, hits = 1,
      minimized = finished and not dropped_input, input_dropped = dropped_input,
    }
    by_sig[sig] = #crashes
  end

  -- Puts one non-passing execution in its bucket.
  local function file(out, data)
    if out.kind == "timeout" then
      timeouts = timeouts + 1
    elseif out.kind == "oom" then
      ooms = ooms + 1
    elseif out.kind == "crash" then
      record_crash(out, data)
    end
  end

  -- 1. Triage: run every seed once.
  for _, data in ipairs(seeds) do
    local out, edges = run_exec(data)
    if out == nil then
      break -- the leak cap: nothing more runs
    end
    if out.kind == "pass" then
      admit(data, 0, edges)
    else
      file(out, data)
    end
  end

  -- 2. The mutation loop.
  local iterations = 0
  local stopped = engine_leak_cap_reached(leaks)
  while not stopped and iterations < options.max_iterations and #corpus > 0 do
    local seed = corpus[rng:bounded(#corpus) + 1]
    local energy = bitmap_calculate_energy(options.base_energy, #seed.data, seed.new_edges, bitmap_count(cumulative))
    local slot = 0
    while slot < energy and iterations < options.max_iterations do
      slot = slot + 1
      local steps = 1 + rng:bounded(4)
      local sub = rng:next_u64()
      local candidate = mutate_mutate_sequence_counted(seed.data, sub, steps, limits_MAX_INPUT_BYTES, limits_MAX_SEQUENCE_WORK)
      candidate = candidate or seed.data -- cannot happen at these sizes; fail safe
      if #corpus > 1 and rng:bounded(4) == 0 then
        local donor = corpus[rng:bounded(#corpus) + 1].data
        local t_split = rng:bounded(#candidate + 1)
        local d_split = rng:bounded(#donor + 1)
        candidate = mutate_splice(candidate, donor, t_split, d_split)
      end
      candidate = s_sub(candidate, 1, options.max_input_len)
      local out, edges = run_exec(candidate)
      if out == nil then
        stopped = true
        break
      end
      iterations = iterations + 1
      if out.kind == "pass" then
        admit(candidate, seed.generation + 1, edges)
      else
        file(out, candidate)
      end
    end
  end

  local records, kept = {}, {}
  for i, c in ipairs(crashes) do
    records[i] = report_crash_record({
      signature = c.signature, kind = c.kind, frames = c.frames, minimal_input = c.minimal, hits = c.hits,
      minimized = c.minimized, input_dropped = c.input_dropped,
    })
  end
  for i, s in ipairs(corpus) do
    kept[i] = s.data
  end
  local total = bitmap_count(cumulative)
  local label
  if tier then
    label = tier(total)
  else
    label = total > 0 and report_TIER_REPORTED or report_TIER_BLACK_BOX
  end
  return report_new({
    iterations = iterations, admitted_seeds = #corpus, total_edges = total, crashes = records,
    timeouts = timeouts, ooms = ooms, crashes_dropped = dropped, admissions_dropped = admissions_dropped,
    leaked = leaks.count, aborted = engine_leak_cap_reached(leaks), coverage_tier = label,
    seed = options.seed, corpus = kept,
  })
end

-- the public table is for callers; this module calls its own locals
Engine.new_leaks = engine_new_leaks
Engine.leak_cap_reached = engine_leak_cap_reached
Engine.guard = engine_guard
Engine.same_crash = engine_same_crash
Engine.minimize = engine_minimize
Engine.minimize_crashing_input = engine_minimize_crashing_input
Engine.run_outcomes = engine_run_outcomes

return Engine
