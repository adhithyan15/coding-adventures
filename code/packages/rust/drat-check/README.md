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

## How it is tested

- **Unit tests** cover every parse error, RUP, RAT, deletion and verdict.
- **Reference checker:** `tests/reference.rs` re-implements DRAT checking
  in the most naive way (full-scan propagation). On 4000 random proofs, the
  two must give the same verdict at the same step, across accepted, not
  implied, absent deletion and no empty clause.
- **Soundness:** for 2000 formulas that brute force shows are satisfiable,
  no generated proof may be accepted.
- **Real solver proofs:** in `cdcl-sat`'s tests, every UNSAT answer in its
  differential corpus is certified by this crate. A negative corpus of
  truncated, reordered and tampered proofs is rejected in full.

## Capabilities

None. It is pure computation over `&str`. See `required_capabilities.json`.
