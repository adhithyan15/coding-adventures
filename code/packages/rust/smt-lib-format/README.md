# smt-lib-format

**LANG24 PR 24-E** — strict SMT-LIB v2 reader/writer over the
`constraint-instructions::Program` IR.

Industry-standard textual format that lets the constraint-VM
interoperate with industry solvers (Z3, CVC5) and run the SMT-LIB
benchmark suite as part of CI.

---

## Position in the stack

```
SMT-LIB v2 textual file (.smt2)
        │
        ▼
   smt-lib-format            ← this crate
        │
        ▼
   constraint-instructions::Program
        │
        ▼
   constraint-vm (PR 24-D) / Z3 bridge (PR 24-H) / debug-sidecar
```

---

## Public API

| Item | Description |
|------|-------------|
| `read(input)` | Parse strict SMT-LIB v2 → `Program`.  Bounded depth (default 1024). |
| `read_with_limit(input, max_depth)` | Caller-supplied depth cap; pass `usize::MAX` to disable (only for trusted input). |
| `write(program)` | Emit `Program` in strict SMT-LIB v2.  One command per line. |
| `DEFAULT_MAX_DEPTH = 1024` | Default parenthesis-nesting cap. |
| `SmtLibError` | Typed errors: `UnexpectedEof`, `UnexpectedCloseParen`, `BadString`, `BadInt`, `BadCommand`, `UnknownCommand`, `BadSort`, `BadLogic`, `BadTerm`, `Program(ProgramError)`, `TooDeep`. |

---

## Coverage (v1 scope)

### Commands handled by both reader and writer

| SMT-LIB v2 command           | constraint-instructions opcode |
|------------------------------|--------------------------------|
| `(set-logic L)`              | `SetLogic`                     |
| `(set-option :k v)`          | `SetOption`                    |
| `(declare-const x S)`        | `DeclareVar`                   |
| `(declare-fun x () S)`       | `DeclareVar`                   |
| `(declare-fun f (S₁ … Sₙ) S)`| `DeclareFn`                    |
| `(assert φ)`                 | `Assert`                       |
| `(check-sat)`                | `CheckSat`                     |
| `(get-model)`                | `GetModel`                     |
| `(get-unsat-core)`           | `GetUnsatCore`                 |
| `(push)` / `(push N)`        | `PushScope` (N copies)         |
| `(pop)`  / `(pop N)`         | `PopScope`  (N copies)         |
| `(reset)`                    | `Reset`                        |
| `(echo "msg")`               | `Echo`                         |
| `(set-info ...)` / `(exit)`  | silently dropped               |

### Predicate vocabulary

Covers SAT + LIA + arrays + quantifiers — the v1 LANG24 logics:

| SMT-LIB form                | `Predicate` variant            |
|-----------------------------|--------------------------------|
| `true` / `false`            | `Bool`                         |
| `<int-literal>`             | `Int`                          |
| `(- n)` / `<dec-literal>`   | `Int`(negative) / `Real`       |
| `(/ n d)`                   | `Real(Rational)`               |
| `<symbol>`                  | `Var`                          |
| `(and …)` `(or …)` `(not …)`| `And` / `Or` / `Not`           |
| `(=> a b)`                  | `Implies`                      |
| `(= a b)`                   | `Eq`                           |
| `(distinct a b)`            | `NEq`                          |
| `(+ …)` / `(- a b)`         | `Add` / `Sub`                  |
| `(* k v)` (k integer, v term)| `Mul` (linear)                |
| `(<= a b)` / `(<)` / `(>=)` / `(>)`| `Le` / `Lt` / `Ge` / `Gt` |
| `(ite c t e)`               | `Ite`                          |
| `(forall ((x S)) φ)`        | `Forall` (one binder)          |
| `(exists ((x S)) φ)`        | `Exists` (one binder)          |
| `(select arr idx)`          | `Select`                       |
| `(store arr idx val)`       | `Store`                        |
| `(f a₁ … aₙ)`               | `Apply`                        |

### Sort vocabulary

`Bool`, `Int`, `Real`, `(_ BitVec w)`,
`(Array idx-sort val-sort)`, plus uninterpreted sort symbols.

---

## SMT-LIB ↔ constraint-instructions divergences

Three intentional differences — see the `constraint-instructions`
README for *why* the internal IR diverges; this crate exists to
bridge them on read/write:

- **`Iff`**: SMT-LIB writes `(= a b)` for both `Eq` and `Iff`
  (overloaded on `Bool` operands).  This crate's reader always
  parses `(= …)` as `Eq`; the writer emits `(=` for both `Eq` and
  `Iff`.  **Round-trip from `Iff` is therefore lossy** (reads
  back as `Eq`).  Users wanting to preserve `Iff` should use
  `constraint-instructions`'s own text format.
- **`Real(num/den)`**: SMT-LIB has bare `n/d` decimal literals
  *and* `(/ n d)`; the writer emits the s-expression form for
  exact round-trip.  Bare decimals are also accepted on read.
