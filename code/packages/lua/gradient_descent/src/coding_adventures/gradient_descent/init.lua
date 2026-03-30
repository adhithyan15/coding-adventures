-- ============================================================================
-- gradient_descent — Iterative weight optimisation via gradient descent
-- ============================================================================
--
-- Gradient descent is the workhorse optimisation algorithm behind virtually
-- every trained machine-learning model.  It answers the question: "given that
-- my current weights produce some loss, how should I change them to produce
-- *less* loss on the next iteration?"
--
-- ## The Core Idea
--
-- Imagine standing on a hilly landscape where elevation represents the loss.
-- Your goal is to reach the lowest valley (minimum loss).  Gradient descent
-- works like this:
--
--   1. Measure the slope (gradient) at your current position.
--   2. Take a small step *opposite* to the slope (downhill).
--   3. Repeat until the valley is flat (gradient ≈ 0) or you've taken enough
--      steps.
--
-- Mathematically, for a weight vector w and loss function L:
--
--   w_new = w - lr * ∇L(w)
--
-- where lr is the learning rate (step size) and ∇L(w) is the gradient
-- vector (direction of steepest ascent).
--
-- ## What This Module Provides
--
-- | Function              | Purpose                                          |
-- |-----------------------|--------------------------------------------------|
-- | GradientDescent.new() | Create a configured optimiser                    |
-- | gd:train()            | Run the full training loop until convergence     |
-- | gd:step()             | Apply one gradient update to the weights         |
-- | gd:numerical_gradient | Approximate gradient via finite differences      |
-- | gd:compute_loss()     | Evaluate the loss function at current weights    |
--
-- ## Learning Rate: The Critical Hyperparameter
--
-- The learning rate lr controls the step size:
--
--   - Too large:  overshoots the minimum; loss oscillates or diverges.
--   - Too small:  very slow convergence; may get stuck in local minima.
--   - Just right: converges smoothly in a reasonable number of iterations.
--
-- A typical starting value is 0.01.  For simple problems, 0.1 works well.
--
-- ## Numerical vs. Analytical Gradients
--
-- Analytical gradients require you to derive ∂L/∂w_i by hand (or via
-- automatic differentiation).  Numerical gradients approximate the derivative
-- using finite differences:
--
--   ∂L/∂w_i ≈ (L(w + ε·eᵢ) - L(w - ε·eᵢ)) / (2ε)
--
-- where eᵢ is the i-th basis vector and ε is a small perturbation (1e-5).
-- Numerical gradients are slower (2n loss evaluations for n weights) but
-- require no closed-form derivative.  They are invaluable for testing that
-- an analytical gradient is correct.
--
-- ## Usage
--
--   local gd_mod = require("coding_adventures.gradient_descent")
--   local lf = require("coding_adventures.loss_functions")
--
--   local gd = gd_mod.new({ learning_rate = 0.1, max_iterations = 1000, tolerance = 1e-6 })
--
--   -- Train a linear model y = w*x (single weight)
--   local weights = {0.0}
--   local inputs  = {{1.0}, {2.0}, {3.0}}
--   local targets = {2.0, 4.0, 6.0}
--
--   local trained_w = gd:train(weights, inputs, targets,
--       function(w, inp, tgt) ... end,
--       function(w, inp, tgt) ... end)
--
-- ============================================================================

local M = {}

M.VERSION = "0.1.0"

-- ============================================================================
-- Constructor
-- ============================================================================

--- GradientDescent.new creates a new gradient descent optimiser.
--
-- ## Parameters (options table)
--
-- | Key            | Type   | Default | Meaning                              |
-- |----------------|--------|---------|--------------------------------------|
-- | learning_rate  | number | 0.01    | Step size for each weight update     |
-- | max_iterations | int    | 1000    | Maximum gradient-descent steps       |
-- | tolerance      | number | 1e-6    | Stop when loss change < tolerance    |
--
-- All three can be omitted; defaults are chosen to work for small problems.
--
-- @param opts  table  Optional configuration table.
-- @return      table  A GradientDescent instance.
function M.new(opts)
    opts = opts or {}
    local self = {
        learning_rate  = opts.learning_rate  or 0.01,
        max_iterations = opts.max_iterations or 1000,
        tolerance      = opts.tolerance      or 1e-6,
    }
    setmetatable(self, { __index = M })
    return self
end

-- ============================================================================
-- step — Apply one weight update
-- ============================================================================

