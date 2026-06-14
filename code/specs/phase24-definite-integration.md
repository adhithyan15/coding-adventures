# Phase 24 — Definite Integration

> **Status**: Implementation complete.
> **PR**: TBD
>
> **Versions**: `symbolic-ir` 0.11.0 (unchanged), `symbolic-vm` 0.44.0,
> `macsyma-runtime` 1.15.0

---

## Motivation

Phases 1–23 implement *indefinite* symbolic integration: `integrate(f, x)`
returns an antiderivative `F(x)` such that `D(F, x) = f`.  The MACSYMA
surface form `integrate(f, x, a, b)` was wired to produce a 4-argument
`Integrate(f, x, a, b)` IR node, but the VM raised `TypeError` on it.

Phase 24 closes that gap by applying the **Fundamental Theorem of
Calculus**:

```
∫_a^b f(x) dx  =  F(b) − F(a)
```

where `F` is any antiderivative of `f`.  When the limits involve `%inf`
or `%minf`, a table of known one-sided limits for the special functions
introduced in Phase 23 is used instead of direct substitution.

---

## Scope

| Feature | Handled |
|---------|---------|
| Finite lower and upper limits | ✓ |
| Semi-infinite intervals `[a, ∞)` and `(-∞, b]` | ✓ |
| Fully-infinite interval `(-∞, ∞)` | ✓ |
| Limits that simplify to known constants (e.g. `erf(1)`) | ✓ — symbolic |
| Divergent integrals (e.g. `∫₀^∞ exp(x) dx`) | Returns unevaluated |
| Integrands with no antiderivative in the VM | Returns unevaluated |

**No new IR heads are introduced.**  The 4-argument `Integrate(f, x, a, b)`
node already exists in `symbolic-ir`.

---

## Algorithm

### Step 1 — Compute the indefinite integral

`F = symbolic_integrate(f, x)` using the full Phases 1–23 machinery (rational
route, Phase 1 table, Phase 23 special-function fallback).  If `F` comes
back as an unevaluated `Integrate(f, x)` node, the definite integral is
also left unevaluated.

### Step 2 — Evaluate F at each limit

For a **finite** limit `a`:

```
F_a = vm.eval( subst(a, x, F) )
```

where `subst(value, var, expr)` is the structural substitution from
`cas_substitution`.

For an **infinite** limit (`IRSymbol("%inf")` or `IRSymbol("%minf")`, also
`IRSymbol("inf")` / `IRSymbol("minf")` for internal use):

```
F_inf = _eval_at_inf(F, x, sign=+1)   # for +∞
F_minf = _eval_at_inf(F, x, sign=-1)  # for −∞
```

If `_eval_at_inf` returns `None` (diverges or unknown), the definite
integral is left unevaluated.

### Step 3 — Return F(b) − F(a)

```python
vm.eval( IRApply(SUB, (F_b, F_a)) )
```

---

## Limit table (`_eval_at_inf`)

`_eval_at_inf(expr, x, sign)` recursively computes `lim_{x → sign·∞} expr`.

### Composite rules

| Form | Condition | Result |
|------|-----------|--------|
| `c` constant | `_is_const_wrt(c, x)` | `c` |
| `ADD(f, g)` | both finite at ±∞ | `ADD(lim_f, lim_g)` |
| `SUB(f, g)` | both finite at ±∞ | `SUB(lim_f, lim_g)` |
| `MUL(c, f)` | `c` constant, `f` has finite lim | `MUL(c, lim_f)` |
| `NEG(f)` | `f` has finite lim | `NEG(lim_f)` |
| `DIV(f, g)` | both finite, `lim_g ≠ 0` | `DIV(lim_f, lim_g)` |

### One-sided limits for special functions

Let `arg_sign` be the sign of the argument as `x → sign·∞` (computed by
`_arg_sgn_at_inf`):

| Function | `arg_sign = +1` | `arg_sign = −1` |
|----------|-----------------|-----------------|
| `erf(u)` | `1` | `−1` |
| `erfc(u)` | `0` | `2` |
| `erfi(u)` | diverges → `None` | diverges → `None` |
| `Si(u)` | `π/2` | `−π/2` |
| `Ci(u)` | `0` | `None` (oscillates) |
| `Shi(u)` | `None` (diverges) | `None` |
| `Chi(u)` | `None` (diverges) | `None` |
| `atan(u)` | `π/2` | `−π/2` |
| `tanh(u)` | `1` | `−1` |
| `coth(u)` | `1` | `−1` |
| `sech(u)` | `0` | `0` |
| `csch(u)` | `0` | `0` |
| `FresnelS(u)` | `1/2` | `−1/2` |
| `FresnelC(u)` | `1/2` | `−1/2` |
| `exp(u)` | diverges → `None` | `0` |

### Argument-sign helper (`_arg_sgn_at_inf`)

Given `arg` and `x → sign·∞`, returns the sign (+1 or −1) that the
argument approaches, or `None` if the argument stays finite:

| `arg` form | Result |
|------------|--------|
| `x` | `sign` |
| `NEG(x)` | `−sign` |
| `MUL(c, x)` with `c > 0` | `sign` |
| `MUL(c, x)` with `c < 0` | `−sign` |
| `ADD(MUL(c,x), b)` | `sgn(c) · sign` |
| `POW(x, n)` with `n` even | `+1` (always positive) |
| `POW(x, n)` with `n` odd | `sign` |
| `NEG(POW(x, n))` | negation of the above |
| `MUL(c, POW(x, n))` | `sgn(c) · POW_sign` |
| constant (no x) | `None` (finite) |

---

## Files changed

| File | Change |
|------|--------|
| `code/specs/phase24-definite-integration.md` | **NEW** this spec |
| `symbolic-vm/src/symbolic_vm/definite_integral.py` | **NEW** |
| `symbolic-vm/src/symbolic_vm/integrate.py` | Handle 4-arg `Integrate` |
| `symbolic-vm/tests/test_phase24.py` | **NEW** ≥30 tests |
| `symbolic-vm/CHANGELOG.md` | 0.44.0 entry |
| `symbolic-vm/pyproject.toml` | Bump to 0.44.0 |
| `macsyma-runtime/CHANGELOG.md` | 1.15.0 entry |
| `macsyma-runtime/pyproject.toml` | Bump to 1.15.0 |

No new heads in `symbolic-ir`.  `macsyma-runtime/cas_handlers.py` needs no
change: the MACSYMA compiler already emits a 4-argument `Integrate` node for
`integrate(f, x, a, b)` and the VM handler now processes it.

---

## Verification examples

```python
# ∫₀¹ x² dx = 1/3
integrate(x^2, x, 0, 1)   →   1/3

# ∫₀^π sin(x) dx = 2
integrate(sin(x), x, 0, %pi)   →   2

# ∫₀^∞ exp(−x²) dx = √π/2
integrate(exp(-x^2), x, 0, %inf)   →   sqrt(%pi)/2

# ∫₋∞^∞ exp(−x²) dx = √π
integrate(exp(-x^2), x, %minf, %inf)   →   sqrt(%pi)

# ∫₀^∞ sin(x)/x dx = π/2
integrate(sin(x)/x, x, 0, %inf)   →   %pi/2

# ∫₀¹ log(x) dx = −1
integrate(log(x), x, 0, 1)   →   −1
```
