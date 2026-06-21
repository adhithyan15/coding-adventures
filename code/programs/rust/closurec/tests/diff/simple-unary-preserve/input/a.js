// Regression oracle for the prefix-unary-operator-drop miscompile.
//
// The bridge (`javascript-parser/src/bridge.rs`) used to discriminate the two
// `unary_expression` grammar alternatives by counting AST *child nodes*. But
// the prefix operator (`!`, `-`, `~`, `typeof`, `void`, `delete`) is a *token*
// child, which `node_children` filters out — so every prefix-operator form
// looked like a bare pass-through and the operator was silently DROPPED:
//
//   !a   bridged to   a          // negation lost — a *miscompile*, not a
//   -b   bridged to   b          // missed optimization
//   ~c   bridged to   c
//
// WHITESPACE_ONLY kept the operators (it never runs the bridge), so the bug
// only showed at SIMPLE/ADVANCED — exactly where correctness matters most.
//
// `a`, `b`, `c` come from side-effecting calls, so they are NOT foldable and
// the unary operators stay as real prefix operators over identifiers. The
// unused `dead` binding is removed by the typed pipeline — its disappearance
// proves this output came from the SIMPLE optimizer, not the WHITESPACE_ONLY
// fallback. And `!(a < b)` must keep its parentheses: emitting `!a < b`
// would reparse as `(!a) < b`, a different program. (A relational operator is
// used here rather than `==` so the negation-push optimization — which would
// rewrite `!(a == b)` to `a != b` — leaves it intact; relational negations are
// not inverted, since `!(a < b)` ≠ `a >= b` under NaN.)
//
// At SIMPLE this becomes:
//   var a=first();var b=second();var c=third();report(!a,-b,~c,!(a < b));
var a = first();
var b = second();
var c = third();
var dead = 4 + 5;
report(!a, -b, ~c, !(a < b));
