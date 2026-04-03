-- ============================================================================
-- Tests for trig -- Trigonometric functions from first principles
-- ============================================================================
--
-- These tests verify that our Maclaurin-series implementations of sine,
-- cosine, and tangent produce correct results across a wide range of inputs.
--
-- We test:
--   1. Landmark values (0, pi/6, pi/4, pi/3, pi/2, pi, etc.)
--   2. Symmetry properties (sin is odd, cos is even)
--   3. The Pythagorean identity: sin^2(x) + cos^2(x) = 1
--   4. Range reduction with large inputs
--   5. Angle conversion round-trips
--   6. Tangent function including near-asymptote behaviour
--
-- ## Why Approximate Equality?
--
-- Floating-point arithmetic is inherently imprecise. The number 0.1, for
-- example, cannot be represented exactly in binary. When we compute
-- sin(pi), we don't get exactly 0 -- we get something like 1.2e-16. So
-- we need a way to say "close enough."
--
-- A tolerance of 1e-10 means we accept results that differ by less than
-- 0.0000000001. This is far more precise than any physical measurement
-- but accounts for the tiny rounding errors in our series computation.

-- Add src/ to the module search path so we can require the package.
package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path

local trig = require("coding_adventures.trig")

--- Helper: check whether two floating-point numbers are close enough.
-- A tolerance of 1e-10 matches the Go test suite and is far more precise
-- than any physical measurement while accommodating series rounding errors.
local function approx_equal(a, b, tol)
    tol = tol or 1e-10
    return math.abs(a - b) < tol
end

-- ============================================================================
-- Constants
-- ============================================================================

describe("constants", function()
    it("exposes PI to double precision", function()
        -- PI should match Lua's built-in math.pi to full double precision.
        assert.is_true(approx_equal(trig.PI, math.pi))
    end)

    it("exposes TWO_PI as exactly 2 * PI", function()
        assert.are.equal(2 * trig.PI, trig.TWO_PI)
    end)

    it("has a version string", function()
        assert.are.equal("0.1.0", trig.VERSION)
    end)
end)

-- ============================================================================
-- Sine Tests
-- ============================================================================

describe("sin", function()
    -- ## Landmark Values
    --
    -- These are the "must-pass" values that every trig implementation needs
    -- to get right. They correspond to the most commonly used angles.

    it("sin(0) = 0", function()
        -- When x = 0, every term in the Maclaurin series is zero (since they
        -- all contain a factor of x), so the sum is zero.
        assert.is_true(approx_equal(trig.sin(0), 0))
    end)

    it("sin(pi/6) = 0.5", function()
        -- 30 degrees. This is the simplest non-trivial exact value.
        assert.is_true(approx_equal(trig.sin(trig.PI / 6), 0.5))
    end)

    it("sin(pi/4) = sqrt(2)/2", function()
        -- 45 degrees. sin and cos are equal at this angle.
        assert.is_true(approx_equal(trig.sin(trig.PI / 4), math.sqrt(2) / 2))
    end)

    it("sin(pi/3) = sqrt(3)/2", function()
        -- 60 degrees.
        assert.is_true(approx_equal(trig.sin(trig.PI / 3), math.sqrt(3) / 2))
    end)

    it("sin(pi/2) = 1", function()
        -- 90 degrees. Sine reaches its maximum value of 1.
        assert.is_true(approx_equal(trig.sin(trig.PI / 2), 1.0))
    end)

    it("sin(pi) = 0", function()
        -- 180 degrees. Sine returns to zero at the half-period.
        assert.is_true(approx_equal(trig.sin(trig.PI), 0.0))
    end)

    it("sin(3*pi/2) = -1", function()
        -- 270 degrees. Sine reaches its minimum value of -1.
        assert.is_true(approx_equal(trig.sin(3 * trig.PI / 2), -1.0))
    end)

    it("sin(2*pi) = 0", function()
        -- 360 degrees. A full cycle returns to zero.
        assert.is_true(approx_equal(trig.sin(trig.TWO_PI), 0.0))
    end)

    -- ## Odd Symmetry: sin(-x) = -sin(x)
    --
    -- Sine is an "odd function," meaning it's antisymmetric about the origin.
    -- Graphically, if you rotate the sine curve 180 degrees around the origin,
    -- you get the same curve.

    it("satisfies odd symmetry: sin(-x) = -sin(x)", function()
        local test_values = {0.5, 1.0, trig.PI / 4, trig.PI / 3, trig.PI / 2, 2.0, 3.0}
        for _, x in ipairs(test_values) do
            assert.is_true(
                approx_equal(trig.sin(-x), -trig.sin(x)),
                string.format("sin(-%g) should equal -sin(%g)", x, x)
            )
        end
    end)

    -- ## Large Input (Range Reduction Stress Test)
    --
    -- sin(1000*pi) should be approximately 0, because 1000*pi is an integer
    -- multiple of pi. This tests that range reduction correctly handles
    -- inputs far outside [-pi, pi].

    it("handles large inputs via range reduction", function()
        assert.is_true(approx_equal(trig.sin(1000 * trig.PI), 0.0))
    end)

    it("handles large negative inputs via range reduction", function()
        assert.is_true(approx_equal(trig.sin(-1000 * trig.PI), 0.0))
    end)

    it("handles moderately large inputs", function()
        -- sin(100) compared against math.sin as a reference
        assert.is_true(approx_equal(trig.sin(100), math.sin(100), 1e-6))
    end)

    -- ## Negative quadrant values

    it("sin(-pi/2) = -1", function()
        assert.is_true(approx_equal(trig.sin(-trig.PI / 2), -1.0))
    end)

    it("sin(-pi) = 0", function()
        assert.is_true(approx_equal(trig.sin(-trig.PI), 0.0))
    end)
end)

