-- ============================================================================
-- activation_functions — Neural network activation functions in pure Lua
-- ============================================================================
--
-- Activation functions are the non-linear "gates" that decide how much a
-- neuron fires. Without them, a neural network — no matter how many layers —
-- collapses to a single linear transformation and cannot learn anything more
-- complex than linear regression.
--
-- ## The Biological Metaphor
--
-- A biological neuron collects weighted electrical signals from its dendrites.
-- When the total input exceeds a threshold, it "fires" an action potential
-- down its axon to downstream neurons. Activation functions mimic this:
-- they take a weighted sum z = w₁x₁ + w₂x₂ + … + b and decide how strongly
-- the neuron should "activate" in response.
--
-- ## What This Module Provides
--
-- | Function         | Range        | Typical Use                           |
-- |------------------|--------------|---------------------------------------|
-- | sigmoid          | (0, 1)       | Binary classification output layer    |
-- | relu             | [0, ∞)       | Hidden layers in deep networks        |
-- | tanh             | (-1, 1)      | Hidden layers, zero-centred output    |
-- | leaky_relu       | (-∞, ∞)      | Addresses "dying ReLU" problem        |
-- | elu              | (-α, ∞)      | Smoother alternative to ReLU          |
-- | softmax          | (0,1) sums=1 | Multi-class classification output     |
--
-- Each activation function also has a _derivative function. These are used
-- during backpropagation to compute ∂Loss/∂z = ∂Loss/∂a · da/dz.
--
-- ## Numerical Stability
--
-- Floating-point arithmetic can overflow (return infinity) or underflow
-- (return zero) for extreme inputs. We guard against this explicitly:
--
--   sigmoid:  clamped at x < -709 → 0.0 and x > 709 → 1.0
--             (math.exp(709) ≈ 8.2e307, math.exp(710) overflows to inf)
--
--   softmax:  we subtract max(x) from every element before exp().
--             This keeps all exponents ≤ 0, so exp() ∈ (0, 1].
--             Result is mathematically identical (the max cancels in
--             numerator and denominator) but avoids any overflow.
--
-- ## Usage
--
--   local af = require("coding_adventures.activation_functions")
--
--   print(af.sigmoid(0))           -- 0.5
--   print(af.relu(-3))             -- 0.0
--   local probs = af.softmax({1, 2, 3})   -- {0.09, 0.245, 0.665}
--
-- ============================================================================

local M = {}

M.VERSION = "0.1.0"

-- ============================================================================
-- LINEAR and its derivative
-- ============================================================================

--- linear(x) -> x
--
-- Linear activation leaves the weighted sum unchanged. It is useful for
-- regression output layers and for baseline visualizations.
function M.linear(x)
    return x
end

--- linear_derivative(x) -> 1
function M.linear_derivative(_x)
    return 1.0
end

-- ============================================================================
-- SIGMOID and its derivative
-- ============================================================================

--- sigmoid(x) → value in (0, 1)
--
-- ## Definition
--
--     σ(x) = 1 / (1 + e^(−x))
--
-- ## Shape
--
-- The sigmoid is an "S"-shaped curve (sigmoidal curve):
--   - As x → −∞, σ(x) → 0   (neuron fully off)
--   - At x = 0,   σ(x) = 0.5 (neuron half active)
--   - As x → +∞, σ(x) → 1   (neuron fully on)
--
-- ## Interpretation as Probability
--
-- σ(x) can be interpreted as a probability because its output is always
-- in (0, 1). This makes sigmoid the natural choice for binary classification
-- output layers, where we want P(class = 1 | input).
--
-- ## Numerical Stability
--
-- For x < −709, e^(−x) = e^709 would overflow to infinity in IEEE 754
-- double precision, making 1/(1+inf) = 0.0. We short-circuit directly.
-- For x > 709, e^(−x) ≈ 0, making σ(x) ≈ 1. We return exactly 1.0.
-- The cutoff 709 comes from the largest finite value of math.exp (≈ e^709).
--
-- @param x  any real number
-- @return   sigmoid value in (0, 1)
function M.sigmoid(x)
    -- Clamp extreme values to avoid floating-point overflow in math.exp
    if x < -709 then return 0.0 end
    if x >  709 then return 1.0 end
    return 1.0 / (1.0 + math.exp(-x))
end

