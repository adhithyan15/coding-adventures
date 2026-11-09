# Changelog — `drat-check`

## Unreleased — 2026-11-09 — hardening audit

An audit of the verification stack found evidence claimed but never produced
(hard-coded test counts, placeholder certificates). This entry records what
was *measured* for this crate; every number below comes from a command run in
the audit session, none is carried over from earlier docs.

### Fixed

- **Leading zeros rejected (VER11 L4, D10).** Tokens `00`, `01`, `-01`, `007`
  and header counts such as `02` are now parse errors, matching the solver's
  now-strict reader (`cdcl-sat` 0.3.2). Previously accepted.

- **Parser differential (robustness; VER11 D6).** The DIMACS and DRAT lexers
  used Rust's integer parsers, which accept `+1` and read `-0` as `0`, so
  `-0` acted as a clause terminator: `p cnf 1 1` / `-0 1 0` read as two
  clauses where every other DIMACS reader reports a syntax error. Tokens are
  now vetted against `-?[0-9]+` (no `+`, no `-0`; header counts digits only).
  Regression tests fail on the old parser (verified before the fix).
  No soundness hole was found: the kernel never accepted a formula, as
  parsed, that is satisfiable.

### Added

- VER11 section 3.4: the precise contract (soundness statement S, grammar
  L1-L4/F1-F9/P1-P6, semantics C1-C8, meaning of "verified") and divergence
  list D1-D9 reconciling spec and code. Binary DRAT is explicitly
  unsupported; an empty proof is never accepted; `Accepted::work <=
  Limits::max_work` on every `Ok`.
- `tests/exhaustive.rs`: bounded-exhaustive tests.
  - Width-2 domain: all 15,180 CNFs over x1..x3, at most 3 clauses of width
    at most 2 (13,494 SAT, 1,686 UNSAT, by brute force). For SAT formulas,
    5,003,728 candidate proofs (empty; every one-step; sampled two-step;
    adds and deletes, over an alphabet using a fresh x4 so RAT occurs; each
    with and without a final empty clause): none accepted. For UNSAT
    formulas, a resolution refutation generated in the test is accepted in
    five variants (plain, reversed literals, duplicated lemmas, with set-matched
    deletions of the duplicates, with a trailing tail, plus rejection when the
    final `0` is cut). Kernel versus an independent full-scan checker on
    1,650,220 proofs: identical verdict and step.
  - Width-3 domain (exercises the watch-replacement search, dead code at
    width 2): 13,244 formulas, 895,362 sampled proofs, kernel equals reference
    and nothing accepted for a SAT formula.
  - Wall time on a 4-core machine in a debug build: about 45 s for the file.
- `tests/contract.rs`: 21 tests, one per rule of section 3.4 (grammar,
  RUP/RAT pivot and partner rules, deletion of unit/reason/duplicate/empty
  clauses, a work-budget sweep proving a valid proof only ever yields
  `ResourceLimit` under any budget, and 100k-variable / 200k-literal inputs).
- A unit test for the strict integer grammar in `src/parse.rs`.

### Verified (commands run 2026-11-09)

- `cargo test -p drat-check`: unit 20, contract 21, exhaustive 5,
  reference 4, doctest 1, all passing.
- `cargo clippy -p drat-check --all-targets -- -D warnings`: clean.
- Consumers pass unchanged: `cargo test -p cdcl-sat`, `-p bit-blaster`,
  `-p constraint-engine`, `-p coding_adventures_warrant_ir`.
- Hand-written mutation testing (scratch copy, mutants discarded): 18
  deliberate bugs, all caught. See the table below.
- Coverage, `cargo llvm-cov` 0.9.1 (see COVERAGE below for the figures).
  `cargo llvm-cov -p drat-check` inside the workspace fails with
  "Argument list too long (os error 7)" in this environment, so coverage was
  measured on a standalone copy of the crate.

### COVERAGE

`cargo llvm-cov` 0.9.1 (`cargo install` found it already present; the
`llvm-tools` component had to be added), all test targets, standalone copy:

