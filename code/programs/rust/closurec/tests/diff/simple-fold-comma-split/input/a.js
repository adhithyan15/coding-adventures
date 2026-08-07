// SIMPLE-level split of a comma-sequence EXPRESSION STATEMENT into one
// statement per operand -- the inverse of loop-body comma-fusion.
//
// The comma operator evaluates its operands left-to-right and discards every
// value but the last. An expression statement already discards its value, so at
// a statement-LIST position (a function body or the program body) running each
// operand as its own `;`-terminated statement is behaviour-identical, and the
// reference compiler normalizes to that form:
//
//   a(), b();          ->  a(); b();      (inside function f)
//   x(), y(), z();     ->  x(); y(); z(); (program body)
//
// A comma sequence in a SINGLE-statement body (an `if`/`for` with no braces) has
// no statement list to splice into, so the sequence stays FUSED there:
//
//   if (cond) p(), q();       -> cond&&(p(),q());   (if collapses to &&, comma kept)
//   for (; run();) step(),tick();  -> comma stays fused as the loop's one body stmt
//
// All call targets are open-world globals, so nothing is removed -- only the
// statement-list splits are observable.
function f() {
  a(), b();
}
x(), y(), z();
if (cond) p(), q();
for (; run(); ) step(), tick();
