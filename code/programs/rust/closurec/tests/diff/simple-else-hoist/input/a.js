// CLOC25 — a redundant `else` is hoisted out after a terminating consequent.
//
// `classify`'s `if` consequent ends in `return`, so when the test is true the
// function exits there; the `else` body therefore only runs when the test was
// false and can be lifted out to right after the `if`, deleting the `else`
// keyword and its braces. The trailing `report(...)` call keeps `classify`
// reachable so tree-shaking does not remove it.
//
// At SIMPLE this becomes:
//   function classify(n){if(n < 0){return negative(n)}record(n);return positive(n)}
// while WHITESPACE_ONLY (which runs no optimization passes) keeps the `else`.
function classify(n) {
  if (n < 0) {
    return negative(n);
  } else {
    record(n);
    return positive(n);
  }
}
report(classify(5));
