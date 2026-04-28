# Changelog

## 0.2.0 — 2026-04-27

**Smarter negative-number display and new MACSYMA sugar rules.**

Four display-quality improvements to the `MacsymaDialect` and walker:

1. **Negative integer coefficient parenthesization** (`walker.py`): The
   threshold for wrapping a negative integer literal in parentheses was
   lowered from `min_prec > 0` to `min_prec > PREC_NEG (55)`.  This means
   `Mul(-2, y)` now prints as `-2*y` instead of the confusing `(-2)*y`,
   while `Pow(x, -3)` correctly still prints as `x^(-3)` because `Pow`'s
   precedence (60) exceeds `PREC_NEG`.

2. **`Add(n<0, b) → b - n` sugar** (`macsyma.py`): A negative integer
   literal as the *first* `Add` argument is now swapped to subtraction form.
   `Add(-1, y)` prints as `y - 1` instead of `(-1) + y`.

3. **`Mul(a, Neg(b)) → Neg(Mul(a, b))` sugar** (`macsyma.py`): Unary minus
   on the second `Mul` argument is pulled to the front.
   `Mul(sin(x), Neg(sin(x)))` prints as `-(sin(x)*sin(x))`.

4. **One-level recursive sugar peek in `Add`** (`macsyma.py`): The
   `Add(a, Neg(b))` → `Sub(a, b)` rule now peeks one level of sugar on the
   second argument.  This means `Add(x, Mul(a, Neg(b)))` renders as
   `x - a*b` instead of `x + -(a*b)`, fixing the product-rule diff output
   of `diff(sin(x)*cos(x), x)` to print as
   `cos(x)*cos(x) - sin(x)*sin(x)`.

8 new tests added; total test count 62 across all dialect test modules,
coverage maintained.

## 0.1.0 — 2026-04-25

Initial release.

- `Dialect` protocol and `BaseDialect` ABC.
- Walker handles every IR node type (`IRSymbol`, `IRInteger`,
  `IRRational`, `IRFloat`, `IRString`, `IRApply`).
- Operator precedence and associativity tracking; parens inserted
  only when required.
- Surface-syntax sugar: `Add(x, Neg(y)) → x - y`, `Mul(x, Inv(y)) →
  x / y`, `Mul(-1, x) → -x`.
- `MacsymaDialect`, `MathematicaDialect`, `MapleDialect`,
  `LispDialect` ship out of the box.
- `register_head_formatter` hook for downstream packages to teach
  the printer about new heads (Matrix, Determinant, Limit, etc.).
- Type-checked (`py.typed`); ruff- and mypy-clean.
