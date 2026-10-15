# Focused coverage reruns need threshold overrides when the package enforces aggregate coverage

A focused `vitest --coverage` rerun proved the previously timed-out build-guard
tests pass quickly, but the command still exited nonzero because a single test
file cannot satisfy package-wide 85% coverage thresholds. For flake isolation,
either run the focused test without coverage or explicitly override all global
thresholds to zero. Use the full coverage command—not a focused run—as the
actual aggregate coverage gate.
