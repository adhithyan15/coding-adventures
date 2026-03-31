-- ============================================================================
-- loss_functions -- Machine learning loss functions and their derivatives
-- ============================================================================
--
-- This module provides the core loss functions used in supervised machine
-- learning: Mean Squared Error (MSE), Mean Absolute Error (MAE), Binary
-- Cross-Entropy (BCE), and Categorical Cross-Entropy (CCE).  It also
-- provides the derivative (gradient) of each loss with respect to the
-- predicted values — the gradient is what gradient-descent optimisers
-- actually use to adjust model weights.
--
-- ## What Is a Loss Function?
--
-- A loss function (also called a cost function or objective function)
-- measures how far a model's predictions are from the true values.  During
-- training we want to minimise the loss.
--
-- For a model that outputs a vector of predictions ŷ = [ŷ₁, ŷ₂, ..., ŷₙ]
-- and a vector of ground-truth targets y = [y₁, y₂, ..., yₙ], the loss
-- function L(y, ŷ) produces a single scalar that summarises the error.
--
-- ## Derivatives: Why They Matter
--
-- Backpropagation, the algorithm that trains neural networks, needs
-- ∂L/∂ŷᵢ — the partial derivative of the loss with respect to each
-- predicted value.  These derivatives tell the optimiser which direction
-- to nudge each prediction (and, transitively, each model weight) to
-- reduce the loss.
--
-- ## Numerical Stability: The Epsilon Clamp
--
-- Both BCE and CCE involve log(ŷ).  If ŷ = 0, log(0) = -∞, which
-- causes NaN in downstream calculations.  We avoid this by clamping ŷ
-- to the range [ε, 1 - ε] where ε = 1e-7.  This is a standard practice
-- in every deep-learning framework (TensorFlow, PyTorch, JAX).
--
-- ## Usage
--
--   local lf = require("coding_adventures.loss_functions")
--
--   local y_true = {0.0, 1.0, 0.0}
--   local y_pred = {0.1, 0.9, 0.2}
--
--   local loss, err = lf.bce(y_true, y_pred)
--   if err then error(err) end
--   print(loss)   -- ~0.0660
--
--   local grad, err = lf.bce_derivative(y_true, y_pred)
--   -- grad[i] = ∂BCE/∂ŷᵢ
--
-- ============================================================================

local M = {}

M.VERSION = "0.1.0"

-- ============================================================================
-- Constants
-- ============================================================================

--- EPSILON is the smallest value we allow a clamped prediction to reach.
--
-- We chose 1e-7 to match common deep-learning frameworks.  It is small
-- enough not to meaningfully distort the loss value, yet large enough to
-- keep log() and division operations well away from infinity or NaN.
local EPSILON = 1e-7

-- ============================================================================
-- Internal helpers
-- ============================================================================

--- clamp returns x restricted to the closed interval [lo, hi].
--
-- This is the fundamental building block for numerical stability in loss
-- functions that involve logarithms (BCE, CCE).  Without clamping, a
-- prediction of exactly 0.0 would cause log(0) = -inf.
--
-- @param x   value to clamp
-- @param lo  lower bound (inclusive)
-- @param hi  upper bound (inclusive)
-- @return    clamped value
local function clamp(x, lo, hi)
    if x < lo then return lo end
    if x > hi then return hi end
    return x
end

--- validate checks that y_true and y_pred are compatible for loss computation.
--
-- Both arrays must:
--   1. Be non-nil tables (we do not support scalar inputs here).
--   2. Have the same length.
--   3. Have at least one element (a loss over an empty set is undefined).
--
-- @param y_true  table of ground-truth values
-- @param y_pred  table of predicted values
-- @return        nil on success, or an error string describing the problem
local function validate(y_true, y_pred)
    if type(y_true) ~= "table" then
        return "y_true must be a table, got " .. type(y_true)
    end
    if type(y_pred) ~= "table" then
        return "y_pred must be a table, got " .. type(y_pred)
    end
    local n = #y_true
    if n == 0 then
        return "y_true and y_pred must not be empty"
    end
    if #y_pred ~= n then
        return string.format(
            "y_true and y_pred must have the same length (got %d and %d)",
            n, #y_pred
        )
    end
    return nil  -- no error
