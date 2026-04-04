-- Tests for the gf256 package.
--
-- These tests verify that GF(2^8) arithmetic is correct for the primitive
-- polynomial 0x11D = x^8 + x^4 + x^3 + x^2 + 1 (Reed-Solomon standard).
--
-- We test:
--   1. add/subtract     — XOR properties
--   2. multiply         — log/antilog table correctness
--   3. divide           — inverse of multiply
--   4. power            — exponentiation
--   5. inverse          — multiplicative inverse
--   6. field axioms     — commutativity, associativity, distributivity
--   7. known vectors    — well-known GF(256) results as a sanity check
--   8. module API       — exported symbols, constants, VERSION

package.path = "../src/?.lua;../src/?/init.lua;" .. package.path

local gf = require("coding_adventures.gf256")

-- ============================================================================
-- add / subtract
-- ============================================================================
describe("add and subtract", function()
    it("add is XOR: 0x53 XOR 0xCA = 0x99", function()
        -- 0x53 = 0101 0011
        -- 0xCA = 1100 1010
        -- XOR  = 1001 1001 = 0x99
        assert.are.equal(0x99, gf.add(0x53, 0xCA))
    end)

    it("add is commutative: add(a,b) = add(b,a)", function()
        assert.are.equal(gf.add(0x12, 0x34), gf.add(0x34, 0x12))
    end)

    it("add(x, x) = 0 for all x (characteristic 2)", function()
        -- Every element is its own additive inverse.
        for _, v in ipairs({1, 2, 0x53, 0xFF, 128, 200}) do
            assert.are.equal(0, gf.add(v, v),
                "add(" .. v .. ", " .. v .. ") should be 0")
        end
    end)

    it("add(x, 0) = x (zero is additive identity)", function()
        assert.are.equal(0x53, gf.add(0x53, 0))
        assert.are.equal(0, gf.add(0, 0))
        assert.are.equal(0xFF, gf.add(0xFF, 0))
    end)

    it("subtract equals add (characteristic 2)", function()
        assert.are.equal(gf.add(0x53, 0xCA), gf.subtract(0x53, 0xCA))
        assert.are.equal(gf.add(1, 2), gf.subtract(1, 2))
    end)

    it("subtract(x, x) = 0", function()
        assert.are.equal(0, gf.subtract(0x80, 0x80))
        assert.are.equal(0, gf.subtract(0xFF, 0xFF))
    end)

    it("add is associative", function()
        local a, b, c = 0x12, 0x34, 0x56
        assert.are.equal(
            gf.add(gf.add(a, b), c),
            gf.add(a, gf.add(b, c))
        )
    end)
end)

-- ============================================================================
-- multiply
-- ============================================================================
describe("multiply", function()
    it("multiply(x, 1) = x (multiplicative identity)", function()
        for _, v in ipairs({1, 2, 0x53, 0x8C, 0xFF}) do
            assert.are.equal(v, gf.multiply(v, 1),
                "multiply(" .. v .. ", 1) should be " .. v)
        end
    end)

    it("multiply(x, 0) = 0 for all x", function()
        for _, v in ipairs({0, 1, 0x53, 0xFF}) do
            assert.are.equal(0, gf.multiply(v, 0))
        end
    end)

    it("multiply(0, x) = 0 for all x", function()
        for _, v in ipairs({0, 1, 0x53, 0xFF}) do
            assert.are.equal(0, gf.multiply(0, v))
        end
    end)

    it("multiply is commutative", function()
        assert.are.equal(gf.multiply(0x53, 0x8C), gf.multiply(0x8C, 0x53))
        assert.are.equal(gf.multiply(3, 7), gf.multiply(7, 3))
    end)

    it("multiply(2, 2) = 4 (no overflow needed)", function()
        assert.are.equal(4, gf.multiply(2, 2))
    end)

    it("multiply(2, 128) reduces modulo 0x11D", function()
        -- 128 = 0x80; 2 * 128 = 256 = 0x100; 0x100 XOR 0x11D = 0x01D = 29
        assert.are.equal(29, gf.multiply(2, 128))
    end)

    it("known pair: multiply(0x53, 0x8C) = 1", function()
        -- 0x53 and 0x8C are multiplicative inverses under 0x11D
        assert.are.equal(1, gf.multiply(0x53, 0x8C))
    end)

    it("multiply is associative", function()
        local a, b, c = 0x12, 0x34, 0x56
        assert.are.equal(
            gf.multiply(gf.multiply(a, b), c),
            gf.multiply(a, gf.multiply(b, c))
        )
    end)

    it("distributive law: a*(b+c) = a*b + a*c", function()
        local a, b, c = 0x12, 0x34, 0x56
        local lhs = gf.multiply(a, gf.add(b, c))
        local rhs = gf.add(gf.multiply(a, b), gf.multiply(a, c))
        assert.are.equal(lhs, rhs)
    end)
end)

