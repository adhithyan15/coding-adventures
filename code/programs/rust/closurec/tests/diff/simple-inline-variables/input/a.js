// SIMPLE-level constant propagation (CLOC13.H) — the propagation runs, but the
// declaration is KEPT (open-world).
//
// `inline-variables` propagates a top-level `const` bound to a literal to its
// use sites. It is a value-copying pass (it does not remove the source
// declaration) and runs at SIMPLE. What does NOT run at SIMPLE is
// `remove-unused-vars` — deleting the now-unreferenced declaration would drop
// an observable global, a CLOSED-WORLD move reserved for ADVANCED. So the
// declaration stays even after its value is copied out:
//
//   sweep 1: `RATE` (a const = 2) is propagated → `base * 2` and `2 + 1`.
//   sweep 2: constant-fold folds `2 + 1` → `3`. (This two-sweep interplay —
//            inline-variables exposing a fold that constant-fold, having
//            already run, only catches on the next pass — is the fixed-point
//            iteration this fixture exercises, entirely within SIMPLE.)
//
// Result: `const RATE=2;total(base*2);margin(3);`. `base` is a free variable,
// so `base * 2` cannot be folded further. The `const RATE = 2` declaration is
// KEPT (remove-unused-vars is ADVANCED-only); under ADVANCED it would be
// dropped, giving `total(base*2);margin(3);`.
const RATE = 2;
total(base * RATE);
margin(RATE + 1);