--- sigmoid_derivative(x) → derivative at x
--
-- ## Formula (using the chain rule)
--
-- Let σ = sigmoid(x). Then:
--
--     dσ/dx = σ · (1 − σ)
--
-- ## Derivation
--
--     σ(x)  = (1 + e^(−x))^(−1)
--     σ'(x) = (1 + e^(−x))^(−2) · e^(−x)
--           = σ(x)² · e^(−x)
--           = σ(x) · [1 − σ(x)]
--
-- The last step uses the identity e^(−x) = (1 − σ(x)) / σ(x).
--
-- ## Properties
--
-- - Maximum derivative is 0.25, at x = 0.
-- - As |x| grows the derivative shrinks toward 0 — this is the "vanishing
--   gradient" problem that makes sigmoids hard to use in deep networks.
--
-- @param x  any real number
-- @return   derivative of sigmoid at x
function M.sigmoid_derivative(x)
    local s = M.sigmoid(x)
    return s * (1.0 - s)
end

-- ============================================================================
-- RELU (Rectified Linear Unit) and its derivative
-- ============================================================================

--- relu(x) → max(0, x)
--
-- ## Definition
--
--     ReLU(x) = max(0, x)
--
-- ## Why ReLU Became Dominant
--
-- Before ReLU, sigmoid and tanh were the standard activations. Both suffer
-- from vanishing gradients in deep networks: their derivatives are < 1
-- everywhere and shrink toward 0 in the tails.
--
-- ReLU's derivative is exactly 1 for all positive inputs — gradients flow
-- without attenuation through active (positive) neurons. This allowed
-- training of much deeper networks (AlexNet 2012 used ReLU and had 8 layers).
--
-- ## Dying ReLU Problem
--
-- If a neuron's input is always negative, its gradient is always 0. The
-- neuron never learns and stays permanently "dead". This motivates variants
-- like Leaky ReLU and ELU.
--
-- @param x  any real number
-- @return   x if x > 0, else 0.0
function M.relu(x)
    -- math.max works perfectly here — clean and idiomatic
    return math.max(0.0, x)
end

--- relu_derivative(x) → sub-gradient of ReLU at x
--
-- ## Formula
--
--     ReLU'(x) = 1   if x > 0
--              = 0   if x ≤ 0
--
-- Technically, ReLU is not differentiable at x = 0 (there is a kink).
-- By convention we assign the sub-gradient 0 at that point. Most deep
-- learning frameworks make this same choice.
--
-- @param x  any real number
-- @return   1 if x > 0, else 0
function M.relu_derivative(x)
    if x > 0 then return 1 else return 0 end
end

-- ============================================================================
-- TANH (Hyperbolic Tangent) and its derivative
-- ============================================================================

--- tanh_activation(x) → value in (-1, 1)
--
-- ## Definition
--
--     tanh(x) = (e^x − e^(−x)) / (e^x + e^(−x))
--
-- ## Relation to Sigmoid
--
--     tanh(x) = 2·σ(2x) − 1
--
-- Tanh is a rescaled and shifted sigmoid: it maps to (−1, 1) instead of
-- (0, 1). Being zero-centred is often an advantage over sigmoid because
-- the activations average to zero, reducing "zig-zagging" during gradient
-- descent.
--
-- ## Implementation
--
-- Lua 5.4 removed math.tanh from the standard library. We implement it
-- directly using the exponential identity:
--
--     tanh(x) = (e^x − e^(−x)) / (e^x + e^(−x))
--
-- This is equivalent to what math.tanh provided in Lua 5.1–5.3 and
-- matches the C runtime implementation.
--
-- @param x  any real number
-- @return   tanh value in (-1, 1)
function M.tanh_activation(x)
    local ep = math.exp(x)
    local en = math.exp(-x)
    return (ep - en) / (ep + en)
end

--- tanh_derivative(x) → derivative at x
--
-- ## Formula
--
--     d/dx tanh(x) = 1 − tanh(x)²
--
-- ## Derivation
--
-- Using the identity sech²(x) = 1 − tanh²(x):
--
--     d/dx tanh(x) = sech²(x) = 1 − tanh²(x)
--
-- ## Properties
--
-- - Maximum derivative is 1.0, at x = 0.
-- - Like sigmoid, the derivative vanishes as |x| grows (vanishing gradient).
-- - The range is (0, 1], symmetric around zero.
--
-- @param x  any real number
-- @return   derivative of tanh at x
function M.tanh_derivative(x)
    local t = M.tanh_activation(x)
    return 1.0 - t * t
