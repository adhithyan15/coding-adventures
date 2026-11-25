-- The mutator algebra (VER06 section 3, VER06p section 3): contracts, hostile
-- arguments and the work caps.
package.path = "../src/?.lua;../src/?/init.lua;" .. package.path
local H = require("helper")
local tb = H.tb
local Fuzz = tb.Fuzz
local Mutate, Limits, Int = Fuzz.Mutate, Fuzz.Limits, tb.Int
local I = Int.parse

local function hex(s)
  return (s:gsub(".", function(c) return string.format("%02x", c:byte()) end))
end

local function unhex(s)
  return (s:gsub("..", function(h) return string.char(tonumber(h, 16)) end))
end

describe("bit_flip and byte_flip", function()
  it("flip the documented bits", function()
    assert.are.equal("0100", hex(Mutate.bit_flip("\0\0", 0, 1)))
    assert.are.equal("0002", hex(Mutate.bit_flip("\0\0", 9, 1)))
    assert.are.equal("3c00", hex(Mutate.bit_flip("\0\0", 2, 4)))
    assert.are.equal("ed3456", hex(Mutate.byte_flip("\x12\x34\x56", 0, 1)))
    assert.are.equal("12cba9", hex(Mutate.byte_flip("\x12\x34\x56", 1, 2)))
  end)

  it("count defaults to 1", function()
    assert.are.equal("0100", hex(Mutate.bit_flip("\0\0", 0)))
    assert.are.equal("ff00", hex(Mutate.byte_flip("\0\0", 0)))
  end)

  it("the worked example of the source: 17 bits of 2 bytes leave bit 0 alone", function()
    assert.are.equal("feff", hex(Mutate.bit_flip("\0\0", 0, 17)))
  end)

  it("windows wrap around the buffer, and offsets are floored: -1 is the last unit", function()
    assert.are.equal("0180", hex(Mutate.bit_flip("\0\0", 15, 2)))
    assert.are.equal("0080", hex(Mutate.bit_flip("\0\0", -1, 1)))
    assert.are.equal("00ff", hex(Mutate.byte_flip("\0\0", -1, 1)))
    assert.are.equal("ffff", hex(Mutate.byte_flip("\0\0", 1, 2)))
  end)

  it("a count of 0 or less, and an empty buffer, change nothing", function()
    assert.are.equal("\1\2", Mutate.bit_flip("\1\2", 3, 0))
    assert.are.equal("\1\2", Mutate.bit_flip("\1\2", 3, -5))
    assert.are.equal("", Mutate.bit_flip("", 3, 4))
    assert.are.equal("", Mutate.byte_flip("", 3, 4))
  end)

  it("are involutions over every 0..2 byte input, for the corpus' offsets and counts", function()
    local inputs = { "" }
    for a = 0, 255 do inputs[#inputs + 1] = string.char(a) end
    for _, a in ipairs({ 0, 1, 127, 128, 255 }) do
      for _, b in ipairs({ 0, 1, 127, 128, 255 }) do inputs[#inputs + 1] = string.char(a, b) end
    end
    for _, s in ipairs(inputs) do
      for _, off in ipairs({ 0, 1, 9, math.maxinteger, -1 }) do
        for _, cnt in ipairs({ 0, 1, 4, 17, math.maxinteger }) do
          local once = Mutate.bit_flip(s, off, cnt)
          assert.are.equal(#s, #once)
          assert.are.equal(s, Mutate.bit_flip(once, off, cnt))
          local b = Mutate.byte_flip(s, off, cnt)
          assert.are.equal(#s, #b)
          assert.are.equal(s, Mutate.byte_flip(b, off, cnt))
        end
      end
    end
  end)

  it("a 1,000-digit count and offset are O(len) and build no big integer from the input", function()
    local big = I("1" .. string.rep("0", 999))
    local count = Int.add(big, 1)
    local data = string.rep("\0\255", 1 << 19) -- 1 MiB
    local calls = 0
    local saved = {}
    -- the only Int functions the flips may use take the OFFSET and COUNT; anything that
    -- could build a number from the input bytes is forbidden for the duration
    for _, name in ipairs({ "shl", "pow2", "parse", "mul", "add", "sub", "from_u64" }) do
      saved[name] = Int[name]
      Int[name] = function() calls = calls + 1 error("a big integer was built") end
    end
    local ok, a, b = pcall(function()
      return Mutate.bit_flip(data, big, count), Mutate.byte_flip(data, big, count)
    end)
    for name, fn in pairs(saved) do Int[name] = fn end
    assert.is_true(ok, a)
    assert.are.equal(0, calls)
    assert.are.equal(#data, #a)
    assert.are.equal(#data, #b)
    -- 10^999 is a multiple of 2^23: an even number of full passes, then ONE unit from offset 0
    assert.are.equal(1, a:byte(1)) -- bit 0 of byte 0
    assert.is_true(a:sub(2) == data:sub(2))
    assert.are.equal(255, b:byte(1)) -- byte 0 inverted
    assert.is_true(b:sub(2) == data:sub(2))
  end)

  it("flipping a whole 1 MiB buffer 'count' times by parity is one pass, in bounded time", function()
    local data = string.rep("\170", 1 << 20)
    local out = H.limited(2e8, Mutate.byte_flip, data, 0, (1 << 20) + 1)
    assert.are.equal(#data, #out)
    -- one full pass (odd) inverts everything; the extra unit flips byte 0 back
    assert.are.equal(170, out:byte(1))
    assert.are.equal(85, out:byte(2))
    assert.are.equal(85, out:byte(#out))
  end)

  it("reject non-integers, non-strings", function()
    assert.has_error(function() Mutate.bit_flip(5, 0, 1) end)
    assert.has_error(function() Mutate.bit_flip("a", 1.5, 1) end)
    assert.has_error(function() Mutate.bit_flip("a", 0, true) end)
    assert.has_error(function() Mutate.byte_flip("a", nil, 1) end)
    assert.has_error(function() Mutate.bit_flip("a", 0, "1") end)
  end)
end)

describe("byte_arithmetic and boundary_insert", function()
  it("wrap modulo 2^(8 width) in both endiannesses", function()
    assert.are.equal("0000", hex(Mutate.byte_arithmetic("\xff\0", 0, 1, 1, "little")))
    assert.are.equal("0001", hex(Mutate.byte_arithmetic("\xff\0", 0, 1, 2, "little")))
    assert.are.equal("0100", hex(Mutate.byte_arithmetic("\0\xff", 0, 1, 2, "big")))
    assert.are.equal("0000000000000000", hex(Mutate.byte_arithmetic("\xff\xff\xff\xff\xff\xff\xff\xff", 0, 1, 8, "little")))
    assert.are.equal("ffffffffffffffff", hex(Mutate.byte_arithmetic("\0\0\0\0\0\0\0\0", 0, -1, 8, "big")))
  end)

  it("accept a delta of any size: only its low 64 bits matter", function()
    assert.are.equal("0100", hex(Mutate.byte_arithmetic("\0\0", 0, I("18446744073709551617"), 1, "little")))
    assert.are.equal("ff00", hex(Mutate.byte_arithmetic("\0\0", 0, I("-18446744073709551617"), 1, "little")))
  end)

  it("return the input for an unsupported width or a too-short buffer", function()
    assert.are.equal("\1\2\3", Mutate.byte_arithmetic("\1\2\3", 0, 1, 3, "little"))
    assert.are.equal("\1\2\3", Mutate.byte_arithmetic("\1\2\3", 0, 1, 4, "little"))
    assert.are.equal("\1\2\3", Mutate.boundary_insert("\1\2\3", 0, 4, 1, "little"))
    assert.are.equal("\1\2\3", Mutate.boundary_insert("\1\2\3", 0, 0, 1, "little"))
    assert.are.equal("", Mutate.boundary_insert("", 0, 1, 1, "little"))
  end)

  it("boundary_insert reduces the value modulo 2^(8 width) and wraps the offset", function()
    assert.are.equal("00000080", hex(Mutate.boundary_insert("\0\0\0\0", 0, 4, 0x80000000, "little")))
    assert.are.equal("80000000", hex(Mutate.boundary_insert("\0\0\0\0", 0, 4, 0x80000000, "big")))
    assert.are.equal("ff00", hex(Mutate.boundary_insert("\0\0", 0, 1, 255 + 256, "little")))
    assert.are.equal("00ff", hex(Mutate.boundary_insert("\0\0", -1, 1, -1, "little")))
    assert.are.equal("ffffffffffffffff", hex(Mutate.boundary_insert("\0\0\0\0\0\0\0\0", 0, 8, I("18446744073709551615"), "little")))
    assert.are.equal("0100000000000000", hex(Mutate.boundary_insert("\0\0\0\0\0\0\0\0", 8, 8, I("18446744073709551617"), "little")))
  end)

  it("default to width 1 and little endian, and refuse any other endianness", function()
    assert.are.equal("0100", hex(Mutate.byte_arithmetic("\0\0", 0, 1)))
    assert.are.equal("0100", hex(Mutate.byte_arithmetic("\0\0", 0, 1, 1, nil)))
    assert.has_error(function() Mutate.byte_arithmetic("\0\0", 0, 1, 1, "middle") end)
    assert.has_error(function() Mutate.boundary_insert("\0\0", 0, 1, 1, "BIG") end)
    assert.has_error(function() Mutate.byte_arithmetic("\0\0", 0, 1.5, 1, "little") end)
  end)
end)

describe("block mutators", function()
  it("block_delete clamps at the end and wraps the offset", function()
    assert.are.equal("010405", hex(Mutate.block_delete("\1\2\3\4\5", 1, 2)))
    assert.are.equal("010203", hex(Mutate.block_delete("\1\2\3\4\5", 3, 99)))
    assert.are.equal("01020304", hex(Mutate.block_delete("\1\2\3\4\5", -1, 1)))
    assert.are.equal("", Mutate.block_delete("\1\2", 0, I("100000000000000000000000000000")))
    assert.are.equal("\1\2", Mutate.block_delete("\1\2", 0, 0))
    assert.are.equal("", Mutate.block_delete("", 0, 5))
  end)

  it("block_insert wraps over len + 1 and an empty input becomes the insert", function()
    assert.are.equal("0102030405", hex(Mutate.block_insert("\1\2\5", 2, "\3\4")))
    assert.are.equal("0102030405aabb", hex(Mutate.block_insert("\1\2\3\4\5", 5, "\170\187")))
    assert.are.equal("aabb", hex(Mutate.block_insert("", 9, "\170\187")))
    assert.are.equal("aabb0102", hex(Mutate.block_insert("\1\2", I("-3"), "\170\187")))
  end)

  it("block_duplicate copies up to length bytes to len + 1 positions", function()
    assert.are.equal("0102030102", hex(Mutate.block_duplicate("\1\2\3", 0, 2, 3)))
    assert.are.equal("0203010203", hex(Mutate.block_duplicate("\1\2\3", 1, 100, 0)))
    assert.are.equal("\1\2", Mutate.block_duplicate("\1\2", 0, 0, 0))
    assert.are.equal("", Mutate.block_duplicate("", 0, 3, 0))
    assert.are.equal("010201020202", hex(Mutate.block_duplicate("\1\2\2", 0, I("1000000000000000000000000"), 2)))
  end)

  it("splice clamps both splits, including a Big", function()
    assert.are.equal("01020708", hex(Mutate.splice("\1\2\3\4", "\5\6\7\8", 2, 2)))
    assert.are.equal("01020304", hex(Mutate.splice("\1\2\3\4", "\5\6\7\8", I("1000000000000000000000000"), I("1000000000000000000000000"))))
    assert.are.equal("05060708", hex(Mutate.splice("\1\2\3\4", "\5\6\7\8", I("-1000000000000000000000000"), -4)))
  end)

  it("none modifies its argument (strings are immutable) and all return strings", function()
    local s = "\1\2\3\4"
    for _, out in ipairs({
      Mutate.bit_flip(s, 1, 3), Mutate.byte_flip(s, 1, 1), Mutate.byte_arithmetic(s, 0, 5, 2),
      Mutate.boundary_insert(s, 0, 2, 9), Mutate.block_delete(s, 0, 1), Mutate.block_insert(s, 0, "x"),
      Mutate.block_duplicate(s, 0, 1, 0), Mutate.splice(s, s, 1, 1), Mutate.mutate_sequence(s, 1, 3),
    }) do
      assert.are.equal("string", type(out))
    end
    assert.are.equal("\1\2\3\4", s)
  end)

  it("reject wrong argument types", function()
    assert.has_error(function() Mutate.block_delete("a", 0.5, 1) end)
    assert.has_error(function() Mutate.block_insert("a", 0, 5) end)
    assert.has_error(function() Mutate.block_duplicate("a", 0, "1", 0) end)
    assert.has_error(function() Mutate.splice("a", 5, 0, 0) end)
    assert.has_error(function() Mutate.splice("a", "b", 0, {}) end)
  end)
end)

describe("mutate_sequence", function()
  it("is deterministic and matches the corpus case", function()
    assert.are.equal("03ff00000000bf080000", hex(Mutate.mutate_sequence(string.rep("\0", 8), 42, 4)))
    assert.are.equal(Mutate.mutate_sequence("abc", 9, 30), Mutate.mutate_sequence("abc", 9, 30))
  end)

  it("refills an empty buffer with 00 and takes zero or negative steps as no steps", function()
    assert.are.equal("", Mutate.mutate_sequence("", 1, 0))
    assert.are.equal("abc", Mutate.mutate_sequence("abc", 1, -5))
    assert.are.equal("abc", Mutate.mutate_sequence("abc", 1, I("-100000000000000000000000")))
    assert.is_true(#Mutate.mutate_sequence("", 1, 1) >= 1)
  end)

  it("never passes max_len by an insert or a duplicate (the draws still happen)", function()
    for seed = 1, 20 do
      local out = Mutate.mutate_sequence("\1\2\3\4", seed, 60, 6)
      assert.is_true(#out <= 6, "length " .. #out)
    end
    -- a skipped insert still consumes its draws, so the stream stays aligned: the same
    -- seed with a generous budget differs only where an insert/duplicate was skipped
    assert.are_not.equal(Mutate.mutate_sequence("\1\2\3\4", 3, 60, 6), Mutate.mutate_sequence("\1\2\3\4", 3, 60, 100000))
  end)

  it("takes a Big seed, reduced modulo 2^64", function()
    assert.are.equal(Mutate.mutate_sequence("abc", I("18446744073709551615"), 10), Mutate.mutate_sequence("abc", -1, 10))
    assert.are.equal(Mutate.mutate_sequence("abc", I("18446744073709551617"), 10), Mutate.mutate_sequence("abc", 1, 10))
  end)

  it("refuses too many steps and too large an input BEFORE allocating", function()
    assert.has_error(function() Mutate.mutate_sequence("a", 1, Limits.MAX_ITERATIONS + 1) end)
    assert.has_error(function() Mutate.mutate_sequence("a", 1, I("1000000000000000000000")) end)
    assert.has_error(function() Mutate.mutate_sequence(string.rep("x", Limits.MAX_INPUT_BYTES + 1), 1, 1) end)
    assert.has_error(function() Mutate.mutate_sequence(1, 1, 1) end)
    assert.has_error(function() Mutate.mutate_sequence("a", 1.5, 1) end)
    assert.has_error(function() Mutate.mutate_sequence("a", 1, 1, 1.5) end)
  end)

  it("charges the REAL work and reports a breach with the work done", function()
    local out, work = Mutate.mutate_sequence_counted("\1\2\3\4", 5, 3, 100, 1 << 28)
    assert.are.equal("string", type(out))
    assert.is_true(work >= 4 + 3 and work <= 4 + 3 * 8)
    -- the initial copy alone is over the cap
    local o2, w2 = Mutate.mutate_sequence_counted("\1\2\3\4", 5, 3, 100, 3)
    assert.is_nil(o2)
    assert.are.equal(0, w2)
    -- the cap is crossed at a step: the result is nil and the work is what was done
    local o3, w3 = Mutate.mutate_sequence_counted("\1\2\3\4", 5, 1000, 100, 10)
    assert.is_nil(o3)
    assert.is_true(w3 <= 10)
    -- a Big cap and a Big max_len saturate
    local o4 = Mutate.mutate_sequence_counted("\1\2", 5, 3, I("100000000000000000000000"), I("100000000000000000000000"))
    assert.are.equal("string", type(o4))
  end)

  it("a failed call is charged: work reports the bytes done before the breach", function()
    local data = string.rep("\1", 100)
    local _, w = Mutate.mutate_sequence_counted(data, 1, 50, 1 << 20, 250)
    -- 100 initial + two 100-byte steps = 300 > 250: breach at the second step (200 so far)
    assert.is_true(w >= 100 and w <= 250)
  end)
end)