- **`BitVec(w)`**: SMT-LIB uses the indexed-identifier form
  `(_ BitVec w)`; this crate handles it on both sides.

## Multi-binder quantifiers

SMT-LIB allows `(forall ((x S₁) (y S₂)) φ)` (multi-binder).
`Predicate::Forall` / `Exists` only carry one binder, so the
reader **lowers** multi-binder quantifiers to nested
single-binder ones.  The writer emits one binder per quantifier
(the simpler form).

---

## Out of scope (v1)

- `let`-bindings (per LANG24 §"Out of scope").
- `define-fun`, `define-sort`, `declare-datatypes`.
- String theory.
- Floating-point theory.
- `set-info` / `get-info` / `get-assignment` / `get-proof` /
  `get-assertions` / `get-value` / `simplify`.
- `as` ascription on terms.

These extensions land in subsequent v2/v3 PRs as their use-cases
arrive (LANG24 §"Theories supported (versioned scope)").

---

## Hardening (security review)

The reader accepts attacker-controllable text and is hardened
against the following classes of malicious input:

- **`i128::MIN` overflow on unary minus.**  `(- N)` where
  `N == i128::MIN` would wrap-overflow on plain negation.  Fixed
  with `checked_neg`; surfaces as `BadInt`.
- **`i128::MIN` overflow on `Mul` coefficient.**  `(* (- N) x)`
  where `N == i128::MIN` would also overflow.  Fixed with
  `checked_neg`; surfaces as `BadTerm`.
- **`Rational::new(_, 0)` panic.**  `(/ n 0)` rejected explicitly
  before construction.  Surfaces as `BadTerm`.
- **`Rational::new(i128::MIN, _)` panic.**  Both `(/ i128::MIN d)`
  and decimals whose numerator equals `i128::MIN` rejected before
  construction.  Surface as `BadTerm` / `BadInt`.
- **Stack-overflow DoS via deep parens.**  Default cap of
  [`DEFAULT_MAX_DEPTH`] = 1024.  Override via
  [`read_with_limit`].  Surfaces as `TooDeep`.
- **Unbounded `(push N)` allocation.**  `N` must be in
  `[0, 10_000]` (defensive cap; SMT-LIB itself unbounded).
  Surfaces as `BadCommand`.
- **Invalid UTF-8 in atoms / string literals.**  Both surface as
  `BadString` rather than producing empty strings or panicking.
- **Decimal literal overflow.**  `1.99…` whose numerator overflows
  `i128` surfaces as `BadInt` (via `checked_mul` / `checked_add`).

## Caller responsibilities

Inherits all `constraint-instructions` non-guarantees (predicate
depth in `Display`, etc.).  The default depth cap covers practical
SMT-LIB benchmark depth (~50); raise it only for trusted input.

---

## Example

```rust
use smt_lib_format::{read, write};

let text = r#"
    (set-logic QF_LIA)
    (declare-const x Int)
    (declare-const y Int)
    (assert (>= x 0))
    (assert (<= (+ x y) 100))
    (push 2)
    (assert (>= y 50))
    (check-sat)
    (get-model)
    (pop 2)
    (check-sat)
"#;

let program = read(text).unwrap();
let serialised = write(&program);
let round_tripped = read(&serialised).unwrap();
assert_eq!(round_tripped, program);
```

---

## Dependencies

- `constraint-core` (path) — `Predicate`, `Sort`, `Logic`, `Rational`.
- `constraint-instructions` (path) — `Program`, `ConstraintInstr`,
  `OptionValue`, `ProgramError`.

That's it.  No other dependencies, no I/O, no FFI, no unsafe.  See
`required_capabilities.json`.

---

## Tests

46 unit tests covering:
- Round-trip per command and per `Predicate` variant.
- Real-world SMT-LIB acceptance (`set-info`, `exit`, multi-binder
  quantifiers, indexed BitVec sort, double-quote-escape strings,
  declare-fun-of-arity-0, decimal literals).
- All `SmtLibError` variants exercised.
- Security-review hardening: `i128::MIN` negation, division by
  zero, parser depth cap, invalid UTF-8 in atoms, `(push N)`
  count overflow, decimal-literal overflow.

```sh
cargo test -p smt-lib-format
```

---

## Roadmap

- **PR 24-D `constraint-vm`** — first executor consumer.  Full
  driver loop on top of `constraint-instructions` + `constraint-engine`.
- **`smt-lib-benchmark-runner`** — pipeline that pulls SMT-LIB
  benchmarks from upstream, runs them through `read` + the
  forthcoming `constraint-vm`, and reports pass/fail counts as a
  CI signal.
- **Strict v2.6 string escapes** — currently we accept the v2.6
  `""` double-quote escape but not `\u{...}` Unicode escapes.
- **`let`-bindings** — `(let ((x e₁) …) body)` desugars to
  substitution at parse time.
- **`define-fun` / `define-sort`** — macro-expand at parse time.
