--- coding_adventures.test_bootstrap.fuzz: the Lua lane of the coverage-guided fuzz
-- engine (VER06p).  Pure Lua 5.4, no dependencies, no I/O.
--
--   local Fuzz = require("coding_adventures.test_bootstrap").Fuzz
--   local report = Fuzz.run({ "\0\0" }, Fuzz.Options.new({ max_iterations = 500, seed = 7 }),
--     function(data, h)
--       h:edge(Fuzz.Handle.edge_id("entered"))
--       if data == "\255\255" then error("boom") end
--     end)
--   Fuzz.Report.clean(report)
--
-- Every execution of `target` runs in its own coroutine under a count hook with a
-- fresh `Handle` (see `Fuzz.Executor`).  Coverage is what the target reports through
-- `Handle.edge` (VER06p section 4.2: line hooks are not used in the hot path).

-- Standard functions are captured at load so that a target that patches the string
-- metatable or a library table cannot make the engine raise (VER06p section 1).
local m_max = math.max
local type, error, ipairs, rawget = type, error, ipairs, rawget

local prefix = "coding_adventures.test_bootstrap.fuzz."

local Fuzz = {
  Limits = require(prefix .. "limits"),
  FailureIds = require(prefix .. "failure_ids"),
  Mutate = require(prefix .. "mutate"),
  Bitmap = require(prefix .. "bitmap"),
  Signature = require(prefix .. "signature"),
  Outcome = require(prefix .. "outcome"),
  Options = require(prefix .. "options"),
  Report = require(prefix .. "report"),
  Handle = require(prefix .. "handle"),
  Executor = require(prefix .. "executor"),
  Engine = require(prefix .. "engine"),
  Hooks = require(prefix .. "hooks"),
  Corpus = require(prefix .. "corpus"),
}

local Engine, Executor, Options, Report, Signature = Fuzz.Engine, Fuzz.Executor, Fuzz.Options, Fuzz.Report, Fuzz.Signature
-- captured at load: a target that rewrites these tables later changes nothing here
local engine_run_outcomes, engine_guard, engine_same_crash, engine_new_leaks = Engine.run_outcomes, Engine.guard, Engine.same_crash, Engine.new_leaks
local executor_make, report_clean, signature_tap_failures, options_is, options_new = Executor.make, Report.clean, Signature.tap_failures, Options.is, Options.new

Fuzz.run_outcomes = engine_run_outcomes
Fuzz.new_leaks = engine_new_leaks
Fuzz.minimize_crashing_input = Engine.minimize_crashing_input
Fuzz.corpus_admit = Fuzz.Bitmap.corpus_admit
Fuzz.calculate_energy = Fuzz.Bitmap.calculate_energy
Fuzz.mutate_sequence = Fuzz.Mutate.mutate_sequence
Fuzz.tap_failures = signature_tap_failures
Fuzz.format_tap_failure = Signature.format_tap_failure
Fuzz.crash_signature = Signature.crash_signature

--- The outcome-level executor for `target`: `exec(data) -> outcome, edges`, one fresh
-- coroutine per call, plus a function counting the target's `edge` calls so far.  Used
-- by `run` and by `verdict`/`verdict_for` to re-verify.
function Fuzz.executor(target, options)
  return executor_make(target, options or options_new())
end

--- Runs a campaign against `target` (a function `(data, handle) -> any`).  Any error
-- escaping it is a crash.  The coverage tier is `caller-reported` if the target called
-- `Handle.edge` during the campaign and `black-box-uninstrumented` otherwise.
-- `flags.leaks` is an optional shared leak tracker (`Fuzz.new_leaks()`): pass the same
-- one to `verdict_for` and the leak cap is shared by the campaign and the
-- re-verification.
function Fuzz.run(initial_corpus, options, target, flags)
  options = options or options_new()
  if not options_is(options) then
    error("options must be Fuzz.Options", 2)
  end
  local exec, edge_calls = executor_make(target, options)
  local leaks = flags ~= nil and rawget(flags, "leaks") or nil
  local function tier()
    return edge_calls() > 0 and Report.TIER_REPORTED or Report.TIER_BLACK_BOX
  end
  return engine_run_outcomes(initial_corpus, options, exec, { tier = tier, leaks = leaks })
end

-- The honest reason a report is not a pass, most specific first.
local function reason_of(r)
  if r.leaked > 0 then
    return "executions-leaked"
  elseif #r.crashes > 0 then
    return "crash"
  elseif r.timeouts > 0 then
    return "timeout"
  elseif r.ooms > 0 then
    return "oom"
  elseif r.crashes_dropped > 0 then
    return "crashes-dropped"
  elseif r.admissions_dropped > 0 then
    return "admissions-dropped"
  end
  return "no-iterations"
end

--- The honest verdict of a report (VER06p section 6).
--
-- Before a crash is reported its minimal input is run once more through `exec`
-- (`exec(data) -> outcome, edges`, guarded by the leak cap): the crash is `reproduced`
-- if the signature comes back and `flaky` if it does not (it still fails the verdict,
-- fail closed); `unverified` when no input was kept or the leak cap was reached.
-- `passed` is true only for a clean report, so zero executions never pass.  With
-- `options.ci` of "on" or "unset" the verdict carries no seed and no witness.
--
-- `leaks` is the tracker shared with the campaign (see `verdict_for`); without one the
-- cap is seeded from `report.leaked`, so a campaign that already leaked four
-- executions re-verifies nothing.
function Fuzz.verdict(report, options, exec, target_id, leaks)
  if not options_is(options) then
    error("options must be Fuzz.Options", 2)
  end
  if type(exec) ~= "function" then
    error("exec must be a function", 2)
  end
  if target_id == nil then
    target_id = "fuzz"
  elseif type(target_id) ~= "string" then
    error("target_id must be a string", 2)
  end
  leaks = leaks or engine_new_leaks()
  leaks.count = m_max(leaks.count, report.leaked)
  local run_exec = engine_guard(exec, leaks)
  local redact = options.ci ~= "off"
  local clean = report_clean(report)

  local crashes, witnesses = {}, {}
  for i, rec in ipairs(report.crashes) do
    local status = "unverified"
    if not rec.input_dropped then
      local out = run_exec(rec.minimal_input)
      if out ~= nil then
        status = engine_same_crash(out, rec.signature) and "reproduced" or "flaky"
      end
      witnesses[#witnesses + 1] = { signature = rec.signature, input = rec.minimal_input }
    end
    crashes[i] = { signature = rec.signature, kind = rec.kind, frames = rec.frames, hits = rec.hits, status = status }
  end
  return {
    passed = clean,
    reason = (not clean) and reason_of(report) or nil,
    lines = signature_tap_failures(report, target_id, 1),
    crashes = crashes,
    iterations = report.iterations,
    timeouts = report.timeouts,
    ooms = report.ooms,
    leaked = leaks.count,
    seed = (not redact) and report.seed or nil,
    witnesses = (not redact) and witnesses or nil,
  }
end

--- `verdict` for a report that `run` produced from `target`: re-verification runs the
-- same target through an executor wired to ONE leak tracker (`leaks`, or one seeded
-- from the report), so the leak cap spans the campaign and its re-verification.
function Fuzz.verdict_for(target, report, options, target_id, leaks)
  leaks = leaks or engine_new_leaks(report.leaked)
  local exec = executor_make(target, options)
  return Fuzz.verdict(report, options, exec, target_id, leaks)
end

return Fuzz
