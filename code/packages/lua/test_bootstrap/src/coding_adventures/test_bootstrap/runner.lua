--- The honest runner (VER05 7.1, VER05e 5, VER05o section 6).
--
--   * zero cases evaluated is `property-exhausted`, never a pass;
--   * a generator error (any value thrown while sampling) is `property-exhausted`;
--   * a property that raises (ANY value, `nil` included) is `property-panic`;
--   * a property holds ONLY when it returns exactly `true`: `false`, `nil`, `1` and
--     "forgot to return" all falsify, so a property cannot pass vacuously;
--   * the reported minimal witness is RE-VERIFIED: if the shrinker raises, returns
--     something that no longer fails, a value the property raises on, or a
--     nonsensical step count, the original candidate is reported with 0 steps;
--   * redacted mode never emits the witness or the seed; an UNSET mode is redacted
--     (fail closed);
--   * the call always returns a verdict (invalid arguments raise instead).
--
-- Errors are classified by `pcall` and `type()` alone (see safe.lua): the engine
-- never calls `tostring` on, or reads a field of, an error value.
--
-- There is no wall-clock timeout: `property-timeout` is registered and never
-- produced, so a property that loops forever hangs the run.

-- Standard functions are captured at load so that a property that patches the
-- string metatable or a library table cannot make the engine raise (VER05o 4).
local m_max, m_type, s_format, s_lower, s_match = math.max, math.type, string.format, string.lower, string.match
local type, pcall, error, next, setmetatable, getmetatable, pairs = type, pcall, error, next, setmetatable, getmetatable, pairs

local Int = require("coding_adventures.test_bootstrap.int")
local Prng = require("coding_adventures.test_bootstrap.prng")
local Limits = require("coding_adventures.test_bootstrap.limits")
local FailureIds = require("coding_adventures.test_bootstrap.failure_ids")
local Budget = require("coding_adventures.test_bootstrap.budget")
local Gen = require("coding_adventures.test_bootstrap.gen")
local Canon = require("coding_adventures.test_bootstrap.canon")
local Safe = require("coding_adventures.test_bootstrap.safe")

local Runner = {}

-- -------------------------------------------------------------------------
-- Options
-- -------------------------------------------------------------------------

local Options = {}
Options.__index = Options
Runner.Options = Options

local CI_MODES = { on = true, off = true, unset = true }
local OPTION_KEYS = { num_tests = true, max_shrink_steps = true, ci = true, parent = true }

--- `{num_tests = 100, max_shrink_steps = 1000, ci = "unset", parent = nil}`.
-- `ci` is "on", "off" or "unset"; "unset" is treated as CI: the engine fails
-- closed and redacts.  The package never reads the environment;
-- `Options.ci_from_value` is a pure helper (pass `os.getenv("CI")`).  `parent` is
-- the budget scope of an enclosing check (spec 5.1): a check nested in a property
-- passes the scope the property was given.
function Options.new(t)
  t = t or {}
  -- `t` is author-supplied: read it with next/rawget only, so no __pairs/__index fires
  for k in next, t do
    if not OPTION_KEYS[k] then
      error("unknown option " .. Canon.clip(Canon.render(k), 40), 2)
    end
  end
  local ci = rawget(t, "ci")
  if ci == nil then
    ci = "unset"
  end
  if not CI_MODES[ci] then
    error("ci must be one of on, off, unset", 2)
  end
  local num_tests = rawget(t, "num_tests")
  if num_tests == nil then
    num_tests = 100
  end
  local max_steps = rawget(t, "max_shrink_steps")
  if max_steps == nil then
    max_steps = 1000
  end
  local parent = rawget(t, "parent")
  Limits.check_range(num_tests, 0, Limits.MAX_CASES, "num_tests")
  Limits.check_range(max_steps, 0, Limits.MAX_SHRINK_STEPS, "max_shrink_steps")
  if parent ~= nil and not Budget.is_scope(parent) then
    error("parent must be a budget scope", 2)
  end
  return setmetatable({ num_tests = num_tests, max_shrink_steps = max_steps, ci = ci, parent = parent }, Options)
end

--- `CI` set to anything except empty, "0" or "false" (case-insensitive, trimmed)
-- means CI.  `value` is the environment value or nil.
function Options.ci_from_value(value)
  if value == nil then
    return "off"
  end
  if type(value) ~= "string" then
    error("CI value must be a string or nil", 2)
  end
  local v = s_lower(s_match(value, "^%s*(.-)%s*$"))
  if v == "" or v == "0" or v == "false" then
    return "off"
  end
  return "on"
end

-- -------------------------------------------------------------------------
-- Work estimate
-- -------------------------------------------------------------------------

local WALK_CAP = 10000

-- Elements of a string or table; ONE visit allowance is shared by the whole walk.
-- Reads with `next` only (no metamethod runs).
local function shallow(value, allowance)
  local ty = type(value)
  if ty == "string" then
    return #value
  elseif ty == "table" and not Int.is_big(value) then
    local n = 0
    for _ in next, value do
      if allowance[1] <= 0 then
        break
      end
      allowance[1] = allowance[1] - 1
      n = n + 1
    end
    return n
  end
  return 0
