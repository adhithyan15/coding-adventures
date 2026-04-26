-- ============================================================================
-- Tests for activation_functions — sigmoid, relu, tanh, elu, softmax
-- ============================================================================
--
-- ## Testing Strategy
--
-- For each activation function we verify:
--
--   1. A golden value computed by hand or against a trusted reference
--      (Python's scipy/numpy, Wolfram Alpha, etc.).
--   2. Edge cases: zero input, very large positive, very large negative.
--   3. Monotonicity / sign checks where applicable.
--   4. Consistency between the function and its derivative:
--      finite-difference numerical gradient should match the analytical
--      gradient to within 1e-5.
--
-- ## Finite Difference Check
--
-- For any function f, the central-difference formula approximates f'(x):
--
--     f'(x) ≈ (f(x + h) − f(x − h)) / (2h)    with h = 1e-5
--
-- We compare this to the analytical derivative. A match within 1e-5 confirms
-- the derivative implementation is correct.
--
-- ## Tolerance
--
-- All floating-point comparisons use a tolerance of 1e-9 unless otherwise
-- noted. This is well within the precision of IEEE 754 double arithmetic.

package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path

local af = require("coding_adventures.activation_functions")

-- ============================================================================
-- Helpers
-- ============================================================================

--- near(a, b, tol) — true if |a - b| < tol
local function near(a, b, tol)
    tol = tol or 1e-9
    return math.abs(a - b) < tol
end

--- near_table(a, b, tol) — true if two tables are element-wise near
local function near_table(a, b, tol)
    if #a ~= #b then return false end
    for i = 1, #a do
        if not near(a[i], b[i], tol) then return false end
    end
    return true
end

--- sum(t) — sum of all elements in table t
local function sum(t)
    local s = 0.0
    for _, v in ipairs(t) do s = s + v end
    return s
end

--- fd_derivative(f, x, h) — finite-difference derivative of f at x
-- Uses the central difference formula for better accuracy than one-sided.
local function fd_derivative(f, x, h)
    h = h or 1e-5
    return (f(x + h) - f(x - h)) / (2.0 * h)
end

-- ============================================================================
-- LINEAR
-- ============================================================================

describe("linear", function()
    it("returns the input unchanged", function()
        assert.is_true(near(af.linear(-3), -3.0))
        assert.is_true(near(af.linear(0), 0.0))
        assert.is_true(near(af.linear(5), 5.0))
    end)
end)

describe("linear_derivative", function()
    it("returns 1 everywhere", function()
        for _, x in ipairs({-3, 0, 5}) do
            assert.is_true(near(af.linear_derivative(x), 1.0))
        end
    end)
end)

-- ============================================================================
-- SIGMOID
-- ============================================================================

describe("sigmoid", function()
    it("returns 0.5 at x=0", function()
        assert.is_true(near(af.sigmoid(0), 0.5))
    end)

    it("approaches 1 for large positive x", function()
        -- sigmoid(100) should be essentially 1.0
        assert.is_true(near(af.sigmoid(100), 1.0, 1e-6))
    end)

    it("approaches 0 for large negative x", function()
        assert.is_true(near(af.sigmoid(-100), 0.0, 1e-6))
    end)

    it("clamps at x < -709 to exactly 0.0", function()
        -- -710 is below the clamping threshold
        assert.are.equal(af.sigmoid(-710), 0.0)
    end)

    it("clamps at x > 709 to exactly 1.0", function()
        assert.are.equal(af.sigmoid(710), 1.0)
    end)

    it("returns correct golden value at x=1", function()
        -- sigmoid(1) = 1/(1+e^(-1)) ≈ 0.7310585786
        assert.is_true(near(af.sigmoid(1), 0.7310585786, 1e-9))
    end)

    it("returns correct golden value at x=-1", function()
        -- sigmoid(-1) = 1/(1+e^1) ≈ 0.2689414214
        assert.is_true(near(af.sigmoid(-1), 0.2689414214, 1e-9))
    end)

    it("is monotonically increasing", function()
        local xs = {-10, -5, -1, 0, 1, 5, 10}
        for i = 1, #xs - 1 do
            assert.is_true(af.sigmoid(xs[i]) < af.sigmoid(xs[i+1]))
        end
    end)
end)

-- ============================================================================
-- SIGMOID DERIVATIVE
-- ============================================================================

describe("sigmoid_derivative", function()
    it("is 0.25 at x=0 (maximum value)", function()
        -- σ(0)*(1-σ(0)) = 0.5 * 0.5 = 0.25
        assert.is_true(near(af.sigmoid_derivative(0), 0.25))
    end)

    it("is positive everywhere", function()
        for _, x in ipairs({-10, -3, -1, 0, 1, 3, 10}) do
            assert.is_true(af.sigmoid_derivative(x) > 0)
        end
    end)

    it("matches finite-difference approximation at x=1", function()
        local analytical = af.sigmoid_derivative(1)
        local numerical  = fd_derivative(af.sigmoid, 1)
        assert.is_true(near(analytical, numerical, 1e-5))
    end)

    it("matches finite-difference approximation at x=-2", function()
        local analytical = af.sigmoid_derivative(-2)
        local numerical  = fd_derivative(af.sigmoid, -2)
        assert.is_true(near(analytical, numerical, 1e-5))
    end)

    it("vanishes for large |x| (vanishing gradient)", function()
        assert.is_true(af.sigmoid_derivative(20) < 1e-7)
        assert.is_true(af.sigmoid_derivative(-20) < 1e-7)
    end)
end)

-- ============================================================================
-- RELU
-- ============================================================================

describe("relu", function()
    it("returns 0 for negative input", function()
        assert.are.equal(af.relu(-5), 0.0)
        assert.are.equal(af.relu(-0.001), 0.0)
    end)

    it("returns 0 at x=0", function()
        assert.are.equal(af.relu(0), 0.0)
    end)

    it("returns identity for positive input", function()
        assert.is_true(near(af.relu(3.5), 3.5))
        assert.is_true(near(af.relu(100), 100))
    end)

    it("is correct for fractional values", function()
        assert.is_true(near(af.relu(0.001), 0.001))
    end)
end)

describe("relu_derivative", function()
    it("returns 0 for negative input", function()
        assert.are.equal(af.relu_derivative(-5), 0)
        assert.are.equal(af.relu_derivative(-0.001), 0)
    end)

    it("returns 0 at x=0 (sub-gradient convention)", function()
        assert.are.equal(af.relu_derivative(0), 0)
    end)

    it("returns 1 for positive input", function()
        assert.are.equal(af.relu_derivative(5), 1)
        assert.are.equal(af.relu_derivative(0.001), 1)
    end)

    it("matches finite-difference approximation away from the kink", function()
        -- Test at x=1 (away from the non-differentiable point x=0)
        local analytical = af.relu_derivative(1)
        local numerical  = fd_derivative(af.relu, 1)
        assert.is_true(near(analytical, numerical, 1e-4))
    end)
end)

-- ============================================================================
-- TANH
-- ============================================================================

describe("tanh_activation", function()
    it("returns 0 at x=0 (zero-centred)", function()
        assert.is_true(near(af.tanh_activation(0), 0.0))
    end)

    it("approaches 1 for large positive x", function()
        assert.is_true(near(af.tanh_activation(100), 1.0, 1e-6))
    end)

    it("approaches -1 for large negative x", function()
        assert.is_true(near(af.tanh_activation(-100), -1.0, 1e-6))
    end)

    it("returns correct golden value at x=1", function()
        -- tanh(1) ≈ 0.7615941559557649
        assert.is_true(near(af.tanh_activation(1), 0.7615941559557649, 1e-9))
    end)

    it("is anti-symmetric: tanh(-x) = -tanh(x)", function()
        for _, x in ipairs({0.5, 1, 2, 3}) do
            assert.is_true(near(af.tanh_activation(-x), -af.tanh_activation(x)))
        end
    end)
end)

describe("tanh_derivative", function()
    it("returns 1 at x=0 (maximum)", function()
        assert.is_true(near(af.tanh_derivative(0), 1.0))
    end)

    it("is non-negative everywhere", function()
        for _, x in ipairs({-10, -3, -1, 0, 1, 3, 10}) do
            assert.is_true(af.tanh_derivative(x) >= 0)
        end
    end)

    it("matches finite-difference approximation at x=1", function()
        local analytical = af.tanh_derivative(1)
        local numerical  = fd_derivative(af.tanh_activation, 1)
        assert.is_true(near(analytical, numerical, 1e-5))
    end)

    it("vanishes for large |x|", function()
        assert.is_true(af.tanh_derivative(10) < 1e-8)
        assert.is_true(af.tanh_derivative(-10) < 1e-8)
    end)
end)

-- ============================================================================
-- LEAKY RELU
-- ============================================================================

describe("leaky_relu", function()
    it("returns identity for positive x (default alpha)", function()
        assert.is_true(near(af.leaky_relu(5), 5))
    end)

    it("returns alpha*x for negative x (default alpha=0.01)", function()
        -- leaky_relu(-10) = 0.01 * (-10) = -0.1
        assert.is_true(near(af.leaky_relu(-10), -0.1, 1e-9))
    end)

    it("returns 0 at x=0", function()
        assert.is_true(near(af.leaky_relu(0), 0.0))
    end)

    it("uses custom alpha correctly", function()
        -- leaky_relu(-5, 0.2) = 0.2 * (-5) = -1.0
        assert.is_true(near(af.leaky_relu(-5, 0.2), -1.0, 1e-9))
    end)

    it("is never positive for negative x (default alpha)", function()
        assert.is_true(af.leaky_relu(-100) < 0)
    end)
end)

describe("leaky_relu_derivative", function()
    it("returns 1 for positive x", function()
        assert.are.equal(af.leaky_relu_derivative(5), 1)
    end)

    it("returns alpha for negative x (default 0.01)", function()
        assert.is_true(near(af.leaky_relu_derivative(-5), 0.01))
    end)

    it("returns alpha for x=0 (sub-gradient convention)", function()
        -- At x=0 we return alpha (leaky side), matching the x<=0 branch
        assert.is_true(near(af.leaky_relu_derivative(0), 0.01))
    end)

    it("uses custom alpha in derivative", function()
        assert.is_true(near(af.leaky_relu_derivative(-1, 0.1), 0.1))
    end)

    it("matches finite-difference away from kink", function()
        local f = function(x) return af.leaky_relu(x) end
        local analytical = af.leaky_relu_derivative(2)
        local numerical  = fd_derivative(f, 2)
        assert.is_true(near(analytical, numerical, 1e-4))
    end)
end)

-- ============================================================================
-- SOFTPLUS
-- ============================================================================

describe("softplus", function()
    it("returns log(2) at x=0", function()
        assert.is_true(near(af.softplus(0), math.log(2.0)))
    end)

    it("matches golden values", function()
        assert.is_true(near(af.softplus(1), 1.3132616875182228, 1e-9))
        assert.is_true(near(af.softplus(-1), 0.31326168751822286, 1e-9))
    end)

    it("stays stable for large positive values", function()
        assert.is_true(af.softplus(1000) > 999.0)
    end)
end)

describe("softplus_derivative", function()
    it("equals sigmoid", function()
        for _, x in ipairs({-1, 0, 1}) do
            assert.is_true(near(af.softplus_derivative(x), af.sigmoid(x)))
        end
    end)

    it("matches finite-difference approximation at x=1", function()
        local analytical = af.softplus_derivative(1)
        local numerical = fd_derivative(af.softplus, 1)
        assert.is_true(near(analytical, numerical, 1e-5))
    end)
end)

-- ============================================================================
-- ELU
-- ============================================================================

describe("elu", function()
    it("returns identity for x >= 0 (default alpha)", function()
        assert.is_true(near(af.elu(0), 0.0))
        assert.is_true(near(af.elu(3), 3.0))
    end)

    it("returns alpha*(e^x - 1) for x < 0", function()
        -- elu(-1, 1.0) = 1.0 * (e^(-1) - 1) = e^(-1) - 1 ≈ -0.6321205588
        local expected = math.exp(-1) - 1
        assert.is_true(near(af.elu(-1), expected, 1e-9))
    end)

    it("saturates at -alpha as x → -∞ (approaches -1 for default alpha=1)", function()
        -- elu(-100, 1.0) ≈ 1.0 * (0 - 1) = -1.0
        assert.is_true(near(af.elu(-100), -1.0, 1e-6))
    end)

    it("uses custom alpha correctly", function()
        -- elu(-1, 2.0) = 2.0 * (e^(-1) - 1)
        local expected = 2.0 * (math.exp(-1) - 1)
        assert.is_true(near(af.elu(-1, 2.0), expected, 1e-9))
    end)

    it("is continuous at x=0", function()
        -- elu(0-eps) should be close to 0, and elu(0) = 0
        assert.is_true(near(af.elu(-1e-10), 0.0, 1e-9))
        assert.is_true(near(af.elu(0), 0.0))
    end)
end)

describe("elu_derivative", function()
    it("returns 1 for x >= 0", function()
        assert.are.equal(af.elu_derivative(0), 1)
        assert.are.equal(af.elu_derivative(5), 1)
    end)

    it("returns alpha*e^x for x < 0", function()
        -- elu_derivative(-1, 1.0) = 1.0 * e^(-1) ≈ 0.3678794412
        assert.is_true(near(af.elu_derivative(-1), math.exp(-1), 1e-9))
    end)

    it("uses custom alpha in derivative", function()
        local expected = 2.0 * math.exp(-1)
        assert.is_true(near(af.elu_derivative(-1, 2.0), expected, 1e-9))
    end)

    it("approaches 0 for very negative x (exponential decay)", function()
        assert.is_true(af.elu_derivative(-100) < 1e-10)
    end)

    it("matches finite-difference approximation at x=-1", function()
        local f = function(x) return af.elu(x) end
        local analytical = af.elu_derivative(-1)
        local numerical  = fd_derivative(f, -1)
        assert.is_true(near(analytical, numerical, 1e-5))
    end)
end)

-- ============================================================================
-- SOFTMAX
-- ============================================================================

describe("softmax", function()
    it("returns probabilities that sum to 1", function()
        local result = af.softmax({1, 2, 3})
        assert.is_true(near(sum(result), 1.0, 1e-12))
    end)

    it("all outputs are positive", function()
        local result = af.softmax({-5, 0, 5, 100})
        for _, v in ipairs(result) do
            assert.is_true(v > 0)
        end
    end)

    it("is invariant to adding a constant (numerical stability property)", function()
        -- softmax({1,2,3}) should equal softmax({1001,1002,1003})
        local r1 = af.softmax({1, 2, 3})
        local r2 = af.softmax({1001, 1002, 1003})
        assert.is_true(near_table(r1, r2, 1e-10))
    end)

    it("handles large values without overflow (max-subtraction trick)", function()
        -- Without the trick, e^1000 overflows; with it, everything is fine
        local result = af.softmax({1000, 1001, 1002})
        assert.is_true(near(sum(result), 1.0, 1e-12))
    end)

    it("gives the correct golden values for {1,2,3}", function()
        -- Computed reference:
        --   e^1 ≈ 2.71828, e^2 ≈ 7.38906, e^3 ≈ 20.08554
        --   sum ≈ 30.19288
        --   probabilities: 0.09003057, 0.24472847, 0.66524096
        local result = af.softmax({1, 2, 3})
        assert.is_true(near(result[1], 0.09003057, 1e-6))
        assert.is_true(near(result[2], 0.24472847, 1e-6))
        assert.is_true(near(result[3], 0.66524096, 1e-6))
    end)

    it("works for a single-element array", function()
        local result = af.softmax({42})
        assert.is_true(near(result[1], 1.0))
    end)

    it("is uniform for all-equal inputs", function()
        -- softmax({c,c,c}) = {1/3, 1/3, 1/3}
        local result = af.softmax({5, 5, 5})
        for _, v in ipairs(result) do
            assert.is_true(near(v, 1.0/3.0, 1e-10))
        end
    end)

    it("errors on empty table", function()
        assert.has_error(function() af.softmax({}) end)
    end)
end)

describe("softmax_derivative", function()
    it("all entries are positive", function()
        local result = af.softmax_derivative({1, 2, 3})
        for _, v in ipairs(result) do
            assert.is_true(v > 0)
        end
    end)

    it("all entries are at most 0.25 (since s_i*(1-s_i) <= 0.25)", function()
        -- For any p ∈ (0,1), p*(1-p) is maximised at p=0.5 giving 0.25
        local result = af.softmax_derivative({1, 2, 3})
        for _, v in ipairs(result) do
            assert.is_true(v <= 0.25 + 1e-12)
        end
    end)

    it("has same length as input", function()
        local result = af.softmax_derivative({1, 2, 3, 4, 5})
        assert.are.equal(#result, 5)
    end)

    it("matches s_i*(1-s_i) for each element", function()
        local s = af.softmax({1, 2, 3})
        local d = af.softmax_derivative({1, 2, 3})
        for i = 1, 3 do
            local expected = s[i] * (1 - s[i])
            assert.is_true(near(d[i], expected, 1e-12))
        end
    end)
end)
