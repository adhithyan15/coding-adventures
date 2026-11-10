# Changelog — `constraint-engine`

## 0.5.0 — 2026-11-10

Faithfulness audit of the BV tactic's `Predicate` to circuit lowering
(`src/bv.rs`; backlog LH-006a), the layer the `bit-blaster` audit (PR
#19081) did not cover. **No unfaithful lowering was found:** every
operation that the tactic already accepted matched an independent
SMT-LIB reference on every checked assignment, and 18 hand-made mutants of
the lowering (swapped branches, operands, operators, off-by-one indices)
were all killed by the new tests. What the audit did find is listed under
Security.

### Added

- Lowering for the rest of the `QF_BV` operations that `bit-blaster` 0.2.0
  already had circuits for: `bvudiv`, `bvurem`, `bvsdiv`, `bvsrem`,
  `concat`, and the indexed forms `(_ extract hi lo)`, `(_ zero_extend k)`
  and `(_ sign_extend k)`. `smt-lib-format` cannot yet read an indexed head
  such as `((_ extract 7 4) x)`, so the tactic takes the flattened shape
  `Apply { f: "_", args: [Var("extract"), Int(7), Int(4), x] }`. Before
  this, all of them answered `Unknown`.
- `bvsmod` (SMT-LIB: the remainder takes the sign of the **divisor**),
  built in `bv.rs` from `bvurem`, `bvneg`, `bvadd` and `ite` exactly as the
  SMT-LIB definition spells it out. `bit-blaster` is unchanged.
- `src/bv_faithfulness.rs` (unit tests inside the module): the audit.
  - Every unary and binary operation, every comparison, `=`, `distinct`,
    every extract slice, the extensions and both `concat` shapes:
    **exhaustive at every width 1 to 8** by simulating the lowered
    circuit against a reference written from the SMT-LIB definitions
    (the test asserts more than 2 million checked assignments). Ternary forms (n-ary
    `bvadd/bvsub/bvmul/bvand/bvor/bvxor`, `ite`) exhaustive at widths 1 to
    4, 5000 seeded random assignments at 5 to 8.
  - Seeded random assignments plus every pair of corner values (0, 1, 2,
    max, max-1, sign boundary, `w-1`, `w`, `w+1`) at 16, 32 and 64 bits.
  - End to end through `BvTactic::solve` (Tseitin, CDCL, DRAT): the
    reference answer is SAT and reads back the pinned inputs, the negation
    is UNSAT (so kernel-certified). Exhaustive at widths 1 to 3, sampled at
    4 and 16.
  - SMT-LIB corner semantics pinned explicitly: `bvudiv x 0 = all ones`,
    `bvurem x 0 = x`, `bvsdiv x 0 = -1/+1`, `bvsrem/bvsmod x 0 = x`,
    `MIN / -1 = MIN`, `MIN srem -1 = 0`, shifts by at least the width,
    extract and extend boundaries, the `bvsdiv/bvsrem/bvsmod` table.
  - Boolean connectives against truth tables; every refusal path with its
    reason; every `#b`, `#x` (both cases) and `(_ bvN w)` literal at widths
    1 to 8 plus the 127/128-bit boundary.
- Certified identities at 16, 32 and 64 bits: add/sub/neg/xor/compare
  identities, concat/extract round trip, division and remainder by zero and
  by one, multiplication by constants (0, 1, 2, 3, 5, 7, `2^(w-1)`,
  `2^(w/2)` squared).
- A line in `BUILD` that runs the one slow test (see Measured) with
  `--release -- --ignored`.

### Security

- **A wide divider was built in full before it was refused.** `check_size`
  runs after an operation finishes, so a division at width 512 first built
  millions of gates before answering `Unknown`.
  Division, remainder and `bvsmod` now estimate their size from the width
  (`16 w^2 + 64 w` gates, checked by `division_cost_estimate_is_conservative`
  to be at or above the real count for each operation) and refuse before
  building. `concat`, `zero_extend` and `sign_extend` check the result
  width before allocating. `bvmul` has the same late check, unchanged here:
  a 512-bit multiplier is about 2.6 M gates, within the tested envelope.
- Indexed identifiers take only non-negative indices that fit `usize`; a
  hostile `zero_extend` by `i128::MAX` is an `Unknown`, not an allocation.

### Changed

- `Lowering::decide` takes the `drat-check` limits as a parameter.
  `BvTactic::solve` always passes the defaults; only tests pass a larger
  budget, so the public tactic's limits are not raised.

### Measured (what is and is not proved)

Coverage matrix (S = simulation of the lowered AIG, E = end to end with
certified UNSAT, P = proof by miter):

| Operation group | w 1 to 8 | w 16, 32, 64 |
|---|---|---|
| bitwise, add, sub, neg, mul, shifts, udiv, urem, sdiv, srem, smod, comparisons, `=`, `distinct`, concat, extract, extensions | S exhaustive; E exhaustive at w 1 to 3, sampled at 4 | S seeded random + corners; E sampled at 16 |
| ternary: n-ary folds, `ite` | S exhaustive at w 1 to 4, sampled at 5 to 8; E exhaustive at w 3 | S seeded random + corners |
| mul commutativity | P certified at w 1 to 7 (8 passes in release, too slow unoptimised) | not proved (see below) |
| mul by constants, low `h` bits of a product equal the `h`-bit product (`16->8`, `32->16`, `32->8`, `64->32`, `64->8`) | | P certified |
| add/sub/neg/logic identities, division by zero/one | | P certified |
| `x * -1 = -x` | P certified w 9, 10, 12 | w 16 needs a drat work budget of 2^34 (4x the default 2^32): P certified in the ignored release test; the default limit answers `Unknown` |

- **Correction to the backlog note.** The wide multiplier miters do not all
  fail on drat-check's work limit. `a*b = b*a` fails from width 10 on
  because the *CDCL search* exhausts its 1 M conflict budget (about 20 s in
  release); the proof checker is never reached. Only `x * -1 = -x` at width
  16 gets as far as the checker, which rejects at 2^32 literal visits. Both
  answer `Unknown`, never `Unsat` (tested), and neither limit was raised.
  For the wide cases the tests use a per-test budget (above) or the
  structural decomposition (low halves).
- **Not proved:** the *high* bits of a multiplier wider than 8 for
  arbitrary operands, and general multiplier miters (`a*b = b*a`,
  distributivity) from width 10 up. They are covered by simulation only
  (random + corners at 16, 32, 64; exhaustive to 8). Widths above 64 are
  covered only structurally (concat/extract/extend wiring at 200 bits).
- Line coverage of `src/bv.rs`: 98.5% (384 of 390 lines), region coverage
  97.6%, function coverage 100% (`cargo llvm-cov` on a standalone copy; the
  workspace run fails with "Argument list too long"). The six uncovered lines
  are `unreachable!` arms and the "search budget exhausted" answer.

## 0.4.0 — 2026-11-02

The LIA tactic is sound: `Unsat` is reported only after an exhaustive
search (LANG24, "LIA as built, and its soundness contract").

### Fixed

- **False `Unsat` from an incomplete search.** The tactic is a bounded
  enumeration, not Cooper's algorithm. When its sample ran out it answered
  `Unsat`, which was wrong in three ways:
  - With no lower bound, the search started at 0, so `x < 0` was "UNSAT".
  - The range was cut to 1 001 values, so `2x = 4000` was "UNSAT".
  - `(and (>= x a) (<= x b))` lost both bounds inside one `And`.

  The search now tracks whether it is exhaustive. It is exhaustive for a
  Bool, for an Int boxed by two real bounds at most 1 000 apart, and for
  contradictory real bounds. When it is not, a failed search answers
  `Unknown`. A sub-search's `Unknown` is remembered instead of being
  treated as a failed candidate.

- **A value left assigned after a failed equality branch.** This predates
  the PR. When `x = k` pinned a variable and the branch failed, the value
  stayed in the assignment. A later branch then judged constraints over it
  "ground", which gave a false `Unsat` (`z = x + 10 ∧ x + z > 10`, boxed).
  The branch now restores the assignment unless it found the model. The
  engine also orders search variables by name, so results are
  reproducible, not dependent on `HashMap` iteration.

### Added

- **Interval propagation** over the top-level linear atoms, including
  negated comparisons. It derives the bounds the constraints imply. For
  example, `x > 5 ∧ y > 5 ∧ x + y < 10` gives `x ≤ 3`, a contradiction,
  and `__v = x ∧ 0 ≤ x < 256` boxes `__v`. Derived bounds are consequences
  of the constraints, so they keep every answer the same and only let the
  search be exhaustive. This is what keeps `lang-refinement-checker`'s
  `clamp_byte` proof a real proof. Each atom costs work linear in its terms
  per round (minimum sum computed once). Total work is capped at 64 rounds
  and 2 million term visits; stopping early is sound.
- **Conjunctions are flattened** before the search, and variables no
  constraint mentions are not searched (they get 0 / false).
- Sampling without bounds looks both ways: a window centred on 0, or a
  window ending at the one real bound.
- `tests/lia_soundness.rs` compares the tactic against brute force on
  mixed Int+Bool problems. Boxed problems are decided (0 `Unknown` in 1 200)
  and never wrong; unboxed problems are never wrong. It also covers every
  input from the security review.

## 0.3.0 — 2026-11-02

VER10: a QF_BV tactic.

### Added

- `bv::BvTactic` decides conjunctions over `Bool` and `BitVec(w)`
  variables (`1 ≤ w ≤ 512`) by bit-blasting them with the new
  `bit-blaster` crate and solving with `cdcl-sat`. Supported: `#b`/`#x`/
  `(_ bvN w)` literals; `bvnot bvneg bvand bvor bvxor bvadd bvsub bvmul`
  (n-ary, left-associative); `bvshl bvlshr bvashr`; unsigned and signed
  comparisons; `=`, `distinct`, Boolean connectives and `ite`. Anything
  else answers `Unknown` with a reason.
- **Certified UNSAT:** the tactic logs a DRAT proof and runs `drat-check`
  on it before reporting `Unsat`. A rejected proof is `Unknown`.
- Bounds: widths ≤ `MAX_BV_WIDTH` (512), circuits ≤ `MAX_AIG_NODES`
  (4 million, checked per operation), `BV_CONFLICT_BUDGET` (1 000 000).
- `Value::BitVec(Vec<bool>)` (least significant bit first), with
  `Value::as_bv_u128` and `#b…` display.

### Security

Findings from the VER10 security review, all fixed and covered by
regression tests in `constraint-vm/tests/qfbv.rs`:

- **A certified false `Unsat` from a `#`-named variable.** For example,
  `(declare-const #b1 (_ BitVec 1))` was read as the literal 1. The BV tactic
  now refuses declared names that begin with `#`.
- **Bit-vector syntax reaching the LIA tactic, with wrong answers both
  ways.** `(declare-const n Int) (assert (= #b01 #b01))` answered `Unsat`.
  Dispatch now also scans the assertions for bit-vector syntax (iteratively,
  so any depth is safe). Bit-vectors mixed with `Int` answer `Unknown`.
- **The LIA tactic misreading what it cannot evaluate.** This flaw predates
  VER10: its evaluator turns anything unknown into `false`, and `not` of
  that into `true`, so `(declare-const n Int) (assert (= m m))` answered
  `Unsat`. Every assertion must now sort-check as `Bool` under the declared
  sorts (`infer_sort`) and stay inside the evaluator's fragment before LIA
  runs; otherwise the answer is `Unknown`.
- **The evaluator was two-valued, and the LIA search trusted it.** Also
  predates VER10. Two concrete false-`Unsat` paths:
  - Bool variables next to `Int` variables were grounded as integers. The
    evaluator treated `Int` vs `Bool` as "unknown", then as false, so
    `(declare-const n Int) (declare-const p Bool) (assert (= p true))`
    answered `Unsat`.
  - Arithmetic saturated at the edge of `i128`, so
    `n + MAX > n + MAX − 1` came out false.

  The fixes:
  - **`eval_int_or_bool` is now three-valued** (Kleene `and`/`or`/`not`/
    `implies`/`iff`/`ite`), with exact checked arithmetic, so overflow is
    unknown. A grounded Bool meets a Boolean by truth value.
  - **The LIA search answers `Unknown`** whenever a ground constraint cannot
    be evaluated, both at the base case and during bound extraction.
  - **Bool variables are searched over {0, 1} only.**
  - **The guard now refuses** integer literals beyond ±2^62, which keeps
    bound arithmetic away from overflow, and arithmetic or comparisons over
    non-`Int` operands.
- **DoS through the BV tactic.** Every kind of work is now bounded:
  - circuit size: `MAX_AIG_NODES` cut from 4M to 250k, about 150 MB;
  - construction work: `MAX_GATE_CALLS`, 2M, since repeated subterms are
    free in size but not in time;
  - search: `BV_WORK_BUDGET`, 2^30 propagation steps, on top of the
    conflict budget;
  - a CNF that `drat-check` could never certify is refused before solving.

### Changed

- `Engine::check_sat` routes problems with any `BitVec` variable, any
  bit-vector syntax, or logic `QF_BV`, to the BV tactic. `QF_BV` and `BitVec(1..=512)` are now
  supported; `BitVec` mixed with `Int` answers `Unknown`.
- The trivial model of an assertion-free problem gives `BitVec` variables
  all-zero values. Widths above the limit get none, so a huge unused
  declaration allocates nothing.

## 0.2.0 — 2026-11-02

VER09: the SAT tactic now runs on the `cdcl-sat` CDCL solver.

### Changed

- **`SatTactic` is an adapter over `cdcl-sat`.** The recursive DPLL over
  string-named literals is replaced by a conflict-driven clause-learning
  solver. The signature, the three outcomes, and the "variables that occur
  in no clause are `false`" model rule are unchanged. The DPLL's
  1 000 000-node budget becomes a 1 000 000-conflict budget, and running
  out still answers `Unknown`.
- **The Cargo description is now true.** It said "SAT CDCL" while the code
  was DPLL; the tactic is now CDCL, via `cdcl-sat`.
- `src/sat.rs` became `src/sat/mod.rs`.

### Security

- **NNF size guard.** `SatTactic::solve` now refuses (`Unknown`) any
  predicate whose negation normal form would exceed `sat::MAX_NNF_NODES`
  (100 000) nodes, computed in linear time without building it. `Iff`
  copies both operands, so a chain of nested `Iff`s made `to_nnf` take
  time and memory exponential in the depth (seconds at depth 20, hours at
  depth 30, far below the depth guard) before any budget applied. That
  case existed before VER09; the security review of this change found it.

### Added

- `sat::CONFLICT_BUDGET`, `sat::MAX_NNF_NODES`.
- `src/sat/dpll_oracle.rs`: the pre-VER09 DPLL, unchanged, compiled only
  under `cfg(test)`. A new test runs 1000 random propositional formulas
  through the adapter, the old DPLL and a truth table, and requires all
  three to agree; every model is checked with `eval_bool`.

## Unreleased — 2026-08-18

Brought under CI: the crate had no `BUILD` file, so nothing ever ran its tests
or linted it.

### Added

- **`BUILD` file — this crate is now built, tested and linted in CI.**

  This crate is a member of the `code/packages/rust` workspace, so it compiled
  whenever a sibling with a `BUILD` file pulled it in as a path dependency. But
  the build tool discovers work by scanning for `BUILD` files, so with none of
  its own it was never a package in its own right: its **test targets were never
  compiled, its assertions never ran, and `cargo clippy --all-targets -- -D
  warnings` never linted it**, on any platform. Adding `BUILD` puts it under the
  same per-package clippy gate and test run as every other watched Rust crate.

  The BUILD is the repo-standard one-liner, `cargo test -p constraint-engine -- --nocapture`,
  kept on a single line: the build tool runs each BUILD line as its own
  `sh -c`, so a backslash continuation would silently truncate the command.
  It was verified green locally first — clippy `-D warnings` clean and a full
  unfiltered `cargo test --no-fail-fast` passing — per the "expect to find
  existing breakage when you start watching a long-unwatched package" rule in
  `lessons.md`.

## 0.1.1 — 2026-05-04

Security hardening — DoS guards on the SAT and LIA tactics.

### Fixed

- **SAT tactic: depth check before CNF conversion** (`sat.rs`).  The SAT
  tactic now calls `depth_of(p) > MAX_PREDICATE_DEPTH` on every assertion
  before invoking `to_cnf()`.  Predicates exceeding the depth limit return
  `SolverResult::Unknown(…)` immediately rather than recursing into stack
  overflow.  The `to_cnf()` call is also guarded via `Result` — a
  `Err(budget_msg)` returns `Unknown(budget_msg)` so the engine degrades
  gracefully to "can't decide" instead of panicking.

- **LIA tactic: `neqs` changed from `Vec<i128>` to `HashSet<i128>`**
  (`lia.rs`).  The candidate-filter step inside `eliminate_all` called
  `neqs.contains(&candidate)` in a tight loop.  With `Vec`, O(n) membership
  cost per candidate turned into O(n²) when a user supplied hundreds of
  disequality constraints (e.g. `x ≠ 1, x ≠ 2, …, x ≠ k`).  `HashSet`
  makes each `.contains()` O(1).

## 0.1.0 — 2026-05-04

Initial release.  **LANG24 PR 24-C.**

### Added

- `Engine` struct: declare variables, assert predicates, `check_sat`, `snapshot`/`reset_all` for scope management.
- `SolverResult` enum: `Sat(Model)`, `Unsat`, `Unknown(String)`.
- `Model` struct: variable → `Value` mapping with `get`/`insert`/`iter`.
- `Value` enum: `Bool(bool)`, `Int(i128)`, `Real(i128, i128)`.
- **LIA tactic** (`lia` module): bounded Cooper variable-elimination for `QF_LIA`.
  - Handles `Ge`, `Le`, `Lt`, `Gt`, `Eq`, `NEq`, `Add`, `Sub`, `Mul` predicates.
  - Multi-variable constraints solved via sequential elimination.
  - Fixed: deferred evaluation of constraints over unbound variables to prevent spurious UNSAT.
- **SAT tactic** (`sat` module): DPLL with unit propagation and pure-literal elimination for `QF_Bool`.
- Nelson-Oppen-style dispatch: integer vars → LIA, bool-only → SAT, mixed → LIA.
- Trivial model generation for empty assertion sets.
- `eval_bool` / `eval_int_or_bool` for model verification.
- 46 unit tests covering both tactics and engine dispatch.