end

-- ============================================================================
-- MSE — Mean Squared Error
-- ============================================================================

--- mse computes the Mean Squared Error between y_true and y_pred.
--
-- ## Definition
--
-- MSE is the average of the squared differences between predicted and true
-- values:
--
--     MSE = (1/n) * Σᵢ (yᵢ - ŷᵢ)²
--
-- ## Properties
--
-- - MSE is always ≥ 0.
-- - MSE = 0 only when the predictions exactly match the targets.
-- - Squaring penalises large errors more heavily than small ones.  A
--   prediction that is 2 units off contributes 4× as much to the loss as
--   a prediction that is 1 unit off.
-- - MSE is differentiable everywhere, which makes it convenient for
--   gradient-based optimisers.
--
-- ## Typical Use Cases
--
-- MSE is the standard loss for regression tasks: predicting house prices,
-- stock returns, or any continuous real-valued target.
--
-- ## Example
--
--   y_true = {1.0, 2.0, 3.0}
--   y_pred = {1.1, 1.9, 3.2}
--   MSE = ((0.1)² + (0.1)² + (0.2)²) / 3
--       = (0.01 + 0.01 + 0.04) / 3
--       = 0.06 / 3 = 0.02
--
-- @param y_true  table of ground-truth values
-- @param y_pred  table of predicted values
-- @return        (scalar loss, nil) on success, or (nil, error_string) on failure
function M.mse(y_true, y_pred)
    local err = validate(y_true, y_pred)
    if err then return nil, err end

    local n   = #y_true
    local sum = 0.0

    for i = 1, n do
        -- Compute the squared residual for this element.
        -- "Residual" means (true - predicted); we square it so sign doesn't matter.
        local diff = y_true[i] - y_pred[i]
        sum = sum + diff * diff
    end

    return sum / n, nil
end

-- ============================================================================
-- MAE — Mean Absolute Error
-- ============================================================================

--- mae computes the Mean Absolute Error between y_true and y_pred.
--
-- ## Definition
--
--     MAE = (1/n) * Σᵢ |yᵢ - ŷᵢ|
--
-- ## Properties
--
-- - MAE is always ≥ 0.
-- - Unlike MSE, MAE penalises all errors equally (linear penalty).
-- - MAE is more robust to outliers: a single very wrong prediction does
--   not dominate the loss the way it does with MSE.
-- - MAE is not differentiable at 0 (the absolute value has a kink there),
--   which is why its derivative returns 0 at exactly that point.
--
-- ## Example
--
--   y_true = {1.0, 2.0, 3.0}
--   y_pred = {1.1, 1.9, 3.2}
--   MAE = (|−0.1| + |0.1| + |−0.2|) / 3
--       = (0.1 + 0.1 + 0.2) / 3
--       = 0.4 / 3 ≈ 0.1333
--
-- @param y_true  table of ground-truth values
-- @param y_pred  table of predicted values
-- @return        (scalar loss, nil) on success, or (nil, error_string) on failure
function M.mae(y_true, y_pred)
    local err = validate(y_true, y_pred)
    if err then return nil, err end

    local n   = #y_true
    local sum = 0.0

    for i = 1, n do
        -- math.abs gives us |yᵢ - ŷᵢ| without needing a conditional.
        sum = sum + math.abs(y_true[i] - y_pred[i])
    end

    return sum / n, nil
end

-- ============================================================================
-- BCE — Binary Cross-Entropy
-- ============================================================================

