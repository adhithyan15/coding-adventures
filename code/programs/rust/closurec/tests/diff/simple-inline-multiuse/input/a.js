// SIMPLE-level multi-use inlining (CLOC13.G).
//
// `inline` now substitutes a small pure function at ALL its call sites,
// not just when it is used once — provided every use is an inlinable
// call and the body fits the size budget (here `x * x` is 3 nodes, the
// budget for one parameter is 2 + 1 = 3).
//
//   sweep 1: both `sq(3)` and `sq(4)` are replaced by `3 * 3` / `4 * 4`;
//            `sq` is now unreferenced and removed by treeshake.
//   sweep 2: constant-fold folds `3 * 3` → 9 and `4 * 4` → 16.
//
// Result: `a(9); b(16);`.
function sq(x) {
  return x * x;
}
a(sq(3));
b(sq(4));
