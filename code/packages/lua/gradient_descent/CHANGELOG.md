# Changelog — coding-adventures-gradient-descent (Lua)

## [0.1.0] — 2026-03-29

### Added
- `GradientDescent.new(opts)` constructor with `learning_rate`, `max_iterations`, `tolerance`.
- `gd:step(weights, gradient)` — single gradient update.
- `gd:compute_loss(weights, inputs, targets, loss_fn)` — loss evaluation wrapper.
- `gd:numerical_gradient(weights, inputs, targets, loss_fn, epsilon)` — central finite-difference gradient approximation.
- `gd:train(weights, inputs, targets, loss_fn, loss_derivative_fn)` — full training loop with early stopping.
- Comprehensive test suite covering convergence on `y = 2x`, gradient accuracy, learning rate effects, and error handling.
