// SIMPLE-level static Number.isSafeInteger(x) fold → boolean.
//
// Number.isSafeInteger (ECMAScript §21.1.2.5) returns true iff its argument is a
// NUMBER that is a finite integer with magnitude ≤ 2^53−1
// (Number.MAX_SAFE_INTEGER = 9007199254740991). It does NO coercion: every
// non-number argument is false. So a numeric literal folds by classification, a
// non-number literal folds to false, and an identifier is declined:
//   * Number.isSafeInteger(7)                → true
//   * Number.isSafeInteger(3.5)              → false (not an integer)
//   * Number.isSafeInteger(9007199254740991) → true  (MAX_SAFE_INTEGER)
//   * Number.isSafeInteger(9007199254740992) → false (2^53, past the safe range)
//   * Number.isSafeInteger(x)                → declined (type unknown)
//
// Each value flows into report(...) so it stays referenced past
// remove-unused-vars and the fold is observable.
var a = Number.isSafeInteger(7);
var b = Number.isSafeInteger(3.5);
var c = Number.isSafeInteger(9007199254740991);
var d = Number.isSafeInteger(9007199254740992);
var e = Number.isSafeInteger(x);
report(a, b, c, d, e);
