# drop-new-error — `new Error(…)` → `Error(…)`

Input: `throw new Error("boom");`

At SIMPLE, closurec drops the redundant `new` on the built-in `Error`
constructor: calling `Error` as an ordinary function constructs an Error object
identically to `new` (ECMAScript §20.5.1.1 — the constructor's `[[Call]]` and
`[[Construct]]` paths converge), so the drop is semantics-preserving.

Expected (SIMPLE): `throw Error("boom");` — byte-identical to the reference
Closure Compiler. The proof the pipeline ran (not a WHITESPACE_ONLY fallback,
which would keep `new Error(...)` verbatim) is the absence of the `new` keyword.

Scope is `Error` only — `RegExp` (unsound when the arg is a regex), `Object`/
`Array` (fold further to `{}`/`[]`), and the Error subtypes are not folded. See
`closure-pass-constant-fold`'s `NewExpression` arm.
