# constraint-instructions

**LANG24 PR 24-B** — `ConstraintInstr` IR + `Program` + text
serialiser/parser for the generic Constraint-VM.

This is the *IR layer* between `constraint-core` (predicates, sorts,
logics) and `constraint-vm` (the executor).  Pure data; no solver
state, no I/O.

It mirrors the layering of the LP07/LP08 stack:

| Constraint-VM             | Logic-VM            | Code-VM           |
|---------------------------|---------------------|-------------------|
| `constraint-core`         | `logic-core`        | (none yet)        |
| **`constraint-instructions`** | `logic-instructions` | `interpreter-ir` |
| `constraint-engine`       | `logic-engine`      | `vm-core`         |
| `constraint-vm`           | `logic-vm`          | `vm-core` (combined) |

---

## Why a separate crate?

Per LANG24 §"Why a separate `constraint-instructions` crate?", the
IR is consumed by *more than one driver*:

- **`constraint-vm`** (PR 24-D) — the canonical executor.
- **`smt-lib-format`** (PR 24-E) — strict SMT-LIB v2 reader/writer
  for industry interop.
- **`z3-bridge`** (PR 24-H) — opt-in fallback that translates this
  IR into Z3's native AST for hard queries.
- **`debug-sidecar` / coverage / profiler** — tools that observe a
  constraint-program use this IR as their grammar (the LANG18 /
  LANG11 / debug-sidecar patterns generalised to constraints).

Decoupling instructions from the executor mirrors LP07 vs LP08
exactly.

---

## What lives here

| Type | Description |
|------|-------------|
| `ConstraintInstr` | The 12-variant opcode set (`#[non_exhaustive]`) |
| `OptionValue`     | Values the `SetOption` opcode carries (`Bool`, `Int`, `Str`) |
| `Program`         | Validated `Vec<ConstraintInstr>` |
| `ProgramError`    | Typed validation errors (`UnmatchedPop`, `BadIdentifier`) |
| `parse_program`   | Text → `Program`.  Round-trips with `Display` |
| `ParseError`      | Typed parser errors |

### The 12 opcodes

| Opcode             | Text form                                         |
|--------------------|---------------------------------------------------|
| `DeclareVar`       | `(declare-var x Int)`                             |
| `DeclareFn`        | `(declare-fn f (Int Bool) Int)`                   |
| `Assert`           | `(assert <predicate>)`                            |
| `CheckSat`         | `(check-sat)`                                     |
| `GetModel`         | `(get-model)`                                     |
| `GetUnsatCore`     | `(get-unsat-core)`                                |
| `PushScope`        | `(push)`                                          |
| `PopScope`         | `(pop)`                                           |
| `Reset`            | `(reset)`                                         |
| `SetLogic`         | `(set-logic QF_LIA)`                              |
| `Echo`             | `(echo "diagnostic")`                             |
| `SetOption`        | `(set-option :produce-models true)`               |

---

## Text format

SMT-LIB-flavoured s-expressions, with a few divergences chosen for
**round-trip exactness** (see "Format choices" below):

```text
(set-logic QF_LIA)
(set-option :produce-models true)
(declare-var x Int)
(declare-var y Int)
(assert (>= x 0))
(assert (<= (+ x y) 100))
(push)
(assert (>= y 50))
(check-sat)
(get-model)
(pop)
(check-sat)
```

- Whitespace is insignificant.
- Comments start with `;` and run to end of line.
- String literals (used for `Echo` / `OptionValue::Str`) are
  double-quoted with `\"`, `\\`, `\n`, `\t` escapes; UTF-8 inside
  literals is preserved verbatim.

### Format choices (vs strict SMT-LIB)

This crate's text format is the **internal** one used by debug
dumps, snapshot tests, and `Program::Display`.  Strict SMT-LIB
reader/writer ships in `smt-lib-format` (PR 24-E).  Three
divergences were chosen to make round-trip exact without sort
information:

| Construct       | This crate       | Strict SMT-LIB   |
|-----------------|------------------|------------------|
| `Iff`           | `(iff a b)`      | `(= a b)` (overloaded with `Eq`) |
| `Real(num/den)` | `(/ num den)`    | `num/den` (bare)  |
| `BitVec(w)`     | `(BitVec w)`     | `(_ BitVec w)`    |

