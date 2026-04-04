-- Tests for the polynomial package.
--
-- These tests verify that polynomial arithmetic produces correct results
-- across all operations. We test at multiple levels:
--
--   1. normalize   — canonical form, trailing zero stripping
--   2. degree      — including the zero polynomial convention
--   3. zero / one  — identity elements
--   4. add         — term-by-term addition
--   5. subtract    — term-by-term subtraction with cancellation
--   6. multiply    — convolution
--   7. divmod      — polynomial long division with verification
--   8. divide      — quotient-only wrapper
--   9. modulo      — remainder-only wrapper
--  10. evaluate    — Horner's method
--  11. gcd         — Euclidean algorithm
--  12. module API  — exported symbols and VERSION

package.path = "../src/?.lua;../src/?/init.lua;" .. package.path

local polynomial = require("coding_adventures.polynomial")

-- ============================================================================
-- Helper: check that two polynomial arrays are coefficient-wise equal,
-- within floating-point tolerance.
-- ============================================================================
local EPSILON = 1e-9

local function poly_equal(a, b)
    if #a ~= #b then return false end
    for i = 1, #a do
        if math.abs(a[i] - b[i]) > EPSILON then return false end
    end
    return true
end

local function poly_str(p)
    local parts = {}
    for i = 1, #p do
        parts[i] = tostring(p[i])
    end
    return "{" .. table.concat(parts, ", ") .. "}"
end

-- ============================================================================
-- normalize
-- ============================================================================
describe("normalize", function()
    it("leaves an already-normalized polynomial unchanged", function()
        local p = {3, 2, 1}
        local n = polynomial.normalize(p)
        assert.is_true(poly_equal(n, {3, 2, 1}))
    end)

    it("strips a single trailing zero", function()
        local n = polynomial.normalize({1, 2, 0})
        assert.is_true(poly_equal(n, {1, 2}))
    end)

    it("strips multiple trailing zeros", function()
        local n = polynomial.normalize({1, 0, 0, 0})
        assert.is_true(poly_equal(n, {1}))
    end)

    it("returns {0} for the zero constant", function()
        local n = polynomial.normalize({0})
        assert.is_true(poly_equal(n, {0}))
    end)

    it("returns {0} for an all-zero array", function()
        local n = polynomial.normalize({0, 0, 0})
        assert.is_true(poly_equal(n, {0}))
    end)

    it("treats near-zero coefficients as zero (floating-point)", function()
        -- 1e-15 is below the 1e-10 threshold
        local n = polynomial.normalize({1, 1e-15})
        assert.is_true(poly_equal(n, {1}))
    end)

    it("preserves non-zero middle coefficients", function()
        local n = polynomial.normalize({0, 1, 0})
        -- Leading 0 at index 3 is stripped; {0, 1} remains
        assert.is_true(poly_equal(n, {0, 1}))
    end)
end)

-- ============================================================================
-- degree
-- ============================================================================
describe("degree", function()
    it("returns 2 for a degree-2 polynomial", function()
        assert.are.equal(2, polynomial.degree({3, 0, 2}))
    end)

    it("returns 0 for a constant polynomial", function()
        assert.are.equal(0, polynomial.degree({7}))
    end)

    it("returns -1 for the zero polynomial {0}", function()
        assert.are.equal(-1, polynomial.degree({0}))
    end)

    it("returns -1 for an all-zero array {0, 0, 0}", function()
        assert.are.equal(-1, polynomial.degree({0, 0, 0}))
    end)

    it("returns correct degree when lower coefficients are zero", function()
        -- {0, 0, 5} = 5x²; degree 2
        assert.are.equal(2, polynomial.degree({0, 0, 5}))
    end)

    it("returns 4 for a degree-4 polynomial", function()
        assert.are.equal(4, polynomial.degree({1, 2, 3, 4, 5}))
    end)
end)

-- ============================================================================
-- zero and one
-- ============================================================================
describe("zero and one", function()
    it("zero() returns {0}", function()
        assert.is_true(poly_equal(polynomial.zero(), {0}))
    end)

    it("one() returns {1}", function()
        assert.is_true(poly_equal(polynomial.one(), {1}))
    end)

    it("zero has degree -1", function()
        assert.are.equal(-1, polynomial.degree(polynomial.zero()))
    end)

    it("one has degree 0", function()
        assert.are.equal(0, polynomial.degree(polynomial.one()))
    end)
end)

