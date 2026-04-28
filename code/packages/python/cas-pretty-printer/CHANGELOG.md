# Changelog

## 0.4.0 — 2026-04-27

**2D box-model layout engine and `MNewton` function name.**

1. **New `box.py` — 2D box engine** (`src/cas_pretty_printer/box.py`):
   - `Box(lines, baseline)` dataclass — width/height as computed properties.
   - `atom_box(text)` — single-line box at baseline 0 for all leaf nodes.
   - `hbox(boxes, sep)` — align baselines horizontally, concatenate columns.
   - `vbox(boxes)` — stack vertically, centre-pad to widest box.
   - `_div_box` — fraction layout with `─` bar at the baseline row.
   - `_pow_box` — superscript layout (exponent above-right of base).
   - `_sqrt_box` — radical layout with `√`, `┌`, `─`, `┐`, `│` box drawing.
   - `pretty_2d(node, dialect)` — dispatcher for Add, Sub, Mul, Div, Pow,
     Sqrt, Neg, List; falls back to linear for unknown heads.

2. **`walker.py` — `style="2d"` support**:
   - `pretty(node, dialect, style="2d")` now routes to `pretty_2d` instead
     of raising `ValueError`. `style="linear"` behaviour is unchanged.
     Unknown styles still raise `ValueError`.

3. **`dialect.py` — `MNewton` name**:
   - Added `"MNewton": "mnewton"` to `_DEFAULT_FUNCTION_NAMES` so
     unevaluated `MNewton(f, x, x0)` nodes pretty-print as `mnewton(f, x, x0)`.

4. **Tests** — 31 new tests in `tests/test_box.py` covering primitives,
   layout geometry, fraction bars, superscripts, radical signs, nested
   fractions, style validation, and MNewton name mapping.

---

## 0.3.0 — 2026-04-27

**Complete MACSYMA function-name alias coverage.**

Two related improvements to make the pretty-printer round-trippable for
every IR head that appears in the MACSYMA completion roadmap:

1. **`_DEFAULT_FUNCTION_NAMES` expansion** (`dialect.py`): Added ~40 missing
   IR-head-to-surface-name entries for heads that can appear unevaluated in
   partially-evaluated trees.  New entries cover: `Cbrt`, `Last`, `Append`,
   `Reverse`, `Range`, `Sort`, `Part`, `Flatten`, `Join`, `MakeList`,
   `Matrix`, `Transpose`, `Determinant`, `Inverse`, `Gcd`, `Lcm`, `Mod`,
   `Floor`, `Ceiling`, `Lhs`, `Rhs`, `At`, `NSolve`, `Collect`, `Together`,
   `RatSimplify`, `Apart`, `TrigSimplify`, `TrigExpand`, `TrigReduce`, `Re`,
   `Im`, `Conjugate`, `Arg`, `RectForm`, `PolarForm`, `IsPrime`, `NextPrime`,
   `PrevPrime`, `FactorInteger`, `Divisors`, `Totient`, `MoebiusMu`,
   `JacobiSymbol`, `ChineseRemainder`, `IntegerLength`.

2. **`MacsymaDialect` MACSYMA-specific overrides** (`macsyma.py`): Updated
   the dialect's `function_names` dict to `{**_DEFAULT_FUNCTION_NAMES, ...}`
   so it inherits all generic spellings and then overrides with MACSYMA names:
   `Select→sublist`, `Inverse→invert`, `RatSimplify→ratsimp`,
   `Apart→partfrac`, `TrigSimplify/TrigExpand/TrigReduce` as before,
   `Re→realpart`, `Im→imagpart`, `Arg→carg`, `IsPrime→primep`,
   `NextPrime→next_prime`, `PrevPrime→prev_prime`, `FactorInteger→ifactor`,
   `MoebiusMu→moebius`, `ChineseRemainder→chinese`, `IntegerLength→numdigits`.

23 new alias tests added to `test_macsyma_dialect.py` verifying each
MACSYMA-specific surface spelling.  Total test count 84 across all dialect
modules; coverage 96 %.

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
