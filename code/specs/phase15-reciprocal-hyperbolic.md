# Phase 15 — Reciprocal Hyperbolic Functions

## Status

**Complete** — shipped in `symbolic-vm` 0.35.0 / `symbolic-ir` 0.8.0 /
`macsyma-compiler` 0.8.0.

---

## Motivation

Phase 13 added the six primary hyperbolic functions (`sinh`, `cosh`, `tanh`,
`asinh`, `acosh`, `atanh`). The standard hyperbolic set also includes three
**reciprocal** functions that MACSYMA exposes:

| MACSYMA name | Definition | IR head |
|---|---|---|
| `coth(x)` | `cosh(x)/sinh(x)` | `COTH` |
| `sech(x)` | `1/cosh(x)` | `SECH` |
| `csch(x)` | `1/sinh(x)` | `CSCH` |

Phase 15 wires these three functions end-to-end: evaluation handlers,
differentiation rules, and bare (no polynomial multiplier) integration
formulas.  Poly×coth/sech/csch integration is deliberately out of scope:
the antiderivative residuals involve `log(sinh)` / `log(cosh)` terms that
belong to a different function class, better handled in a dedicated later phase.

---

## Mathematical Algorithms

### Evaluation

All three are implemented via the existing `_elementary` factory in
`symbolic-vm/handlers.py`. They are **pure real functions** that diverge at
`x = 0` (coth, csch) or converge to `sech(0) = 1`:

| Head | `numeric_fn` | `exact_identities` |
|------|--------------|-------------------|
| `COTH` | `lambda x: cosh(x)/sinh(x)` | `{}` (undefined at 0) |
| `SECH` | `lambda x: 1.0/cosh(x)` | `{0: ONE}` |
| `CSCH` | `lambda x: 1.0/sinh(x)` | `{}` (undefined at 0) |

### Differentiation (chain rule)

```
d/dx coth(u) = -u' / sinh²(u)

d/dx sech(u) = -u'·sinh(u) / cosh²(u)
             = -u'·sech(u)·tanh(u)

d/dx csch(u) = -u'·cosh(u) / sinh²(u)
             = -u'·csch(u)·coth(u)
```

