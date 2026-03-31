-- ============================================================================
-- Tests for loss_functions — MSE, MAE, BCE, CCE and their derivatives
-- ============================================================================
--
-- These tests verify correctness of all eight loss functions and their
-- numerical derivatives.
--
-- ## Testing Strategy
--
-- For each loss function we check:
--   1. A hand-computed golden value with known inputs.
--   2. Error handling: mismatched lengths, empty arrays.
--   3. Edge cases: all-correct predictions (loss = 0 for MSE/MAE), etc.
--
-- For derivatives we verify:
--   1. The gradient has the same length as the inputs.
--   2. The sign of the gradient is correct (should point "away" from target).
--   3. A numerical gradient check: finite-difference approximation should
--      match the analytical gradient to within ~1e-5.
--
-- ## Floating-Point Comparison
--
-- All comparisons use a tolerance of 1e-6.  This is more than enough
-- precision for loss-function calculations that involve only a handful of
-- operations, while forgiving the tiny rounding errors inherent to
-- IEEE 754 double precision.

-- Add the src/ tree to the Lua module search path so `require` can find
-- coding_adventures.loss_functions without an installed rockspec.
package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path

local lf = require("coding_adventures.loss_functions")

-- ============================================================================
-- Helpers
-- ============================================================================

--- near returns true if |a - b| < tol.
-- We use a default tolerance of 1e-6 throughout.
local function near(a, b, tol)
    tol = tol or 1e-6
    return math.abs(a - b) < tol
end

--- near_table returns true if two tables have the same length and every
-- corresponding element satisfies near(a[i], b[i], tol).
local function near_table(a, b, tol)
    if #a ~= #b then return false end
    for i = 1, #a do
        if not near(a[i], b[i], tol) then return false end
    end
    return true
end

--- numerical_gradient computes a finite-difference approximation of
-- ∂L/∂ŷᵢ using the central-difference formula:
--
--     (L(ŷᵢ + h) - L(ŷᵢ - h)) / (2h)
--
-- This is a standard way to verify analytical gradients in tests.
-- h = 1e-5 gives a good balance between truncation and round-off error.
local function numerical_gradient(loss_fn, y_true, y_pred)
    local h   = 1e-5
    local n   = #y_pred
    local num = {}

    for i = 1, n do
        -- Perturb element i upward, compute loss.
        local y_plus  = {}
        local y_minus = {}
        for j = 1, n do
            y_plus[j]  = y_pred[j]
            y_minus[j] = y_pred[j]
        end
        y_plus[i]  = y_pred[i] + h
        y_minus[i] = y_pred[i] - h

        local l_plus,  _ = loss_fn(y_true, y_plus)
        local l_minus, _ = loss_fn(y_true, y_minus)

        -- Central difference: more accurate than forward difference.
        num[i] = (l_plus - l_minus) / (2.0 * h)
    end

    return num
end

-- ============================================================================
-- Version
-- ============================================================================

describe("module metadata", function()
    it("has the correct version string", function()
        assert.are.equal("0.1.0", lf.VERSION)
    end)
end)

-- ============================================================================
-- MSE Tests
-- ============================================================================