end

--- An approximate size of a sampled value in elements (work): a string counts its
-- bytes, a table its entries plus, one level deeper, the size of each string or
-- table child.  Approximate by design; a hostile value is bounded by the allowance.
function Runner.size_of(value)
  local allowance = { WALK_CAP }
  local n = shallow(value, allowance)
  if n == 0 then
    return 0
  end
  local total = n
  if type(value) == "table" and not Int.is_big(value) then
    for _, child in next, value do
      if allowance[1] <= 0 then
        break
      end
      allowance[1] = allowance[1] - 1
      total = total + shallow(child, allowance)
    end
  end
  return total
end

-- -------------------------------------------------------------------------
-- Deep copy (so a defective shrinker cannot damage the original witness)
-- -------------------------------------------------------------------------

-- A structural copy, bounded; `state.failed` when a bound is crossed (then nothing
-- is shrunk: fail closed).  Reads with `next`, copies the metatable by identity
-- without invoking anything.
local function clone(value, memo, state, depth)
  if Int.is_big(value) then
    return Int.add(value, 0) -- a fresh Big, never shared with the original
  end
  if type(value) ~= "table" or Canon.is_text(value) then
    return value
  end
  if memo[value] then
    return memo[value]
  end
  state.nodes = state.nodes + 1
  if state.nodes > 100000 or depth > 64 then
    state.failed = true
    return value
  end
  local copy = {}
  memo[value] = copy
  for k, v in next, value do
    copy[clone(k, memo, state, depth + 1)] = clone(v, memo, state, depth + 1)
  end
  local mt = getmetatable(value)
  if type(mt) == "table" then
    setmetatable(copy, mt)
  end
  return copy
end

-- -------------------------------------------------------------------------
-- Verdicts
-- -------------------------------------------------------------------------

local function verdict(passed, failure_id, message, cases_run, iteration, extra)
  extra = extra or {}
  return {
    passed = passed,
    failed = not passed,
    failure_id = failure_id,
    message = message,
    cases_run = cases_run,
    iteration = iteration,
    shrink_steps = extra.shrink_steps or 0,
    seed = extra.seed,
    counterexample = extra.counterexample,
    minimal = extra.minimal,
    digest = extra.digest,
  }
end

-- The enclosing shrink budget was spent: exhausted, never a pass.
local function spent(iteration)
  return verdict(false, FailureIds.EXHAUSTED, "the enclosing shrink budget was spent at iteration " .. iteration, m_max(0, iteration - 1), iteration)
end

local function exhausted(err, iteration, redact)
  local text
  if Gen.is_exhausted_error(err) then
    text = "filter precondition rejected all candidates at iteration " .. iteration
  elseif redact then
    text = s_format("generator error at iteration %d (%s)", iteration, Safe.redacted(err))
  else
    text = s_format("generator error at iteration %d: %s", iteration, Safe.describe(err))
  end
  return verdict(false, FailureIds.EXHAUSTED, text, iteration - 1, iteration)
end

local function panic(seed, iteration, err, redact)
  if redact then
    return verdict(false, FailureIds.PANIC, s_format("property evaluation raised at iteration %d (%s)", iteration, Safe.redacted(err)), iteration, iteration)
  end
  return verdict(false, FailureIds.PANIC, s_format("property evaluation raised at iteration %d: %s", iteration, Safe.describe(err)), iteration, iteration, { seed = seed })
end

-- `prop(value, scope)` holds only on exactly `true`.  A candidate the property
-- raises on is not a counterexample to the claim; it is a different failure, so it
-- never counts as falsifying (and is never accepted as "smaller").
local function falsifies_with(prop, v, scope)
  local ok, result = pcall(prop, v, scope)
  return ok and result ~= true
end

