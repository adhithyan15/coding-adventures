# Phase 20 — Limits: L'Hôpital, infinity, indeterminate forms, one-sided

## Context

Phase 1 of `cas-limit-series` implemented direct-substitution only.
Phase 20 closes the gap to full MACSYMA parity for the `limit` function
by adding L'Hôpital's rule, limits at ±∞, all seven indeterminate forms,
and one-sided limit direction support.

---

## Mathematical background

### Direct substitution

`lim_{x→a} f(x) = f(a)` when `f` is continuous at `a`. This is what Phase 1
already does. Phase 20 handles all remaining cases.

### L'Hôpital's rule

For `f/g → 0/0` or `∞/∞`:

```
lim f(x)/g(x) = lim f'(x)/g'(x)
```

Applied iteratively up to depth 8. If both derivatives still form an
indeterminate ratio, recurse. If depth exceeded, return unevaluated.

**Requires** a differentiation callable injected from the VM so that
`cas-limit-series` does not depend on `symbolic-vm`.

### Limits at ±∞

Substituting `IRSymbol("inf")` for the variable and evaluating numerically
covers most cases (the numeric evaluator maps `inf` → `math.inf`).

- `exp(inf) = inf`
- `exp(-inf) = 0`
- `log(inf) = inf`
- `1/inf = 0`
- Rational functions: ∞/∞ → L'Hôpital iteratively → correct ratio

### Indeterminate forms

| Form | Reduction |
|------|-----------|
| `0/0` | L'Hôpital on `DIV(N, D)` |
| `∞/∞` | L'Hôpital on `DIV(N, D)` |
| `0·∞` | Rewrite `MUL(a, b)` as `DIV(b, DIV(1, a))` then L'Hôpital |
| `1^∞` | Rewrite `POW(b, e)` as `EXP(MUL(e, LOG(b)))`, recurse |
| `0^0` | Same exp-log transform |
| `∞^0` | Same exp-log transform |
| `∞−∞` | Fallthrough to unevaluated (rare in practice; deferred) |

### One-sided limits

`limit(f, x, a, "plus")` — approach from the right.
`limit(f, x, a, "minus")` — approach from the left.

For `"plus"`: substitute `a + ε` (numerically, ε = 1e-300) before classifying.
For `"minus"`: substitute `a − ε` (numerically).

The substituted symbolic expression uses the original `point` (not the
perturbed one) — the perturbation is only used for numeric classification
of the form (finite / +∞ / -∞ / indeterminate).

---

## Architecture

`cas-limit-series` must not depend on `symbolic-vm`. Differentiation and
simplification are injected as callables:

```
def limit_advanced(
    expr:       IRNode,
    var:        IRSymbol,
    point:      IRNode,
    direction:  str | None = None,   # None | "plus" | "minus"
    *,
    diff_fn:    Callable[[IRNode, IRSymbol], IRNode] | None = None,
    eval_fn:    Callable[[IRNode], IRNode] | None = None,
) -> IRNode
```

The VM's `limit_handler` injects:
- `diff_fn = lambda e, v: vm.eval(_symbolic_diff(e, v))`
- `eval_fn = vm.eval`

---

## Files

| File | Change |
|------|--------|
| `code/specs/phase20-limits.md` | **NEW** this spec |
| `cas-limit-series/src/cas_limit_series/limit_advanced.py` | **NEW** |
| `cas-limit-series/src/cas_limit_series/heads.py` | Add `INF`, `MINF` |
| `cas-limit-series/src/cas_limit_series/__init__.py` | Export `limit_advanced` |
| `cas-limit-series/tests/test_phase20.py` | **NEW** ≥40 tests |
| `cas-limit-series/CHANGELOG.md` | 0.2.0 entry |
| `cas-limit-series/pyproject.toml` | Bump to 0.2.0 |
| `symbolic-vm/src/symbolic_vm/cas_handlers.py` | Upgrade `limit_handler` |
| `symbolic-vm/CHANGELOG.md` | 0.40.0 entry |
| `symbolic-vm/pyproject.toml` | Bump to 0.40.0, `>=0.2.0` |

No new IR heads in `symbolic-ir` — `inf` / `minf` are just existing symbols.

---

## Numeric evaluator

`_num_eval(node) -> float` converts an IR node to a float. Key rules:

- `IRSymbol("inf")` → `math.inf`
- `IRSymbol("minf")` → `-math.inf`
- `IRApply(EXP, (arg,))` → Python `math.exp` (handles ±∞ safely)
- `IRApply(LOG, (arg,))` → `math.log`; returns `-inf` for arg=0, `nan` for negative
- `IRApply(DIV, (n, 0))` → `nan` if n=0 else `±inf`
- `IRApply(SIN/COS, (inf,))` → `nan` (oscillates, no limit)
- All arithmetic heads: standard float arithmetic with Python's overflow rules

---

## Test coverage targets

| Class | Tests | What's covered |
|-------|-------|----------------|
| `TestDirectSub` | 4 | Polynomial, trig, exp at finite continuous points |
| `TestLHopital_ZeroZero` | 8 | sin(x)/x, (1-cos)/x, (exp(x)-1)/x, polynomial ratios |
| `TestLHopital_InfInf` | 6 | Rational functions at ∞; repeated L'Hôpital |
| `TestIndeterminate_ZeroInf` | 5 | x·log(x), x·exp(-x), x^2·exp(-x) |
| `TestIndeterminate_Powers` | 6 | (1+1/x)^x, x^x at 0+, x^(1/x) at ∞ |
| `TestLimitsAtInfinity` | 5 | exp(-x), 1/x, log(x)/x, sin(x)/x (unevaluated) |
| `TestOneSided` | 5 | log(x) at 0+, 1/x at 0+/0-, sqrt(x) at 0+  |
| `TestFallthrough` | 4 | No diff_fn, oscillating, truly unevaluated |
| `TestMacsymaExamples` | 3 | Surface-syntax via VM |
