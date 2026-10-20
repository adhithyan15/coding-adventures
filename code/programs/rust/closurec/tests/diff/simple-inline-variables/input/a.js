// SIMPLE keeps a top-level `const` (CCR-078, #15863).
//
// At SIMPLE the reference compiler is open-world: a top-level binding is a
// property of the global object, which another script may read. It does not
// propagate such a binding to its ordinary uses, not even a `const`. It only
// uses a literal-initialized binding's value to decide a statement condition
// (`if (RATE)`, `RATE && f()`), which then folds away.
//
// The pinned v20260915 oracle leaves this program as it is (minified):
//   const RATE=2;total(base*RATE);margin(RATE+1);
//
// Under ADVANCED (closed world) the const is propagated and dropped:
//   total(base*2);margin(3);
const RATE = 2;
total(base * RATE);
margin(RATE + 1);