--- bce computes the Binary Cross-Entropy between y_true and y_pred.
--
-- ## Definition
--
-- Binary Cross-Entropy is used when each label is a single binary value
-- (0 or 1) and each prediction is a probability in [0, 1]:
--
--     BCE = -(1/n) * Σᵢ [ yᵢ · log(p̂ᵢ) + (1 - yᵢ) · log(1 - p̂ᵢ) ]
--
-- where p̂ᵢ = clamp(ŷᵢ, ε, 1 − ε).
--
-- ## Intuition
--
-- Information theory tells us that the "surprise" of an event with
-- probability p is -log(p).  When yᵢ = 1 (the event happened), we use
-- -log(p̂ᵢ): low loss if the model gave high probability, high loss if
-- the model gave low probability.  When yᵢ = 0, we use -log(1 - p̂ᵢ):
-- low loss if the model gave low probability (i.e., correctly predicted
-- "won't happen").
--
-- ## The Epsilon Clamp
--
-- We clamp predictions to [ε, 1 − ε] to avoid log(0) = −∞.
-- With ε = 1e-7, the maximum distortion is tiny.
--
-- ## Example
--
--   y_true = {1, 0, 1}
--   y_pred = {0.9, 0.1, 0.8}
--   BCE ≈ -[ (log 0.9 + log 0.9) + (0 + log 0.9) + (log 0.8 + 0) ] / 3
--
-- @param y_true  table of binary ground-truth labels (0 or 1)
-- @param y_pred  table of predicted probabilities in [0, 1]
-- @return        (scalar loss, nil) on success, or (nil, error_string) on failure
function M.bce(y_true, y_pred)
    local err = validate(y_true, y_pred)
    if err then return nil, err end

    local n   = #y_true
    local sum = 0.0

    for i = 1, n do
        -- Clamp the prediction to avoid log(0) or log(negative number).
        local p = clamp(y_pred[i], EPSILON, 1.0 - EPSILON)

        -- The binary cross-entropy contribution for element i.
        -- When yᵢ = 1: only the first term contributes (-log(p)).
        -- When yᵢ = 0: only the second term contributes (-log(1-p)).
        sum = sum + y_true[i] * math.log(p) + (1.0 - y_true[i]) * math.log(1.0 - p)
    end

    -- The negative sign converts cross-entropy (which grows as loss decreases)
    -- into a positive loss value we want to minimise.
    return -sum / n, nil
end

-- ============================================================================
-- CCE — Categorical Cross-Entropy
-- ============================================================================

--- cce computes the Categorical Cross-Entropy between y_true and y_pred.
--
-- ## Definition
--
-- Categorical Cross-Entropy is used for multi-class classification where
-- y_true is a one-hot vector and y_pred is a probability distribution
-- over classes (e.g. the output of a softmax layer):
--
--     CCE = -(1/n) * Σᵢ [ yᵢ · log(p̂ᵢ) ]
--
-- where p̂ᵢ = clamp(ŷᵢ, ε, 1 − ε).
--
-- ## Relation to BCE
--
-- BCE is a special case of CCE for exactly two classes.  In BCE we
-- explicitly model P(y=0) = 1 - P(y=1); in CCE every class has its own
-- prediction.
--
-- ## One-Hot Encoding
--
-- For a problem with k classes, y_true is a vector of length k where
-- exactly one element is 1 and the rest are 0:
--
--   3-class example: y_true = {0, 1, 0}  (true class is class 2)
--   y_pred           = {0.2, 0.7, 0.1}
--   CCE = -(0·log(0.2) + 1·log(0.7) + 0·log(0.1)) / 3
--       = -log(0.7) / 3  ≈ 0.1189
--
-- @param y_true  table of one-hot ground-truth labels
-- @param y_pred  table of predicted class probabilities
-- @return        (scalar loss, nil) on success, or (nil, error_string) on failure
function M.cce(y_true, y_pred)
    local err = validate(y_true, y_pred)
    if err then return nil, err end

    local n   = #y_true
    local sum = 0.0

    for i = 1, n do
        -- Clamp each prediction to [ε, 1-ε] to avoid log(0).
        local p = clamp(y_pred[i], EPSILON, 1.0 - EPSILON)

        -- Only terms where yᵢ ≠ 0 contribute.  In a perfectly one-hot
        -- vector only a single term is non-zero, but we support soft
        -- labels too (e.g. yᵢ = 0.9 to represent label smoothing).
        sum = sum + y_true[i] * math.log(p)
    end

    return -sum / n, nil
