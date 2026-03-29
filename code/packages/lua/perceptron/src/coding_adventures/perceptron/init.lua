-- ============================================================================
-- perceptron — Single-layer perceptron neural network
-- ============================================================================
--
-- The perceptron is the atomic building block of neural networks.  Invented
-- by Frank Rosenblatt in 1957, it is the simplest model that can learn to
-- classify linearly separable data.
--
-- ## Biological Inspiration
--
-- A biological neuron:
--   1. Receives weighted input signals from its dendrites.
--   2. Sums them up.
--   3. If the sum exceeds a threshold, fires an action potential.
--
-- The perceptron mirrors this:
--   1. Compute the weighted sum: z = w₁x₁ + w₂x₂ + … + wₙxₙ + b
--   2. Apply activation function: output = f(z)
--   3. If output ≥ threshold, predict class 1; else class 0.
--
-- ## Architecture
--
--   Input layer: x₁, x₂, …, xₙ  (n features)
--      ↓ (weighted connections w₁…wₙ, plus bias b)
--   Single neuron: z = w·x + b → f(z) = output
--
-- ## Activation Functions
--
-- For binary classification, the step function is the classic choice:
--
--   step(z) = 1 if z > 0, else 0
--
-- For differentiable training (gradient-based), sigmoid is preferred:
--
--   sigmoid(z) = 1 / (1 + e^(-z))   range: (0, 1)
--
-- ## The Perceptron Learning Rule
--
-- For the classic Rosenblatt perceptron with step activation:
--
--   error   = target - prediction
--   w_new   = w + lr * error * x
--   b_new   = b + lr * error
--
-- This rule guarantees convergence if the data is linearly separable.
-- It is a special case of gradient descent applied to a 0/1 loss.
--
-- ## What This Module Provides
--
-- | Function             | Purpose                                          |
-- |----------------------|--------------------------------------------------|
-- | Perceptron.new()     | Create a perceptron with given hyperparameters   |
-- | p:predict(input)     | Forward pass: compute output for one example    |
-- | p:train_step()       | One learning step on a single example           |
-- | p:train()            | Full training loop over multiple epochs          |
--
-- ## Usage
--
--   local pm = require("coding_adventures.perceptron")
--   local af = require("coding_adventures.activation_functions")
--
--   local p = pm.new({
--       n_inputs      = 2,
--       learning_rate = 0.1,
--       activation_fn = af.step,
--   })
--
--   -- Train AND gate
--   local inputs  = {{0,0}, {0,1}, {1,0}, {1,1}}
--   local targets = {0,     0,     0,     1    }
--   local trained = p:train(inputs, targets, 100)
--
--   print(trained:predict({1, 1}))   -- 1
--   print(trained:predict({0, 1}))   -- 0
--
-- ============================================================================

local M = {}

M.VERSION = "0.1.0"

-- ============================================================================
-- Activation functions bundled with the module
-- ============================================================================
--
-- These are provided so the perceptron can be used without requiring
-- activation_functions separately, while still being composable with it.

--- step is the classic Rosenblatt step function.
--
-- Returns 1 if z > 0, else 0.
--
-- Truth table:
--   z < 0  → 0   (neuron silent)
--   z = 0  → 0   (ambiguous; we choose 0 to match the "not activated" state)
--   z > 0  → 1   (neuron fires)
--
-- @param z  number  The pre-activation value (weighted sum + bias).
-- @return   number  0 or 1.
function M.step(z)
    return z > 0 and 1 or 0
end

--- sigmoid maps any real number to (0, 1).
--
-- sigmoid(z) = 1 / (1 + e^(-z))
--
-- This is differentiable everywhere (unlike step), enabling gradient-based
-- training.  Its derivative is sigmoid(z) * (1 - sigmoid(z)), which is
-- elegantly expressed in terms of its own output.
--
-- @param z  number  The pre-activation value.
-- @return   number  A probability in (0, 1).
function M.sigmoid(z)
    if z < -709 then return 0.0 end
    if z >  709 then return 1.0 end
    return 1.0 / (1.0 + math.exp(-z))
end

--- sigmoid_derivative returns d(sigmoid)/dz = sigmoid(z) * (1 - sigmoid(z)).
--
-- @param z  number  The pre-activation value.
-- @return   number  The derivative at z.
function M.sigmoid_derivative(z)
    local s = M.sigmoid(z)
    return s * (1.0 - s)
end

-- ============================================================================
-- Constructor
-- ============================================================================

