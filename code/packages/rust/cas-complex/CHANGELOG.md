# Changelog — cas-complex (Rust)

## [0.1.0] — 2026-04-27

### Added

- Initial Rust implementation of complex number operations over symbolic IR.
- `constants` module:
  - `IMAGINARY_UNIT = "I"` — canonical name for the imaginary unit symbol.
  - `RE`, `IM`, `CONJUGATE`, `ABS`, `ARG` — head names for unevaluated forms.
- `normalize` module:
  - `complex_normalize(expr)` — public entry point; returns canonical `a + b·I`.
  - `split_complex(expr)` — internal workhorse; recursively splits any expression
    into `(real_part, imag_part)` IR pair.
  - Handles: `Add`, `Sub`, `Mul`, `Neg`, `Pow(I, n)` cycling, numeric literals.
  - `i_power(n)` — exact `I^n` cycling via `n.rem_euclid(4)`.
  - `assemble(re, im)` — builds canonical form with zero-part suppression.
  - Arithmetic helpers (`add_ir`, `sub_ir`, `mul_ir`, `neg_ir`) with
    zero/one short-circuit and integer constant folding.
- `parts` module:
  - `real_part(z)` — returns `Re(z)` via `split_complex`.
  - `imag_part(z)` — returns `Im(z)` (coefficient of `I`).
  - `conjugate(z)` — negates imaginary part: `a + b·I → a − b·I`.
- `polar` module:
  - `modulus(z)` — `|z| = √(a² + b²)` as `IRFloat`; symbolic → `Abs(z)`.
  - `argument(z)` — `arg(z) = atan2(b, a)` in `(−π, π]` as `IRFloat`;
    symbolic → `Arg(z)`.
- `power` module:
  - `complex_pow(base, exp)` — integer powers via De Moivre's theorem.
  - Exact special cases: `z^0 = 1`, `z^1 = z`, `z^(-1)` via conjugate/|z|².
  - General case: `r^n · (cos(nθ) + i·sin(nθ))`.
  - `snap(v)` — snaps near-integer floats (within 1e-9) to exact `IRInteger`.
- 35 integration tests + 8 doc-tests; all passing.
