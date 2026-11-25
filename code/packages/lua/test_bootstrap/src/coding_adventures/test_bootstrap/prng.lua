--- Deterministic randomness and hashing for test runs (TF01 5.1, VER05 2,
-- VER05o section 3).
--
-- SplitMix64 and FNV-1a are defined on 64-bit words that WRAP.  Lua integers are
-- exactly that: 64-bit, two's complement, wrapping on `+` and `*`, with a logical
-- `>>`.  So the algorithms are written the way the papers write them and need no
-- masking.  What Lua does NOT give us is unsigned comparison, division and
-- printing: those go through `math.ult`, `Prng.udivmod` and `Int.from_u64`.
--
-- A "word" below is a Lua integer read as an unsigned 64-bit pattern.

-- Standard functions are captured at load so that a property that patches the
-- string metatable or a library table cannot make the engine raise (VER05o 4).
local m_max, s_byte = math.max, string.byte
local type, error, setmetatable = type, error, setmetatable

local Int = require("coding_adventures.test_bootstrap.int")
local Hooks = require("coding_adventures.test_bootstrap.hooks")
local Limits = require("coding_adventures.test_bootstrap.limits")

local ult = math.ult
local mtype = math.type

local GOLDEN_GAMMA = 0x9E3779B97F4A7C15
local MIX_MUL1 = 0xBF58476D1CE4E5B9
local MIX_MUL2 = 0x94D049BB133111EB
local FNV_OFFSET_BASIS = 0xCBF29CE484222325
local FNV_PRIME = 0x00000100000001B3
local TWO64 = Int.pow2(64)

local Prng = {}

