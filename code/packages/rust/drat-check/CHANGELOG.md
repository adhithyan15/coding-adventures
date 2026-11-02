# Changelog — `drat-check`

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