end

-- ============================================================================
-- LEAKY RELU and its derivative
-- ============================================================================

--- leaky_relu(x, alpha) → x if x>0 else alpha*x
--
-- ## Motivation
--
-- Standard ReLU kills neurons permanently for x ≤ 0 (gradient = 0).
-- Leaky ReLU fixes this by allowing a small negative slope α (default 0.01).
-- This keeps the gradient non-zero for negative inputs, letting the neuron
-- continue to learn even when its weighted input is negative.
--
-- ## Definition
--
--     LeakyReLU(x; α) = x        if x > 0
--                     = α · x    if x ≤ 0
--
-- where α is a small positive constant, typically 0.01.
--
-- ## Comparison with ReLU
--
--     x = -5:  ReLU = 0,  LeakyReLU(α=0.01) = -0.05
--     x = +5:  both = 5
--
-- The tiny negative output for x < 0 keeps the gradient alive.
--
-- @param x      any real number
-- @param alpha  negative slope coefficient (default 0.01)
-- @return       activated value
function M.leaky_relu(x, alpha)
    alpha = alpha or 0.01
    if x > 0 then
        return x
    else
        return alpha * x
    end
end

--- leaky_relu_derivative(x, alpha) → gradient
--
-- ## Formula
--
--     LeakyReLU'(x; α) = 1   if x > 0
--                      = α   if x ≤ 0
--
-- @param x      any real number
-- @param alpha  negative slope coefficient (default 0.01)
-- @return       1 for positive inputs, alpha for non-positive
function M.leaky_relu_derivative(x, alpha)
    alpha = alpha or 0.01
    if x > 0 then
        return 1
    else
        return alpha
    end
end

-- ============================================================================
-- SOFTPLUS and its derivative
-- ============================================================================

--- softplus(x) -> log(1 + e^x)
--
-- Softplus is a smooth approximation of ReLU. The implementation uses the
-- stable equivalent log(1 + e^(-abs(x))) + max(x, 0).
function M.softplus(x)
    return math.log(1.0 + math.exp(-math.abs(x))) + math.max(x, 0.0)
end

--- softplus_derivative(x) -> sigmoid(x)
function M.softplus_derivative(x)
    return M.sigmoid(x)
end

-- ============================================================================
-- ELU (Exponential Linear Unit) and its derivative
-- ============================================================================

--- elu(x, alpha) → x if x>=0 else alpha*(e^x - 1)
--
-- ## Motivation
--
-- ELU improves on Leaky ReLU in two ways:
--
--   1. Negative saturation: As x → −∞, ELU(x) → −α, whereas Leaky ReLU
--      grows without bound. This bounded negativity acts like a form of
--      regularisation on the activations.
--
--   2. Smooth at zero: The derivative of ELU is continuous at x = 0 (both
--      sides give 1), which can improve gradient-based optimisation.
--
-- ## Definition
--
--     ELU(x; α) = x               if x ≥ 0
--              = α · (e^x − 1)   if x < 0
--
-- For x < 0: e^x ∈ (0, 1), so (e^x − 1) ∈ (−1, 0), so ELU ∈ (−α, 0).
--
-- ## Typical Alpha
--
-- α = 1.0 is the standard default. This gives a mean activation closer to
-- zero than ReLU, which can speed up learning.
--
-- @param x      any real number
-- @param alpha  scale for the exponential part (default 1.0)
-- @return       activated value
function M.elu(x, alpha)
    alpha = alpha or 1.0
    if x >= 0 then
        return x
    else
        return alpha * (math.exp(x) - 1.0)
    end
end

--- elu_derivative(x, alpha) → gradient at x
--
-- ## Formula
--
--     ELU'(x; α) = 1            if x ≥ 0
--               = α · e^x      if x < 0
--
-- ## Derivation
--
-- For x ≥ 0: d/dx [x] = 1
-- For x < 0: d/dx [α(e^x − 1)] = α · e^x
--
-- Note: at x = 0, the derivative from the left is α · e^0 = α.
-- When α = 1.0 (the default), both sides give 1, so ELU is smooth at 0.
--
-- @param x      any real number
-- @param alpha  scale for the exponential part (default 1.0)
-- @return       derivative of ELU at x
function M.elu_derivative(x, alpha)
    alpha = alpha or 1.0
    if x >= 0 then
        return 1
    else
        return alpha * math.exp(x)
    end
