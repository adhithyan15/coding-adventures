--- Integrated shrinking (VER05 4, VER05e 4, VER05o sections 1 and 5).
--
-- A property that fails on [913, -4, 77, 0, 2] has told you nothing; one that
-- fails on [0] has told you what is wrong.  Shrinking walks from the first failing
-- candidate toward the smallest one that still fails.  It is "integrated" because
-- the generator that made the value knows its own bounds, so a shrunk
-- `Gens.integer(5, 10)` never becomes 0.
--
-- The public shrinkers take a `prop` function that returns EXACTLY `true` when the
-- property HOLDS (the corpus convention) and return `minimal, steps`, where steps
-- counts ACCEPTED moves only.
--
-- Integers here are `Int`s (native or Big): every comparison and every piece of
-- arithmetic goes through the Int module, and the bisection step is a FLOORED
-- half (`-81` bisects to `-40`, not `-41`).

-- Standard functions are captured at load so that a property that patches the
-- string metatable or a library table cannot make the engine raise (VER05o 4).
local m_max, m_min, s_byte, s_char, t_concat, t_move, t_remove, t_unpack = math.max, math.min, string.byte, string.char, table.concat, table.move, table.remove, table.unpack
local type, error, rawget, rawlen = type, error, rawget, rawlen

local Int = require("coding_adventures.test_bootstrap.int")
local Hooks = require("coding_adventures.test_bootstrap.hooks")
local Limits = require("coding_adventures.test_bootstrap.limits")
local Budget = require("coding_adventures.test_bootstrap.budget")
local Prng = require("coding_adventures.test_bootstrap.prng")

local Shrink = {}

local NOT_FALSIFYING = "the initial value must falsify the property"
Shrink.NOT_FALSIFYING = NOT_FALSIFYING

local function in_bounds(value, lo, hi)
  return (lo == nil or Int.le(lo, value)) and (hi == nil or Int.le(value, hi))
end

-- A plain copy of a sequence (raw reads: a user-supplied value may carry
-- metamethods, which the engine never triggers).
local function copy(t, n)
  local r = {}
  for i = 1, n or rawlen(t) do
    r[i] = rawget(t, i)
  end
  return r
end

local function slice(t, i, j)
  local r = {}
  for k = i, j do
    r[k - i + 1] = t[k]
  end
  return r
end

-- Byte strings <-> arrays of byte values, in chunks (string.byte and string.char
-- have a stack limit).
local CHUNK = 4096

