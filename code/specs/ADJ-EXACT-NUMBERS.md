# ADJ-EXACT-NUMBERS — exact decimal literals & ground values (no silent f64 truncation)

## Motivation

We ship π to the ADJ standard library as a 39-digit literal
(`3.141592653589793238462643383279502884197`), byte-provenanced to Wolfram MathWorld.
But a query `? math_constant(pi, $V)` binds **`3.141592653589793`** — ~16 significant
digits. The 39 digits survive only inside the `source` provenance span; the *queryable
numeric value* is truncated.

This is **not a deliberate precision cap.** It is a gap in one code path. NUM-1..5 gave the
engine exact, unbounded numbers (`BigInteger`, `BigRational`, `BigDecimal`, `BigDouble`) and
NUM-5 made **arithmetic** exact by default — but that exactness lives only in the *compute
sidecar* (`ExactRational` on derivation nodes, `logic-engine/src/compute.rs`). The paths a
plain **literal** and a **ground fact value** travel were never migrated:

| stage | today | file |
|---|---|---|
| NUMBER token → AST | `Num(f64)` — **digits lost here** | `adj-lang/src/ast.rs:25`, `adapter.rs:2596` (`raw.parse::<f64>()`) |
| AST → ground term | `core_float(x: f64)` | `adj-lang/src/lower.rs:970,979,999` |
| ground term stores | `Number::Int(i64) \| Float(f64)` — **no exact slot** | `logic-core/src/lib.rs:58` |
| arithmetic result | `ExactRational(BigRational)` — exact ✅ | `logic-engine/src/compute.rs:67` |

So a stored constant is truncated to `f64` at **parse time**, upstream of every Big number,
and even if it weren't, the core `Number` enum has nowhere exact to hold it.

## Principle (extends NUM-5's own rule)

NUM-5 established: **exact is the ground truth; `f64` is a labeled, lossy *export*.** This spec
carries that same rule from *arithmetic results* to *literals and ground values*: a number the
user writes is stored **exactly as written**, and `f64` appears only when something explicitly
asks for the lossy export (a numeric backend that is inherently `f64`, an approximate display
mode, etc.). Nothing silently rounds a value the moment it is parsed.

## Design

### Core: a third `Number` variant (`logic-core`)

```rust
pub enum Number {
    Int(i64),
    Float(f64),                 // retained: the labeled-lossy / inherently-approximate value
    Exact(bignum_core::BigDecimal),   // NEW: an exactly-written decimal, unbounded digits
}
```

`BigDecimal` already derives `Clone/PartialEq/Eq/Hash` and implements `Ord`/`PartialOrd` and
`Display` (`bignum-core/src/decimal.rs`), so `Number` keeps its derived comparisons and gets a
faithful `Display` arm (`Number::Exact(d) => write!(f, "{d}")`) that prints all the digits.

- **Equality / unification.** `Number::Exact(a) == Number::Exact(b)` iff the decimals are
  numerically equal (BigDecimal equality is value-correct: `2.50 == 2.5`). Cross-variant
  equality is defined by exact promotion, not `f64` round-off: an `Int(n)` equals
  `Exact(d)` iff `d` is the integer `n`; an `Exact`/`Int` compares to a `Float` only via the
  **labeled-lossy** path (documented as approximate — a `Float` is already inexact, so any
  comparison to it is inexact by construction). Unification of two ground numbers uses exact
  equality when neither side is `Float`.
- **Lossy export.** A single accessor `Number::to_f64_lossy(&self) -> f64` (Int→as, Float→copy,
  Exact→`BigDecimal::to_f64`) is the *only* sanctioned way to obtain an `f64` from a `Number`.
  Grep-able name so audits can find every lossy boundary.

### Parse & lower (`adj-lang`)

- The lexer/adapter parses a NUMBER token by **`BigDecimal::from_str`** (exact) rather than
  `parse::<f64>()`. Integers with no fraction/exponent still become `Int(i64)` when they fit
  (keeps small-int ergonomics and existing `Int` fast paths); everything else becomes
  `Exact(BigDecimal)`. Scientific notation (`6.62607015e-34`) is normalized by `BigDecimal`.
- AST `Num(f64)` becomes `Num(NumLit)` where `NumLit` preserves the exact value (a
  `BigDecimal`, or an `i64` when integral-and-small). `lower_cell` / `lower_term` map `NumLit`
  → `Number::Exact` / `Number::Int` — never through `f64`.

### Compute ingestion (`logic-engine`)

When the evaluator reads a ground `Number` as a compute leaf, it ingests **exactly**:
`Int`→`ExactRational::from_integer`, `Exact(BigDecimal)`→`BigRational` via the decimal's
exact `mantissa / 10^scale` (no `from_integer_f64`, no `f64` hop). A `Float` leaf remains the
one inexact ingress and is documented as such. Net effect: `pi * 2` on the stored π is exact to
all 39 digits, and further arithmetic stays exact (NUM-5).

### Render (`adj-lang-cli`)

The query/JSON renderer prints `Number::Exact` with full digits. `f64`-shaped output is offered
only under an explicit approximate/precision-limited mode (ties into NUM-6
`round_to`/`round_sig`/`to_scientific`), never as the default for an exact value.

## Migration / blast radius

~30 crates pattern-match `Number`. The change is compiler-guided (adding a variant fails every
non-exhaustive match until handled) and, per "nothing released → break compat freely", that is
acceptable. The ~24 peripheral crates (storage, tool audit stores, json serializers) fold the
new arm into their existing `Float` handling via `to_f64_lossy()` — one line each. The
semantically-load-bearing updates are in `logic-engine`, `math-core`, `adj-lang`, and
`adj-lang-cli`.

## PR staging (each: spec/tests → impl → CHANGELOG → README/spec-sync → /security-review → babysit)

- **NX-0 (this doc).** Spec, committed first.
- **NX-1 — core variant.** Add `Number::Exact(BigDecimal)` + `Display` + `to_f64_lossy` +
  exact equality/ordering in `logic-core`; update **all** workspace match sites to compile
  (peripheral crates delegate to `to_f64_lossy`). `cargo build --workspace` green. No behavior
  change yet (nothing *produces* `Exact` — literals still lower as before), so this PR is a pure,
  reviewable type widening.
- **NX-2 — parse & lower.** `adj-lang` parses NUMBER via `BigDecimal::from_str`; AST carries the
  exact literal; `lower_*` emit `Number::Exact`/`Int`. Adds the e2e: `? math_constant(pi,$V)`
  binds the **full** digit string; small ints still bind as `Int`.
- **NX-3 — exact compute ingestion.** `logic-engine` ingests `Exact` leaves into `ExactRational`
  without an `f64` hop; test that arithmetic on a stored high-precision constant stays exact.
- **NX-4 — render + stdlib refresh.** CLI prints exact bindings; re-ground/verify
  `mathematics/constants` and `physics/physical-constants` so their comments (“exact decimal
  expansion”) become *true*, and add a query proving full-digit recall.

## Verification

- `cargo build --workspace` and `cargo test --workspace` green after each PR (shared-`Number`
  change → run downstream consumer tests, per repo lessons).
- Golden: a literal with >17 significant digits round-trips through parse → store → query render
  byte-identically (π to 39 digits).
- No silent `f64`: every `f64` obtained from a `Number` goes through `to_f64_lossy` (grep gate).
- Small-int and existing `Float` behavior unchanged (regression guard on current tests).
