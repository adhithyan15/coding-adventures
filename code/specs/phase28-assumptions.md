# Phase 28 — Assumptions Framework (assume / forget / is)

## Context

Phase 27 (PR #2110) added polynomial inequality solving: `solve(x^2 - 1 > 0, x)`
returns the union of open intervals.  That gives us inequalities as *queries*, but
MACSYMA also has a complementary feature — storing sign information about variables
so that simplification can use it.

Historical MACSYMA's three-function **assumptions system**:

| MACSYMA name | Description |
|---|---|
| `assume(pred)` | Add `pred` to the session's assumption database; return `pred` |
| `forget(pred)` | Remove `pred` from the database; return `pred` |
| `is(pred)` | Return `true`, `false`, or `unknown` by querying the database |

Combined with sign-aware simplification this gives the most common use-case:

```
(%i1) assume(x > 0);
(%o1)                               x > 0
(%i2) abs(x);
(%o2)                                 x
(%i3) forget(x > 0);
(%o3)                               x > 0
(%i4) abs(x);
(%o4)                              abs(x)
```

---

## Scope

Phase 28 implements:

1. **`AssumptionDB`** — a new lightweight class in a new `cas-assumptions` package.
   Stores per-variable sign predicates and answers inference queries.

2. **Handlers** (`Assume`, `Forget`, `Is`) inside `macsyma-runtime`.
   The heads already exist in `macsyma_runtime.heads`.  This phase adds the
   handler implementations and wires them into `MacsymaBackend`.

3. **`abs` simplification** in `symbolic-vm`: when the argument is a bare symbol
   with a known sign, fold to `x` or `-x` rather than leaving unevaluated.

4. **`sign` function** (`Sign(x)`): new head + handler in `symbolic-vm` that returns
   `1`, `-1`, or `0` for numeric inputs, and consults the DB for symbolic inputs.

5. **MACSYMA surface syntax**: `assume`, `forget`, `is`, `sign` names added to
   `macsyma-runtime`'s name table.

---

## Predicates supported

Phase 28 supports **single-variable sign predicates** only:

| Predicate | Example | MACSYMA | IR form |
|---|---|---|---|
| strictly positive | `x > 0` | `assume(x > 0)` | `IRApply(GREATER, (IRSymbol("x"), IRInteger(0)))` |
| strictly negative | `x < 0` | `assume(x < 0)` | `IRApply(LESS, (IRSymbol("x"), IRInteger(0)))` |
| non-negative | `x >= 0` | `assume(x >= 0)` | `IRApply(GREATER_EQUAL, (IRSymbol("x"), IRInteger(0)))` |
| non-positive | `x <= 0` | `assume(x <= 0)` | `IRApply(LESS_EQUAL, (IRSymbol("x"), IRInteger(0)))` |
| non-zero | `x # 0` | `assume(x # 0)` | `IRApply(NOT_EQUAL, (IRSymbol("x"), IRInteger(0)))` (NotEqual head) |

The right-hand side must be the integer literal `0`.  `assume(x > 1)` stores the
fact but `is(x > 0)` will return `unknown` (sub-range inference is deferred).

---

## Inference rules

`AssumptionDB` applies the following chain of entailments when answering `is`:

```
x > 0   ⟹  x >= 0, x != 0, is(x > 0)=true, is(x >= 0)=true, is(x != 0)=true
x < 0   ⟹  x <= 0, x != 0, is(x < 0)=true, is(x <= 0)=true, is(x != 0)=true
x >= 0  ⟹  is(x >= 0)=true
x <= 0  ⟹  is(x <= 0)=true
x != 0  ⟹  is(x != 0)=true
```

Contradiction detection: if both `x > 0` and `x < 0` are stored, `is(x > 0)` still
returns `true` (last-write wins; Phase 28 does not model contradictions).

`is(x = 0)` returns `false` when `x != 0`, `x > 0`, or `x < 0` is known.

---

## `AssumptionDB` API

```python
class AssumptionDB:
    """Session-scoped sign-predicate assumption database.

    Thread-safety: single-threaded session use only.
    """

    def assume(self, pred: IRNode) -> bool:
        """Add predicate.  Returns True if it was stored, False if unrecognised."""

    def forget(self, pred: IRNode) -> bool:
        """Remove predicate.  Returns True if it was present, False otherwise."""

    def is_satisfied(self, pred: IRNode) -> bool | None:
        """Return True/False/None (unknown)."""

    def is_positive(self, var: str) -> bool:
        """True iff x > 0 was assumed."""

    def is_negative(self, var: str) -> bool:
        """True iff x < 0 was assumed."""

    def is_nonneg(self, var: str) -> bool:
        """True iff x >= 0 or x > 0 was assumed."""

    def is_nonpos(self, var: str) -> bool:
        """True iff x <= 0 or x < 0 was assumed."""

    def is_nonzero(self, var: str) -> bool:
        """True iff x != 0 or x > 0 or x < 0 was assumed."""

    def is_zero(self, var: str) -> bool:
        """True iff x = 0 was assumed."""

    def reset(self) -> None:
        """Clear all assumptions (used by kill(all))."""

    def facts(self) -> list[IRNode]:
        """Return all stored predicates as a list."""
```

---

## Handler behaviour

### `Assume`

```
assume(x > 0)  →  stores x > 0 in DB, returns the predicate IRNode
assume(x)      →  raises ValueError (not a predicate)
assume(x > 0, y < 0)  →  stores both, returns List(x>0, y<0)
```

### `Forget`

```
forget(x > 0)  →  removes from DB, returns the predicate IRNode
forget(x > 0, y < 0)  →  removes both, returns List(x>0, y<0)
```

### `Is`

```
is(x > 0)  →  IRSymbol("true") / IRSymbol("false") / IRSymbol("unknown")
```

---

## `Sign` function

New `Sign(x)` head registered in `symbolic-vm`.  MACSYMA surface name: `sign`.

| Input | Output |
|---|---|
| `Sign(3)` | `1` (IRInteger(1)) |
| `Sign(-5)` | `-1` (IRInteger(-1)) |
| `Sign(0)` | `0` (IRInteger(0)) |
| `Sign(3/4)` | `1` |
| `Sign(-0.5)` | `-1` (IRInteger(-1)) |
| `Sign(x)` with `x > 0` assumed | `1` |
| `Sign(x)` with `x < 0` assumed | `-1` |
| `Sign(x)` symbolic, no assumption | `IRApply(Sign, (x,))` unevaluated |

---

## `Abs` simplification

The existing `abs_handler` in `symbolic-vm` already folds numeric inputs.
Phase 28 extends it:

```python
# If arg is a bare symbol with known sign, fold.
db = getattr(vm.backend, 'assumption_db', None)
if db is not None and isinstance(inner, IRSymbol):
    if db.is_nonneg(inner.name):    # x >= 0 or x > 0
        return inner
    if db.is_nonpos(inner.name):    # x <= 0 or x < 0
        return IRApply(NEG, (inner,))
```

---

## Files to change

All paths under `code/packages/python/`.

| File | Change |
|---|---|
| `code/specs/phase28-assumptions.md` | **THIS FILE** |
| `cas-assumptions/pyproject.toml` | **NEW** v0.1.0 |
| `cas-assumptions/CHANGELOG.md` | **NEW** |
| `cas-assumptions/README.md` | **NEW** |
| `cas-assumptions/src/cas_assumptions/__init__.py` | **NEW** |
| `cas-assumptions/src/cas_assumptions/db.py` | **NEW** AssumptionDB |
| `cas-assumptions/tests/test_db.py` | **NEW** ≥20 unit tests |
| `macsyma-runtime/src/macsyma_runtime/assumptions.py` | **NEW** handlers |
| `macsyma-runtime/src/macsyma_runtime/backend.py` | add `assumption_db` attr |
| `macsyma-runtime/src/macsyma_runtime/name_table.py` | add assume/forget/is/sign |
| `macsyma-runtime/src/macsyma_runtime/handlers.py` | wire kill(all) → db.reset() |
| `macsyma-runtime/pyproject.toml` | bump 1.18.0 → 1.19.0 |
| `macsyma-runtime/CHANGELOG.md` | 1.19.0 entry |
| `symbolic-vm/src/symbolic_vm/cas_handlers.py` | extend abs_handler + sign_handler |
| `symbolic-vm/pyproject.toml` | bump 0.47.0 → 0.48.0 |
| `symbolic-vm/CHANGELOG.md` | 0.48.0 entry |
| `symbolic-vm/tests/test_phase28.py` | **NEW** ≥24 tests |

---

## Test plan

### `cas-assumptions/tests/test_db.py` (≥20 tests)

| Class | Tests |
|---|---|
| `TestAssumptionDBBasic` | assume stores, forget removes, double-assume idempotent, forget-absent no-op |
| `TestSignQueries` | is_positive/negative/nonneg/nonpos/nonzero for each predicate type |
| `TestInference` | `x > 0` implies `is_nonneg`, `is_nonzero`; `x < 0` implies `is_nonpos` |
| `TestIsSatisfied` | True/False/None for all combinations |
| `TestReset` | reset() clears all facts |
| `TestFacts` | facts() returns stored predicates |

### `symbolic-vm/tests/test_phase28.py` (≥24 tests)

| Class | Tests |
|---|---|
| `TestPhase28_Sign` | Sign(int), Sign(rat), Sign(float), Sign(sym) unevaluated |
| `TestPhase28_AbsAssumptions` | abs(x) with pos/neg/nonneg/nonpos assumed |
| `TestPhase28_AbsFallthrough` | abs(x) no assumption still unevaluated |
| `TestPhase28_AssumeForgetIs` | full round-trip via MACSYMA surface syntax |
| `TestPhase28_Regressions` | Phase 27 inequality unchanged, Phase 23 abs numeric, Phase 3 |

---

## Surface syntax examples

```macsyma
(%i1) assume(x > 0);
(%o1)                               x > 0
(%i2) is(x > 0);
(%o2)                               true
(%i3) is(x >= 0);
(%o3)                               true
(%i4) abs(x);
(%o4)                                 x
(%i5) sign(x);
(%o5)                                 1
(%i6) forget(x > 0);
(%o6)                               x > 0
(%i7) is(x > 0);
(%o7)                             unknown
(%i8) abs(x);
(%o8)                             abs(x)
(%i9) assume(x < 0);
(%o9)                               x < 0
(%i10) abs(x);
(%o10)                               -x
(%i11) sign(x);
(%o11)                               -1
```