describe("mse", function()
    -- ## Hand-computed golden test
    --
    -- y_true = {1, 2, 3}
    -- y_pred = {1.1, 1.9, 3.2}
    -- residuals: {-0.1, 0.1, -0.2}
    -- squared:   {0.01, 0.01, 0.04}
    -- MSE = (0.01 + 0.01 + 0.04) / 3 = 0.06 / 3 = 0.02
    it("computes the correct MSE for known inputs", function()
        local y_true = {1.0, 2.0, 3.0}
        local y_pred = {1.1, 1.9, 3.2}
        local loss, err = lf.mse(y_true, y_pred)
        assert.is_nil(err)
        assert.is_true(near(loss, 0.02))
    end)

    it("returns 0 when predictions exactly match targets", function()
        local y_true = {1.0, 2.0, 3.0}
        local y_pred = {1.0, 2.0, 3.0}
        local loss, err = lf.mse(y_true, y_pred)
        assert.is_nil(err)
        assert.is_true(near(loss, 0.0))
    end)

    it("works for a single element", function()
        -- (3 - 5)^2 / 1 = 4
        local loss, err = lf.mse({3.0}, {5.0})
        assert.is_nil(err)
        assert.is_true(near(loss, 4.0))
    end)

    it("is symmetric: MSE(y,ŷ) == MSE(ŷ,y)", function()
        -- Squaring means the sign of the residual doesn't matter.
        local y_true = {1.0, 2.0}
        local y_pred = {3.0, 4.0}
        local loss1, _ = lf.mse(y_true, y_pred)
        local loss2, _ = lf.mse(y_pred, y_true)
        assert.is_true(near(loss1, loss2))
    end)

    it("is always non-negative", function()
        local y_true = {-2.0, 0.5, 10.0}
        local y_pred = { 1.0, 2.0,  8.0}
        local loss, err = lf.mse(y_true, y_pred)
        assert.is_nil(err)
        assert.is_true(loss >= 0.0)
    end)

    it("returns an error for mismatched lengths", function()
        local _, err = lf.mse({1.0, 2.0}, {1.0})
        assert.is_not_nil(err)
        assert.is_true(type(err) == "string")
    end)

    it("returns an error for empty arrays", function()
        local _, err = lf.mse({}, {})
        assert.is_not_nil(err)
    end)

    it("returns an error when y_true is not a table", function()
        local _, err = lf.mse(42, {1.0})
        assert.is_not_nil(err)
    end)

    it("returns an error when y_pred is not a table", function()
        local _, err = lf.mse({1.0}, "oops")
        assert.is_not_nil(err)
    end)
end)

-- ============================================================================
-- MAE Tests
-- ============================================================================

describe("mae", function()
    -- y_true = {1, 2, 3}
    -- y_pred = {1.1, 1.9, 3.2}
    -- |residuals| = {0.1, 0.1, 0.2}
    -- MAE = 0.4 / 3 ≈ 0.13333
    it("computes the correct MAE for known inputs", function()
        local y_true = {1.0, 2.0, 3.0}
        local y_pred = {1.1, 1.9, 3.2}
        local loss, err = lf.mae(y_true, y_pred)
        assert.is_nil(err)
        assert.is_true(near(loss, 0.4 / 3.0))
    end)

    it("returns 0 when predictions exactly match targets", function()
        local loss, err = lf.mae({1.0, 2.0}, {1.0, 2.0})
        assert.is_nil(err)
        assert.is_true(near(loss, 0.0))
    end)

    it("works for a single element", function()
        -- |3 - 5| / 1 = 2
        local loss, err = lf.mae({3.0}, {5.0})
        assert.is_nil(err)
        assert.is_true(near(loss, 2.0))
    end)

    it("is symmetric: MAE(y,ŷ) == MAE(ŷ,y)", function()
        local y_true = {1.0, 2.0}
        local y_pred = {3.0, 4.0}
        local l1, _ = lf.mae(y_true, y_pred)
        local l2, _ = lf.mae(y_pred, y_true)
        assert.is_true(near(l1, l2))
    end)

    it("is always non-negative", function()
        local y_true = {-2.0, 0.5, 10.0}
        local y_pred = { 1.0, 2.0,  8.0}
        local loss, err = lf.mae(y_true, y_pred)
        assert.is_nil(err)
        assert.is_true(loss >= 0.0)
    end)

    it("returns an error for mismatched lengths", function()
        local _, err = lf.mae({1.0, 2.0}, {1.0})
        assert.is_not_nil(err)
    end)

    it("returns an error for empty arrays", function()
        local _, err = lf.mae({}, {})
        assert.is_not_nil(err)
    end)
end)

-- ============================================================================
-- BCE Tests
-- ============================================================================

