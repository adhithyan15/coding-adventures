// SIMPLE-level string-padding fold (String#padStart / padEnd).
//
// `"<string>".padStart(<target>[, <pad>])` is compile-time-evaluable on a
// string literal with an integer-literal target length and an optional
// string-literal pad; the `constant-fold` pass collapses it to the padded
// string (JS `String.prototype.padStart`). The pad string (default a single
// space) is repeated and truncated to fill the shortfall. Here
// `"5".padStart(3, "0")` → `"005"`. A non-integer target, a non-literal pad,
// a target over the optimizer's size cap, or a fill that would split a
// surrogate pair are left for the runtime. Under WHITESPACE_ONLY the call
// survives; under SIMPLE it folds.
//
// The value flows into `report(...)` so it stays referenced — otherwise
// remove-unused-vars (the last SIMPLE pass) would delete the declaration and
// the fold would not be observable.
var s = "5".padStart(3, "0");
report(s);
