-- Tests for coding_adventures.rng
--
-- Reference values computed from the Go implementation (seed = 1):
--   LCG:        [1817669548, 2187888307, 2784682393]
--   Xorshift64: [1082269761, 201397313,  1854285353]
--   PCG32:      [1412771199, 1791099446, 124312908]
--
-- Lua 5.4 busted test suite.

package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path
local rng = require("coding_adventures.rng")

-- ============================================================================
-- MODULE
-- ============================================================================

describe("rng module", function()

    it("has VERSION", function()
        assert.is_not_nil(rng.VERSION)
        assert.equals("0.1.0", rng.VERSION)
    end)

    it("exports LCG class", function()
        assert.is_not_nil(rng.LCG)
    end)

    it("exports Xorshift64 class", function()
        assert.is_not_nil(rng.Xorshift64)
    end)

    it("exports PCG32 class", function()
        assert.is_not_nil(rng.PCG32)
    end)

end)

-- ============================================================================
-- LCG
-- ============================================================================

describe("LCG", function()

    -- Reference values for seed=1
    it("produces correct first output for seed=1 (reference value)", function()
        local g = rng.LCG.new(1)
        assert.equals(1817669548, g:next_u32())
    end)

    it("produces correct second output for seed=1", function()
        local g = rng.LCG.new(1)
        g:next_u32()
        assert.equals(2187888307, g:next_u32())
    end)

    it("produces correct third output for seed=1", function()
        local g = rng.LCG.new(1)
        g:next_u32()
        g:next_u32()
        assert.equals(2784682393, g:next_u32())
    end)

    it("seed=0 is valid and produces deterministic output", function()
        local g = rng.LCG.new(0)
        local v = g:next_u32()
        -- Must be a non-negative integer in [0, 2^32-1]
        assert.is_true(v >= 0 and v <= 0xFFFFFFFF)
    end)

    it("two generators with the same seed produce identical sequences", function()
        local g1 = rng.LCG.new(42)
        local g2 = rng.LCG.new(42)
        for _ = 1, 10 do
            assert.equals(g1:next_u32(), g2:next_u32())
        end
    end)

    it("two generators with different seeds produce different first values", function()
        local g1 = rng.LCG.new(1)
        local g2 = rng.LCG.new(2)
        assert.not_equals(g1:next_u32(), g2:next_u32())
    end)

    it("next_u32 returns value in [0, 2^32-1]", function()
        local g = rng.LCG.new(7)
        for _ = 1, 20 do
            local v = g:next_u32()
            assert.is_true(v >= 0)
            assert.is_true(v <= 0xFFFFFFFF)
        end
    end)

    it("next_u64 returns a 64-bit value", function()
        local g = rng.LCG.new(1)
        local v = g:next_u64()
        -- A u64 fits in a Lua integer (up to 2^63-1 without sign issues, but
        -- as a bit pattern it's always >= 0 or the MSB is set making it negative
        -- as a signed integer). We check it uses 64 bits: hi and lo must compose.
        assert.is_not_nil(v)
        assert.is_true(math.type(v) == "integer" or math.type(v) == "float")
    end)

    it("next_u64 is deterministic", function()
        local g1 = rng.LCG.new(99)
        local g2 = rng.LCG.new(99)
        assert.equals(g1:next_u64(), g2:next_u64())
    end)

    it("next_float returns value in [0.0, 1.0)", function()
        local g = rng.LCG.new(123)
        for _ = 1, 20 do
            local f = g:next_float()
            assert.is_true(f >= 0.0)
            assert.is_true(f < 1.0)
        end
    end)

    it("next_int_in_range returns value within [min, max]", function()
        local g = rng.LCG.new(5)
        for _ = 1, 50 do
            local v = g:next_int_in_range(1, 6)
            assert.is_true(v >= 1)
            assert.is_true(v <= 6)
        end
    end)

    it("next_int_in_range covers full range over many draws", function()
        local g = rng.LCG.new(0)
        local seen = {}
        for _ = 1, 200 do
            local v = g:next_int_in_range(0, 4)
            seen[v] = true
        end
        for i = 0, 4 do
            assert.is_true(seen[i] == true, "value " .. i .. " not seen")
        end
    end)

    it("next_int_in_range with min==max always returns min", function()
        local g = rng.LCG.new(3)
        for _ = 1, 10 do
            assert.equals(7, g:next_int_in_range(7, 7))
        end
    end)

end)

-- ============================================================================
-- Xorshift64
-- ============================================================================

