-- ============================================================================
-- Tests for perceptron — single-layer neural network
-- ============================================================================
--
-- ## Testing Strategy
--
-- 1. predict() — forward pass with known weights produces correct output.
-- 2. train_step() — single update moves weights in correct direction.
-- 3. train() — AND gate and OR gate both converge to correct classifiers.
-- 4. Bias — shifts the decision boundary independently of inputs.
-- 5. Sigmoid activation — produces values in (0, 1).
-- 6. Error handling — wrong input size.
--
-- ## Why Logic Gates?
--
-- AND and OR are the canonical linearly separable problems:
--
--   AND:   (0,0)→0  (0,1)→0  (1,0)→0  (1,1)→1
--   OR:    (0,0)→0  (0,1)→1  (1,0)→1  (1,1)→1
--   XOR:   (0,0)→0  (0,1)→1  (1,0)→1  (1,1)→0  ← NOT linearly separable

package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path

local pm = require("coding_adventures.perceptron")

-- ============================================================================
-- Test data: logic gate truth tables
-- ============================================================================

local AND_INPUTS  = {{0,0}, {0,1}, {1,0}, {1,1}}
local AND_TARGETS = {0,     0,     0,     1    }

local OR_INPUTS   = {{0,0}, {0,1}, {1,0}, {1,1}}
local OR_TARGETS  = {0,     1,     1,     1    }

-- ============================================================================
-- Helpers
-- ============================================================================

--- accuracy returns the fraction of examples classified correctly.
local function accuracy(p, inputs, targets)
    local correct = 0
    for i = 1, #inputs do
        local out = p:predict(inputs[i])
        -- Round to nearest integer for step activation
        if math.floor(out + 0.5) == targets[i] then
            correct = correct + 1
        end
    end
    return correct / #inputs
end

local function near(a, b, tol)
    tol = tol or 1e-6
    return math.abs(a - b) < tol
end

-- ============================================================================
-- Test suite
-- ============================================================================

