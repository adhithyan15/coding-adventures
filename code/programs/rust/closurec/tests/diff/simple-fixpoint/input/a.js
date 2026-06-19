// SIMPLE-level fixed-point iteration (CLOC13.F).
//
// The pass pipeline now runs to a FIXED POINT: it sweeps the pass order
// repeatedly while any FixedPoint pass still reports a change, so a
// transform one pass exposes is picked up by an earlier pass on the next
// sweep. This input needs TWO sweeps to fully optimize:
//
//   sweep 1: `inline` substitutes the single-use `double(7)` call with
//            the function body, giving `log(7 * 2)`; `double` is then
//            removed by remove-unused-vars / treeshake.
//   sweep 2: `constant-fold` (which ran BEFORE inline in sweep 1, so it
//            never saw `7 * 2`) now folds `7 * 2` to `14`.
//
// Result: `log(14);`. Before fixed-point iteration the pipeline ran each
// pass once and stopped at `log(7 * 2)`.
function double(x) {
  return x * 2;
}
log(double(7));
