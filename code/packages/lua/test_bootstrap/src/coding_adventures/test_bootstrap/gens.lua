--- The generator algebra (VER05 3, VER05e 3, VER05o section 1).
--
-- DRAW ORDER IS THE COMPATIBILITY CONTRACT between language lanes: the pbt-v1
-- vectors pin exact outputs per seed, so reordering a draw breaks conformance.
-- Nested generators are length-bounded per level, not in total
-- (`list(list(g, 0, n), 0, n)` can produce n x n elements); shrinking, by
-- contrast, shares one work budget across all levels.
--
-- Values: integers are Ints (u8, u64 and i128 included; a u64 above 2^63 - 1 is a
-- Big), bytes are Lua strings, lists and tuples are arrays.

-- Standard functions are captured at load so that a property that patches the
-- string metatable or a library table cannot make the engine raise (VER05o 4).
local m_max, m_type = math.max, math.type
local type, error, rawget, rawlen, ipairs = type, error, rawget, rawlen, ipairs

local Int = require("coding_adventures.test_bootstrap.int")
local Prng = require("coding_adventures.test_bootstrap.prng")
local Limits = require("coding_adventures.test_bootstrap.limits")
local Gen = require("coding_adventures.test_bootstrap.gen")
local Shrink = require("coding_adventures.test_bootstrap.shrink")

local Gens = {}

local TWO64 = Prng.TWO64
local TWO127 = Int.pow2(127)
local TWO128 = Int.pow2(128)
local MIN_I128 = Int.neg(TWO127)
local MAX_I128 = Int.sub(TWO127, 1)
local MAX_U64 = Int.sub(TWO64, 1)

--- The in-range point nearest zero: the integer shrink target.
function Gens.clamp_zero(lo, hi)
  if Int.sign(lo) > 0 then
    return lo
  end
  if Int.sign(hi) < 0 then
    return hi
  end
  return 0
end

local function int_shrinker(lo, hi)
  local target = Gens.clamp_zero(lo, hi)
  return function(value, falsifies, max_steps)
    return Shrink.shrink_int_core(value, target, lo, hi, falsifies, max_steps)
  end
end

--- Always generates `value` and draws nothing.
function Gens.constant(value)
  return Gen.new(function()
    return value
  end)
end

--- bounded(2) == 1.  Shrinks true to false.
function Gens.bool()
  return Gen.new(function(rng)
    return rng:bounded(2) == 1
  end, function(value, falsifies, max_steps)
    if value and max_steps > 0 and falsifies(false) then
      return false, 1
    end
    return value, 0
  end)
end

--- [0, 255]: bounded(256).
function Gens.u8()
  return Gen.new(function(rng)
    return rng:bounded(256)
  end, int_shrinker(0, 255))
end

--- A full 64-bit word: next_u64.
function Gens.u64()
  return Gen.new(function(rng)
    return rng:next_u64()
  end, int_shrinker(0, MAX_U64))
end

--- Two stream words, high first, read as two's complement:
--   words 0x8000...0 : 0   -> -2^127  (top bit set => subtract 2^128)
--   words 0x7fff...f : f.. ->  2^127 - 1
function Gens.sample_i128(rng)
  local hi = rng:next_u64()
  local lo = rng:next_u64()
  local v = Int.add(Int.shl(hi, 64), lo)
  if Int.cmp(v, TWO127) >= 0 then
    return Int.sub(v, TWO128)
  end
  return v
end

function Gens.i128()
  return Gen.new(Gens.sample_i128, int_shrinker(MIN_I128, MAX_I128))
end

local function bounded_int(rng, lo, hi)
  return Int.add(lo, rng:bounded_big(Int.add(Int.sub(hi, lo), 1)))
end

