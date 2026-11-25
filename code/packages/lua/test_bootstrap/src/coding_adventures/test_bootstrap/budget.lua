--- The budget of a shrink, as a TREE of scopes (VER05o section 5, ported from
-- the Java engine's VER05e 4.1 via the Haskell lane's VER05n 5).
--
-- Why a tree?  A property may itself call `Runner.check`, a custom shrinker may
-- call `Gen:shrink`, a filter predicate may do either.  If each of those got a
-- fresh budget the work would multiply with the nesting depth.  Here every public
-- entry point that runs user code opens a CHILD of the scope it is given with its
-- own local cap, and a charge walks from the node up to the outermost root: every
-- node on the path must have room, then every counter is incremented, all or
-- nothing.  Whatever the nesting, the total work of one top-level check is capped
-- by the outermost root.
--
-- Two things are charged: ATTEMPTS (one per property evaluation) and WORK
-- (elements copied, at least 1 per charge).
--
-- THE CARRIER IS AN EXPLICIT PARAMETER (spec 5.1).  There is no "current scope"
-- variable anywhere in this module: a scope is passed down to whoever needs it, so
-- a coroutine that yields in the middle of a check, a coroutine that is closed
-- from outside, and a coroutine that captured a handle cannot corrupt another
-- check's accounting.  Lua is single-threaded, so a charge is atomic with respect
-- to everything but a `coroutine.yield` inside user code, and the charge itself
-- never calls user code.
--
-- A closed node, or a node with a closed ANCESTOR, is SPENT: it refuses every
-- charge.  A stale handle is never a way to obtain a fresh root.
--
-- Scopes are closed with a to-be-closed variable:
--
--     local scope <close> = Budget.open_child(parent, max_steps)
--
-- which closes on every exit: a return, an error, and `coroutine.close` of a
-- suspended coroutine.

-- Standard functions are captured at load so that a property that patches the
-- string metatable or a library table cannot make the engine raise (VER05o 4).
local type, rawget, setmetatable, getmetatable = type, rawget, setmetatable, getmetatable

local Limits = require("coding_adventures.test_bootstrap.limits")
local Hooks = require("coding_adventures.test_bootstrap.hooks")

local mtype = math.type

local Budget = {}
Budget.__index = Budget

--- Most nested owning scopes; deeper scopes join, keeping the walk short.
Budget.MAX_DEPTH = 64
--- Most checks nested inside each other.
Budget.MAX_CHECK_NESTING = 64

local function closer(self)
  self:close()
end
Budget.__close = closer

-- The node a handle stands for: a joined handle aliases its owner.
local function owner(self)
  return rawget(self, "target") or self
end

local function new_node(parent, max_attempts, max_work, is_check)
  local node = setmetatable({
    parent = parent,
    depth_ = parent and parent.depth_ + 1 or 0,
    checks_ = (parent and parent.checks_ or 0) + (is_check and 1 or 0),
    max_attempts = max_attempts,
    max_work = max_work,
    attempts_ = 0,
    work_ = 0,
    closed = false,
    refused_ = false, -- meaningful on the root only
  }, Budget)
  node.root = parent and parent.root or node
  return node
end

-- A joined handle: charges and queries go to the node it aliases; closing it is a
-- no-op.  This is what an internal combinator uses so it never owns a scope.
local function join(node)
  return setmetatable({ target = node }, Budget)
end

local function default_work()
  return Hooks.default_max_work or Limits.MAX_SHRINK_WORK
end

--- A fresh root scope (an outermost budget) with explicit caps.
function Budget.root(max_attempts, max_work)
  return new_node(nil, max_attempts, max_work or default_work(), false)
end

--- A child of `parent` with explicit caps.
function Budget.child(parent, max_attempts, max_work)
  return new_node(owner(parent), max_attempts, max_work or Limits.MAX_SHRINK_WORK, false)
end

--- True for a scope (or a joined handle to one).
function Budget.is_scope(x)
  return type(x) == "table" and getmetatable(x) == Budget
end