--- Unsigned 64-bit division of two words: returns quotient and remainder words.
-- `b` must not be 0.
--
--   b has bit 63 set: the quotient is 0 or 1.
--   a has bit 63 clear: Lua's floor division is already unsigned-correct.
--   otherwise: halve a (so it is positive), divide, double, and correct the
--   at-most-one-too-small quotient.
--
-- Worked example: a = 2^64 - 1 (word -1), b = 3.  a >> 1 = 2^63 - 1; (2^63-1)//3 =
-- 3074457345618258602; doubled 6148914691236517204; r = a - q*3 = 3 >= 3, so
-- q = 6148914691236517205, r = 0.
function Prng.udivmod(a, b)
  if b < 0 then
    if ult(a, b) then
      return 0, a
    end
    return 1, a - b
  end
  if a >= 0 then
    return a // b, a % b
  end
  local q = ((a >> 1) // b) << 1
  local r = a - q * b
  if not ult(r, b) then
    q = q + 1
    r = r - b
  end
  return q, r
end

--- The rejection threshold t = (2^64 - n) mod n for a word n ~= 0, computed
-- UNSIGNED.  `0 - n` is 2^64 - n as a word; the same expression through a signed
-- modulo is wrong (it is 0 for small n, and wrong above 2^63).
function Prng.threshold(n)
  local _, r = Prng.udivmod(0 - n, n)
  return r
end

local Rng = {}
Rng.__index = Rng

--- A new SplitMix64 stream.  `seed` is an Int of any size, reduced modulo 2^64
-- (two's complement: -1 is 2^64 - 1).  A float, boolean or string is refused.
function Prng.new(seed)
  Int.check(seed, "seed")
  return setmetatable({ state = Int.to_u64(seed) }, Rng)
end

--- The factory the engine uses; `Hooks.new_rng` replaces it for sabotage tests.
function Prng.new_rng(seed)
  if Hooks.new_rng then
    return Hooks.new_rng(seed)
  end
  return Prng.new(seed)
end

--- The state IS the stream: never printed.
function Rng:__tostring()
  return "#<Prng.SplitMix64>"
end

--- SplitMix64 step; returns the output WORD.
--
--   state += golden gamma
--   z1  = (z ^ (z >> 30)) * 0xbf58476d1ce4e5b9
--   z2  = (z1 ^ (z1 >> 27)) * 0x94d049bb133111eb
--   out = z2 ^ (z2 >> 31)
--
-- Seed 0's first output is 0xe220a8397b1dcdaf.
function Rng:next_word()
  local s = self.state + GOLDEN_GAMMA
  self.state = s
  local z = (s ~ (s >> 30)) * MIX_MUL1
  z = (z ~ (z >> 27)) * MIX_MUL2
  return z ~ (z >> 31)
end

--- The next output as an Int (0 .. 2^64-1).
function Rng:next_u64()
  return Int.from_u64(self:next_word())
end

--- Uniform word in [0, n) for a word n ~= 0, by rejection sampling.
--
-- Raws below t are redrawn so the remaining 2^64 - t values split into whole
-- residue classes.  bounded(1) still consumes one word (the VER05 2.2 loop run
-- once): the stream position must be identical in every lane.
--
-- Worked example, n = 3: 2^64 = 3 * 6148914691236517205 + 1, so t = 1 and the
-- single raw 0 is thrown away.
function Rng:bounded_word(n)
  local t
  if Hooks.threshold then
    t = Hooks.threshold(n)
  else
    t = Prng.threshold(n)
  end
  while true do
    local x = self:next_word()
    if not ult(x, t) then
      local _, r = Prng.udivmod(x, n)
      return r
    end
  end
end

--- Uniform Int in [0, n) for an Int 1 <= n <= 2^64 (2^64 is one raw word).
function Rng:bounded(n)
  if mtype(n) == "integer" and n > 0 then
    return self:bounded_word(n)
  end
  Int.check(n, "bounded(n)")
  if Int.sign(n) < 1 then
    error("bounded(n) requires n >= 1", 2)
  end
  local c = Int.cmp(n, TWO64)
  if c > 0 then
    error("bounded(n) requires n <= 2^64", 2)
  end
  if c == 0 then
    return Int.from_u64(self:next_word())
  end
  -- 2^63 <= n < 2^64: the span is a word with bit 63 set
  return Int.from_u64(self:bounded_word(Int.to_u64(n)))
end

--- Uniform Int in [0, span) for any span >= 1 (VER05 7.2).
--
--   span <= 2^64   bounded(span)
--   span  > 2^64   k-word rejection: k = max(2, ceil(bitlen(span-1)/64)),
--                  first draw most significant, t = (2^(64k) - span) mod span
function Rng:bounded_big(span)
  Int.check(span, "span")
  if Int.sign(span) < 1 then
    error("span must be >= 1", 2)
  end
  if Int.cmp(span, TWO64) <= 0 then
    return self:bounded(span)
  end
  local bits = Int.bit_length(Int.sub(span, 1))
  if bits > Limits.MAX_SPAN_BITS then
    error("span wider than " .. Limits.MAX_SPAN_BITS .. " bits", 2)
  end
  local words = m_max(2, (bits + 63) // 64)
  local threshold = Int.mod(Int.sub(Int.pow2(64 * words), span), span)
  while true do
    local raw = 0
    for _ = 1, words do
      raw = Int.add(Int.shl(raw, 64), Int.from_u64(self:next_word()))
    end
    if Int.cmp(raw, threshold) >= 0 then
      return Int.mod(raw, span)
    end
  end
end

--- 64-bit FNV-1a of a string (its raw bytes), as a word.
function Prng.fnv1a64_word(data)
  if type(data) ~= "string" then
    error("fnv1a64 expects a string, not " .. type(data), 2)
  end
  local hash = FNV_OFFSET_BASIS
  for i = 1, #data do
    hash = (hash ~ s_byte(data, i)) * FNV_PRIME
  end
  return hash
end

--- 64-bit FNV-1a of a string, as an Int.
function Prng.fnv1a64(data)
  return Int.from_u64(Prng.fnv1a64_word(data))
end

--- sub_seed(run_seed, id) = SplitMix64(run_seed ^ fnv1a64(id)).next_u64  (VER05 2.3)
function Prng.sub_seed(run_seed, property_id)
  Int.check(run_seed, "run_seed")
  if type(property_id) ~= "string" then
    error("property_id must be a string", 2)
  end
  local rng = Prng.new(Int.from_u64(Int.to_u64(run_seed) ~ Prng.fnv1a64_word(property_id)))
  return rng:next_u64()
end

Prng.TWO64 = TWO64
Prng.Rng = Rng

return Prng