describe("bce", function()
    -- ## Hand-computed golden test
    --
    -- y_true = {1, 0, 1}
    -- y_pred = {0.9, 0.1, 0.8}
    --
    -- BCE = -[ 1*log(0.9) + 0*log(0.1) + 0*log(0.9) + 1*log(0.8) ] / 3
    --       Wait: BCE[i] = y*log(p) + (1-y)*log(1-p)
    -- i=1: 1*log(0.9) + 0*log(0.1) = log(0.9)
    -- i=2: 0*log(0.1) + 1*log(0.9) = log(0.9)
    -- i=3: 1*log(0.8) + 0*log(0.2) = log(0.8)
    -- sum = log(0.9) + log(0.9) + log(0.8) ≈ -0.10536 - 0.10536 - 0.22314 = -0.43386
    -- BCE = 0.43386 / 3 ≈ 0.14462

    it("computes the correct BCE for known inputs", function()
        local y_true = {1.0, 0.0, 1.0}
        local y_pred = {0.9, 0.1, 0.8}

        -- Compute the expected value analytically.
        local expected = -(math.log(0.9) + math.log(0.9) + math.log(0.8)) / 3.0

        local loss, err = lf.bce(y_true, y_pred)
        assert.is_nil(err)
        assert.is_true(near(loss, expected))
    end)

    it("is non-negative", function()
        local y_true = {1.0, 0.0, 1.0}
        local y_pred = {0.9, 0.1, 0.8}
        local loss, _ = lf.bce(y_true, y_pred)
        assert.is_true(loss >= 0.0)
    end)

    it("is near 0 when predictions are correct and confident", function()
        -- Near-perfect predictions → low BCE.
        local y_true = {1.0, 0.0}
        local y_pred = {0.9999, 0.0001}
        local loss, err = lf.bce(y_true, y_pred)
        assert.is_nil(err)
        assert.is_true(loss < 0.01)
    end)

    it("is large when predictions are confidently wrong", function()
        local y_true = {1.0, 0.0}
        local y_pred = {0.0001, 0.9999}
        local loss, err = lf.bce(y_true, y_pred)
        assert.is_nil(err)
        assert.is_true(loss > 5.0)
    end)

    it("does not return NaN for predictions at 0 or 1 (clamping)", function()
        -- Without clamping, log(0) = -inf.  With clamping we get a finite number.
        local y_true = {1.0, 0.0}
        local y_pred = {0.0, 1.0}   -- worst-case predictions
        local loss, err = lf.bce(y_true, y_pred)
        assert.is_nil(err)
        assert.is_true(loss == loss)  -- NaN ~= NaN in IEEE 754, so this checks non-NaN
        assert.is_true(loss < math.huge)
    end)

    it("returns an error for mismatched lengths", function()
        local _, err = lf.bce({1.0, 0.0}, {0.9})
        assert.is_not_nil(err)
    end)

    it("returns an error for empty arrays", function()
        local _, err = lf.bce({}, {})
        assert.is_not_nil(err)
    end)
end)

-- ============================================================================
-- CCE Tests
-- ============================================================================

describe("cce", function()
    -- ## Hand-computed golden test (3-class one-hot)
    --
    -- y_true = {0, 1, 0}  (true class is index 2)
    -- y_pred = {0.2, 0.7, 0.1}
    -- CCE = -(0*log(0.2) + 1*log(0.7) + 0*log(0.1)) / 3
    --     = -log(0.7) / 3
    --     ≈ 0.35667 / 3 ≈ 0.11889

    it("computes the correct CCE for a one-hot example", function()
        local y_true = {0.0, 1.0, 0.0}
        local y_pred = {0.2, 0.7, 0.1}
        local expected = -math.log(0.7) / 3.0
        local loss, err = lf.cce(y_true, y_pred)
        assert.is_nil(err)
        assert.is_true(near(loss, expected))
    end)

    it("is non-negative", function()
        local y_true = {0.0, 1.0, 0.0}
        local y_pred = {0.1, 0.8, 0.1}
        local loss, _ = lf.cce(y_true, y_pred)
        assert.is_true(loss >= 0.0)
    end)

    it("is near 0 when the predicted probability of the true class is near 1", function()
        local y_true = {0.0, 1.0, 0.0}
        local y_pred = {0.001, 0.998, 0.001}
        local loss, err = lf.cce(y_true, y_pred)
        assert.is_nil(err)
        assert.is_true(loss < 0.01)
    end)

    it("does not return NaN for zero predictions (clamping)", function()
        local y_true = {0.0, 1.0, 0.0}
        local y_pred = {0.0, 0.0, 1.0}   -- prediction completely wrong
        local loss, err = lf.cce(y_true, y_pred)
        assert.is_nil(err)
        assert.is_true(loss == loss)         -- not NaN
        assert.is_true(loss < math.huge)     -- not inf
    end)

    it("returns an error for mismatched lengths", function()
        local _, err = lf.cce({0.0, 1.0}, {0.3, 0.3, 0.4})
        assert.is_not_nil(err)
    end)

    it("returns an error for empty arrays", function()
        local _, err = lf.cce({}, {})
        assert.is_not_nil(err)
    end)
end)