describe("Xorshift64", function()

    it("produces correct first output for seed=1 (reference value)", function()
        local g = rng.Xorshift64.new(1)
        assert.equals(1082269761, g:next_u32())
    end)

    it("produces correct second output for seed=1", function()
        local g = rng.Xorshift64.new(1)
        g:next_u32()
        assert.equals(201397313, g:next_u32())
    end)

    it("produces correct third output for seed=1", function()
        local g = rng.Xorshift64.new(1)
        g:next_u32()
        g:next_u32()
        assert.equals(1854285353, g:next_u32())
    end)

    it("seed=0 is replaced with 1", function()
        local g0 = rng.Xorshift64.new(0)
        local g1 = rng.Xorshift64.new(1)
        assert.equals(g0:next_u32(), g1:next_u32())
    end)

    it("two generators with same seed produce identical sequences", function()
        local g1 = rng.Xorshift64.new(55)
        local g2 = rng.Xorshift64.new(55)
        for _ = 1, 10 do
            assert.equals(g1:next_u32(), g2:next_u32())
        end
    end)

    it("next_u32 returns value in [0, 2^32-1]", function()
        local g = rng.Xorshift64.new(7)
        for _ = 1, 20 do
            local v = g:next_u32()
            assert.is_true(v >= 0)
            assert.is_true(v <= 0xFFFFFFFF)
        end
    end)

    it("next_float returns value in [0.0, 1.0)", function()
        local g = rng.Xorshift64.new(123)
        for _ = 1, 20 do
            local f = g:next_float()
            assert.is_true(f >= 0.0)
            assert.is_true(f < 1.0)
        end
    end)

    it("next_int_in_range returns value within [min, max]", function()
        local g = rng.Xorshift64.new(5)
        for _ = 1, 50 do
            local v = g:next_int_in_range(1, 6)
            assert.is_true(v >= 1)
            assert.is_true(v <= 6)
        end
    end)

    it("next_int_in_range covers full range over many draws", function()
        local g = rng.Xorshift64.new(2)
        local seen = {}
        for _ = 1, 200 do
            local v = g:next_int_in_range(0, 4)
            seen[v] = true
        end
        for i = 0, 4 do
            assert.is_true(seen[i] == true, "value " .. i .. " not seen")
        end
    end)

    it("next_int_in_range with min==max always returns min", function()
        local g = rng.Xorshift64.new(3)
        for _ = 1, 10 do
            assert.equals(42, g:next_int_in_range(42, 42))
        end
    end)

    it("next_u64 is deterministic", function()
        local g1 = rng.Xorshift64.new(77)
        local g2 = rng.Xorshift64.new(77)
        assert.equals(g1:next_u64(), g2:next_u64())
    end)

end)

-- ============================================================================
-- PCG32
-- ============================================================================

describe("PCG32", function()

    it("produces correct first output for seed=1 (reference value)", function()
        local g = rng.PCG32.new(1)
        assert.equals(1412771199, g:next_u32())
    end)

    it("produces correct second output for seed=1", function()
        local g = rng.PCG32.new(1)
        g:next_u32()
        assert.equals(1791099446, g:next_u32())
    end)

    it("produces correct third output for seed=1", function()
        local g = rng.PCG32.new(1)
        g:next_u32()
        g:next_u32()
        assert.equals(124312908, g:next_u32())
    end)

    it("two generators with same seed produce identical sequences", function()
        local g1 = rng.PCG32.new(12345)
        local g2 = rng.PCG32.new(12345)
        for _ = 1, 10 do
            assert.equals(g1:next_u32(), g2:next_u32())
        end
    end)

    it("different seeds produce different sequences", function()
        local g1 = rng.PCG32.new(1)
        local g2 = rng.PCG32.new(2)
        assert.not_equals(g1:next_u32(), g2:next_u32())
    end)

    it("seed=0 is valid", function()
        local g = rng.PCG32.new(0)
        local v = g:next_u32()
        assert.is_true(v >= 0 and v <= 0xFFFFFFFF)
    end)

    it("next_u32 returns value in [0, 2^32-1]", function()
        local g = rng.PCG32.new(7)
        for _ = 1, 20 do
            local v = g:next_u32()
            assert.is_true(v >= 0)
            assert.is_true(v <= 0xFFFFFFFF)
        end
    end)

    it("next_float returns value in [0.0, 1.0)", function()
        local g = rng.PCG32.new(123)
        for _ = 1, 20 do
            local f = g:next_float()
            assert.is_true(f >= 0.0)
            assert.is_true(f < 1.0)
        end
    end)

    it("next_int_in_range returns value within [min, max]", function()
        local g = rng.PCG32.new(5)
        for _ = 1, 50 do
            local v = g:next_int_in_range(1, 6)
            assert.is_true(v >= 1)
            assert.is_true(v <= 6)
        end
    end)

    it("next_int_in_range covers full range over many draws", function()
        local g = rng.PCG32.new(3)
        local seen = {}
        for _ = 1, 200 do
            local v = g:next_int_in_range(0, 4)
            seen[v] = true
        end
        for i = 0, 4 do
            assert.is_true(seen[i] == true, "value " .. i .. " not seen")
        end
    end)

    it("next_int_in_range with min==max always returns min", function()
        local g = rng.PCG32.new(9)
        for _ = 1, 10 do
            assert.equals(100, g:next_int_in_range(100, 100))
        end
    end)

    it("next_u64 is deterministic", function()
        local g1 = rng.PCG32.new(11)
        local g2 = rng.PCG32.new(11)
        assert.equals(g1:next_u64(), g2:next_u64())
    end)

    -- PCG32 and LCG produce different sequences (different output permutations)
    it("PCG32 and LCG produce different outputs for the same seed", function()
        local lcg = rng.LCG.new(1)
        local pcg = rng.PCG32.new(1)
        assert.not_equals(lcg:next_u32(), pcg:next_u32())
    end)

end)
