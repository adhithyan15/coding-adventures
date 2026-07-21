# Changelog

All notable changes to this project will be documented in this file.

## [0.1.1] - 2026-07-20

### Security

- **Fixed a dimension-product-overflow gap in every array-constructing/
  -growing operation.** `count()` capped each dimension independently
  (`1<<26`) but never their PRODUCT: `zeros(1<<26, 1<<26)`, `eye(1<<26)`,
  repeated matrix self-concatenation (`A = [A A];` many times), and 2-D
  indexing (`A(idx, idx)` with two independently-in-bounds index vectors)
  could each still request an astronomical element count (up to ~4.5e15
  elements, ~36 petabytes) before ever erroring — an allocation-aborting
  denial of service, not a clean error. Found during security review of the
  sibling `scilab-runtime` crate (MA-10d), which shares the identical
  vulnerability class and was fixed the same way first.
- Added `builtins::check_total_elements(name, rows, cols)` (uses
  `checked_mul` so the multiplication itself cannot silently overflow) and
  wired it into `dims()`, `eye()`, `hcat`, `vcat` (checked incrementally as
  the result is built, not just once at the end — closes the
  self-concatenation growth path), and `index_value`'s 2-D indexing arm.
  `octave-runtime` inherits the fix automatically since it reuses this
  crate's `Interpreter` directly.
- Added regression tests: `constructor_rejects_an_astronomical_element_product`,
  `matrix_self_concatenation_cannot_double_past_the_element_cap`,
  `two_d_indexing_rejects_a_product_overflow`.

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