-- ============================================================================
-- MSE Derivative Tests
-- ============================================================================

describe("mse_derivative", function()
    -- ## Formula check
    --
    -- ∂MSE/∂ŷᵢ = (2/n) * (ŷᵢ - yᵢ)

    it("computes the correct gradient for known inputs", function()
        -- n=2, y_true={1,2}, y_pred={3,4}
        -- grad[1] = (2/2)*(3-1) = 2
        -- grad[2] = (2/2)*(4-2) = 2
        local y_true = {1.0, 2.0}
        local y_pred = {3.0, 4.0}
        local grad, err = lf.mse_derivative(y_true, y_pred)
        assert.is_nil(err)
        assert.is_true(near(grad[1], 2.0))
        assert.is_true(near(grad[2], 2.0))
    end)

    it("gradient is zero when predictions equal targets", function()
        local y_true = {1.0, 2.0, 3.0}
        local y_pred = {1.0, 2.0, 3.0}
        local grad, _ = lf.mse_derivative(y_true, y_pred)
        for _, g in ipairs(grad) do
            assert.is_true(near(g, 0.0))
        end
    end)

    it("gradient has the correct sign (positive when over-predicted)", function()
        local y_true = {0.0}
        local y_pred = {1.0}   -- over-predicted → gradient should be positive
        local grad, _ = lf.mse_derivative(y_true, y_pred)
        assert.is_true(grad[1] > 0)
    end)

    it("gradient has the correct sign (negative when under-predicted)", function()
        local y_true = {1.0}
        local y_pred = {0.0}   -- under-predicted → gradient should be negative
        local grad, _ = lf.mse_derivative(y_true, y_pred)
        assert.is_true(grad[1] < 0)
    end)

    it("matches numerical gradient", function()
        local y_true = {1.0, 2.0, 3.0}
        local y_pred = {1.5, 2.5, 2.0}
        local analytic, _ = lf.mse_derivative(y_true, y_pred)
        local numeric     = numerical_gradient(lf.mse, y_true, y_pred)
        assert.is_true(near_table(analytic, numeric, 1e-5))
    end)

    it("returns an error for mismatched lengths", function()
        local _, err = lf.mse_derivative({1.0}, {1.0, 2.0})
        assert.is_not_nil(err)
    end)
end)

-- ============================================================================
-- MAE Derivative Tests
-- ============================================================================

