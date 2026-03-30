# Changelog — CodingAdventures::GradientDescent (Perl)

## [0.01] — 2026-03-29

### Added
- `new(%args)` constructor with `learning_rate`, `max_iterations`, `tolerance`.
- `step($weights, $gradient)` — single gradient update with error handling.
- `compute_loss($w, $inp, $tgt, $loss_fn)` — loss evaluation wrapper.
- `numerical_gradient($w, $inp, $tgt, $loss_fn, $epsilon)` — central finite-difference gradient.
- `train($w, $inp, $tgt, $loss_fn, $grad_fn)` — full training loop with early stopping.
- Comprehensive test suite: defaults, step correctness, numerical gradient accuracy, convergence on y=2x, learning rate effects.