-- ============================================================================
-- Cosine Tests
-- ============================================================================

describe("cos", function()
    -- ## Landmark Values

    it("cos(0) = 1", function()
        -- When x = 0, all terms except the first (which is 1) are zero.
        assert.is_true(approx_equal(trig.cos(0), 1.0))
    end)

    it("cos(pi/6) = sqrt(3)/2", function()
        assert.is_true(approx_equal(trig.cos(trig.PI / 6), math.sqrt(3) / 2))
    end)

    it("cos(pi/4) = sqrt(2)/2", function()
        assert.is_true(approx_equal(trig.cos(trig.PI / 4), math.sqrt(2) / 2))
    end)

    it("cos(pi/3) = 0.5", function()
        assert.is_true(approx_equal(trig.cos(trig.PI / 3), 0.5))
    end)

    it("cos(pi/2) = 0", function()
        -- At 90 degrees, cosine crosses zero.
        assert.is_true(approx_equal(trig.cos(trig.PI / 2), 0.0))
    end)

    it("cos(pi) = -1", function()
        -- At 180 degrees, cosine reaches its minimum value of -1.
        assert.is_true(approx_equal(trig.cos(trig.PI), -1.0))
    end)

    it("cos(3*pi/2) = 0", function()
        assert.is_true(approx_equal(trig.cos(3 * trig.PI / 2), 0.0))
    end)

    it("cos(2*pi) = 1", function()
        -- A full cycle returns to 1.
        assert.is_true(approx_equal(trig.cos(trig.TWO_PI), 1.0))
    end)

    -- ## Even Symmetry: cos(-x) = cos(x)
    --
    -- Cosine is an "even function," meaning it's symmetric about the y-axis.
    -- If you mirror the cosine curve across the y-axis, you get the same curve.

    it("satisfies even symmetry: cos(-x) = cos(x)", function()
        local test_values = {0.5, 1.0, trig.PI / 4, trig.PI / 3, trig.PI / 2, 2.0, 3.0}
        for _, x in ipairs(test_values) do
            assert.is_true(
                approx_equal(trig.cos(-x), trig.cos(x)),
                string.format("cos(-%g) should equal cos(%g)", x, x)
            )
        end
    end)

    -- ## Negative quadrant values

    it("cos(-pi/2) = 0", function()
        assert.is_true(approx_equal(trig.cos(-trig.PI / 2), 0.0))
    end)

    it("cos(-pi) = -1", function()
        assert.is_true(approx_equal(trig.cos(-trig.PI), -1.0))
    end)
end)