--- The scope of a public entry point that runs user code: a CHILD of `parent` with
-- a fresh local cap, or a fresh root when `parent` is nil.  Past MAX_DEPTH it
-- joins instead of nesting.
function Budget.open_child(parent, max_steps, max_work)
  local cap = Limits.attempt_budget(max_steps)
  if parent == nil then
    return new_node(nil, cap, max_work or default_work(), false)
  end
  local p = owner(parent)
  if p.depth_ + 1 >= Budget.MAX_DEPTH then
    return join(p)
  end
  return new_node(p, cap, max_work or Limits.MAX_SHRINK_WORK, false)
end

--- The scope of a whole `Runner.check` (case loop included): a root without a
-- parent, a child otherwise.  Its attempt cap is num_tests plus the shrink attempt
-- cap, so a nested check's cases and its shrink are all charged up the chain.
function Budget.open_check(parent, num_tests, max_steps)
  local cap = num_tests + Limits.attempt_budget(max_steps)
  if parent == nil then
    return new_node(nil, cap, default_work(), true)
  end
  local p = owner(parent)
  if p.depth_ + 1 >= Budget.MAX_DEPTH then
    return join(p)
  end
  return new_node(p, cap, Limits.MAX_SHRINK_WORK, true)
end

--- The same node, as a handle that does not own it (closing it does nothing).
function Budget.join(scope)
  return join(owner(scope))
end

-- -------------------------------------------------------------------------
-- Queries
-- -------------------------------------------------------------------------

function Budget:depth()
  return owner(self).depth_
end

--- Enclosing `check` scopes, this one included.
function Budget:checks()
  return owner(self).checks_
end

function Budget:attempts()
  return owner(self).attempts_
end

function Budget:work()
  return owner(self).work_
end

--- True once this node or any ancestor is closed (the handle is SPENT).
function Budget:is_spent()
  local node = owner(self)
  while node do
    if node.closed then
      return true
    end
    node = node.parent
  end
  return false
end

--- True once any charge anywhere in this scope's tree has been refused.
function Budget:refused()
  return owner(self).root.refused_
end

--- Records that something in this tree could not be afforded (a refused nested
-- check, say), so a would-be pass becomes `property-exhausted`.
function Budget:mark_refused()
  owner(self).root.refused_ = true
end

--- True when this scope has an owning ancestor (it is not an outermost root).
function Budget:nested()
  return owner(self).parent ~= nil
end

--- True while at least one more attempt of one element could be afforded
-- everywhere on the path.
function Budget:room()
  local node = owner(self)
  while node do
    if node.closed or node.attempts_ >= node.max_attempts or node.work_ >= node.max_work then
      return false
    end
    node = node.parent
  end
  return true
end

-- -------------------------------------------------------------------------
-- Charging
-- -------------------------------------------------------------------------

-- Reserve `attempt_cost` attempts and `elements` work on every node of the path,
-- all or nothing.  A cost that is not a non-negative integer is refused (fail
-- closed); zero counts as one.  `cost > max - used` cannot wrap: used <= max.
local function take(self, elements, attempt_cost)
  local start = owner(self)
  local root = start.root
  if mtype(elements) ~= "integer" or elements < 0 then
    root.refused_ = true
    return false
  end
  local cost = elements < 1 and 1 or elements
  local node = start
  while node do
    if node.closed or node.attempts_ >= node.max_attempts or cost > node.max_work - node.work_ then
      root.refused_ = true
      return false
    end
    node = node.parent
  end
  node = start
  while node do
    node.attempts_ = node.attempts_ + attempt_cost
    node.work_ = node.work_ + cost
    node = node.parent
  end
  return true
end

--- Charges one attempt over a candidate of `elements` elements (at least 1) to
-- this node and every ancestor.  Returns false, charging nothing, when any of
-- them cannot afford it: callers must check this BEFORE building the candidate.
function Budget:charge(elements)
  return take(self, elements, 1)
end

--- Charges a copy that is not itself a property evaluation: work only.
function Budget:charge_copy(elements)
  return take(self, elements, 0)
end

--- Wraps a falsifies function so each evaluation is charged one attempt first
-- (false once refused).
function Budget:charged(falsifies)
  return function(value)
    return take(self, 1, 1) and falsifies(value)
  end
end

--- Ends an owning scope; a joined handle does nothing.
function Budget:close()
  if rawget(self, "target") then
    return
  end
  self.closed = true
end

return Budget
