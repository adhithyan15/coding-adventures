# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-06-16

### Added

- Initial release — **MA-3d** of the MATLAB frontend: a tree-walking evaluator
  over `array-runtime` (spec [`MA01`](../../../specs/MA01-matlab-language.md)).
- `Interpreter` (persistent workspace) + `feed`/`eval` entry points producing the
  MATLAB prompt echo (`x =` / `ans =`, `;` suppresses).
- **Matrix products lower to `array_runtime::execute(MatMul, …)`** — the planner
  picks the backend (CPU now, GPU when registered), so `A * B` is accelerated by
  cost with no language-level GPU code. Element-wise operators use the
  `array_runtime::ops` reference path (exact `f64`, scalar broadcasting).
- Matrix/range literals, `+ - .* ./ .^ * ' ~`, comparisons and logicals,
  variables/assignment, 1-based indexing (`A(i)`, `A(i,j)`, `A(:,k)`, `A(end)`),
  `if`/`for`/`while`, and the core builtins (`zeros`/`ones`/`eye`/`size`/`length`/
  `numel`/`sum`/`mean`/`max`/`min`/`abs`/`sqrt`/`transpose`/`disp`).
- DoS bounds: range length and constructor dimensions are capped (`1<<26`), and
  `while` has an iteration limit, so `1:1e18`, `zeros(1e18)`, and runaway loops
  are clean errors.