---

## Round-trip guarantee

For every `Program p` that `Program::new` accepts:

```rust
assert_eq!(parse_program(&p.to_string()).unwrap(), p);
```

`Program::new` enforces this by validating that every identifier
(variable, function, sort, quantifier binder) is:

- Non-empty
- Not a reserved token (`and`, `or`, `not`, `=>`, `iff`, `<=>`,
  `=`, `distinct`, `!=`, `+`, `-`, `*`, `/`, `<=`, `<`, `>=`, `>`,
  `ite`, `forall`, `exists`, `select`, `store`, `true`, `false`,
  `Bool`, `Int`, `Real`, `BitVec`, `Array`)
- Not parseable as an integer literal
- Free of whitespace and s-expression delimiters
  (`(`, `)`, `;`, `"`)

Programs whose names violate any of the above are rejected at
construction with `ProgramError::BadIdentifier` carrying the
offending instruction index, name (truncated for very long
inputs), and reason.

`new_unchecked` skips validation for callers that have already
validated (e.g. the parser after re-parsing a known-good text
serialisation).

---

## Caller responsibilities (non-guarantees)

Inherits all of `constraint-core`'s non-guarantees (predicate depth,
CNF blow-up, `Rational` range).  In addition:

- **Parser depth.**  `parse_program` recurses on parenthesis
  depth without an explicit guard.  Callers parsing untrusted text
  should bound the input length / depth at the boundary.
- **Display depth.**  `Program::Display` (and its `write_predicate`
  helper) recurses on the AST.  Same caveat as
  `Predicate::Display`.

Resource limits on *execution* (instruction count, scope depth,
variable count, solver timeout) live in `constraint-vm` per the
spec — this crate is pure data.

---

## Example

```rust
use constraint_instructions::{ConstraintInstr, OptionValue, Program, parse_program};
use constraint_core::{Logic, Predicate, Sort};

let p = Program::new(vec![
    ConstraintInstr::SetLogic { logic: Logic::QF_LIA },
    ConstraintInstr::DeclareVar { name: "x".into(), sort: Sort::Int },
    ConstraintInstr::Assert {
        pred: Predicate::Ge(
            Box::new(Predicate::Var("x".into())),
            Box::new(Predicate::Int(0)),
        ),
    },
    ConstraintInstr::CheckSat,
])
.unwrap();

let text = p.to_string();
assert_eq!(
    text,
    "(set-logic QF_LIA)\n(declare-var x Int)\n(assert (>= x 0))\n(check-sat)"
);

let parsed = parse_program(&text).unwrap();
assert_eq!(parsed, p);
```

---

## Dependencies

- `constraint-core` (path) — `Predicate`, `Sort`, `Logic`, `Rational`.

That's it.  No other dependencies, no I/O, no FFI.  See
`required_capabilities.json`.

---

## Tests

56 unit tests covering:
- `Display` for every opcode.
- `Mnemonic` for every opcode.
- `Program::new` validation (scope balance + identifier safety).
- Round-trip for every opcode + every `Predicate` variant +
  every `Sort` variant + every `Logic` variant + every
  `OptionValue` kind.
- Parser edge cases: comments, whitespace, unmatched parens,
  unterminated strings, bad escapes, integer overflow on
  `set-option`, validation-error propagation.
- UTF-8 in string literals (multi-byte sequences round-trip;
  invalid UTF-8 is rejected with `ParseError::BadString`).
- Identifier validation (empty / reserved / int-lookalike /
  whitespace / paren / `Apply` head collision / quantifier
  binder collision / `Sort::Uninterpreted` collision).

```sh
cargo test -p constraint-instructions
```

---

## Roadmap

- **PR 24-C `constraint-engine`** — tactic trait + v1 implementations
  (sat-tactic CDCL, lia-tactic Cooper) consuming this IR.
- **PR 24-D `constraint-vm`** — the canonical executor.
- **PR 24-E `smt-lib-format`** — strict SMT-LIB v2 reader/writer.
- **Follow-ups for this crate**:
  - `parse_program_with_limit(input, max_depth)` for parsing
    untrusted text safely.
  - Quoted-symbol syntax (SMT-LIB `|...|`) so reserved-name
    identifiers can opt-in to round-tripping.
