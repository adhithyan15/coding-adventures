// SIMPLE-level dead-branch hoisted-`var` extraction (miscompile fix).
//
// A statically-dead `if` branch is removed, but a `var` declared inside it still
// HOISTS to the enclosing function scope. Dropping it would be a miscompile: a
// later read of the name flips from a declared-`undefined` binding to a
// `ReferenceError`. So the binding is EXTRACTED (its initializer stripped, since
// the branch never runs) as a bare `var z;` placed before the taken `else` body:
//
//   if (false) { var z = compute(); } else use();  ->  var z; use();
//
// `z` is still read afterwards, so the binding must survive. Byte-identical to
// the reference Closure Compiler at SIMPLE. (When several such extractions occur
// in one scope the reference additionally coalesces the bare `var`s to the scope
// top — a separate normalization tracked elsewhere.)
if (false) { var z = compute(); } else use();
sink(z);
