// Big-pass ADVANCED proof: a small geometry module that exercises FOUR
// optimization passes at once, with a runtime-equivalence anchor.
//
// What each declaration is here to demonstrate, and what ADVANCED does to it:
//
//   unusedPerimeter  — never called, never referenced  →  DEAD-CODE ELIMINATION
//                      (tree-shaken away entirely).
//   area / hypotSq   — each called exactly once, with LITERAL arguments  →
//                      SINGLE-USE INLINING + CONSTANT FOLDING: the call sites
//                      collapse to the literal results `12` and `25`. These two
//                      literals are the RUNTIME-EQUIVALENCE ANCHOR — the
//                      optimized program reports the very same numbers the
//                      original would compute at runtime.
//   scale            — called once *and* passed by value to `sink`  →  the
//                      value-use keeps it alive (the inliner declines it), so it
//                      survives and is GLOBAL-RENAMED to a short name (`f`).
//                      Under SIMPLE the name `scale` is kept; the rename is the
//                      ADVANCED-only behaviour this fixture pins.
//
// report / sink are undeclared externs (sinks), so they are left untouched.
//
// Hand-computed runtime values (what the ORIGINAL program reports):
//   area(3, 4)     = 3 * 4         = 12
//   hypotSq(3, 4)  = 3*3 + 4*4     = 9 + 16 = 25
//   scale(7)       = 7 * 10        = 70
// The optimized output reports `12, 25, f(7)` where `f(7) = 7 * 10 = 70` — the
// identical observable behaviour, at ~18% of the original byte size.
function unusedPerimeter(w, h) {
  return 2 * (w + h);
}
function area(w, h) {
  return w * h;
}
function hypotSq(a, b) {
  return a * a + b * b;
}
function scale(x) {
  return x * 10;
}
report(area(3, 4), hypotSq(3, 4), scale(7));
sink(scale);
