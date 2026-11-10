-- ============================================================================
-- coding_adventures.ct_compare -- constant-time comparison and selection (CT01)
-- ============================================================================
--
-- Why this exists
-- ---------------
-- A secret comparison that returns at the first differing byte leaks how far
-- the match got.  An attacker who controls one operand (a MAC tag, a password
-- hash, a session token) can then recover the other one byte at a time by
-- measuring how long each guess takes.  The defence is to look at EVERY byte
-- whatever the data is, folding the differences into one accumulator and
-- looking at the accumulator only once, at the very end.
--
--     a = 41 42 43 44        a = 41 42 43 44
--     b = 41 42 43 44        b = 41 FF 43 44
--     xor 00 00 00 00        xor 00 BD 00 00   <- accumulator OR-s these: 0xBD
--     acc = 0  -> equal      acc = 0xBD -> different (but we still read byte 3)
--
-- This is the Lua lane of CT01 (code/specs/CT01-constant-time-compare.md).
--
-- What this module promises -- and what it does not
-- --------------------------------------------------
-- It promises that the *source* has no content-dependent early return, no
-- content-dependent branch, no content-dependent table index and no
-- content-dependent loop bound.  Lengths are public.
--
-- It does NOT promise wall-clock constant time.  Lua is an interpreted,
-- garbage-collected language: every operation is a bytecode dispatched by the
-- VM, strings are interned/copied by the runtime, table growth triggers
-- reallocation, and the collector may run at any allocation.  Lua offers no
-- way to pin or inspect machine code.  The guarantee is therefore about
-- control flow in the Lua source only, not about cache, branch predictor or
-- instruction timing of the host.
--
-- How the selection bit is received
-- ---------------------------------
-- Lua has no branch-free way to turn a boolean into a number: `b and 1 or 0`
-- IS a branch on the secret.  So, as in the Go, Ruby and Elixir lanes, the
-- selection bit is an INTEGER that must be exactly 0 (pick `right`) or 1
-- (pick `left`), and equality is also offered as bit-returning variants
-- (ct_eq_bit, ct_eq_fixed_bit, ct_eq_u64_bit) so that
--
--     ct_select_bytes(a, b, ct_eq_bit(x, y))
--
-- never converts a boolean with a conditional anywhere.
--
-- Unsigned 64-bit values
-- ----------------------
-- Lua 5.4 integers are 64-bit two's complement and wrap on overflow.  A u64 is
-- passed as a Lua integer holding the same 64 bits; 0xFFFFFFFFFFFFFFFF is the
-- integer -1.  Floats (even 1.0) are rejected, because they cannot carry 64
-- exact bits.
-- ============================================================================

local M = {}

local byte = string.byte
local char = string.char
local math_type = math.type

-- ---------------------------------------------------------------------------
-- Input validation (reads types and ranges only, never "do two inputs match")
-- ---------------------------------------------------------------------------

-- Turn a byte-string input into a Lua string.  Accepts:
--   * a Lua string (its bytes), or
--   * a plain array table of integers 0..255 (no metatable).
-- Anything else is an error: a table with a metatable could lie about its
-- length or contents between validation and use.
local function as_bytes(value, name)
  local kind = type(value)
  if kind == "string" then
    return value
  end
  if kind == "table" and getmetatable(value) == nil then
    local n = rawlen(value)
    local parts = {}
    for i = 1, n do
      local element = rawget(value, i)
      -- (element & ~0xFF) == 0 also rejects negatives; math.type rejects
      -- floats, strings and booleans before the bitwise op could coerce them.
      if math_type(element) ~= "integer" or (element & ~0xFF) ~= 0 then
        error(name .. "[" .. i .. "] must be an integer from 0 to 255", 3)
      end
      parts[i] = char(element)
    end
    return table.concat(parts)
  end
  error(name .. " must be a string or an array of byte values", 3)
end

