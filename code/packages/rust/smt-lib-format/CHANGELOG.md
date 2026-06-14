# Changelog — smt-lib-format

## [0.1.0] — 2026-04-30

Initial release.  **LANG24 PR 24-E** — strict SMT-LIB v2 reader/writer
over the `constraint-instructions::Program` IR.

### Added

- `read(input)` — parse strict SMT-LIB v2 text into a validated
  `Program`.  Bounded depth (default `DEFAULT_MAX_DEPTH = 1024`).
- `read_with_limit(input, max_depth)` — caller-supplied depth cap;
  pass `usize::MAX` to disable.
- `write(program)` — emit `Program` in strict SMT-LIB v2 form.
  One command per line.
- `SmtLibError` (`#[non_exhaustive]`) — `UnexpectedEof`,
  `UnexpectedCloseParen`, `BadString`, `BadInt`, `BadCommand`,
  `UnknownCommand`, `BadSort`, `BadLogic`, `BadTerm`,
  `Program(ProgramError)`, `TooDeep`.

### Coverage

Commands: `set-logic`, `set-option`, `declare-const`,
`declare-fun`, `assert`, `check-sat`, `get-model`,
`get-unsat-core`, `push` / `push N`, `pop` / `pop N`, `reset`,
`echo`, plus `set-info` / `exit` accepted-and-skipped.

Predicate vocabulary: SAT + LIA + arrays + quantifiers (the v1
LANG24 logics).  Boolean combinators, comparisons, integer/real
literals (incl. bare decimals on read), unary/binary minus,
linear `(* k v)`, `(/ n d)` rationals, `ite`, single-binder and
(reader-only) multi-binder quantifiers, `select` / `store`,
uninterpreted-function application.

Sort vocabulary: `Bool`, `Int`, `Real`, `(_ BitVec w)`,
`(Array idx val)`, uninterpreted symbols.

### Format divergences from the constraint-instructions internal text

This crate is the strict-SMT-LIB bridge; constraint-instructions's
own format is the unambiguous internal one.  Three divergences:

- `Iff` ↔ SMT-LIB `(=` (overloaded with `Eq`); round-trip from
  `Iff` is therefore lossy (reads back as `Eq`).
- `Real(num/den)` written as `(/ num den)` (s-expression form);
  bare `n.d` decimals also accepted on read.
- `BitVec(w)` written as `(_ BitVec w)` (SMT-LIB indexed-identifier
  form).

### Hardening (security review)

The reader accepts attacker-controllable text and is hardened against:

- **`i128::MIN` overflow on unary minus** (`(- N)` where
  `N == i128::MIN`) — fixed with `checked_neg`; surfaces as
  `BadInt`.
- **`i128::MIN` overflow on `Mul` coefficient** (`(* (- N) x)`) —
  fixed with `checked_neg`; surfaces as `BadTerm`.
- **`Rational::new(_, 0)` panic** — `(/ n 0)` rejected explicitly
  before construction.  Surfaces as `BadTerm`.
- **`Rational::new(i128::MIN, _)` panic** — both `(/ i128::MIN d)`
  and decimal numerators equal to `i128::MIN` rejected before
  construction.  Surface as `BadTerm` / `BadInt`.
- **Stack-overflow DoS via deep parens** — default cap of
  `DEFAULT_MAX_DEPTH = 1024`, override via `read_with_limit`.
  Surfaces as `TooDeep`.
- **Unbounded `(push N)` allocation** — `N` capped at `[0, 10_000]`;
  surfaces as `BadCommand`.
- **Invalid UTF-8 in atoms / string literals** — both surface as
  `BadString` rather than producing empty strings or panicking.
- **Decimal-literal overflow** — `1.99…` whose numerator exceeds
  `i128::MAX` surfaces as `BadInt` (via `checked_mul` /
  `checked_add`).

### Tests

46 unit tests covering round-trip per command, round-trip per
`Predicate` variant, real-world SMT-LIB acceptance (`set-info`,
`exit`, multi-binder quantifiers, indexed BitVec sort,
double-quote-escape strings, decimal literals), all error variants,
and the security-review hardening above.

### Notes

- Pure data + algorithms.  Two deps (`constraint-core`,
  `constraint-instructions`), both capability-empty.  No I/O, no
  FFI, no unsafe.  See `required_capabilities.json`.
- Filed as follow-ups in the README roadmap: `smt-lib-benchmark-runner`
  CI pipeline; `let` / `define-fun` / `define-sort` desugaring;
  full v2.6 string escape support (`\u{...}`); the constraint-vm
  consumer (PR 24-D).
