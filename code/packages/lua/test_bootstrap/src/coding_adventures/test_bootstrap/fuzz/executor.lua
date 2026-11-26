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
local cocreate, coresume, costatus, coclose, cowrap = coroutine.create, coroutine.resume, coroutine.status, coroutine.close, coroutine.wrap
local corunning = coroutine.running
local coroutine_table = coroutine
local dsethook, dgethook = debug.sethook, debug.gethook
local clock, cgc = os.clock, collectgarbage
local type, error, pcall, rawequal, setmetatable, next = type, error, pcall, rawequal, setmetatable, next

local Limits = require("coding_adventures.test_bootstrap.fuzz.limits")
local Handle = require("coding_adventures.test_bootstrap.fuzz.handle")
local Outcome = require("coding_adventures.test_bootstrap.fuzz.outcome")
local Signature = require("coding_adventures.test_bootstrap.fuzz.signature")
-- captured at load: a target that rewrites Limits later changes nothing here
local limits_HOOK_COUNT, limits_POLL_FLOOR = Limits.HOOK_COUNT, Limits.POLL_FLOOR
-- captured at load: a target that rewrites Signature later changes nothing here
local signature_frames_of = Signature.frames_of
-- captured at load: a target that rewrites Outcome later changes nothing here
local outcome_pass, outcome_timeout, outcome_oom, outcome_crash = Outcome.pass, Outcome.timeout, Outcome.oom, Outcome.crash
-- captured at load: a target that rewrites Handle later changes nothing here
local handle_new, handle_intact, handle_repair, handle_retire, handle_exhausted_which, handle_over_budget, handle_edge_list, handle_breach, handle_sentinel, handle_sentinel_record = Handle.new, Handle.intact, Handle.repair, Handle.retire, Handle.exhausted_which, Handle.over_budget, Handle.edge_list, Handle.breach, Handle.sentinel, Handle.sentinel_record

local executor_TOO_MANY, executor_make_hook, executor_run, executor_make, executor_STALE, executor_YIELDED, executor_HOOK_CLEARED, executor_ENGINE_TAMPERED
local Executor = {}

executor_STALE = "stale-budget-sentinel"
executor_YIELDED = "yielded"
executor_HOOK_CLEARED = "hook-cleared"
executor_ENGINE_TAMPERED = "engine-tampered"
executor_TOO_MANY = "too-many-coroutines"

-- Is `co`'s hook EXACTLY ours: the function, the (empty) mask and the count?  Comparing the
-- function alone misses `debug.sethook(hook, "", 1 << 30)`, which keeps the function and
-- makes it fire once in a billion instructions.
local function hook_intact(co, hook)
  local f, mask, count
  if co == nil then
    f, mask, count = dgethook()
  else
    f, mask, count = dgethook(co)
  end
  return rawequal(f, hook) and mask == "" and count == limits_HOOK_COUNT
end

-- The memory baseline of an execution is `collectgarbage("count")` when it starts, and the
-- count includes garbage not yet collected: a baseline taken with a lot of garbage in the
-- heap would let the target allocate that much more before it is seen.  So when more than
-- SLOP_KB has been allocated since the last full collection this module made, the
-- execution starts with one (amortised: one collection per SLOP_KB of allocation), and the
-- baseline error is bounded by SLOP_KB.
local SLOP_KB = 8 * 1024
local last_live_kb = 0

local function baseline_kb()
  local now = cgc("count")
  if now - last_live_kb > SLOP_KB then
    cgc("collect")
    now = cgc("count")
    last_live_kb = now
  end
  return now
end

