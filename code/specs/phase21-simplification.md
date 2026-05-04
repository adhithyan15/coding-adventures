# Phase 21 — Simplification Suite: `radcan`, `logcontract`, Assumptions

**Status**: Implemented.
**Packages affected**: `symbolic-ir` 0.9.0, `cas-simplify` 0.3.0,
`symbolic-vm` 0.41.0.

---

## Overview

This phase adds three families of simplification that MACSYMA users reach
for constantly:

1. **Assumption framework** — `assume(x > 0)`, `forget()`, `is()`, `sign()`
2. **Radical canonicalization** — `radcan()` merges and simplifies radical
   expressions involving `√`, `^(p/q)`, `exp/log`.
3. **Log contraction / expansion** — `logcontract()` collapses sums of logs
   into a single log; `logexpand()` distributes log over products / powers.
4. **Exponentialize / DeMoivre** — trig/hyp to complex exponential form and
   back.

---

## New IR Heads (`symbolic-ir` 0.9.0)

Nine new `IRSymbol` singletons added to `nodes.py` and exported from
`__init__.py`:

| Constant      | Head string        | Meaning                              |
|---------------|--------------------|--------------------------------------|
| `ASSUME`      | `"Assume"`         | Record a symbol assumption           |
| `FORGET`      | `"Forget"`         | Remove assumption(s)                 |
| `IS`          | `"Is"`             | Query an assumption                  |
| `SIGN`        | `"Sign"`           | Sign function (+1 / 0 / −1)          |
| `RADCAN`      | `"Radcan"`         | Radical canonicalization             |
| `LOGCONTRACT` | `"LogContract"`    | Combine log sums into single log     |
| `LOGEXPAND`   | `"LogExpand"`      | Expand single log into sum           |
| `EXPONENTIALIZE` | `"Exponentialize"` | Trig/hyp → exp form              |
| `DEMOIVRE`    | `"DeMoivre"`       | exp(a+bi) → exp(a)·(cos b + i·sin b) |

---

## `cas-simplify` 0.3.0 — New Modules

### `assumptions.py` — `AssumptionContext`

A mutable per-VM object that records per-symbol property facts:

| Property    | Meaning        | Set by                                      |
|-------------|----------------|---------------------------------------------|
| `positive`  | x > 0          | `assume(x > 0)` / `assume(x, positive)`    |
| `negative`  | x < 0          | `assume(x < 0)` / `assume(x, negative)`    |
| `zero`      | x = 0          | `assume(x = 0)`                             |
| `nonzero`   | x ≠ 0          | `assume(x ≠ 0)` / `assume(x, nonzero)`     |
| `nonneg`    | x ≥ 0          | `assume(x ≥ 0)` / `assume(x, nonneg)`      |
| `nonpos`    | x ≤ 0          | `assume(x ≤ 0)` / `assume(x, nonpos)`      |
| `integer`   | x ∈ ℤ          | `assume(x, integer)`                        |

Query methods: `is_positive(name)`, `is_negative(name)`, `sign_of(name)`,
`is_integer(name)`, `is_true_relation(expr)`.

### `radcan.py` — `radcan(expr, ctx=None)`

Radical canonicalization rules (bottom-up tree walk):

1. `√a · √b = √(ab)` — merge Sqrt args in Mul
2. `√(x² · b) = x · √b` when x > 0 (context-aware) or x is a positive integer
3. `a^(p/q) · b^(p/q) = (ab)^(p/q)` — collect identical rational exponents
4. `Pow(Sqrt(x), 2) = x` — cancel square of square root
5. `Exp(Log(x)) = x`, `Log(Exp(x)) = x` — cancel exp/log pairs

### `logcontract.py` — `logcontract(expr)` and `logexpand(expr, ctx=None)`

**logcontract** (bottom-up):
1. `Log(a) + Log(b) → Log(a·b)`
2. `n·Log(a) → Log(aⁿ)` for rational n
3. `Log(a) − Log(b) → Log(a/b)`

