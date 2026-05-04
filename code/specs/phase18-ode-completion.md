# Phase 18 — ODE Completion (Bernoulli · Exact · Non-homogeneous 2nd-order)

**Package**: `cas-ode` → 0.2.0  
**VM**: `symbolic-vm` → 0.38.0  
**Branch**: `claude/phase18-ode-completion`

---

## Background

`cas-ode` 0.1.0 (Phase ODE-1) shipped four ODE classes:

| Class | Implemented |
|-------|-------------|
| First-order linear `y' + P(x)·y = Q(x)` | ✓ 0.1.0 |
| Separable `y' = f(x)·g(y)` | ✓ 0.1.0 |
| Second-order const-coeff homogeneous | ✓ 0.1.0 |
| Bernoulli `y' + P·y = Q·y^n` | ✗ deferred |

Historical MACSYMA's `ode2` handled roughly twelve classes.  Phase 18 adds
the three most practical missing ones, each in a single `ode.py` extension.

---

## 18a — Bernoulli ODE

```
y' + P(x)·y = Q(x)·y^n      (n ≠ 0, n ≠ 1)
```

**Substitution**: `v = y^(1-n)` → `v' + (1-n)·P(x)·v = (1-n)·Q(x)` (linear in v).

**Algorithm**:
1. Scan terms: find `D(y,x)` (coeff 1), `y^n` terms (collect into Q), `y`
   terms (collect into P).
2. Build reduced linear ODE with `new_p = (1-n)·P`, `new_q = (1-n)·Q`.
3. Call existing `solve_linear_first_order(new_p, new_q, y, x, vm)` — it
   uses y as the variable name for v (valid since P,Q are x-only).
4. Extract v_sol from `Equal(y, v_sol)`; return `Equal(y, v_sol^(1/(1-n)))`.

**MACSYMA example**:
```
ode2(y' + x*y = x*y^3, y, x)
→ Equal(y, (x^2/2·exp(x^2) + %c·exp(x^2))^(-1/2))
```

---

## 18b — Exact ODE

```
M(x,y)·dx + N(x,y)·dy = 0    where  ∂M/∂y = ∂N/∂x
```

Arrives in zero form as `M + N·D(y,x) = 0`.

**Algorithm**:
1. Extract M (y'-free terms) and N (coefficient of `D(y,x)`).
2. Exactness: `vm.eval(D(M, y)) == vm.eval(D(N, x))`.
3. Potential: `F = vm.eval(Integrate(M, x))`.
4. `g'(y) = N − ∂F/∂y`; `g = vm.eval(Integrate(g'(y), y))`.
5. Return `Equal(F + g, C_CONST)` — implicit solution.

Supported: polynomial M, N in x and y.

**MACSYMA example**:
```
ode2(2*x*y + (x^2 - y^2)*'diff(y,x) = 0, y, x)
→ Equal(x^2*y - y^3/3, %c)
```

---

## 18c — 2nd-order non-homogeneous (constant coefficients)

```
a·y'' + b·y' + c·y = f(x)
```

**Forcing families** handled by undetermined coefficients:

| Forcing f(x) | Particular ansatz |
|-------------|------------------|
| constant k | A (or x^s·A, resonance) |
| polynomial Σcₖxᵏ (degree ≤ 2) | polynomial of same degree × x^s |
| e^(αx) | A·e^(αx) × x^s |
| sin(βx) | A·cos(βx) + B·sin(βx) |
| cos(βx) | A·cos(βx) + B·sin(βx) |
| e^(αx)·sin(βx) | e^(αx)·(A·cos + B·sin) |
| e^(αx)·cos(βx) | e^(αx)·(A·cos + B·sin) |

s ∈ {0,1,2} for resonance (α = root of characteristic polynomial).

**Algorithm**:
1. `_collect_second_order_nonhom` — extend coefficient collector to also
   capture x-only forcing terms f(x).
2. `_classify_forcing(f, x)` — return one of the tags above.
3. `_compute_particular(a, b, c, forcing, x)` — solve for the undetermined
   coefficients by substituting the ansatz.
4. Homogeneous solution from existing `solve_second_order_const_coeff`.
5. General solution: `y_h + y_p`.

**MACSYMA examples**:
```
ode2('diff(y,x,2) + 3*'diff(y,x) + 2*y = sin(x), y, x)
ode2('diff(y,x,2) - y = exp(2*x), y, x)
ode2('diff(y,x,2) + y = x, y, x)
```

---

## Implementation plan

### Files changed

| File | Change |
|------|--------|
| `cas-ode/src/cas_ode/ode.py` | Add Sections 8–10 + dispatcher update |
| `cas-ode/tests/test_phase18.py` | New — ≥45 tests |
| `cas-ode/CHANGELOG.md` | 0.2.0 entry |
| `cas-ode/pyproject.toml` | Bump to 0.2.0 |
| `symbolic-vm/CHANGELOG.md` | 0.38.0 entry (cas-ode dep bump) |
| `symbolic-vm/pyproject.toml` | Bump to 0.38.0 |

### Dispatcher order in `solve_ode`

```
1. _try_second_order_nonhom  (new — before homogeneous)
2. _collect_second_order_coeffs → solve_second_order_const_coeff
3. _try_bernoulli              (new)
4. _try_exact                  (new)
5. _collect_linear_first_order → solve_linear_first_order
6. _try_separable
```

Non-homogeneous must be checked before homogeneous, because the same
coefficient collector (`_collect_second_order_coeffs`) silently drops the
forcing term — the new `_collect_second_order_nonhom` is checked first.

---

## Resonance details for `_compute_particular`

For forcing `e^(αx)`, the characteristic polynomial is `p(r) = a·r² + b·r + c`.

| p(α) | p'(α) = 2aα+b | Particular y_p |
|------|--------------|----------------|
| ≠ 0 | — | `e^(αx)/p(α)` |
| = 0 | ≠ 0 | `x·e^(αx)/p'(α)` |
| = 0 | = 0 | `x²·e^(αx)/(2a)` |

For trig forcing `sin(βx)` (similarly `cos`):
```
det = (c − a·β²)² + (b·β)²
A = (c − a·β²)/det,  B = (b·β)/det   [for cos forcing]
A = −(b·β)/det,      B = (c − a·β²)/det   [for sin forcing]
```
If det = 0 (resonance), fall through to unevaluated (x·trig ansatz not yet
implemented).

---

## Test matrix

| Class | Tests | Validates |
|-------|-------|-----------|
| `TestPhase18_Bernoulli` | 10 | n=2,3; P=x,const; resonance; fallthrough |
| `TestPhase18_Exact` | 10 | polynomial M/N; not-exact fallthrough; implicit form |
| `TestPhase18_NonHom` | 12 | const/exp/sin/cos/exp-trig/poly forcing; resonance exp |
| `TestPhase18_Fallthrough` | 7 | non-const coeff 2nd order, Bernoulli n=1, non-exact |
| `TestPhase18_Regressions` | 7 | all four Phase 0.1.0 solver types still work |

Total: ≥ 46 new tests.
