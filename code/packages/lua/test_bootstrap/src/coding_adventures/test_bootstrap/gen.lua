--- A generator of values: it draws from a PRNG stream and knows how to shrink
-- what it drew (VER05 3, 4, VER05o section 5).
--
-- `sampler(rng, scope)` returns a value.  `scope` is the budget scope of the
-- running check (or nil outside one); only `filter` reads it.
--
-- `shrinker(value, falsifies, max_steps, scope)` is nil (values are reported
-- unshrunk) or a function returning `value, steps`, where `falsifies(candidate)`
-- is true while a candidate still falsifies the property and `scope` is the scope
-- the shrinker JOINS.

-- Standard functions are captured at load so that a property that patches the
-- string metatable or a library table cannot make the engine raise (VER05o 4).
local type, error, setmetatable, getmetatable = type, error, setmetatable, getmetatable

local Budget = require("coding_adventures.test_bootstrap.budget")
local Limits = require("coding_adventures.test_bootstrap.limits")

local Gen = {}
Gen.__index = Gen

-- Identity registry of "this generator could not produce a value" errors, so the
-- runner can tell a filter that never accepts from any other sampling error
-- without reading anything from the error value.
local EXHAUSTED = setmetatable({}, { __mode = "k" })

--- A sampling failure the runner reports as `property-exhausted`.
function Gen.exhausted_error(message)
  local err = { message = message }
  EXHAUSTED[err] = true
  return err
end

function Gen.is_exhausted_error(err)
  return type(err) == "table" and EXHAUSTED[err] == true
end

function Gen.new(sampler, shrinker)
  if type(sampler) ~= "function" then
    error("a sampler function is required", 2)
  end
  if shrinker ~= nil and type(shrinker) ~= "function" then
    error("a shrinker must be a function", 2)
  end
  return setmetatable({ sampler = sampler, shrinker = shrinker }, Gen)
end

function Gen.is(x)
  return type(x) == "table" and getmetatable(x) == Gen
end

--- Draws one value.
function Gen:sample(rng, scope)
  return self.sampler(rng, scope)
end

local function run(self, value, falsifies, max_steps, scope)
  if self.shrinker == nil then
    return value, 0
  end
  return self.shrinker(value, falsifies, max_steps, scope)
end

--- Shrinks `value` while `falsifies` holds; `value, 0` if there is no shrinker.  A
-- PUBLIC ENTRY POINT: it opens a child of `parent` (a fresh root when nil) with
-- its own local cap and charges every evaluation of `falsifies`, so however a
-- property or a custom shrinker nests calls, the total work stays inside the
-- outermost budget.
function Gen:shrink(value, max_steps, falsifies, parent)
  if type(falsifies) ~= "function" then
    error("a falsifies function is required", 2)
  end
  Limits.check_budget(max_steps)
  local scope <close> = Budget.open_child(parent, max_steps)
  return run(self, value, scope:charged(falsifies), max_steps, scope)
end

--- For combinators shrinking their components: JOINS the scope it is given
-- instead of opening a new one (a fresh root only when `scope` is nil).  The
-- caller's `falsifies` is already charged, so nothing is charged twice.
function Gen:shrink_nested(value, falsifies, max_steps, scope)
  Limits.check_budget(max_steps)
  if scope == nil then
    local own <close> = Budget.open_child(nil, max_steps)
    return run(self, value, own:charged(falsifies), max_steps, own)
  end
  return run(self, value, falsifies, max_steps, scope)
end

return Gen
