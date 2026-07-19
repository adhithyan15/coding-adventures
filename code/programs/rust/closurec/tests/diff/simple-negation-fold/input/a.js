// Regression oracle for the negation-push optimization
// (upstream Closure's PeepholeMinimizeConditions):
//
//   !(a == b)   →  a != b          !(a === b)  →  a !== b
//
// Sound for the four (in)equality operators only — `!=`/`!==` are *defined*
// as the boolean negation of `==`/`===`. Relational operators are NOT
// inverted: `!(a < b)` is not `a >= b` when an operand is NaN, so the `!`
// over `a < b` must survive verbatim.
//
// `a`, `b` come from side-effecting calls so they don't fold to literals,
// keeping the equality comparison (and thus the rewrite) visible. The unused
// `dead = 8 + 9` binding is KEPT — SIMPLE is open-world and never deletes an
// observable top-level `var` (another script could read it) — but its
// initializer is still constant-folded to `17`, which proves this is the
// SIMPLE optimizer rather than the WHITESPACE_ONLY fallback (which would keep
// `8 + 9` verbatim).
//
// At SIMPLE this becomes:
//   var a=first(),b=second(),dead=17;report(a!=b,a!==b,!(a<b));
var a = first();
var b = second();
var dead = 8 + 9;
report(!(a == b), !(a === b), !(a < b));
