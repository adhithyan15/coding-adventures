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
-- captured at load: a target that rewrites Prng later changes nothing here
local prng_fnv1a64 = Prng.fnv1a64
-- captured at load: a target that rewrites Bitmap later changes nothing here
local bitmap_reduce = Bitmap.reduce
-- captured at load: a target that rewrites Int later changes nothing here
local int_is, int_sign = Int.is, Int.sign

local handle_new, handle_edge, handle_tick, handle_reserve, handle_retire, handle_exhausted_which, handle_over_budget, handle_edge_list, handle_sentinel, handle_sentinel_record, handle_intact, handle_repair, handle_edge_id
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

-- The methods live in a table only the `__index` closure below can reach, and the shared
-- metatable is locked (`__metatable = false`): `getmetatable(h).__index = ...` is an error and
-- there is no table on the handle to write a replacement tick into.  The executor still
-- re-checks everything after each execution (`handle_intact`) and restores it (`repair`).
local METHODS = {}
local function index(_, key)
  return METHODS[key]
end
local META = { __index = index, __metatable = false }

function META:__tostring()
  return "#<Fuzz.Handle>"
end

--- A stable edge id derived from a name (FNV-1a): deterministic across runs and
-- machines (no object ids, no hash randomisation).
function handle_edge_id(label)
  return prng_fnv1a64(label)
end

--- @api private: only the executor (and tests) construct handles.  Returns the handle
-- and its private state.
function handle_new(step_budget, mem_budget, epoch)
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
function handle_edge(h, id)
  local st = STATES[h]
  if st == nil or st.retired then
    return nil
  end
  st.edges[bitmap_reduce(id)] = true
  st.edge_calls = st.edge_calls + 1
  return nil
end

--- @api private: records the first breach (first wins) and returns the sentinel that
-- is, from then on, raised by every tick and reserve.
local function breach(st, which)
  if st.exhausted == nil then
    st.which = which
    st.exhausted = sentinel_for(st, which)
  end
  return st.exhausted
end
Handle.breach = breach

-- Raises the retired sentinel of a retired handle or of a forgery.
local function raise_retired(st)
  error(st == nil and FORGED or sentinel_for(st, "retired"), 0)
end

--- Counts `n` units of work (default 1).  After `step_budget` units the execution is a
-- timeout: tick raises a sentinel and keeps raising it on every later call, whichever
-- budget broke first.  `n <= 0` counts nothing but still checks.  A loop that never
-- ticks is stopped by the executor's count hook, not here.
function handle_tick(h, n)
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
      error(breach(st, "steps"), 0)
    end
    st.steps = st.steps + n
  end
  return nil
end

--- Declares an allocation of `n` bytes about to be made on behalf of the input.  When
-- the running total of one execution passes `mem_budget` the execution is an
-- out-of-memory outcome.  Non-positive `n` reserves nothing.  Exhaustion is shared
-- with `tick`: the first breach of either wins and every later call raises it.
function handle_reserve(h, n)
  local st = STATES[h]
  if st == nil or st.retired then
    raise_retired(st)
  end
  if st.exhausted then
    error(st.exhausted, 0)
  end
  if not int_is(n) then
    error("n must be an Integer", 2)
  end
  if int_sign(n) <= 0 then
    return nil
  end
  -- a Big is positive and above any native budget
  if m_type(n) ~= "integer" or n > st.mem_budget - st.reserved then
    error(breach(st, "memory"), 0)
  end
  st.reserved = st.reserved + n
  return nil
end

METHODS.edge = handle_edge
METHODS.tick = handle_tick
METHODS.reserve = handle_reserve

--- @api private: the executor ends the handle's life.
function handle_retire(st)
  st.retired = true
end

--- @api private: "steps" or "memory", whichever budget broke first, or nil.
function handle_exhausted_which(st)
  return st.which
end

--- @api private: the budget the COUNTERS say was broken, whatever the marker says (a
-- target that cleared the marker still leaves its counters over the budget).
function handle_over_budget(st)
  if st.steps > st.step_budget then
    return "steps"
  elseif st.reserved > st.mem_budget then
    return "memory"
  end
  return nil
end

--- @api private: the reduced edge ids of this execution, ascending.
function handle_edge_list(st)
  local out = {}
  for e in next, st.edges do
    out[#out + 1] = e
  end
  t_sort(out)
  return out
end

--- @api private: the sentinel of a (state, which), made on first use.
function handle_sentinel(st, which)
  return sentinel_for(st, which)
end

--- @api private: `{state, which}` for a sentinel this module issued, else nil.  Looks
-- the object up by identity; it never reads the argument.
function handle_sentinel_record(x)
  if type(x) ~= "table" then
    return nil
  end
  return SENTINELS[x]
end

-- What the module and the metatable looked like when they were loaded.
local SNAPSHOT = {}
local META_SNAPSHOT = {}
for k, v in next, META do
  META_SNAPSHOT[k] = v
end
local METHOD_SNAPSHOT = { edge = METHODS.edge, tick = METHODS.tick, reserve = METHODS.reserve }

--- @api private: are the module's functions, the metatable and the method table exactly as
-- loaded?  A target that rewrote any of them (through the module table or `debug`) made the
-- counters cross-check blind to it.
function handle_intact()
  -- the FULL key set of the metatable, not just the two keys that matter today
  for k, v in next, META do
    if META_SNAPSHOT[k] ~= v then
      return false
    end
  end
  for k in next, META_SNAPSHOT do
    if META[k] == nil then
      return false
    end
  end
  for name, fn in next, SNAPSHOT do
    if Handle[name] ~= fn then
      return false
    end
  end
  for name, fn in next, METHOD_SNAPSHOT do
    if METHODS[name] ~= fn then
      return false
    end
  end
  for name in next, Handle do
    if SNAPSHOT[name] == nil then
      return false
    end
  end
  for name in next, METHODS do
    if METHOD_SNAPSHOT[name] == nil then
      return false
    end
  end
  return true
end

--- @api private: restores everything `intact` checks.
function handle_repair()
  for k in next, META do
    META[k] = nil
  end
  for k, v in next, META_SNAPSHOT do
    META[k] = v
  end
  for name in next, Handle do
    if SNAPSHOT[name] == nil then
      Handle[name] = nil
    end
  end
  for name, fn in next, SNAPSHOT do
    Handle[name] = fn
  end
  for name in next, METHODS do
    if METHOD_SNAPSHOT[name] == nil then
      METHODS[name] = nil
    end
  end
  for name, fn in next, METHOD_SNAPSHOT do
    METHODS[name] = fn
  end
end

-- the public table is for callers; this module calls its own locals
Handle.new = handle_new
Handle.edge = handle_edge
Handle.tick = handle_tick
Handle.reserve = handle_reserve
Handle.retire = handle_retire
Handle.exhausted_which = handle_exhausted_which
Handle.over_budget = handle_over_budget
Handle.edge_list = handle_edge_list
Handle.sentinel = handle_sentinel
Handle.sentinel_record = handle_sentinel_record
Handle.intact = handle_intact
Handle.repair = handle_repair
Handle.edge_id = handle_edge_id


-- taken last, so `intact` and `repair` are part of what is protected
for name, fn in next, Handle do
  SNAPSHOT[name] = fn
end


return Handle