-- ============================================================================
-- Pythagorean Identity: sin^2(x) + cos^2(x) = 1
-- ============================================================================

describe("pythagorean identity", function()
    -- This is perhaps the most important identity in trigonometry. It comes
    -- from the Pythagorean theorem applied to the unit circle: if a point on
    -- the unit circle has coordinates (cos(x), sin(x)), then:
    --
    --     cos^2(x) + sin^2(x) = 1^2 = 1
    --
    -- We test this for a variety of angles spanning different quadrants.

    it("holds for standard angles", function()
        local angles = {
            0, trig.PI / 6, trig.PI / 4, trig.PI / 3, trig.PI / 2,
            trig.PI, 3 * trig.PI / 2, trig.TWO_PI,
            -trig.PI / 4, -trig.PI / 2, -trig.PI,
        }
        for _, x in ipairs(angles) do
            local s = trig.sin(x)
            local c = trig.cos(x)
            local sum = s * s + c * c
            assert.is_true(
                approx_equal(sum, 1.0),
                string.format("sin(%g)^2 + cos(%g)^2 = %g, want 1.0", x, x, sum)
            )
        end
    end)

    it("holds for arbitrary angles", function()
        local angles = {0.1, 0.7, 1.5, 2.5, 5.0, 10.0, 42.0, -7.3}
        for _, x in ipairs(angles) do
            local s = trig.sin(x)
            local c = trig.cos(x)
            local sum = s * s + c * c
            assert.is_true(
                approx_equal(sum, 1.0),
                string.format("sin(%g)^2 + cos(%g)^2 = %g, want 1.0", x, x, sum)
            )
        end
    end)
end)

-- ============================================================================
-- Tangent Tests
-- ============================================================================

describe("tan", function()
    -- ## Definition
    --
    -- tan(x) = sin(x) / cos(x)
    --
    -- Tangent inherits correctness from sin and cos, but we verify it
    -- independently to catch any integration errors.

    it("tan(0) = 0", function()
        assert.is_true(approx_equal(trig.tan(0), 0.0))
    end)

    it("tan(pi/4) = 1", function()
        -- At 45 degrees, sin = cos, so tan = 1.
        assert.is_true(approx_equal(trig.tan(trig.PI / 4), 1.0))
    end)

    it("tan(pi) = 0", function()
        assert.is_true(approx_equal(trig.tan(trig.PI), 0.0))
    end)

    it("tan(-pi/4) = -1", function()
        -- Tangent is an odd function like sine.
        assert.is_true(approx_equal(trig.tan(-trig.PI / 4), -1.0))
    end)

    it("tan(pi/6) = 1/sqrt(3)", function()
        assert.is_true(approx_equal(trig.tan(trig.PI / 6), 1.0 / math.sqrt(3)))
    end)

    it("tan(pi/3) = sqrt(3)", function()
        assert.is_true(approx_equal(trig.tan(trig.PI / 3), math.sqrt(3)))
    end)

    it("satisfies odd symmetry: tan(-x) = -tan(x)", function()
        local test_values = {0.5, 1.0, trig.PI / 6, trig.PI / 4, trig.PI / 3}
        for _, x in ipairs(test_values) do
            assert.is_true(
                approx_equal(trig.tan(-x), -trig.tan(x)),
                string.format("tan(-%g) should equal -tan(%g)", x, x)
            )
        end
    end)

    -- ## Near-Asymptote Behaviour
    --
    -- tan(pi/2) is mathematically undefined (cos(pi/2) = 0). Our series
    -- won't produce exactly zero for cos(pi/2), so tan will return a very
    -- large number rather than inf. We just verify it's large.

    it("produces a very large value near pi/2", function()
        local val = math.abs(trig.tan(trig.PI / 2 - 1e-10))
        assert.is_true(val > 1e9, "tan near pi/2 should be very large")
    end)

    -- ## tan = sin/cos identity

    it("equals sin(x)/cos(x) for various angles", function()
        local angles = {0.1, 0.5, 1.0, 1.5, 2.0, 2.5, 3.0, -0.5, -1.0, -2.0}
        for _, x in ipairs(angles) do
            local expected = trig.sin(x) / trig.cos(x)
            assert.is_true(
                approx_equal(trig.tan(x), expected),
                string.format("tan(%g) should equal sin(%g)/cos(%g)", x, x, x)
            )
        end
    end)
end)

