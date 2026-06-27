# Changelog — math-frontend

All notable changes to the pluggable parser-frontend framework.

## [0.2.0] — 2026-06-27

### Added — neutral-AST coverage for ± / ∓ and binomials

Closes the two honest gaps the LaTeX frontend (LTX01 L6) had to error on because the neutral
AST could not represent them:

- **`BinOp::PlusMinus` / `BinOp::MinusPlus`** — the `±` / `∓` operators (`a ± b` denotes the
  pair {a+b, a−b}; `∓` the opposite pairing). Meaning-bearing binary operators, not
  presentation.
- **`MathExpr::Binom(n, k)`** — a binomial coefficient "n choose k", distinct from `Frac`
  (no division bar).
- **`Capabilities`** gains `plusminus` and `binomials` flags (set by `all()` and the new
  `with_plusminus()` / `with_binomials()` builders); the conformance harness's
  `collect_used`/`over_emitted` now detect and police both, so a frontend emitting ± or a
  binomial without declaring it is flagged (verified by new over-claimer tests).
- Backward-compatible additive enum/struct changes (no removals). Downstream `latex` builds and
  its 136 tests pass unchanged; the `latex` frontend will start *emitting* these in a follow-up.
- +3 tests; **23 unit + 1 doc test** green; clippy `-D warnings` clean. Crate 0.1.0 → 0.2.0.

## [0.1.0] — 2026-06-26

### Added — PFE01 implementation: the framework

- New standalone, **zero-dependency** crate `math-frontend` (added to the Rust workspace
  members). The shared substrate for plugging in math-notation parsers.
- **`MathExpr`** — the notation-agnostic neutral AST: `Number`, `Symbol`, `Bin` (`Add Sub
  Mul Div Pow`), `Unary`, `Frac`, `Root`, `Call` (named `Func`), `BigOp` (with bounds),
  `Subscript`, `Rel`, `Group`, `Text`, `Matrix`. Presentation-only distinctions normalize
  away (`\times`/`\cdot`/juxtaposition → `Mul`).
- **`Number`** — **exact-preserving** numeric literal: parses decimal numerals (sign,
  integer/fraction, `e`-exponent) into a normalized `±digits×10^exp` triple, so `1`/`1.0`/
  `01`/`1e0` compare equal and `0.1` is never silently rounded. Keeps the written form;
  `to_f64()` is explicit and lossy. Zero is canonical (never `-0`). No big-int dependency.
- **`MathFrontend`** trait (total, panic-free, pure) + **`FrontendError`** (spanned, names
  the frontend) + **`Capabilities`** (builder over the constructs a frontend can emit).
- **`FrontendRegistry`** — name-keyed install/lookup/parse; unknown frontend yields a
  spanned error listing the installed ones (never a panic); `with_builtins()` is empty by
  design (LaTeX, the first frontend, registers here once its crate lands).
- **`check_frontend`** — shared conformance harness enforcing the contract: parsing never
  panics (`catch_unwind`), errors are well-formed (correct frontend name + in-range,
  non-inverted span), and capabilities are honest (a frontend may not emit a construct it
  didn't advertise).
- 19 unit tests + 1 doc test; `cargo clippy -- -D warnings` clean; no `unsafe`.

### Notes

- Parsing only — evaluation/lowering is a consumer concern. The LaTeX frontend (full
  LaTeX, per LTX01) and any consumer wiring (e.g. an ADJ `latex"…"` literal) are separate
  efforts that depend on this crate.
