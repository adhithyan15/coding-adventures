## 0.27.0 — 2026-07-02 — AL-pow: the `↑` exponentiation operator (LANG-FULL)

ALGOL 60's exponentiation operator `↑` (§3.3.4; spelled `^` / `**` in our
grammar) is now lowered instead of rejected.  The `factor` / `expr_pow` node is
folded left-to-right, and each `base ↑ exp` uses one of two shapes — both
reusing IIR the code-gen backends already run, so **no new op and no backend
change**:

| exponent | lowering | result type |
|----------|----------|-------------|
| nonnegative integer literal `k` (≤ 64) | `k−1` repeated `mul`s (`x*x*…`); `x↑0 = 1`, `x↑1 = x` | **the base's type** — `integer↑k` stays `integer`, `real↑k` stays `real` |
| `real` exponent (with a `real` base) | the `f64_pow` op (libm `pow`) — the same op BASIC's BA-pow proved on all 7 backends | `real` |

The integer-literal path keeps ALGOL's typing: `2 ↑ 10` is the *integer* 1024
(BASIC always widens to `real`).  A non-literal exponent on an `integer` base, or
a negative literal, is a clean `Unsupported` — those need int→real coercion or
reciprocals not in this slice.

New:
- `emit_pow` / `emit_power_step` / `emit_pow_unroll` methods.
- `literal_nonneg_integer_exponent` helper + `MAX_POW_UNROLL_EXPONENT` (64).
- 9 new unit tests (121 total): integer power, square, `↑0`/`↑1`, precedence vs
  `*`, real-base integer-literal exponent, `real↑real` via pow, and the
  `integer↑real` rejection.

The 7-backend proof (`10 + 2 ^ 5` = 42, integer unroll) lives in `lang-aot`
`lang_matrix.rs`.