--- The count hook of one execution: a closure over its handle state, its start time and
-- its running instruction total.  `o` carries `instruction_budget`, `timeout_ms` and
-- `mem_budget`.  Exposed so the hook's branches can be exercised directly.
--
-- A hook that finds its state RETIRED (a coroutine the target created or retained
-- inherited it, and was resumed later) raises the retired sentinel there.
function executor_make_hook(st, o)
  local started = clock()
  local base_kb = baseline_kb()
  local instructions = 0
  local budget_kb = m_max(o.mem_budget, limits_POLL_FLOOR) / 1024
  local wall = o.timeout_ms / 1000
  local budget = o.instruction_budget
  -- the count at which live memory COULD reach the budget (live <= count, so nothing below
  -- it needs a full collection): after a collection that finds live growth under the budget
  -- this moves up by the headroom, so near-budget churn does not collect every poll
  local next_check_kb = base_kb + budget_kb
  local hook
  hook = function()
    if st.retired then
      error(handle_sentinel(st, "retired"), 0)
    end
    local sentinel = st.exhausted
    if sentinel then
      error(sentinel, 0) -- sticky: a target that caught it once meets it again
    end
    -- the count and mask are still ours?  (A tampered count may mean the hook never fires
    -- again: the executor's post-run check is the backstop for that.)
    -- (the main thread is never a target's thread: a direct call has nothing to police)
    local _, is_main = corunning()
    if not is_main and not hook_intact(nil, hook) then
      st.tampered = true
      error(handle_breach(st, "steps"), 0)
    end
    instructions = instructions + limits_HOOK_COUNT
    if instructions > budget or clock() - started > wall then
      error(handle_breach(st, "steps"), 0)
    end
    local count = cgc("count")
    if count > next_check_kb then
      -- the count includes garbage that has not been collected yet: judge LIVE growth
      cgc("collect")
      count = cgc("count")
      last_live_kb = count
      local growth = count - base_kb
      if growth > budget_kb then
        error(handle_breach(st, "memory"), 0)
      end
      next_check_kb = count + (budget_kb - growth)
    end
  end
  return hook
end

-- A debug hook belongs to ONE thread: `lua_newthread` copies the C trampoline but the Lua
-- function lives in a per-thread registry slot, so a coroutine the TARGET creates has no
-- hook and would run unbounded.  For the duration of an execution the executor therefore
-- replaces `coroutine.create` and `coroutine.wrap` with versions that install the SAME
-- hook closure on what they make (one closure, one shared budget), and restores the
-- previous functions afterwards.  A coroutine made through a reference the target
-- captured before the engine ran is outside this (documented, spec 5.3).
local MAX_NESTED = 4096

-- `nested` is a plain list (field `n` is its length) that holds every coroutine made through
-- the replacements STRONGLY until the executor has checked their hooks: a weak set would let
-- one that cleared its hook, finished and was collected escape the post-run check.
local function install_nested(hook, nested, st)
  local previous_create, previous_wrap = coroutine_table.create, coroutine_table.wrap
  local live = true
  local function create(f)
    if not live then
      return cocreate(f) -- captured during an execution, called after it: the real thing
    end
    if nested.n >= MAX_NESTED then
      -- COMPACT first: a finished coroutine has nothing more to do, so its hook is checked
      -- now and it is dropped (ordinary generators, one per token, finish by the thousand)
      local kept = 0
      for i = 1, nested.n do
        local c = nested[i]
        nested[i] = nil
        if costatus(c) == "dead" then
          if not hook_intact(c, hook) then
            st.tampered = true
          end
        else
          kept = kept + 1
          nested[kept] = c
        end
      end
      nested.n = kept
      if kept >= MAX_NESTED then
        -- 4,096 alive at once: ended with an honest, distinct crash (not a leak)
        st.too_many = true
        error(handle_breach(st, "steps"), 0)
      end
    end
    local co = cocreate(f)
    dsethook(co, hook, "", limits_HOOK_COUNT)
    nested.n = nested.n + 1
    nested[nested.n] = co
    return co
  end
  -- like the real `wrap`, an error closes the coroutine and propagates (the message is
  -- not prefixed with a position: it is passed through untouched)
  local function finish(co, ok, ...)
    if ok then
      return ...
    end
    pcall(coclose, co)
    error((...), 0)
  end
  local function wrap(f)
    if not live then
      return cowrap(f)
    end
    local co = create(f)
    return function(...)
      -- a hook altered inside the wrapped coroutine is recorded as it happens too
      local function after(...)
        if not hook_intact(co, hook) then
          st.tampered = true
        end
        return ...
      end
      return finish(co, after(coresume(co, ...)))
    end
  end
  coroutine_table.create, coroutine_table.wrap = create, wrap
  return function()
    live = false
    coroutine_table.create, coroutine_table.wrap = previous_create, previous_wrap
  end
end

Executor.install_nested = install_nested

-- Turns an error value into a crash.  A budget sentinel that reaches here while the
-- execution is NOT exhausted is stale (retained from another execution, or the
-- retired sentinel of a retired handle): a crash, never a timeout.
local function classify(err, co, body, escaped)
  local frames = signature_frames_of(co, body)
  if handle_sentinel_record(err) ~= nil then
    return outcome_crash(executor_STALE, frames, escaped)
  end
  return outcome_crash(type(err), frames, escaped)
end

--- Runs `target(data, handle)` once.  Returns `outcome, edges, edge_calls`; the edges are
-- those of THIS execution, read after the handle was retired.
function executor_run(target, data, o, epoch)
  if type(target) ~= "function" then
    error("target must be a function", 2)
  end
  if type(data) ~= "string" then
    error("data must be a string", 2)
  end
  local h, st = handle_new(o.step_budget, o.mem_budget, epoch)
  local hook = executor_make_hook(st, o)
  -- not a tail call: this frame marks where the target's frames end
  local function body()
    target(data, h)
  end
  local co = cocreate(body)
  dsethook(co, hook, "", limits_HOOK_COUNT)
  local nested = { n = 0 }
  local ok, err
  do
    -- restored however control leaves, not by inspection alone
    local guard <close> = setmetatable({}, { __close = install_nested(hook, nested, st) })
    ok, err = coresume(co)
  end
  -- Is the engine itself as it was loaded?  (The module table and the handle metatable are
  -- reachable by a target; the counters cross-check cannot see a replaced tick.)
  local engine_tampered = not handle_intact()
  if engine_tampered then
    handle_repair()
  end
  -- read BEFORE any close: is our hook still EXACTLY the hook of the coroutine, and of every
  -- coroutine the target made through create/wrap (still suspended ones included)?
  local tampered = st.tampered == true or not hook_intact(co, hook)
  for i = 1, nested.n do
    if not hook_intact(nested[i], hook) then
      tampered = true
    end
    nested[i] = nil -- the list is cleared once checked
  end
  local yielded = ok and costatus(co) == "suspended"
  if yielded then
    pcall(coclose, co) -- under the same hook; nothing of it can be resumed later
  end
  -- (1) retire, then (2) read: nothing a stale coroutine does can change this verdict
  handle_retire(st)
  local escaped = tampered or yielded or engine_tampered
  local which = handle_exhausted_which(st) or handle_over_budget(st)
  local outcome
  if st.too_many then
    outcome = outcome_crash(executor_TOO_MANY, {}, escaped)
  elseif which then
    outcome = (which == "steps") and outcome_timeout(escaped) or outcome_oom(escaped)
  elseif tampered then
    outcome = outcome_crash(executor_HOOK_CLEARED, {}, true)
  elseif engine_tampered then
    outcome = outcome_crash(executor_ENGINE_TAMPERED, {}, true)
  elseif yielded then
    outcome = outcome_crash(executor_YIELDED, {}, true)
  elseif not ok then
    outcome = classify(err, co, body, false)
  else
    outcome = outcome_pass()
  end
  return outcome, handle_edge_list(st), st.edge_calls
end

--- The outcome-level executor for `target`: `exec(data) -> outcome, edges`, plus a
-- function that reports how many times the target has called `edge` so far (the
-- coverage tier is evidence, not a flag).
function executor_make(target, options)
  if type(target) ~= "function" then
    error("target must be a function", 2)
  end
  local epoch, calls = 0, 0
  local function exec(data)
    epoch = epoch + 1
    local outcome, edges, n = executor_run(target, data, options, epoch)
    calls = calls + n
    return outcome, edges
  end
  return exec, function()
    return calls
  end
end

-- the public table is for callers; this module calls its own locals
Executor.make_hook = executor_make_hook
Executor.run = executor_run
Executor.make = executor_make
Executor.STALE = executor_STALE
Executor.YIELDED = executor_YIELDED
Executor.HOOK_CLEARED = executor_HOOK_CLEARED
Executor.ENGINE_TAMPERED = executor_ENGINE_TAMPERED
Executor.TOO_MANY = executor_TOO_MANY

return Executor
