// SIMPLE-level constant folding (CLOC12.155).
//
// Each declaration's initializer is a constant expression that the
// `constant-fold` pass evaluates at compile time. Under WHITESPACE_ONLY
// these would survive as `1+2`, `3*4`, etc.; under SIMPLE they fold to
// their value. This fixture is the end-to-end oracle for PR-1.
//
// The values are passed to `report(...)` so they stay referenced —
// otherwise remove-unused-vars (now the last SIMPLE pass) would delete
// the whole declarations and the fold would not be observable.
var sum = 1 + 2;
var product = 3 * 4;
var nested = 2 + 3 * 4;
report(sum, product, nested);