end

-- ============================================================================
-- SOFTMAX and its diagonal derivative
-- ============================================================================

--- softmax(values) → array of probabilities that sum to 1
--
-- ## Definition
--
--     softmax(x)_i = e^(x_i) / Σ_j e^(x_j)
--
-- ## Why Softmax for Multi-Class Output?
--
-- In a K-class classifier, the output layer produces K real numbers called
-- "logits". Softmax converts logits into a proper probability distribution:
--
--   1. All outputs are positive (e^x > 0 always).
--   2. All outputs sum to exactly 1.
--   3. The class with the highest logit still has the highest probability.
--   4. The distribution is "soft": even wrong classes get some probability
--      mass, which helps with calibration and confidence estimation.
--
-- ## Numerical Stability: Max-Subtraction Trick
--
-- A naive implementation computes e^(x_i) for each element. For large x_i
-- (e.g., x_i = 1000), e^1000 overflows to infinity in IEEE 754 double.
--
-- The fix: subtract max(x) from every element before exponentiating.
--
--     softmax(x)_i = e^(x_i − max_x) / Σ_j e^(x_j − max_x)
--
-- This is mathematically equivalent because max_x cancels:
--
--     e^(x_i − c) / Σ_j e^(x_j − c)
--   = [e^(x_i) · e^(−c)] / [Σ_j e^(x_j) · e^(−c)]
--   = e^(x_i) / Σ_j e^(x_j)
--
-- After subtracting max_x, all exponents are ≤ 0, so all e^(·) ∈ (0, 1].
--
-- @param values  table (array) of real-valued logits
-- @return        table of probabilities summing to 1.0
function M.softmax(values)
    if type(values) ~= "table" or #values == 0 then
        error("softmax: expected non-empty table, got " .. type(values))
    end

    -- Find the maximum value for numerical stability (the max-subtraction trick)
    local max_val = values[1]
    for i = 2, #values do
        if values[i] > max_val then
            max_val = values[i]
        end
    end

    -- Compute shifted exponentials: e^(x_i - max_val)
    -- Because x_i - max_val ≤ 0, each term is in (0, 1], preventing overflow.
    local exps = {}
    local sum  = 0.0
    for i = 1, #values do
        local e = math.exp(values[i] - max_val)
        exps[i] = e
        sum     = sum + e
    end

    -- Normalize: divide each exponential by the total sum
    local result = {}
    for i = 1, #values do
        result[i] = exps[i] / sum
    end

    return result
end

--- softmax_derivative(values) → diagonal Jacobian entries s_i * (1 - s_i)
--
-- ## The Full Jacobian vs. the Diagonal
--
-- The softmax Jacobian is a K×K matrix:
--
--     J_ij = ∂softmax(x)_i / ∂x_j
--          = softmax(x)_i · (δ_ij − softmax(x)_j)
--
-- where δ_ij = 1 if i = j, else 0.
--
-- In backpropagation you usually pair softmax with Categorical Cross-Entropy
-- loss, in which case the combined gradient simplifies to (ŷ − y), making
-- the full Jacobian unnecessary.
--
-- When you need only the diagonal entries (∂softmax_i / ∂x_i), the formula
-- simplifies to:
--
--     J_ii = s_i · (1 − s_i)
--
-- where s_i = softmax(x)_i. This function returns those diagonal entries.
--
-- ## Analogy with Sigmoid
--
-- Notice that J_ii = s_i · (1 − s_i) is exactly the sigmoid derivative
-- formula! This is not a coincidence: the two-class softmax reduces to
-- sigmoid, so the diagonal of the softmax Jacobian inherits the same form.
--
-- @param values  table of real-valued logits
-- @return        table of diagonal Jacobian entries (s_i * (1 - s_i))
function M.softmax_derivative(values)
    local s = M.softmax(values)
    local result = {}
    for i = 1, #s do
        -- s_i * (1 - s_i): the variance of the Bernoulli(s_i) distribution
        result[i] = s[i] * (1.0 - s[i])
    end
    return result
end

return M
