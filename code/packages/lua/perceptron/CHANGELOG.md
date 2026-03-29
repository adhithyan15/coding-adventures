# Changelog — coding-adventures-perceptron (Lua)

## [0.1.0] — 2026-03-29

### Added
- `Perceptron.new(opts)` constructor with `n_inputs`, `learning_rate`, `activation_fn`, `weights`, `bias`.
- `p:predict(input)` — forward pass returning output and pre-activation z.
- `p:train_step(input, target)` — single Rosenblatt perceptron learning update.
- `p:train(inputs, targets, epochs)` — full training loop.
- Bundled `step`, `sigmoid`, and `sigmoid_derivative` activation functions.
- Test suite: AND gate, OR gate, bias shifting, error handling, activation function tests.