describe("mae_derivative", function()
    it("returns +1/n when over-predicted", function()
        -- y_pred > y_true → gradient = +1/n
        local y_true = {0.0, 0.0}
        local y_pred = {1.0, 2.0}
        local grad, err = lf.mae_derivative(y_true, y_pred)
        assert.is_nil(err)
        assert.is_true(near(grad[1],  0.5))
        assert.is_true(near(grad[2],  0.5))
    end)

    it("returns -1/n when under-predicted", function()
        local y_true = {1.0, 2.0}
        local y_pred = {0.0, 0.0}
        local grad, err = lf.mae_derivative(y_true, y_pred)
        assert.is_nil(err)
        assert.is_true(near(grad[1], -0.5))
        assert.is_true(near(grad[2], -0.5))
    end)

    it("returns 0 at exact match", function()
        local y_true = {1.0, 2.0}
        local y_pred = {1.0, 2.0}
        local grad, _ = lf.mae_derivative(y_true, y_pred)
        assert.is_true(near(grad[1], 0.0))
        assert.is_true(near(grad[2], 0.0))
    end)

    it("handles mixed over- and under-prediction correctly", function()
        -- n=4: grad[1]=+0.25, grad[2]=-0.25, grad[3]=0, grad[4]=+0.25
        local y_true = {0.0, 1.0, 1.0, 0.0}
        local y_pred = {1.0, 0.0, 1.0, 2.0}
        local grad, err = lf.mae_derivative(y_true, y_pred)
        assert.is_nil(err)
        assert.is_true(near(grad[1],  0.25))
        assert.is_true(near(grad[2], -0.25))
        assert.is_true(near(grad[3],  0.0))
        assert.is_true(near(grad[4],  0.25))
    end)

    it("returns an error for mismatched lengths", function()
        local _, err = lf.mae_derivative({1.0, 2.0}, {1.0})
        assert.is_not_nil(err)
    end)
end)

-- ============================================================================
-- BCE Derivative Tests
-- ============================================================================