end

-- ============================================================================
-- MSE Derivative
-- ============================================================================

--- mse_derivative returns ∂MSE/∂ŷᵢ for each i.
--
-- ## Formula
--
-- Differentiating MSE = (1/n) Σ (yᵢ - ŷᵢ)² with respect to ŷᵢ:
--
--     ∂MSE/∂ŷᵢ = (2/n) · (ŷᵢ - yᵢ)
--
-- Note the sign: it is (ŷᵢ - yᵢ), not (yᵢ - ŷᵢ).  This is because
-- we differentiate with respect to ŷᵢ, the predicted value.
--
-- ## Interpretation
--
-- - If ŷᵢ > yᵢ (we over-predicted), the gradient is positive, telling
--   the optimiser to decrease ŷᵢ.
-- - If ŷᵢ < yᵢ (we under-predicted), the gradient is negative, telling
--   the optimiser to increase ŷᵢ.
-- - The 2/n scaling factor is a constant; many implementations absorb
--   the factor of 2 into the learning rate.
--
-- @param y_true  table of ground-truth values
-- @param y_pred  table of predicted values
-- @return        (table of gradients, nil) on success, or (nil, error_string)
function M.mse_derivative(y_true, y_pred)
    local err = validate(y_true, y_pred)
    if err then return nil, err end

    local n    = #y_true
    local grad = {}

    for i = 1, n do
        -- (2/n) * (ŷᵢ - yᵢ)
        -- Subtract true from pred (not the other way around) because
        -- we differentiate the squared error w.r.t. ŷᵢ.
        grad[i] = (2.0 / n) * (y_pred[i] - y_true[i])
    end

    return grad, nil
end

-- ============================================================================
-- MAE Derivative
-- ============================================================================

--- mae_derivative returns ∂MAE/∂ŷᵢ for each i.
--
-- ## Formula
--
-- MAE = (1/n) Σ |yᵢ - ŷᵢ|
--
-- The absolute value function is not differentiable at zero.  The
-- subgradient (a generalisation of the derivative for non-smooth functions)
-- is:
--
--     ∂MAE/∂ŷᵢ = +1/n   if ŷᵢ > yᵢ
--               = -1/n   if ŷᵢ < yᵢ
--               =  0     if ŷᵢ = yᵢ   (subgradient choice: 0 at the kink)
--
-- ## Why +1/n When Over-Predicting?
--
-- If ŷᵢ > yᵢ, then yᵢ - ŷᵢ < 0, so |yᵢ - ŷᵢ| = ŷᵢ - yᵢ.
-- Differentiating ŷᵢ - yᵢ with respect to ŷᵢ gives +1.
-- Dividing by n gives +1/n.
--
-- @param y_true  table of ground-truth values
-- @param y_pred  table of predicted values
-- @return        (table of gradients, nil) on success, or (nil, error_string)
function M.mae_derivative(y_true, y_pred)
    local err = validate(y_true, y_pred)
    if err then return nil, err end

    local n    = #y_true
    local grad = {}

    for i = 1, n do
        if y_pred[i] > y_true[i] then
            grad[i] =  1.0 / n   -- over-predicted: push prediction down
        elseif y_pred[i] < y_true[i] then
            grad[i] = -1.0 / n   -- under-predicted: push prediction up
        else
            grad[i] = 0.0        -- exact match: no gradient signal at this point
        end
    end

    return grad, nil
end

