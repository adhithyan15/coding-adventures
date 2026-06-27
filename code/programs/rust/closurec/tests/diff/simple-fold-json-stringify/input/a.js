// SIMPLE-level static JSON.stringify fold → string literal (primitive subset).
//
// JSON.stringify(x) (ECMAScript §25.5.2) serialises a value to JSON text. The
// fold collapses a call to a STRING literal when the single argument is a
// primitive literal whose JSON text is renderable exactly:
//   * JSON.stringify(42)    → "42"     (a number's ToString is its JSON form)
//   * JSON.stringify(-7)    → "-7"
//   * JSON.stringify(true)  → "true"
//   * JSON.stringify(null)  → "null"
//   * JSON.stringify("x")   → declined (JSON string escaping left to runtime)
//   * JSON.stringify(3.5)   → declined (fractional — fold_string_of_number None)
//
// A string/array/object arg, a non-literal, or a 2nd replacer/space arg is left
// intact. Under WHITESPACE_ONLY every call survives; under SIMPLE the foldable
// ones collapse. Each value flows into report(...) so it stays referenced past
// remove-unused-vars and the fold is observable.
var a = JSON.stringify(42);
var b = JSON.stringify(-7);
var c = JSON.stringify(true);
var d = JSON.stringify(null);
var e = JSON.stringify("x");
var f = JSON.stringify(3.5);
report(a, b, c, d, e, f);