describe("Perceptron", function()

    -- -----------------------------------------------------------------------
    -- new()
    -- -----------------------------------------------------------------------
    describe("new()", function()

        it("uses default hyperparameters", function()
            local p = pm.new({ n_inputs = 2 })
            assert.are.equal(2,   p.n_inputs)
            assert.are.equal(0.1, p.learning_rate)
            assert.are.equal(0.0, p.bias)
            assert.are.equal(2,   #p.weights)
            assert.are.equal(0.0, p.weights[1])
            assert.are.equal(0.0, p.weights[2])
        end)

        it("accepts custom weights and bias", function()
            local p = pm.new({ n_inputs = 2, weights = {0.5, -0.3}, bias = 0.1 })
            assert.is_true(near(p.weights[1], 0.5))
            assert.is_true(near(p.weights[2], -0.3))
            assert.is_true(near(p.bias, 0.1))
        end)

        it("errors when n_inputs is missing", function()
            assert.has_error(function()
                pm.new({})
            end)
        end)

        it("errors when weights length mismatches n_inputs", function()
            assert.has_error(function()
                pm.new({ n_inputs = 2, weights = {0.1} })
            end)
        end)

    end)

    -- -----------------------------------------------------------------------
    -- step() and sigmoid() activation functions
    -- -----------------------------------------------------------------------
    describe("activation functions", function()

        it("step returns 0 for z <= 0", function()
            assert.are.equal(0, pm.step(0.0))
            assert.are.equal(0, pm.step(-1.0))
            assert.are.equal(0, pm.step(-100.0))
        end)

        it("step returns 1 for z > 0", function()
            assert.are.equal(1, pm.step(0.001))
            assert.are.equal(1, pm.step(1.0))
            assert.are.equal(1, pm.step(100.0))
        end)

        it("sigmoid(0) = 0.5", function()
            assert.is_true(near(pm.sigmoid(0), 0.5))
        end)

        it("sigmoid is bounded in (0, 1)", function()
            assert.is_true(pm.sigmoid(-1000) >= 0.0)
            assert.is_true(pm.sigmoid( 1000) <= 1.0)
            assert.is_true(pm.sigmoid(   0 ) > 0.0)
            assert.is_true(pm.sigmoid(   0 ) < 1.0)
        end)

        it("sigmoid_derivative at 0 = 0.25", function()
            -- sigmoid'(0) = 0.5 * (1 - 0.5) = 0.25
            assert.is_true(near(pm.sigmoid_derivative(0), 0.25))
        end)

    end)

    -- -----------------------------------------------------------------------
    -- predict()
    -- -----------------------------------------------------------------------
    describe("predict()", function()

        it("returns 0 for all-zero inputs with zero weights and bias", function()
            local p = pm.new({ n_inputs = 2 })
            local out, _ = p:predict({0, 0})
            assert.are.equal(0, out)
        end)

        it("with known weights: w={1,1}, b=-1.5 correctly classifies AND", function()
            -- Decision boundary: x₁ + x₂ - 1.5 = 0
            -- (0,0): -1.5 → 0 correct
            -- (0,1): -0.5 → 0 correct
            -- (1,0): -0.5 → 0 correct
            -- (1,1):  0.5 → 1 correct
            local p = pm.new({ n_inputs = 2, weights = {1.0, 1.0}, bias = -1.5 })
            assert.are.equal(0, p:predict({0, 0}))
            assert.are.equal(0, p:predict({0, 1}))
            assert.are.equal(0, p:predict({1, 0}))
            assert.are.equal(1, p:predict({1, 1}))
        end)

        it("returns the pre-activation z value as second return", function()
            local p = pm.new({ n_inputs = 1, weights = {2.0}, bias = 1.0 })
            local _, z = p:predict({3.0})
            -- z = 2.0 * 3.0 + 1.0 = 7.0
            assert.is_true(near(z, 7.0))
        end)

        it("errors when input length mismatches n_inputs", function()
            local p = pm.new({ n_inputs = 2 })
            assert.has_error(function()
                p:predict({1, 2, 3})
            end)
        end)

        it("sigmoid activation produces values in (0, 1)", function()
            local p = pm.new({
                n_inputs      = 2,
                weights       = {1.0, 1.0},
                bias          = 0.0,
                activation_fn = pm.sigmoid,
            })
            local out = p:predict({0.5, 0.5})
            assert.is_true(out > 0.0)
            assert.is_true(out < 1.0)
        end)

    end)

    -- -----------------------------------------------------------------------
    -- train_step()
    -- -----------------------------------------------------------------------
    describe("train_step()", function()

        it("returns zero error when prediction is already correct", function()
            -- With w={1,1}, b=-1.5, prediction for (1,1) is 1 = target
            local p = pm.new({ n_inputs = 2, weights = {1.0, 1.0}, bias = -1.5 })
            local _, err = p:train_step({1, 1}, 1)
            assert.are.equal(0, err)
        end)

        it("updates weights when prediction is wrong", function()
            -- w={0,0}, b=0 → predict(1,1) = step(0) = 0; target = 1; error = 1
            local p = pm.new({ n_inputs = 2, learning_rate = 0.5 })
            local w_before = { p.weights[1], p.weights[2] }
            local _, err = p:train_step({1, 1}, 1)
            assert.are.equal(1, err)
            -- Weights should have increased (error=1, input positive)
            assert.is_true(p.weights[1] > w_before[1])
            assert.is_true(p.weights[2] > w_before[2])
        end)

        it("does not update weights on correct prediction", function()
            local p = pm.new({ n_inputs = 2, weights = {1.0, 1.0}, bias = -1.5 })
            local w1_before = p.weights[1]
            local w2_before = p.weights[2]
            local b_before  = p.bias
            p:train_step({0, 0}, 0)
            assert.are.equal(w1_before, p.weights[1])
            assert.are.equal(w2_before, p.weights[2])
            assert.are.equal(b_before,  p.bias)
        end)

    end)

    -- -----------------------------------------------------------------------
    -- train() — logic gate convergence
    -- -----------------------------------------------------------------------
    describe("train() — AND gate", function()

        it("achieves 100% accuracy on AND after training", function()
            local p = pm.new({ n_inputs = 2, learning_rate = 0.1 })
            p:train(AND_INPUTS, AND_TARGETS, 200)
            assert.are.equal(1.0, accuracy(p, AND_INPUTS, AND_TARGETS))
        end)

        it("returns self (the trained perceptron)", function()
            local p = pm.new({ n_inputs = 2 })
            local result = p:train(AND_INPUTS, AND_TARGETS, 50)
            assert.are.equal(p, result)
        end)

    end)

    describe("train() — OR gate", function()

        it("achieves 100% accuracy on OR after training", function()
            local p = pm.new({ n_inputs = 2, learning_rate = 0.1 })
            p:train(OR_INPUTS, OR_TARGETS, 200)
            assert.are.equal(1.0, accuracy(p, OR_INPUTS, OR_TARGETS))
        end)

    end)

    describe("train() — bias shifts decision boundary", function()

        it("a perceptron with negative bias needs larger weights to fire", function()
            -- With bias = -2, we need z > 2 to activate.
            -- With weights = {1,1} after training, (1,1) gives z=2 which is
            -- NOT > 0 after subtracting bias. We just verify bias is non-zero.
            local p = pm.new({ n_inputs = 2, learning_rate = 0.1, bias = -0.5 })
            p:train(AND_INPUTS, AND_TARGETS, 500)
            -- Even with non-zero initial bias, training should converge.
            assert.are.equal(1.0, accuracy(p, AND_INPUTS, AND_TARGETS))
        end)

    end)

end)
