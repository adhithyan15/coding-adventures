// SIMPLE-level unary bitwise-NOT folding.
//
// `~<numeric literal>` is a compile-time-evaluable expression that the
// `constant-fold` pass now collapses under ES `ToInt32` semantics
// (the same coercion the binary `&`/`|`/`^` operators already use).
// Under WHITESPACE_ONLY these would survive as `~5`, `~-1`, `~5.9`;
// under SIMPLE they fold to their value:
//
//   ~5    →  -6   (~ToInt32(5)  = ~5  = -6)
//   ~-1   →   0   (~ToInt32(-1) = ~-1 =  0)
//   ~5.9  →  -6   (ToInt32 truncates toward zero first → ~5)
//   ~~9   →   9   (double complement is the ToInt32 identity; folds
//                  bottom-up in one walk: ~9 → -10, then ~-10 → 9)
//
// The values flow into `report(...)` so they stay referenced —
// otherwise remove-unused-vars (the last SIMPLE pass) would delete the
// whole declarations and the fold would not be observable.
var a = ~5;
var b = ~-1;
var c = ~5.9;
var d = ~~9;
report(a, b, c, d);
