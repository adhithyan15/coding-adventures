-- ============================================================================
-- Tests for gradient_descent — weight optimisation via gradient descent
-- ============================================================================
--
-- ## Testing Strategy
--
-- 1. step() — basic weight update in the right direction.
-- 2. compute_loss() — wrapper delegates correctly.
-- 3. numerical_gradient() — approximates analytical gradient closely.
-- 4. train() — converges on y = 2x (linear regression).
-- 5. Learning rate affects convergence speed.
-- 6. Error handling — mismatched dimensions.
--
-- ## Floating-Point Tolerance
--
-- Comparisons use 1e-4 for gradient checks (finite-difference error) and
-- 1e-2 for trained weights (convergence tolerance).

package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path

local gd_mod = require("coding_adventures.gradient_descent")

-- ============================================================================
-- Helpers
-- ============================================================================

--- near returns true if |a - b| < tol.
local function near(a, b, tol)
    tol = tol or 1e-6
    return math.abs(a - b) < tol
end

--- mse computes mean squared error: sum((w·x - y)^2) / n
-- For a single-weight linear model, the prediction for input x is w[1]*x[1].
local function mse(weights, inputs, targets)
    local sum = 0.0
    for i = 1, #inputs do
        local pred = 0.0
        for j = 1, #weights do
            pred = pred + weights[j] * inputs[i][j]
        end
        local diff = pred - targets[i]
        sum = sum + diff * diff
    end
    return sum / #inputs
end

--- mse_gradient computes ∂MSE/∂w analytically.
-- ∂MSE/∂w_j = (2/n) * sum_i((w·x_i - y_i) * x_i[j])
local function mse_gradient(weights, inputs, targets)
    local n = #inputs
    local grad = {}
    for j = 1, #weights do grad[j] = 0.0 end

    for i = 1, n do
        local pred = 0.0
        for j = 1, #weights do
            pred = pred + weights[j] * inputs[i][j]
        end
        local residual = pred - targets[i]
        for j = 1, #weights do
            grad[j] = grad[j] + (2.0 / n) * residual * inputs[i][j]
        end
    end
    return grad
end

-- ============================================================================
-- Test data: y = 2x (single feature, known solution w = {2.0})
-- ============================================================================

local INPUTS  = {{1.0}, {2.0}, {3.0}, {4.0}, {5.0}}
local TARGETS = {2.0,   4.0,   6.0,   8.0,  10.0}

-- ============================================================================
-- Test suite
-- ============================================================================

