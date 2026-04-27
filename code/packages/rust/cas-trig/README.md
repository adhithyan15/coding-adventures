# cas-trig (Rust)

Symbolic trigonometry operations over the shared CAS IR: exact special
values, numeric evaluation, angle-addition expansion, and power reduction.

## Operations

| Function | Description |
|---|---|
| `sin_eval(arg)` | Evaluate `sin(arg)` — exact or numeric |
| `cos_eval(arg)` | Evaluate `cos(arg)` — exact or numeric |
| `tan_eval(arg)` | Evaluate `tan(arg)` — exact, unevaluated at poles |
| `atan_eval(arg)` | Evaluate `atan(arg)` — numeric |
| `asin_eval(arg)` | Evaluate `asin(arg)` — numeric, unevaluated out-of-domain |
| `acos_eval(arg)` | Evaluate `acos(arg)` — numeric, unevaluated out-of-domain |
| `trig_simplify(expr)` | Walk a tree and evaluate all trig nodes |
| `expand_trig(expr)` | Expand `sin/cos(a±b)` via angle-addition formulas |
| `power_reduce(expr)` | Reduce `sin²/cos²` to half-angle forms |

## Special-value table

Exact values are returned for all rational multiples `n/d · π` where
`d ∈ {1, 2, 3, 4, 6}` after reduction modulo 2π:

| angle | sin | cos | tan |
|-------|-----|-----|-----|
| 0 | 0 | 1 | 0 |
| π/6 (30°) | 1/2 | √3/2 | √3/3 |
| π/4 (45°) | √2/2 | √2/2 | 1 |
| π/3 (60°) | √3/2 | 1/2 | √3 |
| π/2 (90°) | 1 | 0 | ∞ (unevaluated) |
| π (180°) | 0 | −1 | 0 |
| … | (symmetry for [π, 2π)) | | |

Values like `√2/2` are represented exactly in IR as `Mul(Rational(1,2), Sqrt(2))`.

## Evaluation strategy

Each `*_eval` function applies three tiers in order:

1. **Special value**: recognised rational multiple of π → exact `Integer`,
   `Rational`, or `Sqrt(…)` node.
2. **Numeric**: `Integer`, `Float`, `Rational`, or `Symbol("Pi")` argument →
   `Float` result with near-integer snapping.
3. **Unevaluated**: symbolic argument → `Sin(arg)` / `Cos(arg)` etc.

## Usage

```rust
use cas_trig::{sin_eval, cos_eval, tan_eval, expand_trig, power_reduce, PI};
use symbolic_ir::{apply, int, rat, sym, SIN, POW, MUL};

// sin(π/6) = 1/2  (exact)
let pi_6 = apply(sym(MUL), vec![rat(1, 6), sym(PI)]);
assert_eq!(sin_eval(&pi_6), rat(1, 2));

// cos(π) = -1  (exact)
assert_eq!(cos_eval(&sym(PI)), int(-1));

// sin(x + y) → angle-addition expansion
let expr = apply(sym(SIN), vec![apply(sym("Add"), vec![sym("x"), sym("y")])]);
let _expanded = expand_trig(&expr);

// sin²(x) → half-angle form
let sin_sq = apply(sym(POW), vec![apply(sym(SIN), vec![sym("x")]), int(2)]);
let _reduced = power_reduce(&sin_sq);
```

## Stack position

```
symbolic-ir  ←  cas-trig
```