-- ============================================================================
-- divide
-- ============================================================================
describe("divide", function()
    it("divide(x, 1) = x", function()
        for _, v in ipairs({1, 2, 0x53, 0xFF}) do
            assert.are.equal(v, gf.divide(v, 1))
        end
    end)

    it("divide(0, x) = 0 for non-zero x", function()
        assert.are.equal(0, gf.divide(0, 5))
        assert.are.equal(0, gf.divide(0, 0xFF))
    end)

    it("divide is inverse of multiply: divide(multiply(a,b), b) = a", function()
        local a, b = 0x53, 0x8C
        assert.are.equal(a, gf.divide(gf.multiply(a, b), b))
    end)

    it("divide(x, x) = 1 for non-zero x", function()
        for _, v in ipairs({1, 2, 0x53, 0x8C, 0xFF}) do
            assert.are.equal(1, gf.divide(v, v))
        end
    end)

    it("errors on division by zero", function()
        assert.has_error(function()
            gf.divide(0x53, 0)
        end)
    end)

    it("errors on division of zero by zero", function()
        assert.has_error(function()
            gf.divide(0, 0)
        end)
    end)
end)

-- ============================================================================
-- power
-- ============================================================================
describe("power", function()
    it("power(x, 0) = 1 for non-zero x", function()
        for _, v in ipairs({1, 2, 0x53, 0xFF}) do
            assert.are.equal(1, gf.power(v, 0))
        end
    end)

    it("power(0, 0) = 1 by convention", function()
        assert.are.equal(1, gf.power(0, 0))
    end)

    it("power(0, n) = 0 for n > 0", function()
        assert.are.equal(0, gf.power(0, 1))
        assert.are.equal(0, gf.power(0, 5))
    end)

    it("power(x, 1) = x", function()
        assert.are.equal(0x53, gf.power(0x53, 1))
        assert.are.equal(2, gf.power(2, 1))
    end)

    it("generator g=2 raised to 255 equals 1", function()
        -- g^255 = 1 by definition (the multiplicative group has order 255)
        assert.are.equal(1, gf.power(2, 255))
    end)

    it("power(2, 8) = ALOG[8] = 29", function()
        -- 2^8 in GF(256) with 0x11D: 256 XOR 0x11D = 29
        assert.are.equal(29, gf.power(2, 8))
    end)

    it("power(x, 2) = multiply(x, x)", function()
        for _, v in ipairs({2, 3, 0x53, 100}) do
            assert.are.equal(gf.multiply(v, v), gf.power(v, 2))
        end
    end)
end)

-- ============================================================================
-- inverse
-- ============================================================================
describe("inverse", function()
    it("x * inverse(x) = 1 for all non-zero x", function()
        -- Spot-check many values
        for v = 1, 255, 17 do
            local inv = gf.inverse(v)
            assert.are.equal(1, gf.multiply(v, inv),
                v .. " * inverse(" .. v .. ") should be 1, got " ..
                gf.multiply(v, inv))
        end
    end)

    it("inverse(1) = 1", function()
        assert.are.equal(1, gf.inverse(1))
    end)

    it("inverse(0x53) = 0x8C", function()
        assert.are.equal(0x8C, gf.inverse(0x53))
    end)

    it("inverse(0x8C) = 0x53", function()
        assert.are.equal(0x53, gf.inverse(0x8C))
    end)

    it("inverse(inverse(x)) = x for non-zero x", function()
        for _, v in ipairs({2, 0x53, 0x8C, 100, 200}) do
            assert.are.equal(v, gf.inverse(gf.inverse(v)))
        end
    end)

    it("errors on inverse of zero", function()
        assert.has_error(function()
            gf.inverse(0)
        end)
    end)
end)

-- ============================================================================
-- known verification vectors
-- ============================================================================
describe("known field vectors", function()
    it("all 255 non-zero elements have a multiplicative inverse", function()
        for v = 1, 255 do
            local inv = gf.inverse(v)
            assert.are.equal(1, gf.multiply(v, inv),
                "inverse failed for " .. v)
        end
    end)

    it("generator g=2 cycles through all 255 non-zero elements", function()
        local seen = {}
        local val = 1
        for _ = 0, 254 do
            assert.is_nil(seen[val], "duplicate in generator cycle: " .. val)
            seen[val] = true
            val = gf.multiply(val, 2)
        end
        -- After 255 steps we should be back at 1
        assert.are.equal(1, val)
    end)

    it("add(x, x) = 0 for all 256 elements", function()
        for v = 0, 255 do
            assert.are.equal(0, gf.add(v, v))
        end
    end)
end)

-- ============================================================================
-- module API / constants
-- ============================================================================
describe("module API", function()
    it("has VERSION string", function()
        assert.are.equal("0.1.0", gf.VERSION)
    end)

    it("ZERO = 0", function()
        assert.are.equal(0, gf.ZERO)
    end)

    it("ONE = 1", function()
        assert.are.equal(1, gf.ONE)
    end)

    it("PRIMITIVE_POLYNOMIAL = 0x11D", function()
        assert.are.equal(0x11D, gf.PRIMITIVE_POLYNOMIAL)
    end)

    it("exports add", function()
        assert.is_function(gf.add)
    end)

    it("exports subtract", function()
        assert.is_function(gf.subtract)
    end)

    it("exports multiply", function()
        assert.is_function(gf.multiply)
    end)

    it("exports divide", function()
        assert.is_function(gf.divide)
    end)

    it("exports power", function()
        assert.is_function(gf.power)
    end)

    it("exports inverse", function()
        assert.is_function(gf.inverse)
    end)
end)