describe("GradientDescent", function()

    -- -----------------------------------------------------------------------
    -- new()
    -- -----------------------------------------------------------------------
    describe("new()", function()

        it("uses default hyperparameters when none given", function()
            local gd = gd_mod.new()
            assert.are.equal(0.01,  gd.learning_rate)
            assert.are.equal(1000,  gd.max_iterations)
            assert.are.equal(1e-6,  gd.tolerance)
        end)

        it("accepts custom hyperparameters", function()
            local gd = gd_mod.new({ learning_rate = 0.1, max_iterations = 500, tolerance = 1e-8 })
            assert.are.equal(0.1,   gd.learning_rate)
            assert.are.equal(500,   gd.max_iterations)
            assert.are.equal(1e-8,  gd.tolerance)
        end)

    end)

    -- -----------------------------------------------------------------------
    -- step()
    -- -----------------------------------------------------------------------
    describe("step()", function()

        it("moves weights in the opposite direction of the gradient", function()
            -- A positive gradient means we're climbing; step should decrease w.
            local gd = gd_mod.new({ learning_rate = 0.1 })
            local weights  = {1.0, 2.0}
            local gradient = {0.5, -0.5}   -- mixed directions
            local new_w, err = gd:step(weights, gradient)
            assert.is_nil(err)
            -- w[1] = 1.0 - 0.1 * 0.5  = 0.95
            -- w[2] = 2.0 - 0.1 * (-0.5) = 2.05
            assert.is_true(near(new_w[1], 0.95))
            assert.is_true(near(new_w[2], 2.05))
        end)

        it("does not mutate the input weights table", function()
            local gd = gd_mod.new({ learning_rate = 0.1 })
            local weights  = {1.0}
            local gradient = {0.3}
            local new_w, _ = gd:step(weights, gradient)
            -- Original unchanged
            assert.are.equal(1.0, weights[1])
            -- New weights differ
            assert.is_false(near(new_w[1], weights[1]))
        end)

        it("returns error when lengths differ", function()
            local gd = gd_mod.new()
            local _, err = gd:step({1.0, 2.0}, {0.1})
            assert.is_not_nil(err)
            assert.is_truthy(err:find("length"))
        end)

        it("returns error for empty weights", function()
            local gd = gd_mod.new()
            local _, err = gd:step({}, {})
            assert.is_not_nil(err)
        end)

        it("zero gradient leaves weights unchanged", function()
            local gd = gd_mod.new({ learning_rate = 0.5 })
            local weights  = {3.0, -1.0}
            local gradient = {0.0,  0.0}
            local new_w, err = gd:step(weights, gradient)
            assert.is_nil(err)
            assert.is_true(near(new_w[1], 3.0))
            assert.is_true(near(new_w[2], -1.0))
        end)

    end)

    -- -----------------------------------------------------------------------
    -- compute_loss()
    -- -----------------------------------------------------------------------
    describe("compute_loss()", function()

        it("delegates to the loss function correctly", function()
            local gd = gd_mod.new()
            -- With w = {2.0} and targets = 2*x, MSE should be 0
            local loss = gd:compute_loss({2.0}, INPUTS, TARGETS, mse)
            assert.is_true(near(loss, 0.0, 1e-10))
        end)

        it("returns non-zero loss for wrong weights", function()
            local gd = gd_mod.new()
            local loss = gd:compute_loss({0.0}, INPUTS, TARGETS, mse)
            assert.is_true(loss > 0)
        end)

    end)

    -- -----------------------------------------------------------------------
    -- numerical_gradient()
    -- -----------------------------------------------------------------------
    describe("numerical_gradient()", function()

        it("approximates the analytical MSE gradient closely", function()
            -- The analytical and numerical gradients should agree to ~1e-4.
            local gd = gd_mod.new()
            local weights = {1.5}   -- not the optimum
            local num_grad = gd:numerical_gradient(weights, INPUTS, TARGETS, mse)
            local ana_grad = mse_gradient(weights, INPUTS, TARGETS)
            assert.is_true(near(num_grad[1], ana_grad[1], 1e-4))
        end)

        it("returns gradient of correct length", function()
            local gd = gd_mod.new()
            local weights = {1.0, 0.5}
            local inputs  = {{1.0, 0.0}, {0.0, 1.0}}
            local targets = {1.0, 0.5}
            local grad = gd:numerical_gradient(weights, inputs, targets, mse)
            assert.are.equal(2, #grad)
        end)

        it("uses custom epsilon when provided", function()
            -- Both epsilons should give similar results for smooth functions.
            local gd = gd_mod.new()
            local weights = {1.0}
            local g1 = gd:numerical_gradient(weights, INPUTS, TARGETS, mse, 1e-5)
            local g2 = gd:numerical_gradient(weights, INPUTS, TARGETS, mse, 1e-4)
            assert.is_true(near(g1[1], g2[1], 1e-3))
        end)

    end)

    -- -----------------------------------------------------------------------
    -- train()
    -- -----------------------------------------------------------------------
    describe("train()", function()

        it("converges to w ≈ 2.0 for y = 2x with analytical gradient", function()
            local gd = gd_mod.new({
                learning_rate  = 0.1,
                max_iterations = 2000,
                tolerance      = 1e-8,
            })
            local trained_w, err = gd:train({0.0}, INPUTS, TARGETS, mse, mse_gradient)
            assert.is_nil(err)
            assert.is_true(near(trained_w[1], 2.0, 0.01))
        end)

        it("converges to w ≈ 2.0 for y = 2x with numerical gradient", function()
            local gd = gd_mod.new({
                learning_rate  = 0.05,
                max_iterations = 3000,
                tolerance      = 1e-8,
            })
            -- No derivative function — falls back to numerical gradient.
            local trained_w, err = gd:train({0.0}, INPUTS, TARGETS, mse)
            assert.is_nil(err)
            assert.is_true(near(trained_w[1], 2.0, 0.05))
        end)

        it("reduces loss compared to initial weights", function()
            local gd = gd_mod.new({ learning_rate = 0.1, max_iterations = 100 })
            local initial_loss = gd:compute_loss({0.0}, INPUTS, TARGETS, mse)
            local trained_w, _  = gd:train({0.0}, INPUTS, TARGETS, mse, mse_gradient)
            local final_loss   = gd:compute_loss(trained_w, INPUTS, TARGETS, mse)
            assert.is_true(final_loss < initial_loss)
        end)

        it("higher learning rate converges in fewer iterations for simple problems", function()
            -- Fast learner: large lr
            local gd_fast = gd_mod.new({ learning_rate = 0.2, max_iterations = 500 })
            local w_fast, _ = gd_fast:train({0.0}, INPUTS, TARGETS, mse, mse_gradient)
            local loss_fast  = gd_fast:compute_loss(w_fast, INPUTS, TARGETS, mse)

            -- Slow learner: tiny lr, same max iterations
            local gd_slow = gd_mod.new({ learning_rate = 0.001, max_iterations = 500 })
            local w_slow, _ = gd_slow:train({0.0}, INPUTS, TARGETS, mse, mse_gradient)
            local loss_slow  = gd_slow:compute_loss(w_slow, INPUTS, TARGETS, mse)

            -- Fast learner should reach lower loss given same budget of iterations.
            assert.is_true(loss_fast < loss_slow)
        end)

        it("does not mutate the original weight vector", function()
            local gd = gd_mod.new({ learning_rate = 0.1, max_iterations = 10 })
            local initial = {0.0}
            local _, _ = gd:train(initial, INPUTS, TARGETS, mse, mse_gradient)
            assert.are.equal(0.0, initial[1])
        end)

    end)

end)
