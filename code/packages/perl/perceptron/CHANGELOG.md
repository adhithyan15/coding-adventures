# Changelog — CodingAdventures::Perceptron (Perl)

## [Unreleased]

### Changed

- `BUILD_windows` now runs the real test suite on Windows through the shared `code/scripts/run-perl-tests-windows.ps1` helper (native Strawberry Perl, absolute `-I` paths for sibling libraries) instead of the previous skip or bare-`perl` recipe.

## [0.01] — 2026-03-29

### Added
- `new(%args)` constructor with `n_inputs`, `learning_rate`, `activation_fn`, `weights`, `bias`.
- `predict($input)` — forward pass returning (output, z).
- `train_step($input, $target)` — single Rosenblatt learning update.
- `train($inputs, $targets, $epochs)` — full training loop.
- `step($z)`, `sigmoid($z)`, `sigmoid_derivative($z)` activation functions.
- Test suite: AND gate, OR gate, bias convergence, step/sigmoid tests, error handling.