The IR representation uses `SINH` and `COSH` nodes directly (avoiding
`COTH`/`SECH`/`CSCH` self-references in the derivative, which keeps the
simplifier's work simpler):

```
coth'(u):  NEG( DIV(darg,       POW(SINH(u), 2)) )
sech'(u):  NEG( DIV(MUL(darg, SINH(u)), POW(COSH(u), 2)) )
csch'(u):  NEG( DIV(MUL(darg, COSH(u)), POW(SINH(u), 2)) )
```

When `darg` is the integer literal `1` (i.e. `d/dx f(x)`), the
numerator simplifies:

```
coth'(x):  NEG( DIV(ONE,    POW(SINH(x), 2)) )
sech'(x):  NEG( DIV(SINH(x), POW(COSH(x), 2)) )
csch'(x):  NEG( DIV(COSH(x), POW(SINH(x), 2)) )
```

### Integration (bare linear argument)

For a strictly-linear argument `ax+b` with `a ≠ 0`:

```
∫ coth(ax+b) dx  =  (1/a) · log(sinh(ax+b))
∫ sech(ax+b) dx  =  (1/a) · atan(sinh(ax+b))
∫ csch(ax+b) dx  =  (1/a) · log(tanh((ax+b)/2))
```

**Derivation / verification:**

```
d/dx [log(sinh(ax+b))]          = a·cosh(ax+b)/sinh(ax+b) = a·coth(ax+b)    ✓

d/dx [atan(sinh(ax+b))]         = a·cosh(ax+b)/(1+sinh²(ax+b))
                                = a·cosh(ax+b)/cosh²(ax+b)     (1+sinh²=cosh²)
                                = a/cosh(ax+b) = a·sech(ax+b)                ✓

d/dx [log(tanh((ax+b)/2))]      let h = (ax+b)/2
  = (a/2) · sech²(h) / tanh(h)
  = (a/2) · 1 / (sinh(h)·cosh(h))
  = (a/2) · 2 / sinh(ax+b)      [since sinh(2h) = 2·sinh(h)·cosh(h)]
  = a / sinh(ax+b) = a·csch(ax+b)                                            ✓
```

Note: ``−atanh(cosh(ax+b))`` is algebraically equivalent for the csch integral
but requires ``|cosh(ax+b)| < 1`` for real evaluation — which is never satisfied.
The ``log(tanh(half))`` form is real-valued and numerically safe for ``ax+b > 0``.

All three antiderivatives use **existing** IR heads (LOG, ATAN, TANH, SINH) — no
new assembly patterns are required.

IR for each helper function:

```python
# _coth_integral(a, b, x):
arg = linear_to_ir(a, b, x)
result = LOG(SINH(arg)) [/ a  if a ≠ 1]

# _sech_integral(a, b, x):
arg = linear_to_ir(a, b, x)
result = ATAN(SINH(arg)) [/ a  if a ≠ 1]

# _csch_integral(a, b, x):
half_arg = linear_to_ir(a/2, b/2, x)
result = LOG(TANH(half_arg)) [/ a  if a ≠ 1]
```

---

## Fallthrough Behaviour (deferred cases)

The following inputs return **unevaluated** `Integrate(...)`:

| Input | Reason |
|-------|--------|
| `∫ P(x)·coth(ax+b) dx` for `deg P ≥ 1` | residual is `log(sinh)×P'` — different function class |
| `∫ P(x)·sech(ax+b) dx` for `deg P ≥ 1` | residual involves `atan(sinh)×P'` |
| `∫ P(x)·csch(ax+b) dx` for `deg P ≥ 1` | residual involves `atanh(cosh)×P'` |
| `∫ coth(f(x)) dx` for non-linear `f` | not a linear argument |
| `∫ sech(x)^n dx` for `n ≥ 2` | power-reduction formulas deferred |

---

## Package Versions

| Package | Before | After |
|---|---|---|
| `symbolic-ir` | 0.7.6 | 0.8.0 |
| `symbolic-vm` | 0.34.0 | 0.35.0 |
| `macsyma-compiler` | 0.7.0 | 0.8.0 |

`macsyma-runtime` is **unchanged** — coth/sech/csch are compiler-mapped
elementary functions (like sinh/cosh/tanh), not runtime name-table entries.

---

## Files Changed

All paths under `code/packages/python/`.

| File | Change |
|------|--------|
| `symbolic-ir/src/symbolic_ir/nodes.py` | +3 `IRSymbol` definitions: COTH, SECH, CSCH |
| `symbolic-ir/src/symbolic_ir/__init__.py` | import + `__all__` for COTH, SECH, CSCH |
| `symbolic-ir/CHANGELOG.md` | 0.8.0 entry |
| `symbolic-ir/pyproject.toml` | version 0.8.0 |
| `symbolic-vm/src/symbolic_vm/handlers.py` | +3 handler functions + 3 registrations |
| `symbolic-vm/src/symbolic_vm/integrate.py` | imports, Phase 3 head set, bare dispatch, helpers, diff rules |
| `symbolic-vm/tests/test_phase15.py` | **NEW** ≥36 tests |
| `symbolic-vm/CHANGELOG.md` | 0.35.0 entry |
| `symbolic-vm/pyproject.toml` | version 0.35.0, symbolic-ir≥0.8.0 |
| `macsyma-compiler/src/macsyma_compiler/compiler.py` | +3 entries in `_STANDARD_FUNCTIONS` |
| `macsyma-compiler/CHANGELOG.md` | 0.8.0 entry |
| `macsyma-compiler/pyproject.toml` | version 0.8.0, symbolic-ir≥0.8.0 |

---

## Test Coverage

`test_phase15.py` (≥36 tests):

| Class | Tests | What is verified |
|-------|-------|-----------------|
| `TestPhase15_HandlerEval` | 6 | Numeric evaluation; `sech(0) = 1`; unevaluated in symbolic mode |
| `TestPhase15_Differentiation` | 9 | d/dx for each function; chain rule; confirm IR structure |
| `TestPhase15_CothIntegral` | 5 | ∫coth(ax+b) — numerical antiderivative check |
| `TestPhase15_SechIntegral` | 5 | ∫sech(ax+b) — numerical antiderivative check |
| `TestPhase15_CschIntegral` | 5 | ∫csch(ax+b) — numerical antiderivative check |
| `TestPhase15_Fallthrough` | 3 | poly×coth, coth(x²), sech²(x) all unevaluated |
| `TestPhase15_Regressions` | 3 | Phases 14/13/3 still pass |
| `TestPhase15_Macsyma` | 5 | End-to-end via MACSYMA string interface |
