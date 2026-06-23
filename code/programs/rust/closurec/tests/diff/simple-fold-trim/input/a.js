// SIMPLE-level string-trim fold (String#trim / trimStart / trimEnd).
//
// `"<string>".trim()` is compile-time-evaluable on a string literal; the
// `constant-fold` pass collapses it to the string with leading and trailing
// whitespace removed (JS `String.prototype.trim`). JS strips a specific
// WhiteSpace + LineTerminator set (U+0009-000D, U+0020, U+00A0, U+1680,
// U+2000-200A, U+2028, U+2029, U+202F, U+205F, U+3000, U+FEFF) — NOT Rust's
// `char::is_whitespace`, which differs. Here `"  hi  ".trim()` → `"hi"`.
// `trimStart`/`trimEnd` strip one end. Under WHITESPACE_ONLY the call survives;
// under SIMPLE it folds.
//
// The value flows into `report(...)` so it stays referenced — otherwise
// remove-unused-vars (the last SIMPLE pass) would delete the declaration and
// the fold would not be observable.
var s = "  hi  ".trim();
report(s);