-- ============================================================================
-- BCE Derivative
-- ============================================================================

--- bce_derivative returns ∂BCE/∂ŷᵢ for each i.
--
-- ## Formula
--
-- Starting from:
--
--     BCE = -(1/n) Σ [ yᵢ log(p) + (1-yᵢ) log(1-p) ]
--     where p = clamp(ŷᵢ, ε, 1-ε)
--
-- Differentiating with respect to ŷᵢ (assuming ŷᵢ is in the interior
-- of [ε, 1-ε] so the clamp is not active):
--
--     ∂BCE/∂ŷᵢ = (1/n) · [ -yᵢ/p + (1-yᵢ)/(1-p) ]
--              = (1/n) · [ -(yᵢ(1-p) - (1-yᵢ)p) / (p(1-p)) ]
--              = (1/n) · [ (p - yᵢ) / (p(1-p)) ]
--
-- ## Interpretation
--
-- The gradient points away from the target:
--   - When yᵢ = 1 and p ≈ 0 (model confidently wrong), the gradient is
--     large and negative, pushing p upward.
--   - When yᵢ = 0 and p ≈ 1 (model confidently wrong), the gradient is
--     large and positive, pushing p downward.
--
-- @param y_true  table of binary ground-truth labels
-- @param y_pred  table of predicted probabilities
-- @return        (table of gradients, nil) on success, or (nil, error_string)
function M.bce_derivative(y_true, y_pred)
    local err = validate(y_true, y_pred)
    if err then return nil, err end

    local n    = #y_true
    local grad = {}

    for i = 1, n do
        -- Clamp the prediction exactly as we did in the forward pass,
        -- so the gradient is consistent with the loss computation.
        local p = clamp(y_pred[i], EPSILON, 1.0 - EPSILON)

        -- ∂BCE/∂ŷᵢ = (1/n) · (p - yᵢ) / (p · (1-p))
        -- The denominator p*(1-p) is the variance of a Bernoulli(p),
        -- which explains why BCE gradients blow up near 0 or 1.
        grad[i] = (1.0 / n) * (p - y_true[i]) / (p * (1.0 - p))
    end

    return grad, nil
end

-- ============================================================================
-- CCE Derivative
-- ============================================================================

--- cce_derivative returns ∂CCE/∂ŷᵢ for each i.
--
-- ## Formula
--
-- Starting from:
--
--     CCE = -(1/n) Σ [ yᵢ log(p) ]
--     where p = clamp(ŷᵢ, ε, 1-ε)
--
-- Differentiating with respect to ŷᵢ:
--
--     ∂CCE/∂ŷᵢ = -(1/n) · (yᵢ / p)
--
-- ## Note on Softmax + CCE
--
-- In practice, CCE is almost always used in combination with a softmax
-- activation.  When you compose them, the combined gradient simplifies to
-- just (ŷᵢ - yᵢ), which is much cleaner.  The formula here computes the
-- gradient of CCE alone, without the softmax Jacobian.
--
-- @param y_true  table of (one-hot) ground-truth labels
-- @param y_pred  table of predicted class probabilities
-- @return        (table of gradients, nil) on success, or (nil, error_string)
function M.cce_derivative(y_true, y_pred)
    local err = validate(y_true, y_pred)
    if err then return nil, err end

    local n    = #y_true
    local grad = {}

    for i = 1, n do
        -- Clamp the prediction to avoid division by zero.
        local p = clamp(y_pred[i], EPSILON, 1.0 - EPSILON)

        -- ∂CCE/∂ŷᵢ = -(1/n) · (yᵢ / p)
        -- When yᵢ = 0 (not the true class), the gradient is 0.
        -- When yᵢ = 1 (the true class), the gradient is -1/(n·p), which is
        -- negative, meaning increasing ŷᵢ decreases the loss (correct!).
        grad[i] = (-1.0 / n) * (y_true[i] / p)
    end

    return grad, nil
end

return M
