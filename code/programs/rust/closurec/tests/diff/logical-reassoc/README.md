# logical-reassoc — `a&&(b&&c)` → `a&&b&&c`

Input: `x=a&&(b&&c);`

At SIMPLE, closurec left-associates a right-nested same-operator `&&`/`||`:
`a && (b && c)` → `(a && b) && c`. Both operators are fully associative (same
value, same short-circuit point, same left-to-right evaluation order), so the
rewrite is behaviour-preserving. The left-nested form prints without the parens
the right-nested form requires.

Expected (SIMPLE): `x=a&&b&&c;` — byte-identical to the reference Closure
Compiler. The proof the pipeline ran (not a WHITESPACE_ONLY fallback, which
would keep `a&&(b&&c)`) is the absence of the inner parens.

See `closure-pass-constant-fold`'s `fold_logical`.
