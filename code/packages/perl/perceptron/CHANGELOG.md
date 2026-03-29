# Changelog — CodingAdventures::Perceptron (Perl)

## [0.01] — 2026-03-29

### Added
- `new(%args)` constructor with `n_inputs`, `learning_rate`, `activation_fn`, `weights`, `bias`.
- `predict($input)` — forward pass returning (output, z).
- `train_step($input, $target)` — single Rosenblatt learning update.
- `train($inputs, $targets, $epochs)` — full training loop.
- `step($z)`, `sigmoid($z)`, `sigmoid_derivative($z)` activation functions.
- Test suite: AND gate, OR gate, bias convergence, step/sigmoid tests, error handling.
