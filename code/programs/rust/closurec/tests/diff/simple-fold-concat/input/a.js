// SIMPLE-level string-concat fold (String#concat).
//
// `"<string>".concat(<string>, ...)` is compile-time-evaluable when the
// receiver and every argument are string literals; the `constant-fold` pass
// joins them into a single literal (JS `String.prototype.concat`). Here
// `"foo".concat("bar", "baz")` → `"foobarbaz"`. A non-string argument (which JS
// would coerce via `ToString`), a non-literal argument, or a result over the
// optimizer's size cap is left for the runtime. Under WHITESPACE_ONLY the call
// survives; under SIMPLE it folds.
//
// The value flows into `report(...)` so it stays referenced — otherwise
// remove-unused-vars (the last SIMPLE pass) would delete the declaration and
// the fold would not be observable.
var s = "foo".concat("bar", "baz");
report(s);
