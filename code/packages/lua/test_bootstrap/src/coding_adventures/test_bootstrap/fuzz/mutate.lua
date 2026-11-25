--- The mutator algebra (VER06 section 3, VER06p section 3).
--
-- Every mutator takes a Lua string (read as raw bytes) and returns a string.  Lua
-- strings are immutable values, so no mutator can modify its argument.  Offsets,
-- counts, deltas and values are `Int`s: native 64-bit integers or Bigs of up to the
-- corpus's 1,000 digits.  Offsets wrap with floored `mod`.
--
-- No big integer is ever built from INPUT BYTES (the Elixir lane crashed on one of
-- 2^(8 x 1 MiB)): the data is only ever read as bytes.  Arithmetic and boundary
-- values are reduced to their low 64 bits and masked to 8*width bits, which is exact
-- for every width the algebra has (1, 2, 4, 8).

-- Standard functions are captured at load so that a target that patches the string
-- metatable or a library table cannot make the engine raise (VER06p section 1).
local m_max, m_min, m_type, m_maxint, m_minint = math.max, math.min, math.type, math.maxinteger, math.mininteger
local s_sub, s_gsub, s_char, s_byte, s_pack, s_unpack = string.sub, string.gsub, string.char, string.byte, string.pack, string.unpack
local type, error = type, error

local Int = require("coding_adventures.test_bootstrap.int")
local Prng = require("coding_adventures.test_bootstrap.prng")
local Limits = require("coding_adventures.test_bootstrap.fuzz.limits")
-- captured at load: a target that rewrites Limits later changes nothing here
local limits_MAX_ITERATIONS, limits_MAX_INPUT_BYTES, limits_MAX_SEQUENCE_WORK = Limits.MAX_ITERATIONS, Limits.MAX_INPUT_BYTES, Limits.MAX_SEQUENCE_WORK
-- captured at load: a target that rewrites Prng later changes nothing here
local prng_new_rng = Prng.new_rng
-- captured at load: a target that rewrites Int later changes nothing here
local int_is, int_mod, int_tostring, int_sign, int_divmod, int_is_even, int_to_u64, int_cmp = Int.is, Int.mod, Int.tostring, Int.sign, Int.divmod, Int.is_even, Int.to_u64, Int.cmp

local mutate_bit_flip, mutate_byte_flip, mutate_byte_arithmetic, mutate_boundary_insert, mutate_block_delete, mutate_block_insert, mutate_block_duplicate, mutate_splice, mutate_mutate_sequence_counted, mutate_mutate_sequence
local Mutate = {}