local function falsified(seed, iteration, candidate, gen, prop, shrink, opts, redact, check_scope)
  local initial_text = redact and "" or Canon.render(candidate)
  local minimal = candidate
  local steps = 0

  -- Untrusted shrinker: fall back to the original.  The shrinker runs on a COPY of
  -- the candidate, in a scope that is a child of the check scope (itself a child of
  -- any enclosing shrink): every evaluation is charged to the whole chain.
  local state = { nodes = 0, failed = false }
  local copy = clone(candidate, {}, state, 0)
  if not state.failed then
    local ok, value, used = pcall(function()
      local scope <close> = Budget.open_child(check_scope, opts.max_shrink_steps)
      local budgeted = scope:charged(function(v)
        return falsifies_with(prop, v, scope)
      end)
      if shrink then
        return shrink(copy, budgeted, opts.max_shrink_steps, scope)
      end
      return gen:shrink_nested(copy, budgeted, opts.max_shrink_steps, scope)
    end)
    -- Re-verify: the witness we report must really fail the property.  One attempt
    -- is charged to the check scope; if the chain refuses it the shrunk value is not
    -- adopted.
    if ok and m_type(used) == "integer" and used >= 0 and used <= opts.max_shrink_steps and check_scope:charge(1) and falsifies_with(prop, value, check_scope) then
      minimal = value
      steps = used
    end
  end

  if redact then
    local preimage = "iter:" .. iteration .. ";steps:" .. steps .. ";shrunk:" .. Canon.render(minimal)
    local digest = s_format("%016x", Prng.fnv1a64_word(preimage))
    return verdict(
      false,
      FailureIds.COUNTEREXAMPLE,
      s_format("property falsified after %d iterations (shrunk in %d steps, repro-fnv: %s)", iteration, steps, digest),
      iteration,
      iteration,
      { shrink_steps = steps, digest = digest }
    )
  end

  return verdict(
    false,
    FailureIds.COUNTEREXAMPLE,
    s_format(
      "property falsified at iteration %d (seed=%s): initial=%s, minimal=%s after %d shrink steps",
      iteration,
      Int.tostring(seed),
      initial_text,
      Canon.render(minimal),
      steps
    ),
    iteration,
    iteration,
    { shrink_steps = steps, seed = seed, counterexample = candidate, minimal = minimal }
  )
end

local run_check

--- Runs `prop(candidate, scope)` over `options.num_tests` candidates from `seed`.
-- The property returns exactly `true` when it holds.  `shrink` (optional) is
-- `function(candidate, falsifies, max_steps, scope) -> minimal, steps` and
-- replaces the generator's shrinker, so a defective shrinker can be tested.
function Runner.check(seed, gen, prop, options, shrink)
  if type(prop) ~= "function" then
    error("a property function is required", 2)
  end
  if not Gen.is(gen) then
    error("gen must be a Gen", 2)
  end
  Int.check(seed, "seed")
  if shrink ~= nil and type(shrink) ~= "function" then
    error("shrink must be a function", 2)
  end
  local opts = options or Options.new()
  if getmetatable(opts) ~= Options then
    error("options must be Options", 2)
  end
  -- Arguments are valid.  From here on the call ALWAYS returns a verdict: anything the
  -- engine itself cannot finish (a hostile environment, an engine bug) is an honest
  -- property-panic that names only the error's type, never a thrown error and never a pass.
  local done, result = pcall(run_check, seed, gen, prop, opts, shrink)
  if done then
    return result
  end
  return verdict(false, FailureIds.PANIC, "the engine could not complete the check (" .. type(result) .. ")", 0, 0)
end

function run_check(seed, gen, prop, opts, shrink)
  local seed_int = Int.from_u64(Int.to_u64(seed))
  local redact = opts.ci ~= "off"
  if opts.num_tests == 0 then
    return verdict(false, FailureIds.EXHAUSTED, "no cases were evaluated (num_tests is 0); a property that never ran is not a pass", 0, 0)
  end

  -- A check nested in a property (a parent scope was passed) pays one attempt at
  -- entry, one per case after the first and the size of every sampled collection as
  -- work, all charged up the chain: neither nesting depth, branching nor num_tests
  -- multiplies the outermost budget.  A starved nested check reports
  -- property-exhausted without evaluating anything.  Nesting is capped.
  local parent = opts.parent
  local nested = parent ~= nil
  if nested then
    if parent:checks() >= Budget.MAX_CHECK_NESTING or parent:depth() + 1 >= Budget.MAX_DEPTH then
      parent:mark_refused()
      return verdict(false, FailureIds.EXHAUSTED, "checks are nested too deeply; nothing was evaluated", 0, 0)
    end
    if not parent:charge(1) then
      return spent(0)
    end
  end

  -- The WHOLE check runs in one scope (case loop included).
  local scope <close> = Budget.open_check(parent, opts.num_tests, opts.max_shrink_steps)
  local rng = Prng.new_rng(seed_int)
  for iteration = 1, opts.num_tests do
    if nested and iteration > 1 and not scope:charge(1) then
      return spent(iteration)
    end

    local sampled, candidate = pcall(gen.sample, gen, rng, scope)
    if not sampled then
      return exhausted(candidate, iteration, redact)
    end

    if nested and not scope:charge_copy(Runner.size_of(candidate)) then
      return spent(iteration)
    end

    local ran, result = pcall(prop, candidate, scope)
    if not ran then
      return panic(seed_int, iteration, result, redact)
    end
    if result ~= true then
      return falsified(seed_int, iteration, candidate, gen, prop, shrink, opts, redact, scope)
    end
  end

  if not nested and scope:refused() then
    -- A nested check or generator inside this run was starved, so the property was
    -- not fully evaluated: never report that as a pass.
    return verdict(false, FailureIds.EXHAUSTED, "a nested check was starved by the shrink budget; the property was not fully evaluated", opts.num_tests, opts.num_tests)
  end

  return verdict(true, nil, nil, opts.num_tests, 0)
end

return Runner
