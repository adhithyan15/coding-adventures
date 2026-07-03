// SIMPLE-level constant propagation (CLOC13.H).
//
// The `inline-variables` pass propagates a top-level `const` bound to a
// literal to its use sites; remove-unused-vars then deletes the emptied
// declaration, and the fixed-point constant-fold sweep folds the now
// concrete arithmetic:
//
//   sweep 1: `RATE` (a const = 2) is propagated → `base * 2` and
//            `2 + 1`; the `const RATE = 2;` declaration is now unused
//            and removed by remove-unused-vars.
//   sweep 2: constant-fold folds `2 + 1` → `3`.
//
// Result: `total(base * 2); margin(3);`. `base` is a free variable, so
// `base * 2` cannot be folded further.
const RATE = 2;
total(base * RATE);
margin(RATE + 1);
