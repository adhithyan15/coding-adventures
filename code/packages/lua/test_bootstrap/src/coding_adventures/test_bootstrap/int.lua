--- Mathematical integers over Lua's 64-bit integers (VER05o section 3).
--
-- Lua 5.4 integers are 64-bit, SIGNED and WRAPPING.  The property engine needs
-- three things that is not:
--
--   * unsigned 64-bit words (the PRNG outputs 0 .. 2^64-1),
--   * the 128-bit `i128` generator and spans wider than 2^64 for rejection
--     sampling,
--   * decimal integers of up to 1,000 digits read from the corpus.
--
-- So an `Int` is either
--
--   * a native Lua integer, when the value is in [-2^63, 2^63-1], or
--   * a "Big": a private sign-magnitude record whose magnitude is a little-endian
--     array of 24-bit limbs.
--
-- The representation is NORMALIZED: a value that fits in a native integer is
-- ALWAYS native.  Two equal Ints are therefore either two equal native integers
-- or two Bigs with equal limbs; use `Int.eq`, never `==`, for the general case
-- (a Big is a table).
--
-- Why 24-bit limbs?  A product of two limbs is below 2^48, so a whole column of
-- schoolbook multiplication, plus carries, stays far inside a signed 64-bit
-- integer with no overflow reasoning at all.
--
-- A FLOAT is not an Int, even `3.0`: `Int.is(3.0)` is false and every function
-- here that receives a float raises, so a float can never slip into the PRNG
-- path.  (Lua also folds `t[3.0]` into `t[3]`; that is another reason the engine
-- never uses an Int as a table key.)
--
-- Words (unsigned 64-bit patterns held in Lua integers) are converted with
-- `Int.from_u64` / `Int.to_u64`.  A word is never compared with `<`.

-- Standard functions are captured at load so that a property that patches the
-- string metatable or a library table cannot make the engine raise (VER05o 4).
local m_max, s_find, s_format, s_sub, t_concat = math.max, string.find, string.format, string.sub, table.concat
local type, error, setmetatable, tonumber = type, error, setmetatable, tonumber

local Int = {}

--- Widest value `shl`/`pow2` will build, and most digits `parse` accepts by default
-- (CWE-400: the guards live here, so no caller has to remember them).
Int.MAX_BITS = 1 << 20
Int.MAX_PARSE_DIGITS = 1000

local mtype = math.type
local BASE = 1 << 24
local MASK = BASE - 1

-- Registry of Big tables.  Membership is by identity, so a user table can never
-- be mistaken for a Big and no metamethod is involved.
local BIGS = setmetatable({}, { __mode = "k" })

-- -------------------------------------------------------------------------
-- Magnitudes: arrays of limbs, least significant first, no trailing zero limb.
-- Zero is the empty array.
-- -------------------------------------------------------------------------

local function trim(m)
  local n = #m
  while n > 0 and m[n] == 0 do
    m[n] = nil
    n = n - 1
  end
  return m
end

-- `w` is an unsigned 64-bit pattern; `>>` is a logical shift, so a "negative"
-- Lua integer is read as its unsigned value.
local function mag_from_word(w)
  local m, i = {}, 0
  while w ~= 0 do
    i = i + 1
    m[i] = w & MASK
    w = w >> 24
  end
  return m
end

local function mag_cmp(a, b)
  local na, nb = #a, #b
  if na ~= nb then
    return na < nb and -1 or 1
  end
  for i = na, 1, -1 do
    local x, y = a[i], b[i]
    if x ~= y then
      return x < y and -1 or 1
    end
  end
  return 0
end