function Shrink.string_to_bytes(s)
  local out, n = {}, 0
  for i = 1, #s, CHUNK do
    local chunk = { s_byte(s, i, m_min(i + CHUNK - 1, #s)) }
    t_move(chunk, 1, #chunk, n + 1, out)
    n = n + #chunk
  end
  return out
end

function Shrink.bytes_to_string(list)
  local parts = {}
  local n = rawlen(list)
  for i = 1, n, CHUNK do
    parts[#parts + 1] = s_char(t_unpack(list, i, m_min(i + CHUNK - 1, n)))
  end
  return t_concat(parts)
end

--- Bisects then unit-steps `current` toward `target`.
--
-- Each turn tries, in order,
--   1. the bisection candidate  current - floor(diff / 2)   when |diff| > 1,
--   2. the unit candidate       current -/+ 1 toward the target.
-- A candidate is accepted only if inside [lo, hi] (nil = unbounded), different from
-- current and still falsifying.  No acceptable candidate ends the walk.
--
-- Worked example (corpus shrink_int/54321_le_100, property x <= 100):
-- 54321 -> 27161 -> 13581 -> ... -> 101 in 15 accepted moves; 100 would satisfy the
-- property, so neither candidate from 101 is accepted.
function Shrink.int_walk(current, target, lo, hi, falsifies, budget)
  local steps = 0
  while steps < budget and not Int.eq(current, target) do
    local diff = Int.sub(current, target)
    local moved = false
    if Int.cmp(Int.abs(diff), 1) > 0 then
      local cand = Hooks.bisect(current, diff)
      if not Int.eq(cand, current) and in_bounds(cand, lo, hi) and falsifies(cand) then
        current = cand
        steps = steps + 1
        moved = true
      end
    end
    if not moved then
      local cand = Hooks.unit(current, diff)
      if not Int.eq(cand, current) and in_bounds(cand, lo, hi) and falsifies(cand) then
        current = cand
        steps = steps + 1
        moved = true
      end
    end
    if not moved then
      break
    end
  end
  return current, steps
end

--- The target probe followed by the walk (VER05 4.2).
function Shrink.shrink_int_core(initial, target, lo, hi, falsifies, max_steps)
  if max_steps == 0 then
    return initial, 0 -- no budget, no shrinking
  end
  -- the target itself fails: cannot do better
  if in_bounds(target, lo, hi) and not Int.eq(target, initial) and falsifies(target) then
    return target, 1
  end
  return Shrink.int_walk(initial, target, lo, hi, falsifies, max_steps)
end

--- A byte value shrunk toward 0 by the integer walk.
function Shrink.byte_walk(value, falsifies, budget)
  return Shrink.int_walk(value, 0, 0, 255, falsifies, budget)
end

--- Shrinks a falsifying sequence (an array) (VER05 4.3).  `falsifies` always
-- receives a FRESH array it may keep or mutate; the ELEMENTS are not cloned.
--
--   probe        min_len == 0: does the empty list fail?  Then it is the answer, one
--                step.  (min_len > 0: probe the length-min_len prefix and carry on.)
--   bisection    keep the left half, else the right half, while one fails.
--   deletion     single elements, right to left.
--   element-wise left to right via `shrink_elem(value, falsifies, steps, scope)`
--                (nil means none).
--
-- Worked example (corpus list_sum_le_20, [10 5 8 4]): the empty list and both halves
-- satisfy the property; deleting index 4 leaves [10 5 8] (sum 23, still failing);
-- element-wise 10 -> 8 gives the 1-minimal [8 5 8].
--
-- Every candidate is CHARGED (its copy) to the budget tree BEFORE it is built; once
-- refused, every loop ends.  When max_steps == 0 or the budget is already spent the
-- input is returned as is (it may be the same table).  `scope` is joined; with no
-- scope a fresh root is used for the call.
function Shrink.shrink_sequence(initial, falsifies, max_steps, min_len, shrink_elem, scope)
  if max_steps == 0 then
    return initial, 0
  end
  local own <close> = (scope == nil) and Budget.open_child(nil, max_steps) or nil
  scope = scope or own
  local size = rawlen(initial)
  if not scope:charge_copy(size) then
    return initial, 0
  end
  local current = copy(initial, size)
  local steps = 0

  if #current > min_len and scope:charge_copy(min_len) and falsifies(slice(current, 1, min_len)) then
    if min_len == 0 then
      return {}, 1
    end
    current = slice(current, 1, min_len)
    steps = 1
  end

  local floor = m_max(1, min_len)

  -- Phase 1: bisection (left half first, then right half).
  while #current > floor and steps < max_steps do
    local mid = #current // 2
    if mid >= min_len and scope:charge_copy(mid) and falsifies(slice(current, 1, mid)) then
      current = slice(current, 1, mid)
      steps = steps + 1
    else
      local right_len = #current - mid
      if right_len >= min_len and scope:charge_copy(right_len) and falsifies(slice(current, mid + 1, #current)) then
        current = slice(current, mid + 1, #current)
        steps = steps + 1
      else
        break
      end
    end
  end

  -- Phase 2: single-element deletion, right to left.
  if not Hooks.skip_deletion then
    local i = #current
    while i >= 1 and #current > floor and steps < max_steps and scope:room() do
      if not scope:charge_copy(#current - 1) then -- charge BEFORE copying
        break
      end
      local cand = copy(current, #current)
      t_remove(cand, i)
      if falsifies(cand) then
        current = cand
        steps = steps + 1
        i = m_min(i, #current)
      else
        i = i - 1
      end
    end
  end

  -- Phase 3: element-wise, left to right.
  if shrink_elem then
    local count = #current
    local index = 1
    while index <= count do
      local remaining = max_steps - steps
      if remaining <= 0 or not scope:room() then
        break
      end
      local at = index
      local function with_element(candidate)
        if not scope:charge_copy(#current) then
          return false
        end
        local probe = copy(current, #current)
        probe[at] = candidate
        return falsifies(probe)
      end
      local value, used = shrink_elem(current[index], with_element, remaining, scope)
      if used > 0 then
        current[index] = value
        steps = steps + used
      end
      index = index + 1
    end
  end

  return current, steps
end

local function check_prop(prop)
  if type(prop) ~= "function" then
    error("a property function is required", 3)
  end
end

--- Shrinks a falsifying integer toward `target` inside [lo, hi] (the corpus calls
-- these min and max).  `prop(x)` is exactly `true` when the property holds.
-- Raises when `initial` satisfies it, for lo > hi, an initial outside the bounds or
-- a budget outside [0, MAX_SHRINK_STEPS].  Returns `minimal, steps`.
function Shrink.shrink_int(initial, target, lo, hi, prop, max_steps, parent)
  check_prop(prop)
  max_steps = max_steps or 1000
  Int.check(initial, "initial")
  Int.check(target, "target")
  Int.check(lo, "min")
  Int.check(hi, "max")
  Limits.check_budget(max_steps)
  if Int.cmp(lo, hi) > 0 then
    error("min exceeds max", 2)
  end
  if not in_bounds(initial, lo, hi) then
    error("initial value must lie within [min, max]", 2)
  end
  local scope <close> = Budget.open_child(parent, max_steps)
  if not scope:charge(1) then -- starved by an enclosing shrink
    return initial, 0
  end
  if prop(initial, scope) == true then
    error(NOT_FALSIFYING, 2)
  end
  local function falsifies(v)
    return scope:charge(1) and prop(v, scope) ~= true
  end
  return Shrink.shrink_int_core(initial, target, lo, hi, falsifies, max_steps)
end

--- Shrinks a falsifying list of integers: empty probe, halves, single deletion,
-- then each element toward `elem_target` (unbounded).
function Shrink.shrink_list(initial, elem_target, prop, max_steps, parent)
  check_prop(prop)
  max_steps = max_steps or 1000
  if type(initial) ~= "table" then
    error("initial must be a list", 2)
  end
  Int.check(elem_target, "elem_target")
  for i = 1, rawlen(initial) do
    Int.check(rawget(initial, i), "element")
  end
  Limits.check_budget(max_steps)
  local scope <close> = Budget.open_child(parent, max_steps)
  -- charged before the first copies (the working copy and the copy the first
  -- property call receives) and for the initial evaluation
  local size = rawlen(initial)
  if not scope:charge_copy(2 * size) or not scope:charge(1) then
    return initial, 0
  end
  local items = copy(initial, size)
  if prop(copy(items, size), scope) == true then
    error(NOT_FALSIFYING, 2)
  end
  local function walk(value, falsifies, steps)
    return Shrink.int_walk(value, elem_target, nil, nil, falsifies, steps)
  end
  local function falsifies(c)
    return scope:charge(1) and prop(c, scope) ~= true
  end
  return Shrink.shrink_sequence(items, falsifies, max_steps, 0, walk, scope)
end

--- Shrinks falsifying bytes (a Lua string): sequence shrinking over byte values
-- toward 0x00.
function Shrink.shrink_bytes(initial, prop, max_steps, parent)
  check_prop(prop)
  max_steps = max_steps or 1000
  if type(initial) ~= "string" then
    error("initial must be a string", 2)
  end
  Limits.check_budget(max_steps)
  local scope <close> = Budget.open_child(parent, max_steps)
  -- the copy, the array conversion and the string conversion back
  if not scope:charge_copy(3 * #initial) or not scope:charge(1) then
    return initial, 0
  end
  if prop(initial, scope) == true then
    error(NOT_FALSIFYING, 2)
  end
  -- each attempt also converts its candidate back to a string: charge that copy too
  local function falsifies(c)
    return scope:charge_copy(#c) and scope:charge(1) and prop(Shrink.bytes_to_string(c), scope) ~= true
  end
  local list, steps = Shrink.shrink_sequence(Shrink.string_to_bytes(initial), falsifies, max_steps, 0, Shrink.byte_walk, scope)
  return Shrink.bytes_to_string(list), steps
end

--- Replays a failure from nothing but `seed`: draw up to 1000 values bounded(1000);
-- the first that falsifies is the initial failure, then shrunk toward 0 over
-- [0, 1000].  Same seed, same pair, forever.  Returns `initial_fail, minimal`.
function Shrink.simulate_replay(seed, prop, parent)
  check_prop(prop)
  local scope <close> = Budget.open_child(parent, 1000)
  local rng = Prng.new_rng(seed)
  local first = nil
  for _ = 1, 1000 do
    if not scope:charge(1) then
      break
    end
    local v = rng:bounded(1000)
    if prop(v, scope) ~= true then
      first = v
      break
    end
  end
  if first == nil then
    error("property was not falsified within 1000 draws", 2)
  end
  local minimal = Shrink.shrink_int(first, 0, 0, 1000, prop, 1000, scope)
  return first, minimal
end

return Shrink