--- Perceptron.new creates a new perceptron.
--
-- ## Options
--
-- | Key           | Type     | Default        | Description                  |
-- |---------------|----------|----------------|------------------------------|
-- | n_inputs      | int      | required       | Number of input features     |
-- | learning_rate | number   | 0.1            | Perceptron learning rate     |
-- | activation_fn | function | M.step         | Activation function          |
-- | weights       | table    | zeros          | Initial weight vector        |
-- | bias          | number   | 0.0            | Initial bias                 |
--
-- If weights are not provided, they are initialised to 0.  Zero initialisation
-- is fine for the perceptron learning rule (it eventually finds the decision
-- boundary regardless of starting point, for linearly separable data).
--
-- @param opts  table  Configuration table.  n_inputs is required.
-- @return      table  A Perceptron instance.
function M.new(opts)
    opts = opts or {}
    assert(opts.n_inputs and opts.n_inputs > 0,
        "Perceptron.new: n_inputs must be a positive integer")

    local n = opts.n_inputs
    local weights = opts.weights or {}
    if #weights == 0 then
        for _ = 1, n do weights[#weights + 1] = 0.0 end
    end
    assert(#weights == n,
        string.format("Perceptron.new: weights length (%d) must equal n_inputs (%d)", #weights, n))

    local self = {
        n_inputs      = n,
        learning_rate = opts.learning_rate or 0.1,
        activation_fn = opts.activation_fn or M.step,
        weights       = weights,
        bias          = opts.bias or 0.0,
    }
    setmetatable(self, { __index = M })
    return self
end

-- ============================================================================
-- predict — Forward pass
-- ============================================================================

--- predict computes the perceptron's output for a single input vector.
--
-- ## Algorithm
--
--   1. Compute weighted sum:  z = w₁x₁ + w₂x₂ + … + wₙxₙ + b
--   2. Apply activation:      output = f(z)
--
-- The bias b is a learnable offset that shifts the decision boundary
-- independently of the input.  Without it, the boundary must pass
-- through the origin — the bias lets it sit anywhere.
--
-- ## Diagram
--
--   x₁ ──[w₁]──┐
--   x₂ ──[w₂]──┤── Σ ──[+b]──[f]── output
--   xₙ ──[wₙ]──┘
--
-- @param input  table  Input feature vector, length = n_inputs.
-- @return       number Output value (after activation).
-- @return       number Pre-activation value z (useful for training).
function M:predict(input)
    assert(#input == self.n_inputs,
        string.format("predict: input length (%d) must equal n_inputs (%d)", #input, self.n_inputs))

    -- Compute weighted sum z = Σ wᵢ·xᵢ + b
    local z = self.bias
    for i = 1, self.n_inputs do
        z = z + self.weights[i] * input[i]
    end

    -- Apply activation function
    local output = self.activation_fn(z)
    return output, z
end

-- ============================================================================
-- train_step — Single perceptron learning update
-- ============================================================================

--- train_step performs one Rosenblatt perceptron learning step.
--
-- ## The Perceptron Learning Rule
--
-- Given:
--   - input   x = [x₁, …, xₙ]
--   - target  t ∈ {0, 1}
--   - current prediction ŷ = predict(x)
--   - error   e = t - ŷ
--
-- Update:
--   w_new[i] = w[i] + lr * e * x[i]   for each i
--   b_new    = b    + lr * e
--
-- When the prediction is correct (e = 0), no update is made.
-- When wrong (e = ±1), weights are nudged toward the correct answer.
--
-- ## Convergence Guarantee
--
-- For linearly separable data, this rule is guaranteed to converge to a
-- correct classifier in a finite number of steps (Novikoff's theorem, 1962).
-- For non-separable data, it oscillates forever — use logistic regression or
-- a multi-layer network instead.
--
-- @param input   table   Input feature vector.
-- @param target  number  Ground-truth label (0 or 1 for classic perceptron).
-- @return        number  Predicted output.
-- @return        number  Error (target - prediction).
function M:train_step(input, target)
    local output, _ = self:predict(input)
    local err = target - output

    -- Update weights and bias only when there is an error.
    if err ~= 0 then
        for i = 1, self.n_inputs do
            self.weights[i] = self.weights[i] + self.learning_rate * err * input[i]
        end
        self.bias = self.bias + self.learning_rate * err
    end

    return output, err
end

-- ============================================================================
-- train — Full training loop
-- ============================================================================

--- train runs the perceptron learning rule for multiple epochs.
--
-- An epoch is one complete pass through all training examples.
-- Multiple epochs are needed because a single pass may not converge.
--
-- ## What Is an Epoch?
--
-- If we have 4 training examples and run for 100 epochs, we perform
-- 100 × 4 = 400 individual train_step() calls.  Each call potentially
-- updates weights; over many epochs, the weights converge to a solution.
--
-- ## Return Value
--
-- Returns a new Perceptron table with the trained weights and bias.
-- The original perceptron is mutated in place and also returned (same
-- table reference), matching the convention of other training functions
-- in this codebase.
--
-- @param inputs   table  Array of input vectors.
-- @param targets  table  Array of target values, same length as inputs.
-- @param epochs   int    Number of full passes through the training set.
-- @return         table  The trained perceptron (self, mutated in place).
function M:train(inputs, targets, epochs)
    assert(#inputs == #targets,
        string.format("train: inputs length (%d) must equal targets length (%d)", #inputs, #targets))
    epochs = epochs or 100

    for _epoch = 1, epochs do
        for i = 1, #inputs do
            self:train_step(inputs[i], targets[i])
        end
    end

    return self
end

return M