**logexpand** (bottom-up):
1. `Log(a^n) → n·Log(a)` for rational/integer n
2. `Log(a·b) → Log(a) + Log(b)`
3. `Log(a/b) → Log(a) − Log(b)`

### `exponentialize.py` — `exponentialize(expr)` and `demoivre(expr)`

**exponentialize** converts trig/hyp → complex exponential:

```
sin(x)  → (exp(i·x) − exp(−i·x)) / (2·i)
cos(x)  → (exp(i·x) + exp(−i·x)) / 2
tan(x)  → −i · (exp(i·x) − exp(−i·x)) / (exp(i·x) + exp(−i·x))
sinh(x) → (exp(x) − exp(−x)) / 2
cosh(x) → (exp(x) + exp(−x)) / 2
tanh(x) → (exp(x) − exp(−x)) / (exp(x) + exp(−x))
```

**demoivre** converts complex exponentials → trig:

```
exp(a + b·i) → exp(a) · (cos(b) + i·sin(b))
exp(b·i)     → cos(b) + i·sin(b)
```

---

## `symbolic-vm` 0.41.0 — Changes

### `vm.py`

`VM.__init__` gains `self.assumptions = AssumptionContext()`. This is the
per-session store shared between all handlers.

### `cas_handlers.py` — New Handlers

| Handler              | IR Head        | Action                                 |
|----------------------|----------------|----------------------------------------|
| `assume_handler`     | `Assume`       | Parse & record relation/property       |
| `forget_handler`     | `Forget`       | Remove fact(s); no args = forget all   |
| `is_handler`         | `Is`           | Query → `true` / `false` / `unknown`   |
| `sign_handler`       | `Sign`         | → 1 / -1 / 0 / unevaluated            |
| `radcan_handler`     | `Radcan`       | Call `radcan(arg, ctx=vm.assumptions)` |
| `logcontract_handler`| `LogContract`  | Call `logcontract(arg)`                |
| `logexpand_handler`  | `LogExpand`    | Call `logexpand(arg, ctx=vm.assumptions)` |
| `exponentialize_handler` | `Exponentialize` | Call `exponentialize(arg)`      |
| `demoivre_handler`   | `DeMoivre`     | Call `demoivre(arg)`                   |

---

## MACSYMA Surface Syntax

The macsyma-compiler already maps function-call notation to the correct heads;
no compiler changes are needed. Examples:

```macsyma
assume(x > 0);                       /* Assume(Greater(x, 0)) */
radcan(sqrt(x^2 * y));               /* x * sqrt(y) */
logcontract(log(a) + log(b));        /* log(a*b) */
logexpand(log(x^3));                 /* 3*log(x) */
exponentialize(sin(x));              /* (exp(i*x) - exp(-i*x)) / (2*i) */
demoivre(exp(x + %i*y));             /* exp(x) * (cos(y) + i*sin(y)) */
is(x > 0);                           /* true */
sign(x);                             /* 1 */
forget(x > 0);
```

---

## Testing

`cas-simplify/tests/test_phase21.py` — 60+ tests across 6 classes:

| Class                     | Count | What is tested                          |
|---------------------------|-------|-----------------------------------------|
| `TestAssumptionContext`   | 10    | assume/forget/is_positive/sign_of       |
| `TestRadcan`              | 10    | sqrt merging, perfect-square extraction |
| `TestLogcontract`         | 10    | log sum/product/difference contraction  |
| `TestLogexpand`           | 10    | log product/power/quotient expansion    |
| `TestExponentialize`      | 10    | sin/cos/sinh/cosh/tanh to exp           |
| `TestDeMoivre`            | 10    | exp(a+bi) decomposition                 |

---

## Deferred

- Denesting: `√(a + b·√c)` → nested radical simplification. Complex
  algorithm; deferred to Phase 23 or later.
- Context-sensitive `logexpand(log(a*b)) → log(a)+log(b)` gated on
  positivity — always expanded in this phase for simplicity.
- `assume(n, integer)` does not yet affect `simplify` or `integrate` paths
  (integration branch selection deferred to Phase 23).
