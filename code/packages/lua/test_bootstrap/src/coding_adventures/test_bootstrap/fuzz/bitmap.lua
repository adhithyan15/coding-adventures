--- The execution map, admission and the power schedule (VER06 section 4,
-- VER06p section 4.1).
--
-- A map is a SET of edge ids reduced into [0, 65536): bit e set means edge e was
-- executed.  Collisions are the known price of a fixed-size map; AFL pays the same
-- one.  The set form costs O(edges hit) to list, where an 8 KiB bitmap would cost
-- O(8 KiB) per execution.

-- Standard functions are captured at load so that a target that patches the string
-- metatable or a library table cannot make the engine raise (VER06p section 1).
local m_max, m_floor, m_sqrt, m_type = math.max, math.floor, math.sqrt, math.type
local t_sort = table.sort
local type, error, setmetatable, rawlen, rawget, next = type, error, setmetatable, rawlen, rawget, next

local Int = require("coding_adventures.test_bootstrap.int")
local Limits = require("coding_adventures.test_bootstrap.fuzz.limits")
-- captured at load: a target that rewrites Limits later changes nothing here
local limits_COVERAGE_MAP_SIZE = Limits.COVERAGE_MAP_SIZE
-- captured at load: a target that rewrites Int later changes nothing here
local int_is, int_mod, int_tostring = Int.is, Int.mod, Int.tostring

local bitmap_reduce, bitmap_new, bitmap_corpus_admit, bitmap_calculate_energy
local Bitmap = {}
Bitmap.__index = Bitmap

--- An edge id (any Int) as its slot in the 65,536-entry map: floored `mod 65536`,
-- so -1 is 65535 and a Big is reduced by `int_mod`.  (For a native integer the low
-- sixteen bits ARE the floored remainder, in two's complement.)
function bitmap_reduce(edge)
  if m_type(edge) == "integer" then
    return edge & 0xFFFF
  end
  if not int_is(edge) then
    error("edge must be an Integer, not " .. (m_type(edge) == "float" and "a float" or type(edge)), 2)
  end
  return int_mod(edge, limits_COVERAGE_MAP_SIZE)
end

function bitmap_new()
  return setmetatable({ set = {}, n = 0 }, Bitmap)
end

function Bitmap:record(edge)
  local e = bitmap_reduce(edge)
  if not self.set[e] then
    self.set[e] = true
    self.n = self.n + 1
  end
end

function Bitmap:has(edge)
  return self.set[bitmap_reduce(edge)] == true
end

--- OR a trace (a list of edge ids) into this cumulative map; returns `has_new,
-- count_new`.
function Bitmap:admit(trace)
  local count_new = 0
  for i = 1, rawlen(trace) do
    local e = bitmap_reduce(rawget(trace, i))
    if not self.set[e] then
      self.set[e] = true
      self.n = self.n + 1
      count_new = count_new + 1
    end
  end
  return count_new > 0, count_new
end

--- The covered edges, ascending.
function Bitmap:covered_edges()
  local out = {}
  for e in next, self.set do
    out[#out + 1] = e
  end
  t_sort(out)
  return out
end

function Bitmap:count()
  return self.n
end

function Bitmap:reset()
  self.set, self.n = {}, 0
end

function Bitmap:__tostring()
  return "#<Fuzz.Bitmap edges=" .. self.n .. ">"
end

-- A table key for an unreduced edge: the native integer itself, or the decimal text
-- of a Big (a string never collides with an integer key, and no float is ever a key).
local function key_of(edge)
  if m_type(edge) == "integer" then
    return edge
  end
  if not int_is(edge) then
    error("edge must be an Integer, not " .. (m_type(edge) == "float" and "a float" or type(edge)), 3)
  end
  return int_tostring(edge)
end

--- Pure novelty check: does `trace` hold an edge absent from `existing`?  Returns
-- `admitted, new_edges` counting UNIQUE novel edges over the full, unreduced ids.
--
--   corpus_admit({1, 2, 3}, {2, 4})  --> true, 1
--   corpus_admit({1, 2, 3}, {1, 2})  --> false, 0
function bitmap_corpus_admit(existing, trace)
  local have, seen, novel = {}, {}, 0
  for i = 1, rawlen(existing) do
    have[key_of(rawget(existing, i))] = true
  end
  for i = 1, rawlen(trace) do
    local k = key_of(rawget(trace, i))
    if not have[k] and not seen[k] then
      seen[k] = true
      novel = novel + 1
    end
  end
  return novel > 0, novel
end

--- Mutation slots for a seed: max(1, round(base * (1 + new/total) / sqrt(len))).
-- The edge bonus is 1.0 while nothing is covered.  Rounding is half away from zero
-- (floor(x + 0.5)) so every lane agrees on exact halves.
--
-- base 10, len 4, 3 new of 6 total: 10 * 1.5 / 2 = 7.5 rounds to 8.
function bitmap_calculate_energy(base_energy, seed_len, new_edges, total_edges)
  local base = m_max(1, base_energy) + 0.0
  local bonus = 1.0
  if total_edges > 0 then
    bonus = 1.0 + (new_edges + 0.0) / total_edges
  end
  return m_max(1, m_floor(base * bonus / m_sqrt(m_max(1, seed_len)) + 0.5))
end

-- the public table is for callers; this module calls its own locals
Bitmap.reduce = bitmap_reduce
Bitmap.new = bitmap_new
Bitmap.corpus_admit = bitmap_corpus_admit
Bitmap.calculate_energy = bitmap_calculate_energy

return Bitmap
