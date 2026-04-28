-- coding_adventures.rng
-- ============================================================================
--
-- THREE CLASSIC PSEUDORANDOM NUMBER GENERATORS
--
-- This module implements three well-known PRNGs, each offering a different
-- trade-off between simplicity, period length, and statistical quality.
--
-- ## The Algorithms
--
--   LCG (Linear Congruential Generator, Knuth 1948)
--   ─────────────────────────────────────────────────
--   The simplest useful PRNG. State advances via:
--
--     state = (state × a + c) mod 2^64
--
--   Output is the upper 32 bits of state. Fast and full-period (every 64-bit
--   value appears exactly once per cycle), but consecutive outputs are
--   correlated — not suitable for simulation or cryptography.
--
--   Constants (Knuth / Numerical Recipes, satisfy Hull-Dobell theorem):
--     a = 6364136223846793005
--     c = 1442695040888963407
--
--   Xorshift64 (Marsaglia 2003)
--   ────────────────────────────
--   Three XOR-shift operations scramble 64-bit state with no multiplication:
--
--     x ^= x << 13
--     x ^= x >> 7
--     x ^= x << 17
--
--   Period: 2^64 − 1. State 0 is a fixed point (0 XOR 0 = 0 forever), so
--   seed 0 is replaced with 1. Output is the lower 32 bits.
--
--   PCG32 (O'Neill 2014)
--   ──────────────────────
--   Uses the same LCG recurrence but applies an XSH RR output permutation
--   (XOR-Shift High / Random Rotate) before returning:
--
--     1. xorshifted = ((old >> 18) ^ old) >> 27   — mix high bits down
--     2. rot        = old >> 59                    — 5-bit rotation amount
--     3. output     = rotr32(xorshifted, rot)      — scatter all bits
--
--   Passes all known statistical test suites (TestU01 BigCrush, PractRand).
--
-- ## Lua Integer Notes
--
--   Lua 5.4 uses 64-bit signed integers. Arithmetic on integers wraps
--   automatically modulo 2^64 — no explicit masking needed for + and *.
--   Bitwise operators (&, |, ~, <<, >>) also operate on the full 64-bit
--   pattern. For 32-bit output we mask: value & 0xFFFFFFFF.
--   For unsigned right-shift of a 64-bit value, Lua's >> is logical (zero-fill),
--   which is what we need.
--
-- ## Usage
--
--   local rng = require("coding_adventures.rng")
--
--   local g = rng.LCG.new(42)
--   print(g:next_u32())          -- uint32 in [0, 2^32)
--   print(g:next_u64())          -- uint64 in [0, 2^64) as Lua integer
--   print(g:next_float())        -- float in [0.0, 1.0)
--   print(g:next_int_in_range(1, 6))  -- integer in [1, 6]
--
--   -- Same API for Xorshift64 and PCG32:
--   local xs = rng.Xorshift64.new(42)
--   local pcg = rng.PCG32.new(42)
--
-- ============================================================================

local M = {}

M.VERSION = "0.1.0"

-- ============================================================================
-- CONSTANTS
-- ============================================================================

-- LCG_MULTIPLIER and LCG_INCREMENT are the Knuth / Numerical Recipes constants.
-- Together they satisfy the Hull-Dobell theorem: full period 2^64.
--
-- Note: Lua integer literals must be written carefully for large values.
-- These are the exact 64-bit patterns needed:
local LCG_MULTIPLIER = 6364136223846793005  -- 0x5851F42D4C957F2D
local LCG_INCREMENT  = 1442695040888963407  -- 0x14057B7EF767814F

-- FLOAT_DIV: divides a uint32 to produce a float in [0.0, 1.0).
local FLOAT_DIV = 4294967296.0  -- 2^32

-- ============================================================================
-- INTERNAL HELPERS
-- ============================================================================

-- mask32(v): extract the lower 32 bits of a 64-bit integer as an unsigned
-- value. The result is a Lua integer in the range [0, 2^32 - 1].
local function mask32(v)
    return v & 0xFFFFFFFF
end

-- rotr32(v, rot): rotate 32-bit value v right by rot positions.
--
-- A rotation is like a shift, except bits that fall off the right end
-- reappear on the left. For a 32-bit value:
--
--   rotr32(v, r) = (v >> r) | (v << (32 - r))
--
-- We must mask to 32 bits because Lua operates on 64-bit integers.
-- The rotation amount is already a small number (0-31 for PCG32).
local function rotr32(v, rot)
    rot = rot & 31   -- ensure rotation is in [0, 31]
    return mask32((v >> rot) | (v << (32 - rot)))
end

-- lcg_advance(state): one step of the LCG recurrence.
-- Returns the new state (64-bit, wraps mod 2^64 automatically).
local function lcg_advance(state)
    return state * LCG_MULTIPLIER + LCG_INCREMENT
end

-- rejection_sample(next_u32_fn, min, max): uniform integer in [min, max].
--
-- ## Why Rejection Sampling?
--
-- Naïve modulo (value % range) over-samples low values whenever 2^32 is not
-- evenly divisible by range. For example, with range = 3:
--   2^32 = 4294967296 = 3 × 1431655765 + 1
-- So value 0 would be drawn 1431655766 times vs. 1431655765 for 1 and 2.
--
-- Fix: compute the "rejection threshold" — the smallest multiple of range
-- reachable by a uint32. Discard any draw below threshold. Expected extra
-- draws per call is < 2 for all range sizes.
--
--   threshold = (2^32 - range) % range  =  (-range) % range  (mod 2^32)
--
local function rejection_sample(next_u32_fn, min_val, max_val)
    if min_val > max_val then
        error("next_int_in_range requires min_val <= max_val, got " .. min_val .. " > " .. max_val, 2)
    end
    local min, max = min_val, max_val
    local range = max - min + 1
    -- (-range) mod range in 32-bit arithmetic:
    -- In Lua we compute: (2^32 - range) % range, but since range fits in 32
    -- bits and Lua uses 64-bit integers, we can just do:
    local u_range = range & 0xFFFFFFFF
    -- threshold = (2^32 mod range) = (-u_range mod u_range) mod 2^32
    -- Equivalent: mask32((-u_range)) % u_range
    local neg_range = mask32(-u_range)
    local threshold = neg_range % u_range
    while true do
        local r = next_u32_fn()
        if r >= threshold then
            return min + (r % range)
        end
    end
end

-- ============================================================================
-- LCG — Linear Congruential Generator
-- ============================================================================

-- LCG is the simplest PRNG that has a full 2^64 period. Despite its
-- simplicity, it is adequate for non-cryptographic simulations and
-- serves as the backbone for the more sophisticated PCG32.
--
-- The output function discards the lower 32 bits because they have shorter
-- sub-periods — e.g., the lowest bit simply alternates 0-1-0-1... The upper
-- 32 bits are much better distributed.

local LCG = {}
LCG.__index = LCG

-- LCG.new(seed): create a new LCG seeded with the given integer.
-- Any seed value (including 0) is valid.
function LCG.new(seed)
    seed = math.tointeger(seed) or 0
    return setmetatable({ _state = seed }, LCG)
end

-- next_u32(): advance LCG state and return upper 32 bits.
--
--   new_state = state × a + c   (mod 2^64, wraps automatically)
--   output    = new_state >> 32
function LCG:next_u32()
    self._state = lcg_advance(self._state)
    return mask32(self._state >> 32)
end

-- next_u64(): combine two consecutive next_u32 calls: (hi << 32) | lo.
function LCG:next_u64()
    local hi = self:next_u32()
    local lo = self:next_u32()
    return (hi << 32) | lo
end

-- next_float(): return a float64 in [0.0, 1.0).
function LCG:next_float()
    return self:next_u32() / FLOAT_DIV
end

-- next_int_in_range(min, max): uniform integer in [min, max] inclusive.
-- Uses rejection sampling to eliminate modulo bias.
function LCG:next_int_in_range(min, max)
    return rejection_sample(function() return self:next_u32() end, min, max)
end

M.LCG = LCG

-- ============================================================================
-- Xorshift64 — Marsaglia XOR-Shift Generator
-- ============================================================================

-- Xorshift64 uses three carefully chosen XOR-shift constants to achieve
-- a maximal period of 2^64 - 1 with no multiplication, just bit operations.
--
-- The three shift amounts (13, 7, 17) were found by Marsaglia's exhaustive
-- search over all triples. Each shift scatters bits that previous shifts
-- left untouched.
--
-- State 0 is a fixed point: 0 XOR anything = 0, so the generator stays at 0
-- forever. We replace seed 0 with 1.

local Xorshift64 = {}
Xorshift64.__index = Xorshift64

-- Xorshift64.new(seed): create a new Xorshift64. Seed 0 → replaced with 1.
function Xorshift64.new(seed)
    seed = math.tointeger(seed) or 0
    if seed == 0 then seed = 1 end
    return setmetatable({ _state = seed }, Xorshift64)
end

-- next_u32(): apply three XOR-shifts; return lower 32 bits.
--
--   x ^= x << 13
--   x ^= x >> 7
--   x ^= x << 17
--   output = lower 32 bits of x
function Xorshift64:next_u32()
    local x = self._state
    x = x ~ (x << 13)   -- ~ is XOR in Lua 5.4
    x = x ~ (x >> 7)
    x = x ~ (x << 17)
    self._state = x
    return mask32(x)
end

-- next_u64(): two consecutive next_u32 calls.
function Xorshift64:next_u64()
    local hi = self:next_u32()
    local lo = self:next_u32()
    return (hi << 32) | lo
end

-- next_float(): float in [0.0, 1.0).
function Xorshift64:next_float()
    return self:next_u32() / FLOAT_DIV
end

-- next_int_in_range(min, max): uniform integer in [min, max].
function Xorshift64:next_int_in_range(min, max)
    return rejection_sample(function() return self:next_u32() end, min, max)
end

M.Xorshift64 = Xorshift64

-- ============================================================================
-- PCG32 — Permuted Congruential Generator
-- ============================================================================

-- PCG32 wraps the same LCG recurrence in a clever output permutation called
-- XSH RR (XOR-Shift High / Random Rotate). The key insight:
--
--   1. Use the LCG to advance state (good mixing).
--   2. Use the HIGH bits of the OLD state to permute the output.
--
-- Why use the old state instead of the new?  Because the rotation amount
-- (bits 59-63) and the value being rotated come from different bit positions
-- of the same word, giving them statistical independence.
--
-- XSH RR step by step:
--   old       = 64-bit state before advance
--   xs        = (old >> 18) ^ old           -- mix bits 18-63 with bits 0-45
--   xorshift  = xs >> 27                    -- keep upper 32 bits of mix
--   rot       = old >> 59                   -- rotation amount (0-31)
--   output    = rotr32(xorshift, rot)       -- rotate right by rot
--
-- Initialization (initseq warm-up):
--   1. Advance once from state=0 to stir in the increment.
--   2. Add the seed to state.
--   3. Advance once more to scatter seed bits.
-- This ensures even seed 0 or 1 produce well-distributed sequences.

local PCG32 = {}
PCG32.__index = PCG32

-- PCG32.new(seed): create a new PCG32 with the given seed.
-- The increment is fixed to LCG_INCREMENT (already odd, ensuring full period).
function PCG32.new(seed)
    seed = math.tointeger(seed) or 0
    local inc = LCG_INCREMENT  -- already odd (bit 0 is set)
    local g = setmetatable({ _state = 0, _increment = inc }, PCG32)
    -- Warm-up step 1: advance from state=0 to mix in increment
    g._state = lcg_advance(g._state)
    -- Warm-up step 2: add seed to state
    g._state = g._state + seed
    -- Warm-up step 3: advance once more to scatter seed bits
    g._state = lcg_advance(g._state)
    return g
end

-- next_u32(): advance PCG32 state and return XSH RR permuted 32-bit output.
function PCG32:next_u32()
    local old = self._state
    -- Advance the LCG
    self._state = old * LCG_MULTIPLIER + self._increment

    -- XSH RR permutation on the OLD state
    -- Step 1: xorshift — mix bits 18-63 of old into bits 0-45, then take top 32
    local xorshifted = mask32(((old >> 18) ~ old) >> 27)
    -- Step 2: rotation amount from the top 5 bits (bits 59-63)
    local rot = old >> 59
    -- Step 3: rotate right
    return rotr32(xorshifted, rot)
end

-- next_u64(): two consecutive next_u32 calls.
function PCG32:next_u64()
    local hi = self:next_u32()
    local lo = self:next_u32()
    return (hi << 32) | lo
end

-- next_float(): float in [0.0, 1.0).
function PCG32:next_float()
    return self:next_u32() / FLOAT_DIV
end

-- next_int_in_range(min, max): uniform integer in [min, max].
function PCG32:next_int_in_range(min, max)
    return rejection_sample(function() return self:next_u32() end, min, max)
end

M.PCG32 = PCG32

-- ============================================================================

return M