-- ============================================================================
-- Angle Conversion Tests
-- ============================================================================

describe("radians", function()
    -- ## Landmark Conversions

    it("converts 0 degrees to 0 radians", function()
        assert.is_true(approx_equal(trig.radians(0), 0))
    end)

    it("converts 90 degrees to pi/2", function()
        assert.is_true(approx_equal(trig.radians(90), trig.PI / 2))
    end)

    it("converts 180 degrees to pi", function()
        assert.is_true(approx_equal(trig.radians(180), trig.PI))
    end)

    it("converts 360 degrees to 2*pi", function()
        assert.is_true(approx_equal(trig.radians(360), trig.TWO_PI))
    end)

    it("converts negative angles correctly", function()
        assert.is_true(approx_equal(trig.radians(-180), -trig.PI))
    end)

    it("converts 45 degrees to pi/4", function()
        assert.is_true(approx_equal(trig.radians(45), trig.PI / 4))
    end)

    it("converts 30 degrees to pi/6", function()
        assert.is_true(approx_equal(trig.radians(30), trig.PI / 6))
    end)

    it("converts 60 degrees to pi/3", function()
        assert.is_true(approx_equal(trig.radians(60), trig.PI / 3))
    end)

    it("converts 270 degrees to 3*pi/2", function()
        assert.is_true(approx_equal(trig.radians(270), 3 * trig.PI / 2))
    end)
end)

describe("degrees", function()
    -- ## Landmark Conversions

    it("converts 0 radians to 0 degrees", function()
        assert.is_true(approx_equal(trig.degrees(0), 0))
    end)

    it("converts pi/2 to 90 degrees", function()
        assert.is_true(approx_equal(trig.degrees(trig.PI / 2), 90))
    end)

    it("converts pi to 180 degrees", function()
        assert.is_true(approx_equal(trig.degrees(trig.PI), 180))
    end)

    it("converts 2*pi to 360 degrees", function()
        assert.is_true(approx_equal(trig.degrees(trig.TWO_PI), 360))
    end)

    it("converts negative angles correctly", function()
        assert.is_true(approx_equal(trig.degrees(-trig.PI), -180))
    end)

    it("converts pi/4 to 45 degrees", function()
        assert.is_true(approx_equal(trig.degrees(trig.PI / 4), 45))
    end)

    it("converts pi/6 to 30 degrees", function()
        assert.is_true(approx_equal(trig.degrees(trig.PI / 6), 30))
    end)
end)

-- ============================================================================
-- Round-Trip Conversion Tests
-- ============================================================================

describe("round-trip conversion", function()
    -- Converting degrees -> radians -> degrees should return the original
    -- value. This catches errors in either conversion function.

    it("degrees -> radians -> degrees preserves the original value", function()
        local test_values = {0, 30, 45, 60, 90, 120, 180, 270, 360, -45, -90}
        for _, deg in ipairs(test_values) do
            local got = trig.degrees(trig.radians(deg))
            assert.is_true(
                approx_equal(got, deg),
                string.format("degrees(radians(%g)) = %g, want %g", deg, got, deg)
            )
        end
    end)

    it("radians -> degrees -> radians preserves the original value", function()
        local test_values = {0, trig.PI / 6, trig.PI / 4, trig.PI / 3, trig.PI / 2, trig.PI, -trig.PI / 4}
        for _, rad in ipairs(test_values) do
            local got = trig.radians(trig.degrees(rad))
            assert.is_true(
                approx_equal(got, rad),
                string.format("radians(degrees(%g)) = %g, want %g", rad, got, rad)
            )
        end
    end)
end)

-- ============================================================================
-- Cross-Validation Against math Library
-- ============================================================================