-- Interest values of section 3.4, by width in bytes (1-based Lua arrays).
Mutate.INTEREST_8 = { 0, 1, 127, 128, 255 }
Mutate.INTEREST_16 = { 0, 1, 255, 256, 32767, 32768, 65535 }
Mutate.INTEREST_32 = { 0, 1, 65535, 65536, 0x7FFFFFFF, 0x80000000, 0xFFFFFFFF }
-- 64-bit values above 2^63 are written as the WORD (two's complement) they pack to.
Mutate.INTEREST_64 = { 0, 1, 0xFFFFFFFF, 0x100000000, 0x7FFFFFFFFFFFFFFF, 0x8000000000000000, -1 }
-- Arithmetic deltas of section 3.3.
Mutate.DELTAS = { -35, -8, -1, 1, 8, 35 }

local WIDTHS = { [1] = true, [2] = true, [4] = true, [8] = true }

-- 255 - byte, as a gsub replacement table: one C-speed pass inverts any run of bytes.
local INVERT = {}
for i = 0, 255 do
  INVERT[s_char(i)] = s_char(255 - i)
end

local function check_bytes(value, name)
  if type(value) ~= "string" then
    error(name .. " must be a string, not " .. type(value), 3)
  end
  return value
end

local function check_int(value, name)
  if not int_is(value) then
    error(name .. " must be an Integer, not " .. (m_type(value) == "float" and "a float" or type(value)), 3)
  end
  return value
end

local function endian_of(endian)
  if endian == "little" then
    return "<"
  elseif endian == "big" then
    return ">"
  end
  error("endian must be 'little' or 'big'", 3)
end

-- x mod n for an Int x and a native n >= 1; the result is a native in [0, n).
local function mod_n(x, n)
  if m_type(x) == "integer" then
    return x % n
  end
  return int_mod(x, n)
end

-- An Int as a native integer, saturating at the int64 bounds.
local function native(x)
  if m_type(x) == "integer" then
    return x
  end
  return int_sign(x) < 0 and m_minint or m_maxint
end

-- `x` clamped into the native range [lo, hi].
local function clamp(x, lo, hi)
  return m_max(lo, m_min(hi, native(x)))
end

local function invert(s)
  return (s_gsub(s, ".", INVERT))
end

-- Inverts the bytes [a, b) (0-based, half open) of `s`.
local function invert_bytes(s, a, b)
  if b <= a then
    return s
  end
  return s_sub(s, 1, a) .. invert(s_sub(s, a + 1, b)) .. s_sub(s, b + 1)
end

-- Inverts the bits [lo, hi) (0-based, half open, bit i = bit i%8 of byte i//8) of `s`.
local function invert_bits(s, lo, hi)
  if hi <= lo then
    return s
  end
  local first, last = lo // 8, (hi - 1) // 8
  if first == last then
    local mask = ((1 << (hi - lo)) - 1) << (lo % 8)
    return s_sub(s, 1, first) .. s_char(s_byte(s, first + 1) ~ mask) .. s_sub(s, first + 2)
  end
  local head_mask = (0xFF << (lo % 8)) & 0xFF
  local tail_mask = (1 << ((hi - 1) % 8 + 1)) - 1
  return s_sub(s, 1, first)
    .. s_char(s_byte(s, first + 1) ~ head_mask)
    .. invert(s_sub(s, first + 2, last))
    .. s_char(s_byte(s, last + 1) ~ tail_mask)
    .. s_sub(s, last + 2)
end

--- Inverts `count` consecutive units (bits, unit 1, or bytes, unit 8) from
-- `offset mod units`, wrapping around the buffer.  Only the PARITY of each unit's
-- flips matters (XOR twice is a no-op): every unit is hit `count // units` times and
-- the first `count % units` (from the wrapped start) once more, so the work is O(len)
-- for ANY count, where a loop of `count` single flips would never end (VER06 9.2).
--
-- Worked example, 2 bytes = 16 bits, bit_flip(offset 0, count 17): 17 // 16 = 1
-- (every bit once, odd) and 17 % 16 = 1 (bit 0 once more, so twice, even): bits
-- 1..15 are inverted and bit 0 is unchanged.
local function parity_flip(raw, unit, offset, count)
  local n = #raw
  if n == 0 or int_sign(count) <= 0 then
    return raw
  end
  local units = (n * 8) // unit
  local base = mod_n(offset, units)
  local full, rem = int_divmod(count, units)
  local out = raw
  if not int_is_even(full) then
    out = invert(out)
  end
  if rem > 0 then
    -- the window [base, base + rem) may run past the end and re-enter at 0; as
    -- rem < units the two pieces never overlap
    local stop = base + rem
    local flip = (unit == 1) and invert_bits or invert_bytes
    out = flip(out, base, m_min(stop, units))
    if stop > units then
      out = flip(out, 0, stop - units)
    end
  end
  return out
end

--- Invert `count` consecutive bits from `bit_offset mod (8 * len)`.
-- bit_flip("\0\0", 9) is 00 02.
function mutate_bit_flip(data, bit_offset, count)
  check_bytes(data, "data")
  check_int(bit_offset, "bit_offset")
  count = count == nil and 1 or count
  check_int(count, "count")
  return parity_flip(data, 1, bit_offset, count)
end

--- XOR `count` consecutive bytes with 0xFF from `byte_offset mod len`.
function mutate_byte_flip(data, byte_offset, count)
  check_bytes(data, "data")
  check_int(byte_offset, "byte_offset")
  count = count == nil and 1 or count
  check_int(count, "count")
  return parity_flip(data, 8, byte_offset, count)
end

-- Reads or writes `width` bytes at the 0-based `at`: (fmt, at) helpers.
local function format_of(order, width)
  return order .. "I" .. width
end

--- Add signed `delta` modulo 2^(8*width) to the integer at the offset.
-- u16 little-endian: ff 00 is 0x00ff; +1 gives 0x0100, stored 00 01.
function mutate_byte_arithmetic(data, byte_offset, delta, width, endian)
  check_bytes(data, "data")
  check_int(byte_offset, "byte_offset")
  check_int(delta, "delta")
  width = width == nil and 1 or width
  check_int(width, "width")
  local order = endian_of(endian == nil and "little" or endian)
  if not WIDTHS[width] or #data < width then
    return data
  end
  local at = mod_n(byte_offset, #data - width + 1)
  local fmt = format_of(order, width)
  local value = s_unpack(fmt, data, at + 1) + int_to_u64(delta) -- words wrap natively
  if width < 8 then
    value = value & ((1 << (8 * width)) - 1)
  end
  return s_sub(data, 1, at) .. s_pack(fmt, value) .. s_sub(data, at + width + 1)
end

--- Overwrite `width` bytes with `val mod 2^(8*width)` at the offset.
-- boundary_insert(00 00 00 00, 0, 4, 0x80000000, "little") is 00 00 00 80.
function mutate_boundary_insert(data, byte_offset, width, val, endian)
  check_bytes(data, "data")
  check_int(byte_offset, "byte_offset")
  check_int(width, "width")
  check_int(val, "val")
  local order = endian_of(endian == nil and "little" or endian)
  if not WIDTHS[width] or #data < width then
    return data
  end
  local at = mod_n(byte_offset, #data - width + 1)
  local value = int_to_u64(val)
  if width < 8 then
    value = value & ((1 << (8 * width)) - 1)
  end
  return s_sub(data, 1, at) .. s_pack(format_of(order, width), value) .. s_sub(data, at + width + 1)
end

--- Delete up to `length` bytes at `offset mod len`; clamped at the end.
function mutate_block_delete(data, offset, length)
  check_bytes(data, "data")
  check_int(offset, "offset")
  check_int(length, "length")
  local size = #data
  if size == 0 or int_sign(length) <= 0 then
    return data
  end
  local at = mod_n(offset, size)
  local cut = m_min(native(length), size - at)
  return s_sub(data, 1, at) .. s_sub(data, at + cut + 1)
end

--- Insert at `offset mod (len + 1)`; an empty input becomes the insert.
function mutate_block_insert(data, offset, insert_bytes)
  check_bytes(data, "data")
  check_int(offset, "offset")
  check_bytes(insert_bytes, "insert_bytes")
  local size = #data
  if size == 0 then
    return insert_bytes
  end
  local at = mod_n(offset, size + 1)
  return s_sub(data, 1, at) .. insert_bytes .. s_sub(data, at + 1)
end

--- Copy up to `length` bytes from `src_offset mod len` to `dst_offset mod (len + 1)`
-- (the end of the buffer is a valid insertion point).
function mutate_block_duplicate(data, src_offset, length, dst_offset)
  check_bytes(data, "data")
  check_int(src_offset, "src_offset")
  check_int(length, "length")
  check_int(dst_offset, "dst_offset")
  local size = #data
  if size == 0 or int_sign(length) <= 0 then
    return data
  end
  local src = mod_n(src_offset, size)
  local chunk = s_sub(data, src + 1, src + m_min(native(length), size - src))
  local dst = mod_n(dst_offset, size + 1)
  return s_sub(data, 1, dst) .. chunk .. s_sub(data, dst + 1)
end

--- target[0, ts) .. donor[ds, ...) with both splits clamped into their buffers.
function mutate_splice(target, donor, target_split, donor_split)
  check_bytes(target, "target")
  check_bytes(donor, "donor")
  check_int(target_split, "target_split")
  check_int(donor_split, "donor_split")
  local t = clamp(target_split, 0, #target)
  local d = clamp(donor_split, 0, #donor)
  return s_sub(target, 1, t) .. s_sub(donor, d + 1)
end

--- `steps` seeded mutations, each chosen by one bounded(7) draw:
--
--   0 bit_flip, count 1            3 boundary_insert, width 1, INTEREST_8
--   1 byte_flip, count 1           4 block_delete (len 1..n/2), skipped if n <= 1
--   2 byte_arithmetic, DELTAS      5 block_insert of one random byte
--                                  6 block_duplicate (len 1..n/2), skipped if n <= 1
--
-- An empty buffer is refilled to 00 before every step.  The draw order is the
-- cross-lane contract (every draw goes into a local first: Lua does not define the
-- order in which call arguments are evaluated).  `max_len` is VER06 section 3.5's
-- length budget: an insert or duplicate that would pass it is skipped AFTER its
-- draws, so the stream stays aligned and memory stays bounded.
--
-- Returns `bytes, work` where work is the bytes copied (the initial copy plus one
-- buffer per step), or `nil, work` when the step that would cross `work_cap` is
-- reached.  A failed call is charged what it did.
function mutate_mutate_sequence_counted(data, seed, steps, max_len, work_cap)
  check_bytes(data, "data")
  check_int(seed, "seed")
  check_int(steps, "steps")
  check_int(max_len, "max_len")
  check_int(work_cap, "work_cap")
  if int_cmp(steps, limits_MAX_ITERATIONS) > 0 then
    error("steps exceeds MAX_ITERATIONS (" .. limits_MAX_ITERATIONS .. ")", 2)
  end
  if #data > limits_MAX_INPUT_BYTES then
    error("input exceeds MAX_INPUT_BYTES", 2)
  end
  local cap, limit = native(work_cap), native(max_len)
  local cur = data
  local work = #cur -- the initial copy is work too
  if work > cap then
    return nil, 0
  end
  local rng = prng_new_rng(seed)
  local count = m_max(native(steps), 0)
  local deltas, interest = Mutate.DELTAS, Mutate.INTEREST_8
  for _ = 1, count do
    if #cur == 0 then
      cur = "\0"
    end
    if work + #cur > cap then
      return nil, work
    end
    work = work + #cur
    local n = #cur
    local choice = rng:bounded(7)
    if choice == 0 then
      local at = rng:bounded(n * 8)
      cur = mutate_bit_flip(cur, at, 1)
    elseif choice == 1 then
      local at = rng:bounded(n)
      cur = mutate_byte_flip(cur, at, 1)
    elseif choice == 2 then
      local delta = deltas[rng:bounded(#deltas) + 1]
      local at = rng:bounded(n)
      cur = mutate_byte_arithmetic(cur, at, delta, 1, "little")
    elseif choice == 3 then
      local val = interest[rng:bounded(#interest) + 1]
      local at = rng:bounded(n)
      cur = mutate_boundary_insert(cur, at, 1, val, "little")
    elseif choice == 4 then
      if n > 1 then
        local length = 1 + rng:bounded(m_max(1, n // 2))
        local at = rng:bounded(n)
        cur = mutate_block_delete(cur, at, length)
      end
    elseif choice == 5 then
      local byte = s_char(rng:bounded(256))
      local at = rng:bounded(n + 1)
      if n + 1 <= limit then
        cur = mutate_block_insert(cur, at, byte)
      end
    else
      if n > 1 then
        local length = 1 + rng:bounded(m_max(1, n // 2))
        local src = rng:bounded(n)
        local dst = rng:bounded(n + 1)
        if n + m_min(length, n - src) <= limit then
          cur = mutate_block_duplicate(cur, src, length, dst)
        end
      end
    end
  end
  return cur, work
end

--- The bytes of `mutate_sequence_counted` under the per-call cap; an error if it
-- is crossed.
function mutate_mutate_sequence(data, seed, steps, max_len)
  if max_len == nil then
    max_len = limits_MAX_INPUT_BYTES
  end
  local out = mutate_mutate_sequence_counted(data, seed, steps, max_len, limits_MAX_SEQUENCE_WORK)
  if out == nil then
    error("mutate_sequence work cap of " .. limits_MAX_SEQUENCE_WORK .. " bytes exceeded", 2)
  end
  return out
end

-- the public table is for callers; this module calls its own locals
Mutate.bit_flip = mutate_bit_flip
Mutate.byte_flip = mutate_byte_flip
Mutate.byte_arithmetic = mutate_byte_arithmetic
Mutate.boundary_insert = mutate_boundary_insert
Mutate.block_delete = mutate_block_delete
Mutate.block_insert = mutate_block_insert
Mutate.block_duplicate = mutate_block_duplicate
Mutate.splice = mutate_splice
Mutate.mutate_sequence_counted = mutate_mutate_sequence_counted
Mutate.mutate_sequence = mutate_mutate_sequence

return Mutate
