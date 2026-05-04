# cas-complex — Complex Number Arithmetic and Normalization

> **Status**: New spec. Implements complex-number IR support: the
> `ImaginaryUnit` constant, arithmetic normalization, and `Re`, `Im`,
> `Conjugate`, `Abs`, `Arg`, `RectForm`, `PolarForm` heads.
> Parent: `symbolic-computation.md`. Depends on `cas-simplify`.

## Why this package exists

Complex numbers appear naturally the moment you solve `x² + 1 = 0`.
`cas-solve` already lists `{i, -i}` as outputs but defers the
representation question here. The implementation strategy is:

- Represent the imaginary unit as a pre-bound symbol `ImaginaryUnit`
  that satisfies `ImaginaryUnit² = -1`.
- Extend the existing arithmetic handlers (Add, Mul, Pow) with
  simplification rules that automatically normalize expressions
  containing `ImaginaryUnit` into rectangular form `a + b·i`.
- Add utility heads (`Re`, `Im`, `Conjugate`, `Abs`, `Arg`,
  `RectForm`, `PolarForm`) to decompose and transform complex expressions.

No new IR node type is needed — complex numbers are represented entirely
as IR trees using existing node types plus the `ImaginaryUnit` symbol.

## Reuse story

| MACSYMA              | Mathematica          | Maple              | IR head / constant       |
|----------------------|----------------------|--------------------|--------------------------|
| `%i`                 | `I`                  | `I`                | `IRSymbol("ImaginaryUnit")` |
| `realpart(z)`        | `Re[z]`              | `Re(z)`            | `Re`                     |
| `imagpart(z)`        | `Im[z]`              | `Im(z)`            | `Im`                     |
| `conjugate(z)`       | `Conjugate[z]`       | `conjugate(z)`     | `Conjugate`              |
| `cabs(z)`            | `Abs[z]`             | `abs(z)`           | `Abs` (extended)         |
| `carg(z)`            | `Arg[z]`             | `argument(z)`      | `Arg`                    |
| `rectform(z)`        | `ComplexExpand[z]`   | `convert(z, rect)` | `RectForm`               |
| `polarform(z)`       | `ToPolarCoordinates` | `convert(z, polar)`| `PolarForm`              |

## Scope

In:

