# Changelog

## 0.4.0 — 2026-04-27

**Number theory MACSYMA names wired (B3).**

Adds `MACSYMA_NAME_TABLE` entries for all number-theory heads:
`primep`→`IsPrime`, `next_prime`→`NextPrime`, `prev_prime`→`PrevPrime`,
`ifactor`→`FactorInteger`, `divisors`→`Divisors`, `totient`→`Totient`,
`moebius`→`MoebiusMu`, `jacobi`→`JacobiSymbol`, `chinese`→`ChineseRemainder`,
`numdigits`→`IntegerLength`.

6 new pipeline tests in `test_cas_pipeline.py` cover the MACSYMA surface
syntax end-to-end.

## 0.3.0 — 2026-04-27

**MACSYMA completion roadmap items C2, C3, C4, C5 wired.**

Implements the language-layer bindings for the new IR heads added to
`symbolic-vm` 0.20.0, plus improvements to `ev` flag handling.

**Name-table additions** (`MACSYMA_NAME_TABLE`):
- `lhs` → `Lhs`  — left-hand side of an equation (C5).
- `rhs` → `Rhs`  — right-hand side of an equation (C5).
- `at`  → `At`   — point evaluation (C4).
- `makelist` corrected: now maps to `MakeList` (proper generative list)
  instead of `Range` (plain integer range).

**`ev` flag improvements** (C3):
- `expand` flag: applies `Expand` to the result.
- `factor` flag: applies `Factor` to the result.
- `float` flag: alias for `numer` (force floating-point collapse).
- Unknown flags continue to be silently ignored.

**Tests added**:
- 9 new pipeline tests in `test_cas_pipeline.py` covering `lhs`, `rhs`,
  `makelist` (3-arg, 4-arg, 5-arg), and `at` (single rule, multi-rule).
- 3 new ev tests in `test_ev.py` covering `float`, `expand`, `factor` flags.

## 0.2.0 — 2026-04-27

**Name table wired; constants pre-bound.**

This release completes the two missing connections that prevented the MACSYMA
REPL from dispatching algebraic operations to the CAS substrate.

**`backend.py`** — `MacsymaBackend.__init__` now pre-binds:
- `%pi` → `IRFloat(math.pi)`
- `%e`  → `IRFloat(math.e)`

Users can now type `%pi` and `%e` without defining them first.

**`language.py` (macsyma-repl)** — `extend_compiler_name_table(_STANDARD_FUNCTIONS)`
is now called at REPL module load time. This merges `MACSYMA_NAME_TABLE`
into the compiler's `_STANDARD_FUNCTIONS` dict so that `factor`, `expand`,
`simplify`, `solve`, `subst`, `limit`, `taylor`, `length`, `first`, etc.
all compile to canonical IR heads (`Factor`, `Expand`, `Simplify`, …) rather
than opaque user-function calls.

**Architecture note** (see also `symbolic-vm` 0.19.0): The substrate handlers
themselves (`Factor`, `Solve`, `Simplify`, `Length`, `Determinant`, `Limit`,
…) now live in `symbolic-vm`'s `SymbolicBackend` — the inner doll. The
`MacsymaBackend` (outer doll) only adds MACSYMA-specific operations:
`Display`, `Suppress`, `Kill`, `Ev`, and the two constant bindings above.
Future Maple and Mathematica backends will extend `SymbolicBackend` directly
and inherit all algebraic operations without touching any MACSYMA code.

## 0.1.0 — 2026-04-25

Initial release — Phase A skeleton.

- `MacsymaBackend` — `SymbolicBackend` subclass with MACSYMA-specific
  heads (`Display`, `Suppress`, `Kill`, `Ev`) and option flags.
- `History` — input/output table, resolves `%`, `%i1`, `%o1`, ...
  via a backend lookup hook.
- `Display` / `Suppress` heads (`;` vs `$` statement terminators).
  Identity handlers; the REPL inspects the wrapper before eval to
  decide whether to print.
- `Kill(symbol)` and `Kill(all)` handlers.
- `Ev(expr, ...flags)` — minimal first cut: only the `numer` flag
  is honored.
- `MACSYMA_NAME_TABLE` — extends `macsyma-compiler`'s standard-name
  map so identifiers like `expand`, `factor`, `subst`, `solve`,
  `taylor`, `limit` route to canonical heads (the substrate handlers
  may not yet exist; the user gets `Expand(...)` unevaluated until
  they do).
- Type-checked, ruff- and mypy-clean.