-- ============================================================================
-- add
-- ============================================================================
describe("add", function()
    it("adds two polynomials of the same length", function()
        -- {1,2,3} + {4,5,6} = {5,7,9}
        local result = polynomial.add({1, 2, 3}, {4, 5, 6})
        assert.is_true(poly_equal(result, {5, 7, 9}),
            "expected {5,7,9} got " .. poly_str(result))
    end)

    it("adds polynomials of different lengths", function()
        -- {1,2,3} + {4,5} = {5,7,3}
        local result = polynomial.add({1, 2, 3}, {4, 5})
        assert.is_true(poly_equal(result, {5, 7, 3}))
    end)

    it("adds zero polynomial on the left", function()
        local p = {3, 2, 1}
        local result = polynomial.add(polynomial.zero(), p)
        assert.is_true(poly_equal(result, {3, 2, 1}))
    end)

    it("adds zero polynomial on the right", function()
        local p = {3, 2, 1}
        local result = polynomial.add(p, polynomial.zero())
        assert.is_true(poly_equal(result, {3, 2, 1}))
    end)

    it("adds two zero polynomials to produce zero", function()
        local result = polynomial.add(polynomial.zero(), polynomial.zero())
        assert.are.equal(-1, polynomial.degree(result))
    end)

    it("result is normalized when coefficients cancel", function()
        -- {1, 2, 3} + {-1, -2, -3} = {0, 0, 0} → {0}
        local result = polynomial.add({1, 2, 3}, {-1, -2, -3})
        assert.are.equal(-1, polynomial.degree(result))
    end)
end)

-- ============================================================================
-- subtract
-- ============================================================================
describe("subtract", function()
    it("subtracts two polynomials of the same length", function()
        -- {5,7,3} - {1,2,3} = {4,5,0} → {4,5}
        local result = polynomial.subtract({5, 7, 3}, {1, 2, 3})
        assert.is_true(poly_equal(result, {4, 5}))
    end)

    it("subtracts a shorter polynomial from a longer one", function()
        -- {1,2,3} - {1,2} = {0,0,3} → normalized as {0,0,3}
        local result = polynomial.subtract({1, 2, 3}, {1, 2})
        assert.is_true(poly_equal(result, {0, 0, 3}))
    end)

    it("subtracting a polynomial from itself gives zero", function()
        local p = {3, 2, 1}
        local result = polynomial.subtract(p, p)
        assert.are.equal(-1, polynomial.degree(result))
    end)

    it("subtracting zero leaves the polynomial unchanged", function()
        local p = {4, 5}
        local result = polynomial.subtract(p, polynomial.zero())
        assert.is_true(poly_equal(result, {4, 5}))
    end)

    it("subtracting from zero negates", function()
        local result = polynomial.subtract(polynomial.zero(), {1, 2})
        assert.is_true(poly_equal(result, {-1, -2}))
    end)
end)

-- ============================================================================
-- multiply
-- ============================================================================
describe("multiply", function()
    it("multiplies two degree-1 polynomials", function()
        -- (1 + 2x)(3 + 4x) = 3 + 10x + 8x²
        local result = polynomial.multiply({1, 2}, {3, 4})
        assert.is_true(poly_equal(result, {3, 10, 8}),
            "expected {3,10,8} got " .. poly_str(result))
    end)

    it("multiplies by one polynomial gives identity", function()
        local p = {3, 2, 1}
        local result = polynomial.multiply(p, polynomial.one())
        assert.is_true(poly_equal(result, {3, 2, 1}))
    end)

    it("multiplies by zero polynomial gives zero", function()
        local p = {3, 2, 1}
        local result = polynomial.multiply(p, polynomial.zero())
        assert.are.equal(-1, polynomial.degree(result))
    end)

    it("multiplies two zero polynomials to give zero", function()
        local result = polynomial.multiply(polynomial.zero(), polynomial.zero())
        assert.are.equal(-1, polynomial.degree(result))
    end)

    it("degree of product equals sum of degrees", function()
        local a = {1, 2, 3}  -- degree 2
        local b = {4, 5}     -- degree 1
        local result = polynomial.multiply(a, b)
        assert.are.equal(3, polynomial.degree(result))
    end)

    it("multiplies constant polynomials", function()
        -- 3 × 4 = 12
        local result = polynomial.multiply({3}, {4})
        assert.is_true(poly_equal(result, {12}))
    end)

    it("multiplication is commutative", function()
        local a = {1, 2, 3}
        local b = {4, 5}
        local ab = polynomial.multiply(a, b)
        local ba = polynomial.multiply(b, a)
        assert.is_true(poly_equal(ab, ba))
    end)
end)