describe("bce_derivative", function()
    it("has the correct length", function()
        local y_true = {1.0, 0.0, 1.0}
        local y_pred = {0.8, 0.2, 0.7}
        local grad, err = lf.bce_derivative(y_true, y_pred)
        assert.is_nil(err)
        assert.are.equal(3, #grad)
    end)

    it("gradient is negative when y=1 and p > y (over-predicted)", function()
        -- When y=1 and p > 0.5, the gradient should push p toward 1 (upward).
        -- Wait: grad = (1/n)*(p - y) / (p*(1-p)).
        -- If y=1 and p=0.9: (0.9-1)/(0.9*0.1) = -0.1/0.09 = -1.111  → negative. Correct.
        local y_true = {1.0}
        local y_pred = {0.9}
        local grad, err = lf.bce_derivative(y_true, y_pred)
        assert.is_nil(err)
        assert.is_true(grad[1] < 0)
    end)

    it("gradient is positive when y=0 and p > 0 (non-zero prediction for true-0)", function()
        -- y=0, p=0.9: (0.9-0)/(0.9*0.1) = 10  → positive (push p down). Correct.
        local y_true = {0.0}
        local y_pred = {0.9}
        local grad, err = lf.bce_derivative(y_true, y_pred)
        assert.is_nil(err)
        assert.is_true(grad[1] > 0)
    end)

    it("matches numerical gradient", function()
        local y_true = {1.0, 0.0, 1.0}
        local y_pred = {0.7, 0.3, 0.6}
        local analytic, _ = lf.bce_derivative(y_true, y_pred)
        local numeric     = numerical_gradient(lf.bce, y_true, y_pred)
        assert.is_true(near_table(analytic, numeric, 1e-4))
    end)

    it("does not produce NaN or Inf for boundary predictions (clamping)", function()
        local y_true = {1.0, 0.0}
        local y_pred = {0.0, 1.0}
        local grad, err = lf.bce_derivative(y_true, y_pred)
        assert.is_nil(err)
        for _, g in ipairs(grad) do
            assert.is_true(g == g)           -- not NaN
            assert.is_true(g < math.huge)    -- not +inf
            assert.is_true(g > -math.huge)   -- not -inf
        end
    end)

    it("returns an error for mismatched lengths", function()
        local _, err = lf.bce_derivative({1.0}, {0.5, 0.5})
        assert.is_not_nil(err)
    end)
end)

-- ============================================================================
-- CCE Derivative Tests
-- ============================================================================

describe("cce_derivative", function()
    it("has the correct length", function()
        local y_true = {0.0, 1.0, 0.0}
        local y_pred = {0.2, 0.7, 0.1}
        local grad, err = lf.cce_derivative(y_true, y_pred)
        assert.is_nil(err)
        assert.are.equal(3, #grad)
    end)

    it("gradient is zero for non-true classes (y=0)", function()
        -- ∂CCE/∂ŷᵢ = -(1/n) * (yᵢ / p).  When yᵢ = 0, gradient is 0.
        local y_true = {0.0, 1.0, 0.0}
        local y_pred = {0.2, 0.7, 0.1}
        local grad, err = lf.cce_derivative(y_true, y_pred)
        assert.is_nil(err)
        assert.is_true(near(grad[1], 0.0))
        assert.is_true(near(grad[3], 0.0))
    end)

    it("gradient is negative for the true class (decreasing loss = increasing p)", function()
        -- y[2]=1, p=0.7: -(1/3)*(1/0.7) = -0.476  → negative. Correct.
        local y_true = {0.0, 1.0, 0.0}
        local y_pred = {0.2, 0.7, 0.1}
        local grad, _ = lf.cce_derivative(y_true, y_pred)
        assert.is_true(grad[2] < 0)
    end)

    it("matches the analytic formula for the true class", function()
        local n = 3
        local y_true = {0.0, 1.0, 0.0}
        local y_pred = {0.2, 0.7, 0.1}
        local expected = -(1.0 / n) * (1.0 / 0.7)
        local grad, _ = lf.cce_derivative(y_true, y_pred)
        assert.is_true(near(grad[2], expected))
    end)

    it("matches numerical gradient", function()
        local y_true = {0.0, 1.0, 0.0}
        local y_pred = {0.2, 0.7, 0.1}
        local analytic, _ = lf.cce_derivative(y_true, y_pred)
        local numeric     = numerical_gradient(lf.cce, y_true, y_pred)
        assert.is_true(near_table(analytic, numeric, 1e-4))
    end)

    it("does not produce NaN for zero predictions (clamping)", function()
        local y_true = {0.0, 1.0, 0.0}
        local y_pred = {0.0, 0.0, 1.0}
        local grad, err = lf.cce_derivative(y_true, y_pred)
        assert.is_nil(err)
        for _, g in ipairs(grad) do
            assert.is_true(g == g)
            assert.is_true(g < math.huge)
            assert.is_true(g > -math.huge)
        end
    end)

    it("returns an error for mismatched lengths", function()
        local _, err = lf.cce_derivative({0.0, 1.0}, {0.3, 0.3, 0.4})
        assert.is_not_nil(err)
    end)
end)

-- ============================================================================
-- Cross-function Consistency Tests
-- ============================================================================

describe("cross-function consistency", function()
    -- The loss functions and their derivatives should be consistent:
    -- adding epsilon*grad to y_pred should decrease (or at least not
    -- increase) the loss.

    local function check_descent(loss_fn, deriv_fn, y_true, y_pred, step)
        step = step or 0.001
        local loss_before, _ = loss_fn(y_true, y_pred)
        local grad, _         = deriv_fn(y_true, y_pred)

        -- Take one gradient step: ŷ_new = ŷ - step * grad
        local y_new = {}
        for i = 1, #y_pred do
            y_new[i] = y_pred[i] - step * grad[i]
        end

        local loss_after, _ = loss_fn(y_true, y_new)
        return loss_after <= loss_before + 1e-9  -- allow tiny floating-point slack
    end

    it("MSE derivative enables gradient descent", function()
        local y_true = {1.0, 2.0, 3.0}
        local y_pred = {0.5, 2.5, 2.5}
        assert.is_true(check_descent(lf.mse, lf.mse_derivative, y_true, y_pred))
    end)

    it("MAE derivative enables gradient descent", function()
        local y_true = {1.0, 2.0}
        local y_pred = {0.0, 3.0}
        assert.is_true(check_descent(lf.mae, lf.mae_derivative, y_true, y_pred))
    end)

    it("BCE derivative enables gradient descent", function()
        local y_true = {1.0, 0.0}
        local y_pred = {0.6, 0.4}
        assert.is_true(check_descent(lf.bce, lf.bce_derivative, y_true, y_pred))
    end)

    it("CCE derivative enables gradient descent", function()
        local y_true = {0.0, 1.0, 0.0}
        local y_pred = {0.3, 0.5, 0.2}
        assert.is_true(check_descent(lf.cce, lf.cce_derivative, y_true, y_pred))
    end)
end)