--- step applies one gradient descent update to the weight vector.
--
-- This is the fundamental operation of gradient descent:
--
--   w_new[i] = w[i] - lr * gradient[i]
--
-- The negative sign is crucial: we move *opposite* to the gradient because
-- the gradient points uphill (toward higher loss) and we want to go downhill
-- (toward lower loss).
--
-- ## Why subtract, not add?
--
-- The gradient ∇L(w) is the direction of steepest *increase* of L.
-- Subtracting lr * ∇L moves us toward steeper *decrease*.
--
-- Truth table for a single weight:
--
--   gradient > 0  →  w decreases  (we were climbing right; step left)
--   gradient < 0  →  w increases  (we were climbing left;  step right)
--   gradient = 0  →  w unchanged  (we're at a flat point — possibly a minimum)
--
-- @param weights   table  Current weight vector (array of numbers).
-- @param gradient  table  Gradient vector, same length as weights.
-- @return          table  Updated weight vector (new table; inputs unchanged).
-- @return          string Error message, or nil on success.
function M:step(weights, gradient)
    if #weights ~= #gradient then
        return nil, string.format(
            "step: weights length (%d) must equal gradient length (%d)",
            #weights, #gradient
        )
    end
    if #weights == 0 then
        return nil, "step: weights must be non-empty"
    end

    local new_weights = {}
    for i = 1, #weights do
        -- w_new = w - lr * grad
        new_weights[i] = weights[i] - self.learning_rate * gradient[i]
    end
    return new_weights, nil
end

-- ============================================================================
-- compute_loss — Evaluate the loss function at given weights
-- ============================================================================

--- compute_loss evaluates loss_fn at the given weights, inputs, and targets.
--
-- This is a thin wrapper that calls the caller-supplied loss function with
-- the standard (weights, inputs, targets) signature and returns the scalar
-- loss value.
--
-- @param weights   table     Current weight vector.
-- @param inputs    table     Input matrix (array of arrays).
-- @param targets   table     Target values (array of numbers).
-- @param loss_fn   function  function(weights, inputs, targets) → number.
-- @return          number    Scalar loss value.
function M:compute_loss(weights, inputs, targets, loss_fn)
    return loss_fn(weights, inputs, targets)
end

-- ============================================================================
-- numerical_gradient — Finite-difference gradient approximation
-- ============================================================================

--- numerical_gradient approximates the gradient of loss_fn at the given
-- weights using the central-difference formula:
--
--   ∂L/∂w_i ≈ (L(w + ε·eᵢ) - L(w - ε·eᵢ)) / (2ε)
--
-- where:
--   - eᵢ is the unit vector in dimension i (all zeros except position i = 1)
--   - ε (epsilon) is a small perturbation, default 1e-5
--
-- ## Why central difference (not forward difference)?
--
-- Forward difference: (f(x+h) - f(x)) / h   — error O(h)
-- Central difference: (f(x+h) - f(x-h)) / (2h) — error O(h²)
--
-- Central difference is more accurate for the same ε because the leading
-- error term cancels out.  For ε = 1e-5, forward difference has error ~1e-5
-- while central difference has error ~1e-10.
--
-- ## Computational Cost
--
-- For n weights, this performs 2n loss function evaluations.  For models with
-- millions of parameters, this is impractical — that's why backpropagation
-- (analytical gradients) is used in practice.  For small educational models,
-- numerical gradients are perfectly fine and extremely simple to implement.
--
-- @param weights   table     Current weight vector.
-- @param inputs    table     Input matrix.
-- @param targets   table     Target values.
-- @param loss_fn   function  function(weights, inputs, targets) → number.
-- @param epsilon   number    Perturbation size. Default 1e-5.
-- @return          table     Gradient vector, same length as weights.
function M:numerical_gradient(weights, inputs, targets, loss_fn, epsilon)
    epsilon = epsilon or 1e-5
    local grad = {}

    for i = 1, #weights do
        -- Perturb weight i upward
        local w_plus = {}
        for j = 1, #weights do w_plus[j] = weights[j] end
        w_plus[i] = weights[i] + epsilon

        -- Perturb weight i downward
        local w_minus = {}
        for j = 1, #weights do w_minus[j] = weights[j] end
        w_minus[i] = weights[i] - epsilon

        -- Central difference
        local loss_plus  = loss_fn(w_plus,  inputs, targets)
        local loss_minus = loss_fn(w_minus, inputs, targets)
        grad[i] = (loss_plus - loss_minus) / (2.0 * epsilon)
    end

    return grad
end

-- ============================================================================
-- train — Full gradient-descent training loop
-- ============================================================================

--- train runs the gradient descent loop until convergence or max_iterations.
--
-- ## Algorithm
--
--   1. Evaluate loss at current weights.
--   2. Compute gradient (analytical if loss_derivative_fn given, else numerical).
--   3. Update weights: w = w - lr * gradient.
--   4. If |loss_new - loss_old| < tolerance, stop early (converged).
--   5. Repeat up to max_iterations times.
--
-- ## Convergence Criterion
--
-- We stop when the absolute change in loss between consecutive iterations is
-- less than `tolerance`.  This is a simple but effective criterion:
--
--   |L(w_t) - L(w_{t-1})| < tolerance
--
-- Note that this can trigger if the loss plateaus at a saddle point or local
-- minimum, not just the global minimum.  For convex problems (like linear
-- regression with MSE loss), any stationary point is the global minimum.
--
-- ## Parameters
--
-- @param weights              table     Initial weight vector.
-- @param inputs               table     Input matrix (array of arrays).
-- @param targets              table     Target values.
-- @param loss_fn              function  function(w, inp, tgt) → scalar loss.
-- @param loss_derivative_fn   function  function(w, inp, tgt) → gradient vector.
--                                       If nil, numerical gradient is used.
-- @return                     table     Trained weight vector.
-- @return                     string    Error, or nil on success.
function M:train(weights, inputs, targets, loss_fn, loss_derivative_fn)
    -- Work on a copy so the original is not mutated.
    local w = {}
    for i = 1, #weights do w[i] = weights[i] end

    local prev_loss = self:compute_loss(w, inputs, targets, loss_fn)

    for _iter = 1, self.max_iterations do
        -- Compute gradient — analytical if provided, numerical otherwise.
        local grad
        if loss_derivative_fn then
            grad = loss_derivative_fn(w, inputs, targets)
        else
            grad = self:numerical_gradient(w, inputs, targets, loss_fn)
        end

        -- Apply the weight update.
        local new_w, err = self:step(w, grad)
        if err then return nil, err end
        w = new_w

        -- Check convergence.
        local curr_loss = self:compute_loss(w, inputs, targets, loss_fn)
        if math.abs(curr_loss - prev_loss) < self.tolerance then
            break
        end
        prev_loss = curr_loss
    end

    return w, nil
end

return M