-- ============================================================================
-- divmod
-- ============================================================================
describe("divmod", function()
    it("divides {5,1,3,2} by {2,1} giving q={3,-1,2}, r={-1}", function()
        -- 5 + x + 3x² + 2x³  divided by  2 + x
        local q, r = polynomial.divmod({5, 1, 3, 2}, {2, 1})
        assert.is_true(poly_equal(q, {3, -1, 2}),
            "expected q={3,-1,2} got " .. poly_str(q))
        assert.is_true(poly_equal(r, {-1}),
            "expected r={-1} got " .. poly_str(r))
    end)

    it("divides exactly (zero remainder)", function()
        -- (x+1)(x+2) = x² + 3x + 2 = {2, 3, 1}
        -- {2,3,1} / {1,1} should give q={2,1}, r={0}
        local q, r = polynomial.divmod({2, 3, 1}, {1, 1})
        assert.is_true(poly_equal(q, {2, 1}),
            "q = " .. poly_str(q))
        assert.are.equal(-1, polynomial.degree(r))
    end)

    it("quotient is zero when divisor has higher degree", function()
        local q, r = polynomial.divmod({1, 2}, {1, 2, 3})
        assert.are.equal(-1, polynomial.degree(q))
        assert.is_true(poly_equal(r, {1, 2}))
    end)

    it("divides by a constant", function()
        -- {4, 6, 2} / {2} = {2, 3, 1}
        local q, r = polynomial.divmod({4, 6, 2}, {2})
        assert.is_true(poly_equal(q, {2, 3, 1}),
            "q = " .. poly_str(q))
        assert.are.equal(-1, polynomial.degree(r))
    end)

    it("errors on division by zero polynomial", function()
        assert.has_error(function()
            polynomial.divmod({1, 2}, {0})
        end)
    end)

    it("remainder has degree less than divisor", function()
        local a = {1, 2, 3, 4, 5}
        local b = {1, 2, 3}
        local _, r = polynomial.divmod(a, b)
        assert.is_true(polynomial.degree(r) < polynomial.degree(b))
    end)

    it("verifies a = b*q + r for a degree-4 example", function()
        local a = {1, 2, 3, 4, 5}
        local b = {1, 1}
        local q, r = polynomial.divmod(a, b)
        -- Verify: b*q + r should equal a
        local reconstructed = polynomial.add(polynomial.multiply(b, q), r)
        assert.is_true(poly_equal(
            polynomial.normalize(a),
            polynomial.normalize(reconstructed)
        ), "b*q+r != a: " .. poly_str(reconstructed))
    end)
end)

-- ============================================================================
-- divide (quotient only)
-- ============================================================================
describe("divide", function()
    it("returns the quotient of divmod", function()
        local q = polynomial.divide({5, 1, 3, 2}, {2, 1})
        assert.is_true(poly_equal(q, {3, -1, 2}))
    end)

    it("errors on division by zero", function()
        assert.has_error(function()
            polynomial.divide({1, 2}, {0})
        end)
    end)
end)

-- ============================================================================
-- modulo (remainder only)
-- ============================================================================
describe("modulo", function()
    it("returns the remainder of divmod", function()
        local r = polynomial.modulo({5, 1, 3, 2}, {2, 1})
        assert.is_true(poly_equal(r, {-1}))
    end)

    it("returns zero when divisible exactly", function()
        local r = polynomial.modulo({2, 3, 1}, {1, 1})
        assert.are.equal(-1, polynomial.degree(r))
    end)

    it("errors on division by zero", function()
        assert.has_error(function()
            polynomial.modulo({1, 2}, {0})
        end)
    end)
end)

