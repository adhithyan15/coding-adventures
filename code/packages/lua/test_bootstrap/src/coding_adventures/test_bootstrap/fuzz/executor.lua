--- One target execution in its OWN coroutine under a count hook (VER06p section 5).
--
-- Why a coroutine per execution.  Lua cannot kill a running function; the only things
-- that interrupt Lua code are an ERROR RAISED FROM A DEBUG HOOK and a YIELD.  So each
-- execution runs `target(data, handle)` in a fresh coroutine with a count hook
-- installed on that coroutine alone.  The hook is the step, wall and memory budget:
--
--   * every call adds HOOK_COUNT (1,000) VM instructions to the execution's total and
--     past `instruction_budget` the execution is a TIMEOUT;
--   * every call reads `os.clock()` and past `timeout_ms` the execution is a TIMEOUT;
--   * every call compares the growth of `collectgarbage("count")` with `mem_budget`
--     (after a full collection, because the count includes garbage) and past it the
--     execution is an OOM.  Best effort: see section 5.5 of the spec.
--
-- A breach is recorded on the handle state (the first breach wins) and its sentinel is
-- raised from the hook.  A target that catches the sentinel is not rescued: the
-- verdict is judged AFTER the handle is retired, from the sticky marker and from the
-- counters, so a swallowed breach is still a timeout or an oom.
--
-- What the executor does NOT do, and what it detects, is in section 5.3 of the spec: a
-- target that clears the hook or yields out of the coroutine is caught after the fact
-- (crash kinds `hook-cleared` and `yielded`, both counted as LEAKED); an unbounded
-- `pcall` loop or a single long C call cannot be stopped in-process.
--
-- Errors are classified by `type()` of the error value and nothing else: the engine
-- never calls `tostring`, never reads a field and never compares a target value with
-- a metamethod-aware operator.

-- Standard functions are captured at load so that a target that patches the string
-- metatable or a library table cannot make the engine raise (VER06p section 1).
local m_max = math.max
local cocreate, coresume, costatus, coclose = coroutine.create, coroutine.resume, coroutine.status, coroutine.close
local dsethook, dgethook = debug.sethook, debug.gethook
local clock, cgc = os.clock, collectgarbage
local type, error, pcall, rawequal = type, error, pcall, rawequal

local Limits = require("coding_adventures.test_bootstrap.fuzz.limits")
local Handle = require("coding_adventures.test_bootstrap.fuzz.handle")
local Outcome = require("coding_adventures.test_bootstrap.fuzz.outcome")
local Signature = require("coding_adventures.test_bootstrap.fuzz.signature")

local Executor = {}

Executor.STALE = "stale-budget-sentinel"
Executor.YIELDED = "yielded"
Executor.HOOK_CLEARED = "hook-cleared"

--- The count hook of one execution: a closure over its handle state, its start time and
-- its running instruction total.  `o` carries `instruction_budget`, `timeout_ms` and
-- `mem_budget`.  Exposed so the hook's branches can be exercised directly.
--
-- A hook that finds its state RETIRED (a coroutine the target created or retained
-- inherited it, and was resumed later) raises the retired sentinel there.
function Executor.make_hook(st, o)
  local started = clock()
  local base_kb = cgc("count")
  local instructions = 0
  local budget_kb = m_max(o.mem_budget, Limits.POLL_FLOOR) / 1024
  local wall = o.timeout_ms / 1000
  local budget = o.instruction_budget
  return function()
    if st.retired then
      error(Handle.sentinel(st, "retired"), 0)
    end
    local sentinel = st.exhausted
    if sentinel then
      error(sentinel, 0) -- sticky: a target that caught it once meets it again
    end
    instructions = instructions + Limits.HOOK_COUNT
    if instructions > budget or clock() - started > wall then
      error(Handle.breach(st, "steps"), 0)
    end
    if cgc("count") - base_kb > budget_kb then
      -- the count includes garbage that has not been collected yet: judge LIVE growth
      cgc("collect")
      if cgc("count") - base_kb > budget_kb then
        error(Handle.breach(st, "memory"), 0)
      end
    end
  end
end

-- Turns an error value into a crash.  A budget sentinel that reaches here while the
-- execution is NOT exhausted is stale (retained from another execution, or the
-- retired sentinel of a retired handle): a crash, never a timeout.
local function classify(err, co, body, escaped)
  local frames = Signature.frames_of(co, body)
  if Handle.sentinel_record(err) ~= nil then
    return Outcome.crash(Executor.STALE, frames, escaped)
  end
  return Outcome.crash(type(err), frames, escaped)
end

--- Runs `target(data, handle)` once.  Returns `outcome, edges, edge_calls`; the edges are
-- those of THIS execution, read after the handle was retired.
function Executor.run(target, data, o, epoch)
  if type(target) ~= "function" then
    error("target must be a function", 2)
  end
  if type(data) ~= "string" then
    error("data must be a string", 2)
  end
  local h, st = Handle.new(o.step_budget, o.mem_budget, epoch)
  local hook = Executor.make_hook(st, o)
  -- not a tail call: this frame marks where the target's frames end
  local function body()
    target(data, h)
  end
  local co = cocreate(body)
  dsethook(co, hook, "", Limits.HOOK_COUNT)
  local ok, err = coresume(co)
  -- read BEFORE any close: is our hook still the coroutine's hook?
  local tampered = not rawequal(dgethook(co), hook)
  local yielded = ok and costatus(co) == "suspended"
  if yielded then
    pcall(coclose, co) -- under the same hook; nothing of it can be resumed later
  end
  -- (1) retire, then (2) read: nothing a stale coroutine does can change this verdict
  Handle.retire(st)
  local escaped = tampered or yielded
  local which = Handle.exhausted_which(st) or Handle.over_budget(st)
  local outcome
  if which then
    outcome = (which == "steps") and Outcome.timeout(escaped) or Outcome.oom(escaped)
  elseif tampered then
    outcome = Outcome.crash(Executor.HOOK_CLEARED, {}, true)
  elseif yielded then
    outcome = Outcome.crash(Executor.YIELDED, {}, true)
  elseif not ok then
    outcome = classify(err, co, body, false)
  else
    outcome = Outcome.pass()
  end
  return outcome, Handle.edge_list(st), st.edge_calls
end

--- The outcome-level executor for `target`: `exec(data) -> outcome, edges`, plus a
-- function that reports how many times the target has called `edge` so far (the
-- coverage tier is evidence, not a flag).
function Executor.make(target, options)
  if type(target) ~= "function" then
    error("target must be a function", 2)
  end
  local epoch, calls = 0, 0
  local function exec(data)
    epoch = epoch + 1
    local outcome, edges, n = Executor.run(target, data, options, epoch)
    calls = calls + n
    return outcome, edges
  end
  return exec, function()
    return calls
  end
end

return Executor
