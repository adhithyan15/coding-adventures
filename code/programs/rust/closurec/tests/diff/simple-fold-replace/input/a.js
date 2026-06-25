// SIMPLE-level string replace / replaceAll fold (String#replace / replaceAll).
//
// On a string literal with literal search + replacement arguments, the
// `constant-fold` pass collapses the call to a single string literal:
//   - `replace(from, to)`     substitutes the FIRST match;
//   - `replaceAll(from, to)`  substitutes EVERY match.
// The string overload matches `from` LITERALLY (no regex), so `.` is a literal
// dot. Here `"a-b-c".replaceAll("-","_")` → `"a_b_c"` and
// `"aXbXc".replace("X","-")` → `"a-bXc"` (first X only). Under WHITESPACE_ONLY
// the calls survive; under SIMPLE they fold. (Verified against V8.)
//
// The values flow into `report(...)` so they stay referenced — otherwise
// remove-unused-vars (the last SIMPLE pass) would delete the declarations and
// the fold would not be observable.
var a = "a-b-c".replaceAll("-", "_");
var b = "aXbXc".replace("X", "-");
report(a, b);