-- ============================================================================
-- evaluate
-- ============================================================================
describe("evaluate", function()
    it("evaluates 3 + x + 2x² at x=4 gives 39", function()
        -- 3 + 4 + 2*16 = 3 + 4 + 32 = 39
        local val = polynomial.evaluate({3, 1, 2}, 4)
        assert.are.equal(39, val)
    end)

    it("evaluates constant polynomial", function()
        local val = polynomial.evaluate({7}, 100)
        assert.are.equal(7, val)
    end)

    it("evaluates zero polynomial to 0", function()
        local val = polynomial.evaluate(polynomial.zero(), 5)
        assert.are.equal(0, val)
    end)

    it("evaluates at x=0 gives constant term", function()
        local val = polynomial.evaluate({5, 3, 2}, 0)
        assert.are.equal(5, val)
    end)

    it("evaluates at x=1 gives sum of coefficients", function()
        -- 1 + 2 + 3 = 6
        local val = polynomial.evaluate({1, 2, 3}, 1)
        assert.are.equal(6, val)
    end)

    it("evaluates at x=-1 gives alternating sum", function()
        -- 1 - 2 + 3 = 2
        local val = polynomial.evaluate({1, 2, 3}, -1)
        assert.are.equal(2, val)
    end)

    it("evaluates a degree-1 polynomial correctly", function()
        -- 3 + 2x at x=5 = 13
        local val = polynomial.evaluate({3, 2}, 5)
        assert.are.equal(13, val)
    end)

    it("Horner agrees with naive evaluation for degree-3", function()
        -- 1 + 2x + 3x² + 4x³ at x=2
        -- Naive: 1 + 4 + 12 + 32 = 49
        local val = polynomial.evaluate({1, 2, 3, 4}, 2)
        assert.are.equal(49, val)
    end)
end)

-- ============================================================================
-- gcd
-- ============================================================================
describe("gcd", function()
    it("gcd of (x+1)(x+2) and (x+1)(x+3) is (x+1)", function()
        -- (x+1)(x+2) = x² + 3x + 2 = {2, 3, 1}
        -- (x+1)(x+3) = x² + 4x + 3 = {3, 4, 1}
        -- gcd should be x+1 = {1, 1} (or a scalar multiple)
        local a = {2, 3, 1}
        local b = {3, 4, 1}
        local g = polynomial.gcd(a, b)
        -- After Euclidean algorithm, the gcd is returned normalized.
        -- We verify that g divides both a and b with zero remainder.
        local ra = polynomial.modulo(a, g)
        local rb = polynomial.modulo(b, g)
        assert.are.equal(-1, polynomial.degree(ra),
            "g does not divide a; remainder: " .. poly_str(ra))
        assert.are.equal(-1, polynomial.degree(rb),
            "g does not divide b; remainder: " .. poly_str(rb))
    end)

    it("gcd of coprime polynomials is a non-zero constant", function()
        -- {6, 7, 1} = (x+1)(x+6),  {6, 5, 1} = (x+2)(x+3)  (no common root)
        local g = polynomial.gcd({6, 7, 1}, {6, 5, 1})
        -- GCD of coprime polynomials is a nonzero constant (degree 0 or -1 if scaled to 0)
        assert.is_true(polynomial.degree(g) <= 0)
    end)

    it("gcd(p, zero) = p", function()
        local p = {1, 2, 1}
        local g = polynomial.gcd(p, polynomial.zero())
        -- gcd with zero should return the polynomial itself (normalized)
        assert.is_true(poly_equal(g, polynomial.normalize(p)))
    end)

    it("gcd(zero, p) = p", function()
        local p = {1, 2, 1}
        local g = polynomial.gcd(polynomial.zero(), p)
        assert.is_true(poly_equal(g, polynomial.normalize(p)))
    end)

    it("gcd(p, p) = p", function()
        local p = {1, 1}
        local g = polynomial.gcd(p, p)
        -- gcd(p, p) = p (up to normalization and scalar)
        assert.is_true(polynomial.degree(g) == polynomial.degree(p))
    end)
end)

-- ============================================================================
-- module API
-- ============================================================================
describe("module API", function()
    it("has a VERSION string", function()
        assert.are.equal("0.1.0", polynomial.VERSION)
    end)

    it("exports normalize", function()
        assert.is_function(polynomial.normalize)
    end)

    it("exports degree", function()
        assert.is_function(polynomial.degree)
    end)

    it("exports zero", function()
        assert.is_function(polynomial.zero)
    end)

    it("exports one", function()
        assert.is_function(polynomial.one)
    end)

    it("exports add", function()
        assert.is_function(polynomial.add)
    end)

    it("exports subtract", function()
        assert.is_function(polynomial.subtract)
    end)

    it("exports multiply", function()
        assert.is_function(polynomial.multiply)
    end)

    it("exports divmod", function()
        assert.is_function(polynomial.divmod)
    end)

    it("exports divide", function()
        assert.is_function(polynomial.divide)
    end)

    it("exports modulo", function()
        assert.is_function(polynomial.modulo)
    end)

    it("exports evaluate", function()
        assert.is_function(polynomial.evaluate)
    end)

    it("exports gcd", function()
        assert.is_function(polynomial.gcd)
    end)
end)