| File | Regions | Missed | Lines | Missed | Functions |
|---|---|---|---|---|---|
| check.rs | 657 | 3 (99.54%) | 304 | 0 (100%) | 34/34 |
| lib.rs | 178 | 2 (98.88%) | 85 | 0 (100%) | 11/11 |
| parse.rs | 418 | 2 (99.52%) | 204 | 1 (99.51%) | 24/24 |
| **Total** | **1253** | **7 (99.44%)** | **593** | **1 (99.83%)** | **69/69** |

Caveats: the unit-test modules live inside `src/*.rs`, so their lines are in
these totals. The single missed line is `other => panic!(...)` in a
`parse.rs` unit test (it runs only when that test fails); every production
line executes. The first coverage run found one uncovered production line
(`is_rat` running out of budget while building a resolvent), which
`c8_budget_sweep_through_the_rat_check` now covers. Branch coverage is not
reported by this toolchain (stable). The numbers are for the final tree.

### Mutation results

| Mutant | Caught by (examples) |
|---|---|
| M1 skip the RAT resolvent RUP checks | c3_rat_cases, run_rejections, soundness exhaustive, differential |
| M2 accept when the empty clause is missing | c6_termination_rules, every_rejection_kind, completeness |
| M3 watch search `2..` becomes `3..` | width_three exhaustive (alone), c4_ordering, reference differential |
| M4 deleting a unit clause has no effect | c5_deleting_unit_and_reason_clauses_takes_effect, units_and_empty_clauses |
| M5 RAT pivot is the last literal | c3_rat_cases, run_accepts_rat_extension, differential |
| M6 tautological lemma not RUP | c2_rup_cases, tautologies_and_repeated_literals_are_rup |
| M7 deletion matches order-sensitively | c5_deletion_rules, deletion_matches_sets_and_removes_one_copy |
| M8 empty clause accepted without RUP | c6_termination_rules, soundness exhaustive, ~20 more |
| M9 RAT partners contain the pivot, not its negation | c3_rat_cases, module_rat_example |
| M10 unit clauses not enqueued in RUP | c2_rup_cases, run_refutes_and_counts, ~25 more |
| M11 work budget never exhausts | c8_work_budget_sweep, running_out_of_work_rejects_and_never_accepts; the hostile-proof test then runs unbounded (killed after minutes) |
| M12 empty clause not counted in database | f2_empty_clause_in_formula, units_and_empty_clauses |
| M13 formula clause-count check removed | f2_f3_f4_f5_clause_rules, formula_errors |
| M14 truncated bare `d` not detected | p3_p4_truncation_and_limits, proof_errors |
| M15 propagation: satisfied test inverted | c2_rup_cases, module_rup_example (and the unit-test binary loops forever: killed by a watchdog) |
| M16 deleted clauses still propagate | c5_deletion_rules, deleted_clauses_stop_propagating, soundness exhaustive |
| M17 `%` no longer ends the formula | f6_percent_terminator, formula_ok |
| M18 unit clash no longer a conflict | c2_rup_cases, c3_rat_cases, units_and_empty_clauses |

Lesson recorded: `lessons.d/a-trusted-kernel-must-not-delegate-token-grammar-to-the-host-language.md`.

## 0.1.0 — 2026-11-02

Initial release: the VER11 trusted kernel for SAT proofs.

### Added

- `check` / `check_with_limit`: forward DRAT checking of a text proof
  against a DIMACS formula, both given as `&str`. Lemmas must be RUP or
  RAT; deletions must name a present clause (matched as a set); the proof
  must add the empty clause. Returns `Accepted` with counts, or a
  `Rejection` naming the step and line.
- Its own strict DIMACS reader and DRAT text reader (`parse` module), which
  share no code with `cdcl-sat`, and a variable-index cap (default 2^20,
  never above 2^30).
- A work budget (`Limits`, `check_with_limits`, default 2^32 literal
  visits; `Accepted::work` reports the cost). Exhaustion rejects with
  `Rejection::ResourceLimit` and can never cause an acceptance. Added after
  the security review timed a 75 KB formula and a short RAT-heavy proof
  at minutes of checking.
- Tests: every error path; agreement with a naive full-scan reference
  checker on 4000 random proofs; no proof accepted for 2000 satisfiable
  formulas.