describe("cross-validation against math library", function()
    -- While our goal is to implement trig from scratch, we can use Lua's
    -- built-in math.sin and math.cos as a reference to verify accuracy
    -- across many data points.

    it("sin matches math.sin for many angles", function()
        for i = -31, 31 do
            local x = i * 0.1 * trig.PI
            assert.is_true(
                approx_equal(trig.sin(x), math.sin(x), 1e-8),
                string.format("sin(%g): got %g, expected %g", x, trig.sin(x), math.sin(x))
            )
        end
    end)

    it("cos matches math.cos for many angles", function()
        for i = -31, 31 do
            local x = i * 0.1 * trig.PI
            assert.is_true(
                approx_equal(trig.cos(x), math.cos(x), 1e-8),
                string.format("cos(%g): got %g, expected %g", x, trig.cos(x), math.cos(x))
            )
        end
    end)

    it("tan matches math.tan for angles away from asymptotes", function()
        -- Avoid pi/2 and 3*pi/2 where tangent is undefined.
        local safe_angles = {0, 0.1, 0.5, 1.0, -0.5, -1.0, 2.0, 2.5, 3.0, -2.0}
        for _, x in ipairs(safe_angles) do
            assert.is_true(
                approx_equal(trig.tan(x), math.tan(x), 1e-8),
                string.format("tan(%g): got %g, expected %g", x, trig.tan(x), math.tan(x))
            )
        end
    end)
end)

-- ============================================================================
-- Complementary Angle Identity: sin(x) = cos(pi/2 - x)
-- ============================================================================

describe("complementary angle identity", function()
    -- The co-function identity states that sine and cosine are related by
    -- a phase shift of pi/2:
    --
    --     sin(x) = cos(pi/2 - x)
    --     cos(x) = sin(pi/2 - x)
    --
    -- This identity is the reason cosine is called "co-sine" -- it's the
    -- sine of the complementary angle.

    it("sin(x) = cos(pi/2 - x)", function()
        local angles = {0, 0.3, 0.7, 1.0, trig.PI / 6, trig.PI / 4, trig.PI / 3}
        for _, x in ipairs(angles) do
            assert.is_true(
                approx_equal(trig.sin(x), trig.cos(trig.PI / 2 - x)),
                string.format("sin(%g) should equal cos(pi/2 - %g)", x, x)
            )
        end
    end)

    it("cos(x) = sin(pi/2 - x)", function()
        local angles = {0, 0.3, 0.7, 1.0, trig.PI / 6, trig.PI / 4, trig.PI / 3}
        for _, x in ipairs(angles) do
            assert.is_true(
                approx_equal(trig.cos(x), trig.sin(trig.PI / 2 - x)),
                string.format("cos(%g) should equal sin(pi/2 - %g)", x, x)
            )
        end
    end)
end)

-- ============================================================================
-- Double Angle Identities
-- ============================================================================

describe("double angle identities", function()
    -- sin(2x) = 2 * sin(x) * cos(x)
    -- cos(2x) = cos^2(x) - sin^2(x)

    it("sin(2x) = 2*sin(x)*cos(x)", function()
        local angles = {0.1, 0.5, 1.0, trig.PI / 6, trig.PI / 4, trig.PI / 3, -0.7, 2.0}
        for _, x in ipairs(angles) do
            local lhs = trig.sin(2 * x)
            local rhs = 2 * trig.sin(x) * trig.cos(x)
            assert.is_true(
                approx_equal(lhs, rhs),
                string.format("sin(2*%g): got %g, expected %g", x, lhs, rhs)
            )
        end
    end)

    it("cos(2x) = cos^2(x) - sin^2(x)", function()
        local angles = {0.1, 0.5, 1.0, trig.PI / 6, trig.PI / 4, trig.PI / 3, -0.7, 2.0}
        for _, x in ipairs(angles) do
            local lhs = trig.cos(2 * x)
            local rhs = trig.cos(x)^2 - trig.sin(x)^2
            assert.is_true(
                approx_equal(lhs, rhs),
                string.format("cos(2*%g): got %g, expected %g", x, lhs, rhs)
            )
        end
    end)
end)

-- ============================================================================
-- sqrt tests
-- ============================================================================