--- A uniform integer over the inclusive range [min, max], any width.  The offset
-- from min is drawn by bounded_big (VER05 7.2).  Shrinks toward the in-range point
-- nearest zero and never leaves [min, max].
function Gens.integer(min, max)
  Int.check(min, "min")
  Int.check(max, "max")
  if Int.cmp(min, max) > 0 then
    error("integer needs min <= max", 2)
  end
  if Int.bit_length(Int.add(Int.sub(max, min), 1)) > Limits.MAX_SPAN_BITS then
    error("integer span wider than " .. Limits.MAX_SPAN_BITS .. " bits", 2)
  end
  return Gen.new(function(rng)
    return bounded_int(rng, min, max)
  end, int_shrinker(min, max))
end

local function check_lengths(min_len, max_len, limit, what)
  for _, v in ipairs({ min_len, max_len }) do
    if m_type(v) ~= "integer" then
      error(what .. " lengths must be integers (a float is never a length)", 3)
    end
  end
  if min_len < 0 or max_len < min_len then
    error(what .. " needs 0 <= min_len <= max_len", 3)
  end
  if max_len > limit then
    error(what .. " max_len " .. max_len .. " exceeds " .. limit, 3)
  end
end

--- A byte string: L = min_len + bounded(span), then L draws of bounded(256).
-- Shrinks by sequence shrinking, respecting min_len.
function Gens.bytes(min_len, max_len)
  check_lengths(min_len, max_len, Limits.MAX_INPUT_BYTES, "bytes")
  local span = max_len - min_len + 1
  local function sampler(rng)
    local n = min_len + rng:bounded(span)
    local list = {}
    for i = 1, n do
      list[i] = rng:bounded(256)
    end
    return Shrink.bytes_to_string(list)
  end
  local function shrinker(value, falsifies, max_steps, scope)
    -- The array conversion (and the string one on the way out) are copies too:
    -- charge them first, and do nothing once the budget is spent.
    if max_steps == 0 or (scope and not scope:charge_copy(2 * #value)) then
      return value, 0
    end
    local function wrapped(c)
      return (scope == nil or scope:charge_copy(#c)) and falsifies(Shrink.bytes_to_string(c))
    end
    local list, steps = Shrink.shrink_sequence(Shrink.string_to_bytes(value), wrapped, max_steps, min_len, Shrink.byte_walk, scope)
    return Shrink.bytes_to_string(list), steps
  end
  return Gen.new(sampler, shrinker)
end

--- Transforms generated values.  Drops shrinking: the function has no inverse.
function Gens.map(gen, fn)
  if type(fn) ~= "function" then
    error("a function is required", 2)
  end
  return Gen.new(function(rng, scope)
    return fn(gen:sample(rng, scope))
  end)
end

--- Keeps only values for which `pred` returns exactly `true`.  After exactly
-- `max_retries` rejected samples sampling fails with an exhausted error.
-- Shrinking is preserved: a candidate failing the predicate is simply not a valid
-- input.
function Gens.filter(gen, pred, max_retries)
  if type(pred) ~= "function" then
    error("a predicate function is required", 2)
  end
  max_retries = max_retries or Limits.DEFAULT_MAX_RETRIES
  Limits.check_range(max_retries, 1, Limits.MAX_RETRIES, "max_retries")
  local function sampler(rng, scope)
    -- Every retry inside a check is charged, so stacked filters (each retrying
    -- 10,000 times) cannot multiply uncharged work: in a NESTED check a retry is an
    -- attempt charged up the chain; in a top-level check it is one element of work
    -- against the root's 64M allowance.
    local nested = scope ~= nil and scope:nested()
    for _ = 1, max_retries do
      if scope then
        local ok
        if nested then
          ok = scope:charge(1)
        else
          ok = scope:charge_copy(1)
        end
        if not ok then
          error(Gen.exhausted_error("the enclosing shrink budget was spent while filtering"), 0)
        end
      end
      local v = gen:sample(rng, scope)
      if pred(v) == true then
        return v
      end
    end
    error(Gen.exhausted_error("filter rejected " .. max_retries .. " samples"), 0)
  end
  local function shrinker(value, falsifies, max_steps, scope)
    -- Charge BEFORE the user predicate runs: it is property code and may be costly.
    return gen:shrink_nested(value, function(c)
      return (scope == nil or scope:charge(1)) and pred(c) == true and falsifies(c)
    end, max_steps, scope)
  end
  return Gen.new(sampler, shrinker)
end

--- Dependent generation: sample, then sample from the generator `fn` returns.
function Gens.flat_map(gen, fn)
  if type(fn) ~= "function" then
    error("a function is required", 2)
  end
  return Gen.new(function(rng, scope)
    return fn(gen:sample(rng, scope)):sample(rng, scope)
  end)
end

--- Picks one generator uniformly (bounded(n), then that arm's draws).
function Gens.one_of(gens)
  if type(gens) ~= "table" or rawlen(gens) == 0 then
    error("one_of needs at least one generator", 2)
  end
  local arms = {}
  for i = 1, rawlen(gens) do
    arms[i] = rawget(gens, i)
  end
  return Gen.new(function(rng, scope)
    return arms[rng:bounded(#arms) + 1]:sample(rng, scope)
  end)
end

--- A weighted choice among `{weight, gen}` pairs: pick = bounded(total); arms are
-- walked in order subtracting each weight, so a zero-weight arm can never be
-- selected.  Empty, negative weight, zero total or total above 2^32 is refused.
function Gens.frequency(arms_in)
  if type(arms_in) ~= "table" or rawlen(arms_in) == 0 then
    error("frequency needs at least one arm", 2)
  end
  local total = 0
  local owned = {}
  for i = 1, rawlen(arms_in) do
    local pair = rawget(arms_in, i)
    if type(pair) ~= "table" then
      error("frequency arms must be {weight, gen} pairs", 2)
    end
    local weight, gen = rawget(pair, 1), rawget(pair, 2)
    Limits.check_range(weight, 0, Limits.MAX_FREQUENCY_TOTAL, "frequency weight")
    total = total + weight
    if total > Limits.MAX_FREQUENCY_TOTAL then
      error("frequency total weight exceeds 2^32", 2)
    end
    owned[i] = { weight, gen }
  end
  if total == 0 then
    error("frequency total weight must be > 0", 2)
  end
  return Gen.new(function(rng, scope)
    local pick = rng:bounded(total)
    for _, arm in ipairs(owned) do
      if pick < arm[1] then
        return arm[2]:sample(rng, scope)
      end
      pick = pick - arm[1]
    end
    error("unreachable: pick < total")
  end)
end

--- A list: L = min_len + bounded(span), then L elements in order.  Shrinks by
-- sequence shrinking with the element generator's shrinker for the element-wise
-- phase.
function Gens.list(elem, min_len, max_len)
  check_lengths(min_len, max_len, Limits.MAX_LIST_LEN, "list")
  local span = max_len - min_len + 1
  local function sampler(rng, scope)
    local n = min_len + rng:bounded(span)
    local out = {}
    for i = 1, n do
      out[i] = elem:sample(rng, scope)
    end
    return out
  end
  local function shrinker(value, falsifies, max_steps, scope)
    local function shrink_elem(v, f, n, s)
      return elem:shrink_nested(v, f, n, s)
    end
    return Shrink.shrink_sequence(value, falsifies, max_steps, min_len, shrink_elem, scope)
  end
  return Gen.new(sampler, shrinker)
end

-- One shrink-wide evaluation budget for a tuple: the repeat-until-no-progress loop
-- calls each component shrinker once per pass, so without a shared counter the work
-- is quadratic in max_steps.
local function global_budget(falsifies, max_steps)
  local budget = Limits.attempt_budget(max_steps)
  local spent = 0
  return function(v)
    spent = spent + 1
    return spent <= budget and falsifies(v)
  end
end

--- Samples each generator left to right (an array) and shrinks one coordinate at a
-- time, in repeated passes until a pass makes no progress.
function Gens.tuple(gens)
  if type(gens) ~= "table" or rawlen(gens) == 0 then
    error("tuple needs at least one generator", 2)
  end
  if rawlen(gens) > Limits.MAX_TUPLE_ARITY then
    error("tuple arity exceeds " .. Limits.MAX_TUPLE_ARITY, 2)
  end
  local arms = {}
  for i = 1, rawlen(gens) do
    arms[i] = rawget(gens, i)
  end
  local function sampler(rng, scope)
    local out = {}
    for i, g in ipairs(arms) do
      out[i] = g:sample(rng, scope)
    end
    return out
  end
  local function shrinker(value, raw_falsifies, max_steps, scope)
    local falsifies = global_budget(raw_falsifies, max_steps)
    local current = {}
    for i = 1, #arms do
      current[i] = rawget(value, i)
    end
    local steps = 0
    local progress = true
    while progress and steps < max_steps do
      progress = false
      for index, gen in ipairs(arms) do
        local remaining = max_steps - steps
        if remaining > 0 then
          local function slot(candidate)
            local probe = {}
            for i = 1, #arms do
              probe[i] = current[i]
            end
            probe[index] = candidate
            return falsifies(probe)
          end
          local shrunk, used = gen:shrink_nested(current[index], slot, remaining, scope)
          if used > 0 then
            current[index] = shrunk
            steps = steps + used
            progress = true
          end
        end
      end
    end
    return current, steps
  end
  return Gen.new(sampler, shrinker)
end

-- -- corpus entry points (pbt-v1) -------------------------------------------

--- `count` primitives of `type_name` ("bool", "u8", "u64", "i128") from `seed`.
function Gens.gen_primitive(seed, type_name, count)
  Limits.check_count(count)
  local rng = Prng.new_rng(seed)
  local sampler
  if type_name == "bool" then
    sampler = function(r)
      return r:bounded(2) == 1
    end
  elseif type_name == "u8" then
    sampler = function(r)
      return r:bounded(256)
    end
  elseif type_name == "u64" then
    sampler = function(r)
      return r:next_u64()
    end
  elseif type_name == "i128" then
    sampler = Gens.sample_i128
  else
    error("unknown primitive type", 2)
  end
  local out = {}
  for i = 1, count do
    out[i] = sampler(rng)
  end
  return out
end

--- `count` uniform integers over [lo, hi] from `seed`.
function Gens.gen_bounded(seed, lo, hi, count)
  Int.check(lo, "min")
  Int.check(hi, "max")
  if Int.cmp(lo, hi) > 0 then
    error("need lo <= hi", 2)
  end
  Limits.check_count(count)
  -- Each draw is about as wide as the span; count x width bounds what the returned
  -- list can hold (a 1,000-digit span x 10^6 draws would not).
  local width = m_max(1, (Int.bit_length(Int.sub(hi, lo)) + 7) // 8)
  if count * width > Limits.MAX_GEN_BYTES_TOTAL then
    error("count " .. count .. " x " .. width .. "-byte span exceeds the generation budget", 2)
  end
  if Int.bit_length(Int.add(Int.sub(hi, lo), 1)) > Limits.MAX_SPAN_BITS then
    error("span wider than " .. Limits.MAX_SPAN_BITS .. " bits", 2)
  end
  local rng = Prng.new_rng(seed)
  local out = {}
  for i = 1, count do
    out[i] = bounded_int(rng, lo, hi)
  end
  return out
end

--- `count` byte strings with length in [min_len, max_len] from `seed`.
function Gens.gen_bytes(seed, min_len, max_len, count)
  check_lengths(min_len, max_len, Limits.MAX_INPUT_BYTES, "gen_bytes")
  Limits.check_count(count) -- applies even when max_len == 0
  if max_len > 0 and count > Limits.MAX_GEN_BYTES_TOTAL // max_len then
    error("count " .. count .. " x max_len " .. max_len .. " exceeds the total of " .. Limits.MAX_GEN_BYTES_TOTAL .. " bytes", 2)
  end
  local gen = Gens.bytes(min_len, max_len)
  local rng = Prng.new_rng(seed)
  local out = {}
  for i = 1, count do
    out[i] = gen:sample(rng)
  end
  return out
end

Gens.MIN_I128 = MIN_I128
Gens.MAX_I128 = MAX_I128
Gens.MAX_U64 = MAX_U64

return Gens
