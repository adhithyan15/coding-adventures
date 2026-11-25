--- What a target sees: coverage reporting and two deterministic budgets
-- (VER06p section 5.2).
--
-- A campaign allocates a FRESH handle for every execution and retires it the moment
-- the execution ends.  A handle is a table with NO fields: its state lives in a
-- module-private weak table keyed by the handle, so a target holding the handle
-- cannot read or rewrite a counter through it.  After retirement `edge` does nothing
-- and `tick`/`reserve` raise a retired sentinel, so a coroutine or closure that
-- retained the handle can only ever touch its own dead state, never a later
-- execution's counters or edges.  A table that is not a handle (a forgery) is
-- treated exactly like a retired one.
--
-- Sentinels are tables with no fields recognised by IDENTITY through a private weak
-- registry, never by message or content: a table a target builds itself is not one.
--
-- Honest limit: this is one VM.  `debug.getupvalue` or `debug.getregistry` reach this
-- state, and `rawset` reaches anything.  The executor cross-checks the counters
-- against the budgets after the run, which catches casual tampering only.

-- Standard functions are captured at load so that a target that patches the string
-- metatable or a library table cannot make the engine raise (VER06p section 1).
local m_type = math.type
local t_sort = table.sort
local type, error, setmetatable, next = type, error, setmetatable, next

local Int = require("coding_adventures.test_bootstrap.int")
local Prng = require("coding_adventures.test_bootstrap.prng")
local Bitmap = require("coding_adventures.test_bootstrap.fuzz.bitmap")

local Handle = {}

local STATES = setmetatable({}, { __mode = "k" }) -- handle -> state
local SENTINELS = setmetatable({}, { __mode = "k" }) -- sentinel -> { state, which }

local function new_sentinel(state, which)
  local s = {}
  SENTINELS[s] = { state = state, which = which }
  return s
end

-- The retired sentinel of a forged (unknown) handle: one per process, so a loop that
-- calls tick on a forgery mints nothing.
local FORGED = new_sentinel(nil, "retired")

-- One sentinel per (state, which), made lazily and kept: a stale coroutine spinning on
-- a retired handle cannot mint unbounded objects.
local function sentinel_for(st, which)
  local s = st.sentinels[which]
  if s == nil then
    s = new_sentinel(st, which)
    st.sentinels[which] = s
  end
  return s
end

local METHODS = {}
local META = { __index = METHODS }

function META:__tostring()
  return "#<Fuzz.Handle>"
end

--- A stable edge id derived from a name (FNV-1a): deterministic across runs and
-- machines (no object ids, no hash randomisation).
function Handle.edge_id(label)
  return Prng.fnv1a64(label)
end

--- @api private: only the executor (and tests) construct handles.  Returns the handle
-- and its private state.
function Handle.new(step_budget, mem_budget, epoch)
  local st = {
    retired = false,
    step_budget = step_budget,
    mem_budget = mem_budget,
    steps = 0,
    reserved = 0,
    edges = {},
    edge_calls = 0,
    exhausted = nil,
    which = nil,
    epoch = epoch or 0,
    sentinels = {},
  }
  local h = setmetatable({}, META)
  STATES[h] = st
  return h, st
end

--- Reports that the target executed the program point with this id (reduced mod
-- 65,536).  A no-op on a retired, retained or forged handle.
function Handle.edge(h, id)
  local st = STATES[h]
  if st == nil or st.retired then
    return nil
  end
  st.edges[Bitmap.reduce(id)] = true
  st.edge_calls = st.edge_calls + 1
  return nil
end

--- @api private: records the first breach (first wins) and returns the sentinel that
-- is, from then on, raised by every tick and reserve.
function Handle.breach(st, which)
  if st.exhausted == nil then
    st.which = which
    st.exhausted = sentinel_for(st, which)
  end
  return st.exhausted
end

-- Raises the retired sentinel of a retired handle or of a forgery.
local function raise_retired(st)
  error(st == nil and FORGED or sentinel_for(st, "retired"), 0)
end

--- Counts `n` units of work (default 1).  After `step_budget` units the execution is a
-- timeout: tick raises a sentinel and keeps raising it on every later call, whichever
-- budget broke first.  `n <= 0` counts nothing but still checks.  A loop that never
-- ticks is stopped by the executor's count hook, not here.
function Handle.tick(h, n)
  local st = STATES[h]
  if st == nil or st.retired then
    raise_retired(st)
  end
  if st.exhausted then
    error(st.exhausted, 0)
  end
  if n == nil then
    n = 1
  elseif m_type(n) ~= "integer" then
    error("n must be a native Integer", 2)
  end
  if n > 0 then
    if n > st.step_budget - st.steps then
      error(Handle.breach(st, "steps"), 0)
    end
    st.steps = st.steps + n
  end
  return nil
end

--- Declares an allocation of `n` bytes about to be made on behalf of the input.  When
-- the running total of one execution passes `mem_budget` the execution is an
-- out-of-memory outcome.  Non-positive `n` reserves nothing.  Exhaustion is shared
-- with `tick`: the first breach of either wins and every later call raises it.
function Handle.reserve(h, n)
  local st = STATES[h]
  if st == nil or st.retired then
    raise_retired(st)
  end
  if st.exhausted then
    error(st.exhausted, 0)
  end
  if not Int.is(n) then
    error("n must be an Integer", 2)
  end
  if Int.sign(n) <= 0 then
    return nil
  end
  -- a Big is positive and above any native budget
  if m_type(n) ~= "integer" or n > st.mem_budget - st.reserved then
    error(Handle.breach(st, "memory"), 0)
  end
  st.reserved = st.reserved + n
  return nil
end

METHODS.edge = Handle.edge
METHODS.tick = Handle.tick
METHODS.reserve = Handle.reserve

--- @api private: the executor ends the handle's life.
function Handle.retire(st)
  st.retired = true
end

--- @api private: "steps" or "memory", whichever budget broke first, or nil.
function Handle.exhausted_which(st)
  return st.which
end

--- @api private: the budget the COUNTERS say was broken, whatever the marker says (a
-- target that cleared the marker still leaves its counters over the budget).
function Handle.over_budget(st)
  if st.steps > st.step_budget then
    return "steps"
  elseif st.reserved > st.mem_budget then
    return "memory"
  end
  return nil
end

--- @api private: the reduced edge ids of this execution, ascending.
function Handle.edge_list(st)
  local out = {}
  for e in next, st.edges do
    out[#out + 1] = e
  end
  t_sort(out)
  return out
end

--- @api private: the sentinel of a (state, which), made on first use.
function Handle.sentinel(st, which)
  return sentinel_for(st, which)
end

--- @api private: `{state, which}` for a sentinel this module issued, else nil.  Looks
-- the object up by identity; it never reads the argument.
function Handle.sentinel_record(x)
  if type(x) ~= "table" then
    return nil
  end
  return SENTINELS[x]
end

return Handle