- Pre-binding `ImaginaryUnit` in every backend that includes this package
  so `%i`, `I`, or `i` (depending on the frontend's name table) resolves
  immediately.
- Simplification rules for `ImaginaryUnit` powers:
  - `ImaginaryUnit^0 = 1`
  - `ImaginaryUnit^1 = ImaginaryUnit`
  - `ImaginaryUnit^2 = -1`
  - `ImaginaryUnit^3 = -ImaginaryUnit`
  - `ImaginaryUnit^4 = 1` (and modular reduction for higher powers)
- Arithmetic normalization: automatically collect real and imaginary parts
  when `ImaginaryUnit` is present in an `Add` or `Mul` expression.
- `Re(z)`, `Im(z)` — extract real and imaginary parts when `z` is in
  rectangular form `a + b·ImaginaryUnit`.
- `Conjugate(z)` — negate the imaginary part.
- `Abs(z)` extended to complex inputs — returns `Sqrt(Re(z)^2 + Im(z)^2)`.
  (The existing `Abs` handler already covers reals; this extends it.)
- `Arg(z)` — the principal argument `arctan(Im(z)/Re(z))`.
- `RectForm(z)` — rewrite `z` as `a + b·ImaginaryUnit` with real `a`, `b`.
- `PolarForm(z)` — rewrite `z` as `r·exp(ImaginaryUnit·θ)`.

Out:

- Arbitrary-precision complex arithmetic (that's the job of floats + IR).
- Full complex analysis (branch cuts, Riemann surfaces) — research territory.
- Gaussian integers (Z[i] factoring) — deferred to a future
  `cas-number-theory` extension.

## Public interface

```python
from cas_complex import (
    IMAGINARY_UNIT,          # IRSymbol("ImaginaryUnit")
    normalize_complex,       # IRNode → IRNode  (collect real+imag parts)
    re_part,                 # IRNode → IRNode
    im_part,                 # IRNode → IRNode
    conjugate,               # IRNode → IRNode
    build_complex_handler_table,  # () → dict[str, Handler]
    COMPLEX_SIMPLIFY_RULES,  # list[Rule]  — ImaginaryUnit power rules
)
```

`COMPLEX_SIMPLIFY_RULES` is a list of `cas-pattern-matching` rules
installed on `SymbolicBackend` at startup. They fire inside the normal
simplification loop so `i^2` folds to `-1` automatically without any
explicit `TrigSimplify`/`ComplexSimplify` call.

## The ImaginaryUnit representation

`ImaginaryUnit = IRSymbol("ImaginaryUnit")`.

Frontend name tables map surface syntax to this symbol:

```python
# macsyma_runtime/name_table.py
{"%i": IRSymbol("ImaginaryUnit")}

# future mathematica_runtime/name_table.py
{"I": IRSymbol("ImaginaryUnit")}
```

The symbol is pre-bound in the backend's environment as a self-referential
binding: `env["ImaginaryUnit"] = IRSymbol("ImaginaryUnit")`. This prevents
the VM from treating it as a free user variable while still allowing it to
appear in IR trees exactly as written. (The same pattern is used for `True`
and `False` in `SymbolicBackend`.)

## Arithmetic simplification rules

These rules are loaded into `SymbolicBackend`'s rule set:

```
Pow(ImaginaryUnit, 0)  →  1
Pow(ImaginaryUnit, 1)  →  ImaginaryUnit
Pow(ImaginaryUnit, 2)  →  -1
Pow(ImaginaryUnit, 3)  →  Neg(ImaginaryUnit)
Pow(ImaginaryUnit, n)  →  Pow(ImaginaryUnit, Mod(n, 4))   for integer n ≥ 4
Mul(ImaginaryUnit, ImaginaryUnit)  →  -1
Mul(a_, ImaginaryUnit, Mul(b_, ImaginaryUnit))  →  Neg(Mul(a_, b_))
```

The modular-reduction rule for general integer exponents (`n ≥ 4`) is
applied by a small Python function in the handler (not a pure pattern
rule) since `n mod 4` requires arithmetic on `n`.

### Rectangular normalization

After every `Add` or `Mul` containing `ImaginaryUnit`, a normalization
pass collects the expression into `a + b·ImaginaryUnit` form:

- Walk the tree looking for sub-expressions that are multiples of
  `ImaginaryUnit` (`Mul(c, ImaginaryUnit)`).
- Separate real and imaginary parts.
- Return `Add(real_part, Mul(imag_part, ImaginaryUnit))` or just
  `real_part` if `imag_part = 0`.

This is `normalize_complex` and is called by the `Mul`/`Add` handlers
when `ImaginaryUnit` is detected in the argument list.

## Heads added

| Head              | Arity | Meaning                                         |
|-------------------|-------|-------------------------------------------------|
| `Re`              | 1     | Real part of `a + b·i`: returns `a`.            |
| `Im`              | 1     | Imaginary part: returns `b`.                    |
| `Conjugate`       | 1     | Complex conjugate: `a + b·i → a - b·i`.         |
| `Abs`             | 1     | Extended: `|z| = sqrt(a² + b²)` for complex `z`.|
| `Arg`             | 1     | Principal argument `arctan2(b, a)`.             |
| `RectForm`        | 1     | Rewrite as `a + b·ImaginaryUnit`.               |
| `PolarForm`       | 1     | Rewrite as `r·Exp(Mul(ImaginaryUnit, theta))`.  |

`Abs` already exists as a VM handler (in `cas_handlers.py`) for real
inputs. `cas-complex` extends it: if the input is known to be complex
(i.e., contains `ImaginaryUnit`), compute `Sqrt(Add(Pow(a,2), Pow(b,2)))`.
Otherwise fall through to the existing real handler.

## Algorithm

### Re / Im

Extract parts from normalized form `a + b·ImaginaryUnit`:

```python
def re_part(expr: IRNode) -> IRNode:
    expr = normalize_complex(expr)
    # Case: Add(a, Mul(b, ImaginaryUnit)) → a
    if is_rect_form(expr):
        return real_component(expr)
    # Case: pure real (no ImaginaryUnit) → expr itself
    if not contains(expr, IMAGINARY_UNIT):
        return expr
    # Otherwise unevaluated
    return IRApply(RE, (expr,))
```

### Conjugate

```python
def conjugate(expr: IRNode) -> IRNode:
    expr = normalize_complex(expr)
    a, b = split_rect(expr)   # a + b*i
    return IRApply(ADD, (a, IRApply(MUL, (IRApply(NEG, (b,)), IMAGINARY_UNIT))))
```

### RectForm

Recursively distribute `ImaginaryUnit` through the expression, apply
arithmetic rules, and collect with `normalize_complex`. Handles
nested complex sub-expressions by recursion.

### PolarForm

1. `a, b = split_rect(RectForm(expr))`
2. `r = Abs(expr)` → `Sqrt(Add(Pow(a, 2), Pow(b, 2)))`
3. `theta = Arg(expr)` → `Atan2(b, a)` (uses `Atan` which exists already)
4. Return `Mul(r, Exp(Mul(IMAGINARY_UNIT, theta)))`

## Effect on cas-solve

Once `cas-complex` is present, `cas-solve` can return complex roots for
`x² + 1 = 0` as `[ImaginaryUnit, Neg(ImaginaryUnit)]` instead of
falling through unevaluated.

## MACSYMA name table entries

```python
# macsyma_runtime/name_table.py additions
COMPLEX_NAME_TABLE = {
    "%i":        IRSymbol("ImaginaryUnit"),
    "realpart":  IRSymbol("Re"),
    "imagpart":  IRSymbol("Im"),
    "conjugate": IRSymbol("Conjugate"),
    "cabs":      IRSymbol("Abs"),
    "carg":      IRSymbol("Arg"),
    "rectform":  IRSymbol("RectForm"),
    "polarform": IRSymbol("PolarForm"),
}
```

## Backend integration

`cas-complex` ships `build_complex_handler_table()` and
`COMPLEX_SIMPLIFY_RULES`. These are wired in `SymbolicBackend.__init__`
the same way as `build_cas_handler_table()`:

```python
# in symbolic_vm/backends.py  SymbolicBackend.__init__
from cas_complex import build_complex_handler_table, COMPLEX_SIMPLIFY_RULES
handlers.update(build_complex_handler_table())
self._rules = list(COMPLEX_SIMPLIFY_RULES) + existing_rules
self._env["ImaginaryUnit"] = IMAGINARY_UNIT
```

## Test strategy

- `Pow(ImaginaryUnit, 2) = -1`.
- `Pow(ImaginaryUnit, 4) = 1`.
- `Pow(ImaginaryUnit, 7) = Neg(ImaginaryUnit)`.
- `Mul(ImaginaryUnit, ImaginaryUnit) = -1`.
- `Add(3, Mul(4, ImaginaryUnit))` stays in rect form.
- `Mul(Add(1, ImaginaryUnit), Add(1, Neg(ImaginaryUnit))) = 2`.
  (i.e., `(1+i)(1-i) = 2`)
- `Mul(Add(1, ImaginaryUnit), Add(1, ImaginaryUnit)) = Add(Mul(2, ImaginaryUnit), 0)`
  ... actually `2i`. Verify rect form.
- `Re(Add(3, Mul(4, ImaginaryUnit))) = 3`.
- `Im(Add(3, Mul(4, ImaginaryUnit))) = 4`.
- `Conjugate(Add(3, Mul(4, ImaginaryUnit))) = Sub(3, Mul(4, ImaginaryUnit))`.
- `Abs(Add(3, Mul(4, ImaginaryUnit))) = 5` (after numeric fold).
- `RectForm(Mul(Exp(Mul(ImaginaryUnit, Div(Pi, 2))))) = ImaginaryUnit`
  (Euler's formula, requires TrigSimplify co-operation).
- `%i` in MACSYMA session resolves to `ImaginaryUnit` IR node.
- `solve(x^2 + 1, x)` returns `[ImaginaryUnit, Neg(ImaginaryUnit)]` once
  `cas-complex` is present.
- Coverage: ≥85%.

## Package layout

```
code/packages/python/cas-complex/
  src/cas_complex/
    __init__.py
    constants.py      # IMAGINARY_UNIT sentinel, pre-bind logic
    rules.py          # ImaginaryUnit power rules for SymbolicBackend
    normalize.py      # normalize_complex, split_rect, is_rect_form
    parts.py          # re_part, im_part, conjugate
    polar.py          # RectForm, PolarForm, Arg handlers
    handlers.py       # build_complex_handler_table()
    py.typed
  tests/
    test_powers.py
    test_arithmetic.py
    test_parts.py
    test_polar.py
    test_macsyma_pipeline.py   # end-to-end: "%i^2;" → -1
```

## Dependencies

`coding-adventures-symbolic-ir`,
`coding-adventures-cas-simplify`,
`coding-adventures-cas-pattern-matching`.

## Future extensions

- Gaussian integer factoring: `FactorGaussian(a + b·i)` in the
  `cas-number-theory` package.
- Full `Exponentialize` head: rewrite `sin(x)` / `cos(x)` as
  complex exponentials `(e^{ix} ± e^{-ix})/2`. Enables `cas-trig`'s
  Phase 2 `TrigReduce` for `sinⁿ` for arbitrary `n`.
- Hypercomplex numbers (quaternions) — a separate package if needed.