local function mag_add(a, b)
  local r, carry = {}, 0
  local n = m_max(#a, #b)
  for i = 1, n do
    local s = (a[i] or 0) + (b[i] or 0) + carry
    r[i] = s & MASK
    carry = s >> 24
  end
  if carry > 0 then
    r[n + 1] = carry
  end
  return r
end

-- a >= b is required.
local function mag_sub(a, b)
  local r, borrow = {}, 0
  for i = 1, #a do
    local d = a[i] - (b[i] or 0) - borrow
    if d < 0 then
      d = d + BASE
      borrow = 1
    else
      borrow = 0
    end
    r[i] = d
  end
  return trim(r)
end

-- a * m + add, for m and add below 2^24.
local function mag_mul_small_add(a, m, add)
  local r, carry = {}, add
  local n = #a
  for i = 1, n do
    local p = a[i] * m + carry
    r[i] = p & MASK
    carry = p >> 24
  end
  local i = n
  while carry > 0 do
    i = i + 1
    r[i] = carry & MASK
    carry = carry >> 24
  end
  return r
end

local function mag_mul(a, b)
  local na, nb = #a, #b
  if na == 0 or nb == 0 then
    return {}
  end
  local r = {}
  for i = 1, na + nb do
    r[i] = 0
  end
  for i = 1, na do
    local carry, ai = 0, a[i]
    for j = 1, nb do
      local cur = r[i + j - 1] + ai * b[j] + carry
      r[i + j - 1] = cur & MASK
      carry = cur >> 24
    end
    r[i + nb] = r[i + nb] + carry
  end
  return trim(r)
end

local function mag_shl(a, k)
  if #a == 0 then
    return {}
  end
  local limbs, bits = k // 24, k % 24
  local r = {}
  for i = 1, limbs do
    r[i] = 0
  end
  local carry = 0
  for i = 1, #a do
    local v = (a[i] << bits) | carry
    r[limbs + i] = v & MASK
    carry = v >> 24
  end
  if carry > 0 then
    r[limbs + #a + 1] = carry
  end
  return r
end

local function mag_bitlen(a)
  local n = #a
  if n == 0 then
    return 0
  end
  local top, bits = a[n], 0
  while top > 0 do
    bits = bits + 1
    top = top >> 1
  end
  return (n - 1) * 24 + bits
end

-- Divides by a single limb (d < 2^24).
local function mag_divmod_small(a, d)
  local n = #a
  local q = {}
  for i = 1, n do
    q[i] = 0
  end
  local rem = 0
  for i = n, 1, -1 do
    local cur = (rem << 24) | a[i]
    q[i] = cur // d
    rem = cur % d
  end
  return trim(q), rem
end

-- Schoolbook binary long division.  O(bits * limbs): only ever used for spans
-- above 2^64 and for decimal output, both rare and small.
local function mag_divmod(a, b)
  if mag_cmp(a, b) < 0 then
    return {}, a
  end
  if #b == 1 then
    local q, r = mag_divmod_small(a, b[1])
    return q, (r == 0) and {} or { r }
  end
  local q, r = {}, {}
  for i = 1, #a do
    q[i] = 0
  end
  for bit = mag_bitlen(a) - 1, 0, -1 do
    local limb = bit // 24 + 1
    local off = bit % 24
    r = mag_mul_small_add(r, 2, (a[limb] >> off) & 1)
    if mag_cmp(r, b) >= 0 then
      r = mag_sub(r, b)
      q[limb] = q[limb] | (1 << off)
    end
  end
  return trim(q), r
end

-- The low 64 bits of a magnitude, as a word (wrapping).
local function mag_low_word(m)
  return (m[1] or 0) | ((m[2] or 0) << 24) | ((m[3] or 0) << 48)
end

-- -------------------------------------------------------------------------
-- Native <-> Big
-- -------------------------------------------------------------------------

local function mkbig(neg, mag)
  local b = { neg = neg, mag = mag }
  BIGS[b] = true
  return b
end

-- Normalizes a (sign, magnitude) pair: native when it fits in an int64.
local function norm(neg, mag)
  local bits = mag_bitlen(mag)
  if bits == 0 then
    return 0
  end
  if bits <= 63 then
    local u = 0
    for i = #mag, 1, -1 do
      u = (u << 24) | mag[i]
    end
    return neg and -u or u
  end
  if bits == 64 and neg then
    local u = 0
    for i = #mag, 1, -1 do
      u = (u << 24) | mag[i]
    end
    if u == math.mininteger then
      return math.mininteger
    end
  end
  return mkbig(neg, mag)
end

-- Sign and magnitude of any valid Int.  Floats and everything else are refused.
local function parts(x)
  if mtype(x) == "integer" then
    if x < 0 then
      return true, mag_from_word(0 - x) -- 0 - minint is minint: read unsigned, 2^63
    end
    return false, mag_from_word(x)
  end
  if type(x) == "table" and BIGS[x] then
    return x.neg, x.mag
  end
  error("not an Int: " .. type(x) .. (mtype(x) == "float" and " (a float is never an Int)" or ""), 3)
end

-- -------------------------------------------------------------------------
-- Public API
-- -------------------------------------------------------------------------

--- True for a native Lua integer or a Big.  Floats are not Ints.
function Int.is(x)
  return mtype(x) == "integer" or (type(x) == "table" and BIGS[x] == true)
end

--- Raises a TypeError-style error unless `x` is an Int.
function Int.check(x, name)
  if not Int.is(x) then
    local what = mtype(x) == "float" and "a float" or type(x)
    error((name or "value") .. " must be an Integer, not " .. what, 3)
  end
  return x
end

--- True for a Big (a value outside the int64 range).
function Int.is_big(x)
  return type(x) == "table" and BIGS[x] == true
end

function Int.from_u64(w)
  if w >= 0 then
    return w
  end
  return norm(false, mag_from_word(w))
end

--- The low 64 bits of `x` in two's complement, as a word (wrapping).
function Int.to_u64(x)
  if mtype(x) == "integer" then
    return x
  end
  local neg, mag = parts(x)
  local u = mag_low_word(mag)
  if neg then
    u = 0 - u
  end
  return u
end

function Int.cmp(a, b)
  if mtype(a) == "integer" and mtype(b) == "integer" then
    return a < b and -1 or (a > b and 1 or 0)
  end
  local na, ma = parts(a)
  local nb, mb = parts(b)
  if #ma == 0 and #mb == 0 then
    return 0
  end
  if na ~= nb then
    return na and -1 or 1
  end
  local c = mag_cmp(ma, mb)
  return na and -c or c
end

function Int.eq(a, b)
  return Int.cmp(a, b) == 0
end

function Int.lt(a, b)
  return Int.cmp(a, b) < 0
end

function Int.le(a, b)
  return Int.cmp(a, b) <= 0
end

function Int.sign(a)
  if mtype(a) == "integer" then
    return a < 0 and -1 or (a > 0 and 1 or 0)
  end
  local neg, mag = parts(a)
  if #mag == 0 then
    return 0
  end
  return neg and -1 or 1
end

local function signed_add(na, ma, nb, mb)
  if na == nb then
    return norm(na, mag_add(ma, mb))
  end
  local c = mag_cmp(ma, mb)
  if c == 0 then
    return 0
  elseif c > 0 then
    return norm(na, mag_sub(ma, mb))
  end
  return norm(nb, mag_sub(mb, ma))
end

function Int.add(a, b)
  if mtype(a) == "integer" and mtype(b) == "integer" then
    local r = a + b
    -- signed overflow iff both operands differ in sign from the result
    if ((a ~ r) & (b ~ r)) >= 0 then
      return r
    end
  end
  local na, ma = parts(a)
  local nb, mb = parts(b)
  return signed_add(na, ma, nb, mb)
end

function Int.sub(a, b)
  if mtype(a) == "integer" and mtype(b) == "integer" then
    local r = a - b
    -- signed overflow iff the operands differ in sign and the result differs from a
    if ((a ~ b) & (a ~ r)) >= 0 then
      return r
    end
  end
  local na, ma = parts(a)
  local nb, mb = parts(b)
  return signed_add(na, ma, not nb, mb)
end

function Int.neg(a)
  if mtype(a) == "integer" and a ~= math.mininteger then
    return -a
  end
  local n, m = parts(a)
  return norm(not n, m)
end

function Int.abs(a)
  if Int.sign(a) < 0 then
    return Int.neg(a)
  end
  return a
end

function Int.mul(a, b)
  local na, ma = parts(a)
  local nb, mb = parts(b)
  return norm(na ~= nb, mag_mul(ma, mb))
end

--- a * 2^k for k >= 0.
function Int.shl(a, k)
  if mtype(k) ~= "integer" or k < 0 then
    error("shift count must be a non-negative integer", 2)
  end
  local n, m = parts(a)
  if mag_bitlen(m) + k > Int.MAX_BITS then
    error("shift result wider than " .. Int.MAX_BITS .. " bits", 2)
  end
  return norm(n, mag_shl(m, k))
end

--- 2^k for k >= 0.
function Int.pow2(k)
  if mtype(k) ~= "integer" or k < 0 or k >= Int.MAX_BITS then
    error("2^k needs an integer 0 <= k < " .. Int.MAX_BITS, 2)
  end
  return norm(false, mag_shl({ 1 }, k))
end

--- Number of bits of |a| (0 for 0).
function Int.bit_length(a)
  local _, m = parts(a)
  return mag_bitlen(m)
end

function Int.is_even(a)
  if mtype(a) == "integer" then
    return (a & 1) == 0
  end
  local _, m = parts(a)
  return ((m[1] or 0) & 1) == 0
end

--- Floored division and modulo (the sign of the remainder follows the divisor,
-- as Lua's `//` and `%` and Python's divmod do).  `b` must not be zero.
function Int.divmod(a, b)
  if Int.sign(b) == 0 then
    error("division by zero", 2)
  end
  if mtype(a) == "integer" and mtype(b) == "integer" and b ~= -1 then
    return a // b, a % b
  end
  local na, ma = parts(a)
  local nb, mb = parts(b)
  local q, r = mag_divmod(ma, mb)
  local rz = (#r == 0)
  if na == nb then
    -- remainder takes the divisor's sign (here also the dividend's)
    return norm(false, q), norm(nb, r)
  end
  if rz then
    return norm(true, q), 0
  end
  -- signs differ and the division is inexact: round the quotient down
  return norm(true, mag_add(q, { 1 })), norm(nb, mag_sub(mb, r))
end

function Int.mod(a, b)
  local _, r = Int.divmod(a, b)
  return r
end

--- floor(a / 2), exact for negatives (-3 -> -2): the bisection step of the
-- integer shrinker.  `>>` would be a LOGICAL shift on a negative Lua integer.
function Int.floor_half(a)
  if mtype(a) == "integer" then
    return a // 2
  end
  local q = Int.divmod(a, 2)
  return q
end

--- Canonical decimal text -> Int.  The caller has validated the grammar; this
-- only refuses what it cannot represent.
function Int.parse(text, max_digits)
  max_digits = max_digits or Int.MAX_PARSE_DIGITS
  if type(text) ~= "string" then
    error("Int.parse needs a string", 2)
  end
  local neg = false
  local digits = text
  if s_sub(text, 1, 1) == "-" then
    neg = true
    digits = s_sub(text, 2)
  end
  if digits == "" or s_find(digits, "%D") then
    error("not a decimal integer", 2)
  end
  if #digits > max_digits then
    error("integer of more than " .. max_digits .. " digits", 2)
  end
  local mag = {}
  local pos = 1
  local first = #digits % 7
  if first == 0 then
    first = 7
  end
  local len = first
  while pos <= #digits do
    local chunk = tonumber(s_sub(digits, pos, pos + len - 1))
    local mult = 1
    for _ = 1, len do
      mult = mult * 10
    end
    mag = mag_mul_small_add(mag, mult, chunk)
    pos = pos + len
    len = 7
  end
  return norm(neg, trim(mag))
end

--- Int -> canonical decimal text.
function Int.tostring(a)
  if mtype(a) == "integer" then
    return s_format("%d", a)
  end
  local neg, mag = parts(a)
  if #mag == 0 then
    return "0"
  end
  local pieces = {}
  while #mag > 0 do
    local r
    mag, r = mag_divmod_small(mag, 10000000)
    pieces[#pieces + 1] = (#mag > 0) and s_format("%07d", r) or s_format("%d", r)
  end
  local out = {}
  for i = #pieces, 1, -1 do
    out[#out + 1] = pieces[i]
  end
  return (neg and "-" or "") .. t_concat(out)
end

return Int