describe("sqrt", function()
    it("sqrt(0) is 0", function()
        assert.are.equal(trig.sqrt(0), 0.0)
    end)

    it("sqrt(1) is 1", function()
        assert.is_true(approx_equal(trig.sqrt(1), 1.0))
    end)

    it("sqrt(4) is 2", function()
        assert.is_true(approx_equal(trig.sqrt(4), 2.0))
    end)

    it("sqrt(9) is 3", function()
        assert.is_true(approx_equal(trig.sqrt(9), 3.0))
    end)

    it("sqrt(2) ≈ 1.41421356237", function()
        assert.is_true(approx_equal(trig.sqrt(2), 1.41421356237))
    end)

    it("sqrt(0.25) is 0.5", function()
        assert.is_true(approx_equal(trig.sqrt(0.25), 0.5))
    end)

    it("sqrt(1e10) ≈ 1e5", function()
        assert.is_true(approx_equal(trig.sqrt(1e10), 1e5, 1e-4))
    end)

    it("sqrt(2)^2 ≈ 2.0 (roundtrip)", function()
        local s = trig.sqrt(2)
        assert.is_true(approx_equal(s * s, 2.0))
    end)

    it("sqrt(-1) raises an error", function()
        assert.has_error(function() trig.sqrt(-1) end)
    end)
end)

-- ============================================================================
-- atan tests
-- ============================================================================

describe("atan", function()
    it("atan(0) is 0", function()
        assert.are.equal(trig.atan(0.0), 0.0)
    end)

    it("atan(1) ≈ pi/4", function()
        assert.is_true(approx_equal(trig.atan(1), trig.PI / 4))
    end)

    it("atan(-1) ≈ -pi/4", function()
        assert.is_true(approx_equal(trig.atan(-1), -trig.PI / 4))
    end)

    it("atan(sqrt(3)) ≈ pi/3", function()
        assert.is_true(approx_equal(trig.atan(trig.sqrt(3)), trig.PI / 3))
    end)

    it("atan(1/sqrt(3)) ≈ pi/6", function()
        assert.is_true(approx_equal(trig.atan(1.0 / trig.sqrt(3)), trig.PI / 6))
    end)

    it("atan(large positive) approaches pi/2", function()
        assert.is_true(approx_equal(trig.atan(1e10), trig.PI / 2, 1e-5))
    end)

    it("atan(large negative) approaches -pi/2", function()
        assert.is_true(approx_equal(trig.atan(-1e10), -trig.PI / 2, 1e-5))
    end)

    it("atan(tan(pi/4)) ≈ pi/4 (roundtrip)", function()
        assert.is_true(approx_equal(trig.atan(trig.tan(trig.PI / 4)), trig.PI / 4))
    end)
end)

-- ============================================================================
-- atan2 tests
-- ============================================================================

describe("atan2", function()
    it("atan2(0, 1) = 0 (positive x-axis)", function()
        assert.is_true(approx_equal(trig.atan2(0, 1), 0.0))
    end)

    it("atan2(1, 0) = pi/2 (positive y-axis)", function()
        assert.is_true(approx_equal(trig.atan2(1, 0), trig.PI / 2))
    end)

    it("atan2(0, -1) = pi (negative x-axis)", function()
        assert.is_true(approx_equal(trig.atan2(0, -1), trig.PI))
    end)

    it("atan2(-1, 0) = -pi/2 (negative y-axis)", function()
        assert.is_true(approx_equal(trig.atan2(-1, 0), -trig.PI / 2))
    end)

    it("atan2(1, 1) ≈ pi/4 (Q1)", function()
        assert.is_true(approx_equal(trig.atan2(1, 1), trig.PI / 4))
    end)

    it("atan2(1, -1) ≈ 3*pi/4 (Q2)", function()
        assert.is_true(approx_equal(trig.atan2(1, -1), 3 * trig.PI / 4))
    end)

    it("atan2(-1, -1) ≈ -3*pi/4 (Q3)", function()
        assert.is_true(approx_equal(trig.atan2(-1, -1), -3 * trig.PI / 4))
    end)

    it("atan2(-1, 1) ≈ -pi/4 (Q4)", function()
        assert.is_true(approx_equal(trig.atan2(-1, 1), -trig.PI / 4))
    end)
end)
