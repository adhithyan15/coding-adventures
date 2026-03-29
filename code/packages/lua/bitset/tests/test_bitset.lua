-- Tests for coding_adventures.bitset
--
-- We exercise the full API:
--   new, set, clear, test, size, popcount, set_bits,
--   bitwise_and, bitwise_or, bitwise_xor
--
-- Lua 5.4 busted test suite.

package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path
local Bitset = require("coding_adventures.bitset")

describe("bitset", function()

    -- -----------------------------------------------------------------------
    -- Version
    -- -----------------------------------------------------------------------

    it("has VERSION", function()
        assert.is_not_nil(Bitset.VERSION)
        assert.equals("0.1.0", Bitset.VERSION)
    end)

    -- -----------------------------------------------------------------------
    -- Construction
    -- -----------------------------------------------------------------------

    it("new creates a bitset of the given logical size", function()
        local bs = Bitset.new(100)
        assert.equals(100, Bitset.size(bs))
    end)

    it("new creates a bitset with all bits cleared", function()
        local bs = Bitset.new(64)
        for i = 0, 63 do
            assert.is_false(Bitset.test(bs, i))
        end
    end)

    it("new accepts size 0", function()
        local bs = Bitset.new(0)
        assert.equals(0, Bitset.size(bs))
        assert.equals(0, Bitset.popcount(bs))
    end)

    it("new works for exactly 64 bits (one word)", function()
        local bs = Bitset.new(64)
        assert.equals(64, Bitset.size(bs))
        assert.equals(0, Bitset.popcount(bs))
    end)

    it("new works for 128 bits (two words)", function()
        local bs = Bitset.new(128)
        assert.equals(128, Bitset.size(bs))
        assert.equals(0, Bitset.popcount(bs))
    end)

    -- -----------------------------------------------------------------------
    -- set and test
    -- -----------------------------------------------------------------------

    it("set returns a new bitset with the target bit set", function()
        local bs = Bitset.new(100)
        local bs2 = Bitset.set(bs, 42)
        assert.is_true(Bitset.test(bs2, 42))
    end)

    it("set does not modify the original bitset (functional style)", function()
        local bs = Bitset.new(100)
        local _ = Bitset.set(bs, 42)
        assert.is_false(Bitset.test(bs, 42))
    end)

    it("set bit 0 (lowest bit of first word) works", function()
        local bs = Bitset.new(64)
        bs = Bitset.set(bs, 0)
        assert.is_true(Bitset.test(bs, 0))
        assert.is_false(Bitset.test(bs, 1))
    end)

    it("set bit 63 (highest bit of first word) works", function()
        local bs = Bitset.new(64)
        bs = Bitset.set(bs, 63)
        assert.is_true(Bitset.test(bs, 63))
        assert.is_false(Bitset.test(bs, 62))
    end)

    it("set bit 64 (first bit of second word) works", function()
        local bs = Bitset.new(128)
        bs = Bitset.set(bs, 64)
        assert.is_true(Bitset.test(bs, 64))
        assert.is_false(Bitset.test(bs, 63))
        assert.is_false(Bitset.test(bs, 65))
    end)

    it("set bit beyond initial size grows the bitset", function()
        local bs = Bitset.new(10)
        bs = Bitset.set(bs, 99)
        assert.is_true(Bitset.test(bs, 99))
        assert.is_true(Bitset.size(bs) >= 100)
    end)

    it("test returns false for unset bits around a set bit", function()
        local bs = Bitset.new(100)
        bs = Bitset.set(bs, 42)
        assert.is_false(Bitset.test(bs, 0))
        assert.is_false(Bitset.test(bs, 41))
        assert.is_false(Bitset.test(bs, 43))
        assert.is_false(Bitset.test(bs, 99))
    end)

    it("test returns false for index beyond bitset size", function()
        local bs = Bitset.new(10)
        assert.is_false(Bitset.test(bs, 200))
    end)

    it("can set multiple non-adjacent bits independently", function()
        local bs = Bitset.new(200)
        bs = Bitset.set(bs, 0)
        bs = Bitset.set(bs, 42)
        bs = Bitset.set(bs, 99)
        bs = Bitset.set(bs, 127)
        bs = Bitset.set(bs, 199)
        assert.is_true(Bitset.test(bs, 0))
        assert.is_true(Bitset.test(bs, 42))
        assert.is_true(Bitset.test(bs, 99))
        assert.is_true(Bitset.test(bs, 127))
        assert.is_true(Bitset.test(bs, 199))
        assert.is_false(Bitset.test(bs, 1))
        assert.is_false(Bitset.test(bs, 100))
    end)

    -- -----------------------------------------------------------------------
    -- clear
    -- -----------------------------------------------------------------------

    it("clear clears a previously set bit", function()
        local bs = Bitset.new(100)
        bs = Bitset.set(bs, 42)
        bs = Bitset.clear(bs, 42)
        assert.is_false(Bitset.test(bs, 42))
    end)

    it("clear does not affect adjacent bits", function()
        local bs = Bitset.new(100)
        bs = Bitset.set(bs, 0)
        bs = Bitset.set(bs, 42)
        bs = Bitset.set(bs, 99)
        bs = Bitset.clear(bs, 42)
        assert.is_true(Bitset.test(bs, 0))
        assert.is_false(Bitset.test(bs, 42))
        assert.is_true(Bitset.test(bs, 99))
    end)

    it("clear on already-clear bit is a no-op", function()
        local bs = Bitset.new(100)
        bs = Bitset.clear(bs, 50)
        assert.is_false(Bitset.test(bs, 50))
        assert.equals(0, Bitset.popcount(bs))
    end)

    it("clear returns new bitset without modifying the original", function()
        local bs = Bitset.new(100)
        bs = Bitset.set(bs, 10)
        local bs2 = Bitset.clear(bs, 10)
        assert.is_true(Bitset.test(bs, 10))
        assert.is_false(Bitset.test(bs2, 10))
    end)

    -- -----------------------------------------------------------------------
    -- popcount
    -- -----------------------------------------------------------------------

    it("popcount of all-zeros bitset is 0", function()
        local bs = Bitset.new(100)
        assert.equals(0, Bitset.popcount(bs))
    end)

    it("popcount counts three set bits correctly", function()
        local bs = Bitset.new(200)
        bs = Bitset.set(bs, 0)
        bs = Bitset.set(bs, 42)
        bs = Bitset.set(bs, 99)
        assert.equals(3, Bitset.popcount(bs))
    end)

    it("popcount counts all 64 bits in a full word", function()
        local bs = Bitset.new(64)
        for i = 0, 63 do
            bs = Bitset.set(bs, i)
        end
        assert.equals(64, Bitset.popcount(bs))
    end)

    it("popcount works correctly across two words", function()
        local bs = Bitset.new(128)
        bs = Bitset.set(bs, 5)   -- word 1
        bs = Bitset.set(bs, 70)  -- word 2
        assert.equals(2, Bitset.popcount(bs))
    end)

    it("popcount decreases after clear", function()
        local bs = Bitset.new(64)
        bs = Bitset.set(bs, 10)
        bs = Bitset.set(bs, 20)
        assert.equals(2, Bitset.popcount(bs))
        bs = Bitset.clear(bs, 10)
        assert.equals(1, Bitset.popcount(bs))
    end)

    -- -----------------------------------------------------------------------
    -- set_bits
    -- -----------------------------------------------------------------------

    it("set_bits returns empty list when no bits are set", function()
        local bs = Bitset.new(64)
        local bits = Bitset.set_bits(bs)
        assert.equals(0, #bits)
    end)

    it("set_bits returns the correct indices in ascending order", function()
        local bs = Bitset.new(200)
        bs = Bitset.set(bs, 0)
        bs = Bitset.set(bs, 42)
        bs = Bitset.set(bs, 99)
        local bits = Bitset.set_bits(bs)
        assert.equals(3, #bits)
        assert.equals(0,  bits[1])
        assert.equals(42, bits[2])
        assert.equals(99, bits[3])
    end)

    it("set_bits returns a single element list for one set bit", function()
        local bs = Bitset.new(64)
        bs = Bitset.set(bs, 63)
        local bits = Bitset.set_bits(bs)
        assert.equals(1, #bits)
        assert.equals(63, bits[1])
    end)

    -- -----------------------------------------------------------------------
    -- bitwise_and
    -- -----------------------------------------------------------------------

    it("bitwise_and returns only bits set in both operands", function()
        local a = Bitset.new(64)
        a = Bitset.set(a, 1)
        a = Bitset.set(a, 2)
        a = Bitset.set(a, 3)

        local b = Bitset.new(64)
        b = Bitset.set(b, 2)
        b = Bitset.set(b, 3)
        b = Bitset.set(b, 4)

        local c = Bitset.bitwise_and(a, b)
        assert.is_false(Bitset.test(c, 1))
        assert.is_true(Bitset.test(c, 2))
        assert.is_true(Bitset.test(c, 3))
        assert.is_false(Bitset.test(c, 4))
        assert.equals(2, Bitset.popcount(c))
    end)

    it("bitwise_and of disjoint sets is empty", function()
        local a = Bitset.new(64)
        a = Bitset.set(a, 0)
        a = Bitset.set(a, 1)

        local b = Bitset.new(64)
        b = Bitset.set(b, 2)
        b = Bitset.set(b, 3)

        local c = Bitset.bitwise_and(a, b)
        assert.equals(0, Bitset.popcount(c))
    end)

    -- -----------------------------------------------------------------------
    -- bitwise_or
    -- -----------------------------------------------------------------------

    it("bitwise_or returns the union of both bitsets", function()
        local a = Bitset.new(64)
        a = Bitset.set(a, 1)
        a = Bitset.set(a, 2)

        local b = Bitset.new(64)
        b = Bitset.set(b, 2)
        b = Bitset.set(b, 3)

        local c = Bitset.bitwise_or(a, b)
        assert.is_true(Bitset.test(c, 1))
        assert.is_true(Bitset.test(c, 2))
        assert.is_true(Bitset.test(c, 3))
        assert.equals(3, Bitset.popcount(c))
    end)

    it("bitwise_or of identical sets equals the original", function()
        local a = Bitset.new(64)
        a = Bitset.set(a, 5)
        a = Bitset.set(a, 10)
        local c = Bitset.bitwise_or(a, a)
        assert.equals(2, Bitset.popcount(c))
        assert.is_true(Bitset.test(c, 5))
        assert.is_true(Bitset.test(c, 10))
    end)

    -- -----------------------------------------------------------------------
    -- bitwise_xor
    -- -----------------------------------------------------------------------

    it("bitwise_xor keeps bits that appear in exactly one set", function()
        local a = Bitset.new(64)
        a = Bitset.set(a, 1)
        a = Bitset.set(a, 2)

        local b = Bitset.new(64)
        b = Bitset.set(b, 2)
        b = Bitset.set(b, 3)

        local c = Bitset.bitwise_xor(a, b)
        assert.is_true(Bitset.test(c, 1))   -- only in a
        assert.is_false(Bitset.test(c, 2))  -- in both → cleared
        assert.is_true(Bitset.test(c, 3))   -- only in b
        assert.equals(2, Bitset.popcount(c))
    end)

    it("bitwise_xor of a set with itself yields all zeros", function()
        local a = Bitset.new(64)
        a = Bitset.set(a, 5)
        a = Bitset.set(a, 10)
        local c = Bitset.bitwise_xor(a, a)
        assert.equals(0, Bitset.popcount(c))
    end)

    it("bitwise_xor is commutative", function()
        local a = Bitset.new(64)
        a = Bitset.set(a, 7)
        local b = Bitset.new(64)
        b = Bitset.set(b, 15)
        local ab = Bitset.bitwise_xor(a, b)
        local ba = Bitset.bitwise_xor(b, a)
        assert.equals(Bitset.popcount(ab), Bitset.popcount(ba))
        assert.is_true(Bitset.test(ab, 7))
        assert.is_true(Bitset.test(ba, 7))
        assert.is_true(Bitset.test(ab, 15))
        assert.is_true(Bitset.test(ba, 15))
    end)

end)
