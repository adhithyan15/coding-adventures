# simple-fold-math-trunc-sign

Locks the byte output of SIMPLE-level static `Math.trunc(n)` / `Math.sign(n)`
folding (`closure-pass-constant-fold`). When the single argument is a numeric
literal the call collapses to the result as a numeric literal, verified
byte-identical to the reference Closure Compiler
(`closure-compiler-v20260712.jar`, SIMPLE, `--language_out NO_TRANSPILE`):

- `Math.trunc(4.9)` -> `4`, `Math.trunc(-4.9)` -> `-4` (round toward zero)
- `Math.sign(7)` -> `1`, `Math.sign(-3)` -> `-1`, `Math.sign(0)` -> `0`

Declined (left intact), matching the reference:

- `Math.trunc(x)` — non-literal argument
- `Math.sqrt(16)` — the reference does not fold transcendental `Math` methods
  even when the result is exact
- `m.trunc(1.5)` — only the bare global `Math` folds
- a `-0` result (`Math.trunc(-0.5)` === -0) — declined for lack of a faithful
  `-0` literal spelling, the same policy as `Math.ceil(-0.5)` (unit-tested)

See `input/a.js` for the source and `expected.stdout` for the locked output.
