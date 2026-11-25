--- Bounds of a campaign; every field is validated (CWE-400).
--
-- `minimize_budget` is the most target executions spent minimising ONE novel crash;
-- on top of it the whole campaign spends at most `max_iterations` minimisation
-- executions in total.  `step_budget` and `mem_budget` are the per-execution budgets
-- of `Handle.tick` and `Handle.reserve`; `instruction_budget` and `timeout_ms` are the
-- per-execution bounds the count hook enforces (VER06p 5.1).

-- Standard functions are captured at load so that a target that patches the string
-- metatable or a library table cannot make the engine raise (VER06p section 1).
local error, next, rawget, setmetatable, getmetatable = error, next, rawget, setmetatable, getmetatable

local Int = require("coding_adventures.test_bootstrap.int")
local Canon = require("coding_adventures.test_bootstrap.canon")
local PbtLimits = require("coding_adventures.test_bootstrap.limits")
local Limits = require("coding_adventures.test_bootstrap.fuzz.limits")

local Options = {}
Options.__index = Options

local CI_MODES = { on = true, off = true, unset = true }

-- name -> { default, lo, hi }
local NATIVE_FIELDS = {
  max_iterations = { 1000, 0, Limits.MAX_ITERATIONS },
  max_input_len = { 4096, 0, Limits.MAX_INPUT_BYTES },
  base_energy = { 10, 0, Limits.MAX_ITERATIONS },
  minimize_budget = { Limits.DEFAULT_MINIMIZE_BUDGET, 0, Limits.MAX_MINIMIZE_BUDGET },
  step_budget = { Limits.DEFAULT_STEP_BUDGET, 0, Limits.MAX_STEP_BUDGET },
  mem_budget = { Limits.DEFAULT_MEM_BUDGET, 0, Limits.MAX_MEM_BUDGET },
  timeout_ms = { Limits.DEFAULT_TIMEOUT_MS, 0, Limits.MAX_TIMEOUT_MS },
  instruction_budget = { Limits.DEFAULT_INSTRUCTION_BUDGET, 0, Limits.MAX_INSTRUCTION_BUDGET },
}

--- `Options.new{max_iterations = 1000, seed = 42, ..., ci = "unset"}`.  `t` is
-- author-supplied: it is read with `next`/`rawget` only, so no `__pairs` or
-- `__index` fires.  `ci` is "on", "off" or "unset"; "unset" is CI (fail closed): the
-- verdict carries no seed and no witness.
function Options.new(t)
  t = t or {}
  for k in next, t do
    if not NATIVE_FIELDS[k] and k ~= "seed" and k ~= "ci" then
      error("unknown option " .. Canon.clip(Canon.render(k), 40), 2)
    end
  end
  local o = {}
  for name, spec in next, NATIVE_FIELDS do
    local v = rawget(t, name)
    if v == nil then
      v = spec[1]
    end
    PbtLimits.check_range(v, spec[2], spec[3], name)
    o[name] = v
  end
  local seed = rawget(t, "seed")
  if seed == nil then
    seed = 42
  end
  Int.check(seed, "seed")
  o.seed = seed
  local ci = rawget(t, "ci")
  if ci == nil then
    ci = "unset"
  end
  if not CI_MODES[ci] then
    error("ci must be one of on, off, unset", 2)
  end
  o.ci = ci
  return setmetatable(o, Options)
end

--- True for an Options this module built.
function Options.is(x)
  return getmetatable(x) == Options
end

--- The seed is private (VER06 section 6): never printed.
function Options:__tostring()
  return "#<Fuzz.Options max_iterations=" .. self.max_iterations .. ">"
end

return Options
