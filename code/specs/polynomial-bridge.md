# Polynomial Bridge — IR ↔ Polynomial for CAS Integration

## Why this module exists

Phase 2 of the symbolic integrator needs to decide, given an arbitrary
symbolic expression tree, whether that expression is a rational function
of the integration variable `x` — and if so, convert it to a concrete
`(numerator, denominator)` pair of `Polynomial` values over Q so the
rational-function algorithms (Hermite reduction, Rothstein–Trager) can
run on it.

The bridge is the answer. It lives in `symbolic-vm` because that's the
package that owns the "IR walks that aren't VM handlers" slot, and it
depends on both `symbolic-ir` (to read trees) and `polynomial` (to emit
polynomials). Neither of those two packages should ever depend on the
other — the bridge is the one-way coupling point.

## Scope

- **One variable at a time.** The caller names an `IRSymbol` — every
  occurrence of that symbol becomes `x` in the polynomial; every other
  symbol is treated as an *algebraic constant* and enters the
  coefficient tuple as itself. Phase 2c only actually integrates over
  Q (rational coefficients); free symbols inside coefficients are a
  deliberate restriction enforced at the CAS level, not at the bridge.
  The bridge itself only emits coefficients for literals `IRInteger`,
  `IRRational`; anything else in a coefficient position makes it return
  `None`.
- **Rational functions only.** No trig, log, exp, sqrt. Those are
  transcendental and demand Phase 3 machinery (the Risch structure
  theorem).
- **Integer exponents only.** `x^n` with `n` a literal integer (any
  sign). Symbolic or fractional exponents make it return `None` — they
  take the integrand outside Q(x).

## Two functions

### `to_rational(f, x) -> tuple[Polynomial, Polynomial] | None`

Structural recursion over an `IRNode`:

| Input node                         | Output `(num, den)`                          |
|------------------------------------|----------------------------------------------|
| `IRInteger(c)`, `IRRational(p/q)`  | `((Fraction(c),), (Fraction(1),))`           |
| `x` (the named symbol)             | `((0, 1), (1,))`                             |
| `Add(a, b)`                        | `(n_a · d_b + n_b · d_a, d_a · d_b)`         |
| `Sub(a, b)`                        | `(n_a · d_b − n_b · d_a, d_a · d_b)`         |
| `Neg(a)`                           | `(−n_a, d_a)`                                |
| `Mul(a, b)`                        | `(n_a · n_b, d_a · d_b)`                     |
| `Div(a, b)`                        | `(n_a · d_b, d_a · n_b)`                     |
| `Pow(base, IRInteger(n))` `n ≥ 0`  | `(n_base^n, d_base^n)`                       |
| `Pow(base, IRInteger(n))` `n < 0`  | `(d_base^|n|, n_base^|n|)` if `n_base ≠ 0`   |
| anything else                      | `None`                                       |

Non-numeric literal floats (`IRFloat`) are rejected. Floats are the
opposite of exact — admitting them into the rational-function pipeline
is how a CAS silently produces wrong answers.

If any sub-call returns `None`, the whole call returns `None`. This
gives the caller a clean rational-or-not gate.

### `from_polynomial(p, x) -> IRNode`

Emits a **canonical** IR tree for the polynomial `p` evaluated at the
named symbol `x`:

- Zero polynomial → `IRInteger(0)`.
- Constant `(c,)` → `_coef(c)` (`IRInteger` if c is a whole number,
  else `IRRational(p, q)`).
- Otherwise, build the `Add` of non-zero terms, each term being
  `Mul(coef, Pow(x, IRInteger(i)))` — with coefficient `1` collapsed,
  exponent `0` omitted, and exponent `1` emitting bare `x`.

The output shape matches what the Phase 1 integrator and the
differentiator already emit, so the VM's existing simplifier cleans it
up without a round-trip through `vm.eval`. The bridge itself does not
invoke the VM.

## Non-goals

- **Common-factor cancellation.** `to_rational(x^2 - 1, x) /
  to_rational(x - 1, x)` returns `(x^2 - 1) / (x - 1)` — not `x + 1`.
  That's what `gcd` on the numerator and denominator is for, and
  Hermite reduction does exactly that cancellation as one of its first
  steps. The bridge stays structural.
- **Partial-fraction decomposition.** Phase 2c.
- **Detecting free symbols in coefficient position.** An integrand like
  `(x + y) / (x - 1)` with an unbound `y` is *almost* a rational
  function — but Q[x] doesn't know about `y`. The bridge returns
  `None` on any non-literal in coefficient position; the CAS layer
  decides whether to push back to the user or try a richer
  representation.
- **Converting back through `vm.eval`.** `from_polynomial` is a pure
  tree builder; the caller composes it with `vm.eval` if they want
  simplification.

## Round-trip guarantees

- For any polynomial `p` over Q, `to_rational(from_polynomial(p, x), x)`
  returns `(p, (Fraction(1),))` — the constant-1 denominator.
- For any `IRNode` `f` that is a rational function of `x`,
  `to_rational(f, x)` returns some `(n, d)` such that
  `n / d` is mathematically equal to `f`. The particular `(n, d)` pair
  isn't canonical (no cancellation); only the quotient is invariant.

## Tests

- Round-trip all of: `x`, `x + 1`, `x² − 1`, `(x² + 1) / (x − 1)`,
  `1 / (x · (x − 1))`, `-(x)`.
- Rejection cases: `sin(x)`, `log(x)`, `x^y` with `y` symbolic,
  `x^(1/2)` (rational exponent), `IRFloat(1.5)` in any position.
- Free symbol as constant: `y + x` → `None` (Phase 2c restricts to Q).
- Structural round-trip shape: `from_polynomial((Fraction(3),), x)` is
  `IRInteger(3)`, not `Add(Mul(3, Pow(x, 0)))`.
- Coverage target ≥ 95%.
