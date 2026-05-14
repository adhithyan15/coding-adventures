# Changelog

## 0.1.1 — 2026-05-14

**Bug fixes: infinite recursion guard in `laplace_handler` and `exp(-t)` pattern recognition.**

### `handlers.py` — infinite recursion guard

`laplace_handler` passed every result from `laplace_transform` through `vm.eval()`.  When
`laplace_transform` fell through to the unevaluated form `IRApply(LAPLACE, (f, t, s))`, the
VM looked up the `Laplace` head, called `laplace_handler` again, which fell through again —
causing a `RecursionError` for any unrecognised input (e.g. `laplace(exp(-t), t, s)`).

Fixed by checking whether the result is still a `Laplace(…)` node before calling `vm.eval()`.
If it is, the unevaluated form is returned directly.

### `table.py` — `exp(-t)` and `exp(-(a·t))` pattern recognition

`_match_exp` previously only recognised `Exp(t)` and `Exp(Mul(a, t))`.  The MACSYMA parser
represents `-t` as `Neg(t)` (not `Mul(-1, t)`), so `exp(-t)` compiled to `Exp(Neg(t))`
which fell through the table, triggering the recursion bug above.

Extended `_match_exp` to also handle:
- `Exp(Neg(t))` → `a = −1`
- `Exp(Neg(Mul(a, t)))` → `a = −a`

Result: `laplace(exp(-t), t, s)` → `1/(s+1)`, and `laplace(exp(-2*t), t, s)` → `1/(s+2)`.

## 0.1.0 — 2026-04-27

**Initial release: Laplace transform and inverse Laplace transform.**

Implements the full Laplace transform pipeline for the MACSYMA symbolic computation system.

### New features

- `laplace_transform(f, t, s)` — forward Laplace transform via table lookup + linearity rules.
  Handles: 1, t^n, exp(at), sin(ωt), cos(ωt), exp(at)·sin(ωt), exp(at)·cos(ωt),
  t·exp(at), t^n·exp(at), sinh(at), cosh(at), t·sin(ωt), t·cos(ωt),
  DiracDelta(t), UnitStep(t).
  Applies linearity: L{c·f} = c·L{f} and L{f+g} = L{f}+L{g}.
  Falls through to unevaluated `Laplace(f, t, s)` for unrecognized patterns.

- `inverse_laplace(F, s, t)` — inverse Laplace transform via direct table lookup
  and partial-fraction decomposition.
  Handles direct forms: 1/s, A/(s-a), ω/(s²+ω²), s/(s²+ω²), a/(s²-a²), s/(s²-a²).
  Partial fractions: decomposes P(s)/Q(s) into simple-pole terms and inverts each.
  Falls through to unevaluated `ILT(F, s, t)` for unrecognized patterns.

- New IR head symbols: `DIRAC_DELTA = IRSymbol("DiracDelta")`, `UNIT_STEP = IRSymbol("UnitStep")`,
  `LAPLACE = IRSymbol("Laplace")`, `ILT = IRSymbol("ILT")`.
  DiracDelta and UnitStep are canonical here and shared with the future cas-fourier package.

- `build_laplace_handler_table()` — returns the VM handler table for integration
  with `symbolic-vm`'s `build_cas_handler_table()`.

- Handler implementations:
  - `laplace_handler` — dispatches `Laplace(f, t, s)` IR to `laplace_transform`
  - `ilt_handler` — dispatches `ILT(F, s, t)` IR to `inverse_laplace`
  - `dirac_delta_handler` — evaluates DiracDelta at numeric arguments (DiracDelta(0) → 1)
  - `unit_step_handler` — evaluates UnitStep with Heaviside convention (UnitStep(0) → 1/2)

### Package structure

```
src/cas_laplace/
  __init__.py       public API
  heads.py          IR head symbols
  table.py          forward transform table + matchers
  laplace.py        laplace_transform() top-level function
  inverse_table.py  inverse table + partial-fraction engine
  ilt.py            inverse_laplace() re-export
  handlers.py       VM handlers + build_laplace_handler_table()
```
