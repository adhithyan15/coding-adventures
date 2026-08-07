# drop-new-obj-arr — `new Array(1,2,3)` → `[1,2,3]`

Input: `var z=new Array(1,2,3);`

At SIMPLE, closurec folds `new` on the standard `Array`/`Object` constructors
(the siblings of the `new Error(…)` → `Error(…)` fold). Calling `Array`/`Object`
as an ordinary function constructs the same value as `new`, so the drop is
semantics-preserving. A 2+-argument `new Array(…)` becomes an array literal;
`new Array()` → `[]`, `new Object()` → `{}`, `new Object(x)` → `Object(x)`, and a
single-argument `new Array(x)` keeps the call form (a lone argument is a length,
so `Array(3)` ≠ `[3]`).

Expected (SIMPLE): `var z=[1,2,3];` — byte-identical to the reference Closure
Compiler. The proof the pipeline ran (not a WHITESPACE_ONLY fallback, which
would keep `new Array(1,2,3)`) is the array literal `[1,2,3]`.

See `closure-pass-constant-fold`'s `fold_standard_constructor`.