local function check_u64(value, name)
  if math_type(value) ~= "integer" then
    error(name .. " must be an integer holding 64 bits", 3)
  end
end

-- Fold the XOR of every byte pair into one accumulator in 0..255.
-- Precondition: both strings have the same (public) length.
local function difference(left, right)
  local acc = 0
  for i = 1, #left do
    acc = acc | (byte(left, i) ~ byte(right, i))
  end
  return acc
end

-- ---------------------------------------------------------------------------
-- Equality
-- ---------------------------------------------------------------------------

-- 1 when the byte strings are equal, else 0.  A length mismatch is 0 and reads
-- neither input; lengths are public.
--
-- For acc in 0..255, (acc - 1) is -1 (all 64 bits set) exactly when acc == 0,
-- and otherwise 0..254 (top bit clear).  A LOGICAL shift right by 63 (Lua's >>
-- is logical) therefore yields 1 for "equal" and 0 for "different", with no
-- comparison and no conditional.
function M.ct_eq_bit(left, right)
  local l = as_bytes(left, "left")
  local r = as_bytes(right, "right")
  if #l ~= #r then
    return 0
  end
  return (difference(l, r) - 1) >> 63
end

-- Fixed-width companion.  Lua has no fixed-size array type, so this is the
-- same operation as ct_eq_bit; it exists so call sites read the same in every
-- CT01 lane.
function M.ct_eq_fixed_bit(left, right)
  return M.ct_eq_bit(left, right)
end

-- Boolean equality, without early exit.  The boolean is the PUBLIC result.
function M.ct_eq(left, right)
  return M.ct_eq_bit(left, right) == 1
end

function M.ct_eq_fixed(left, right)
  return M.ct_eq_bit(left, right) == 1
end

-- ---------------------------------------------------------------------------
-- Selection
-- ---------------------------------------------------------------------------

-- Select `left` when choice == 1 and `right` when choice == 0, returning a
-- fresh string.  (Lua strings are immutable, so freshness cannot be observed;
-- the result never aliases an input in any way a caller can mutate.)
--
-- The mask trick: -1 & 0xFF = 0xFF and -0 & 0xFF = 0x00, so
--
--     out = right ~ ((left ~ right) & mask)
--
-- is `left` under mask 0xFF and `right` under mask 0x00, using the same
-- instructions either way.
--
-- choice must be an integer with all bits except bit 0 clear.  The test
-- `(choice & ~1) ~= 0` is one computation whose result is the same for both
-- valid values, so a valid call does identical work whichever bit is secret.
function M.ct_select_bytes(left, right, choice)
  local l = as_bytes(left, "left")
  local r = as_bytes(right, "right")
  if #l ~= #r then
    error("CT_SELECT_LENGTH_MISMATCH: ct_select_bytes requires equal-length byte strings", 2)
  end
  if math_type(choice) ~= "integer" or (choice & ~1) ~= 0 then
    error("ct_select_bytes requires choice to be the integer 0 or 1", 2)
  end
  local mask = -choice & 0xFF
  local out = {}
  for i = 1, #l do
    local lb = byte(l, i)
    local rb = byte(r, i)
    out[i] = char(rb ~ ((lb ~ rb) & mask))
  end
  return table.concat(out)
end

-- ---------------------------------------------------------------------------
-- Unsigned 64-bit equality
-- ---------------------------------------------------------------------------

-- d = left XOR right is zero exactly when the values are equal.  (d | -d) has
-- its top bit set exactly when d ~= 0 (for d ~= 0 either d or -d is negative
-- as a signed word), so shifting the top bit down and flipping it gives the
-- equality bit.  All 64 bit positions are folded by one fixed operation
-- sequence.
function M.ct_eq_u64_bit(left, right)
  check_u64(left, "left")
  check_u64(right, "right")
  local d = left ~ right
  return ((d | -d) >> 63) ~ 1
end

function M.ct_eq_u64(left, right)
  return M.ct_eq_u64_bit(left, right) == 1
end

return M
