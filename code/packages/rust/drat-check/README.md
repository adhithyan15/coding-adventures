# `drat-check`

The trusted kernel for SAT proofs: a small, dependency-free **DRAT proof
checker**. **VER11** of the verified shipping program
([`code/specs/VER11-drat-proofs.md`](../../../specs/VER11-drat-proofs.md)).

## Why it exists

In VER00 §7.6, a property is proved by asking a SAT solver whether a
counterexample exists. "No" (UNSAT) is the proof. But the solver,
[`cdcl-sat`](../cdcl-sat), is thousands of lines of heuristics, and a bug in
it could say UNSAT when the answer is SAT. That would be a false proof.

So the solver is not trusted. With every UNSAT answer it emits a **DRAT
certificate**: the clauses it learned, in order, ending with the empty
clause. This crate replays the certificate against the original formula and
accepts only if every step is justified. Only this crate has to be right, so
it is kept small enough to read in full.

```text
  formula ──► cdcl-sat ──► UNSAT + DRAT proof ──┐
     │                                          ▼
     └────────────────────────────────────► drat-check ──► Accepted / Rejection
```

## What it checks

| Step | Accepted when |
|---|---|
| `l₁ … lₖ 0` (add) | the lemma is **RUP**: assuming it false, unit propagation reaches a conflict. Otherwise it is **RAT** on its first literal: every resolvent on that literal is RUP |
| `d l₁ … lₖ 0` (delete) | the clause is present (matched as a set) |
| `0` (empty clause) | it is RUP; the formula is then UNSAT and checking stops |

Rejected: truncated proofs, deletions of absent clauses, malformed input,
variables over the limit, and proofs that never add the empty clause.

## Design for auditability

- **No dependencies, not even on the solver.** It reads DIMACS with its own
  code, so a parser bug cannot fool the solver and the checker in the same
  way.
- **`src/check.rs` is the kernel.** It is a few hundred lines, explained
  inline.
- **Forward and stateless between steps.** Each RUP check starts from an
  empty assignment and undoes everything afterwards.
- **Bounded input.** Variable indices are capped (default 2^20), so hostile
  input cannot force a huge allocation. Work is capped too (default 2^32
  literal visits, `Limits::max_work`). A proof built to be slow is cut off
  with `ResourceLimit`. Running out can only reject, never accept.

## Usage

```rust
let formula = "p cnf 2 4\n1 2 0\n1 -2 0\n-1 2 0\n-1 -2 0\n";
let accepted = drat_check::check(formula, "1 0\n0\n").unwrap();
assert_eq!(accepted.lemmas, 2);
assert!(drat_check::check(formula, "0\n").is_err());
```

## Accepted input, precisely

The contract is VER11 section 3.4; the short version:

- **Text only.** DIMACS CNF and DRAT *text*. **Binary DRAT is not
  supported** (it will fail as a bad token; never feed it).
- **Integers** are `-?[0-9]+` in `i64` range. `+1` and `-0` are rejected
  (they used to be accepted, and `-0` acted as a clause terminator).
- **Comments** are whole lines starting with `c`; `%` alone on a line ends a
  formula. CRLF and tabs are fine; a BOM is not.
- **The proof must contain the `0` step.** An empty proof is never accepted.
  The proof is parsed completely first, so a malformed tail after the empty
  clause still rejects; well-formed steps after it are not checked.
- **Soundness (S).** If `check` returns `Ok`, the formula text is
  unsatisfiable. The argument is in VER11 section 3.4.1.

## How it is tested

Run `cargo test -p drat-check` (about 30 s, most of it the exhaustive file).

- **Unit tests** (`src/*.rs`): every parse error, RUP, RAT, deletion and
  verdict path.
- **`tests/exhaustive.rs`**: bounded-exhaustive. Every CNF over x1..x3 with
  at most three clauses of width at most two (15,180 formulas: 13,494 SAT,
  1,686 UNSAT, satisfiability by brute force). For SAT formulas, no
  candidate proof is accepted (about 5 million: empty, every one-step, a
  deterministic sample of two-step proofs over an alphabet with a fresh
  variable so RAT occurs, each with and without a final empty clause). For
  UNSAT formulas, a resolution refutation generated in the test is accepted
  in five syntactic variants. The kernel is compared with an independent
  full-scan checker on 1.65 million proofs, verdict and step.
- **`tests/contract.rs`**: one test per rule of VER11 section 3.4: grammar,
  RAT pivot and partner rules, deletion of unit and reason clauses,
  duplicates, tautologies, a work-budget sweep and large inputs.
- **`tests/reference.rs`**: the older random differential against a naive
  checker, and a soundness check on 2000 random satisfiable formulas.
- **Real solver proofs:** in `cdcl-sat`'s tests every UNSAT answer in its
  differential corpus is certified by this crate.
- **Mutation check (by hand, 2026-11-09):** deliberate bugs in a scratch copy
  of the kernel are caught by the suite; the table is in the CHANGELOG.
- **Coverage** (`cargo llvm-cov`, measured on a standalone copy of the crate
  because the workspace invocation fails with E2BIG): see the CHANGELOG.

## Capabilities

None. It is pure computation over `&str`. See `required_capabilities.json`.
