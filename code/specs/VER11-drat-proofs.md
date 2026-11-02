# VER11 — Proof certificates and the trusted kernel, part 1: DRAT

**Status:** Accepted. Implemented by `code/packages/rust/drat-check` (the
kernel) and the proof-logging part of `code/packages/rust/cdcl-sat`.

**Layer:** Proof machinery, layer 3 (VER00 §7.5 step 3, §7.6.6 item 2).

**Builds on:** VER00, VER09 (the CDCL solver whose UNSAT answers this
certifies).

**Consumed by:** VER10 (bit-blasted proofs), VER18, VER19, and the
assurance report's `proved` label (VER00 §7.6.5): no E5 result is labelled
`proved` unless its certificate passed this kernel.

This spec covers the propositional part of the kernel: **DRAT** certificates
for SAT-level UNSAT answers. Proof terms for SMT theories (VER00 §7.5 step 3,
"proof terms for theories") are a later part of VER11.

---

## 1. Why a certificate, and why a separate checker

A CDCL solver is thousands of lines of heuristics, and a bug anywhere in
them can turn into a wrong UNSAT. A wrong UNSAT is the worst kind of bug in
this program: it is a "proof" that a property holds when it does not.

So the solver is not trusted. Instead, every UNSAT answer comes with a
**certificate**: the sequence of clauses the solver learned, ending in the
empty clause. A separate, small program, the **checker**, replays the
certificate against the original formula and accepts it only if every
step is justified. Then only the checker has to be right, and it is small
enough to read in full.

```
   formula ──► cdcl-sat ──► UNSAT + DRAT proof ──┐
      │                                          ▼
      └──────────────────────────────────────► drat-check ──► accepted / rejected
                                                (trusted)
```

## 2. DRAT in one page

A DRAT proof is a list of steps. Each step **adds** a clause (a lemma) or
**deletes** one:

```
   1 -2 0        add the clause (x1 ∨ ¬x2)
   d 3 4 0       delete the clause (x3 ∨ x4)
   0             add the empty clause: the formula is UNSAT
```

**RUP (reverse unit propagation).** A lemma `C = (l₁ ∨ … ∨ lₖ)` is RUP with
respect to the current clauses `F` if assigning every `lᵢ` false and running
unit propagation on `F` reaches a conflict. Then `F` implies `C`: any
assignment satisfying `F` and falsifying `C` would survive that propagation,
and it does not. Every clause a CDCL solver learns is RUP (VER09 §4.3).

**RAT (resolution asymmetric tautology).** A lemma `C` with first literal `p`
is RAT if for every clause `D` in `F` that contains `¬p`, the resolvent
`C ∪ (D ∖ {¬p})` is RUP. A RAT lemma is not implied by `F`, but adding it
keeps `F` satisfiable if it was. RAT covers preprocessing techniques that
introduce new variables. VER09's solver never needs it, but the kernel
supports it so that later work (preprocessing, extended resolution) can use
the same kernel.

**Deletion** removes a clause. It cannot make a satisfiable formula
unsatisfiable, so it is always safe. It keeps checking fast.

**Soundness.** Every accepted addition preserves satisfiability, so if the
empty clause is added, the original formula was unsatisfiable.

## 3. The kernel: `drat-check`

### 3.1 Rules

- **No dependency on the solver.** `drat-check` has no dependencies at all.
  It parses DIMACS itself, with its own code, so a bug in `cdcl-sat`'s
  parser cannot make both sides misread a formula in the same way.
- **Small and readable in full.** The checking core (`check.rs`) stays at a
  few hundred lines with its comments. Speed is secondary to clarity.
- **Forward checking.** Steps are checked in order, each against the clauses
  alive at that point. (Backward checking, as in `drat-trim`, is faster but
  more complicated. It can come later as a second, untrusted pre-pass,
  never as the kernel.)
- **Strict.** The kernel rejects anything doubtful rather than guessing:

| Input | Verdict |
|---|---|
| Formula fails the DIMACS rules of VER09 §5 | rejected (formula error) |
| A proof token is not an integer, or `d` is not at the start of a step | rejected (parse error, with line) |
| The proof ends inside a step (no terminating `0`) | rejected: truncated |
| An added lemma is neither RUP nor RAT | rejected, naming the step and line |
| A deleted clause is not present | rejected |
| The proof never adds the empty clause | rejected: incomplete |
| A variable index above the limit (default 2²⁰) | rejected |
| Checking needs more than the work budget (default 2³² literal visits) | rejected: resource limit, naming the step |
| The empty clause is added and passes RUP | **accepted**; later steps are not checked |

Deleting a clause the proof never added and the formula never had is a
sign of a proof that was spliced or reordered, so it is an error rather than
`drat-trim`'s warning.

### 3.2 Unit propagation in the kernel

RUP checks run often, so the kernel uses two watched literals, like the
solver, but in a deliberately simpler form:

- Each check starts from an empty assignment, assigns the negated lemma,
  enqueues every live unit clause, and propagates. Afterwards every
  assignment is undone. There is no persistent trail, which removes the
  classic subtle cases around deleting unit clauses that are reasons.
- The two-watched-literal invariant needs no repair between checks, because
  each check ends with everything unassigned.
- Deleted clauses are dropped from watch lists lazily, when propagation
  meets them.
- Clauses are matched for deletion by their sorted, de-duplicated literal
  set, so a solver may permute a clause's literals (as watching does) and
  still delete it.

### 3.3 API

```rust
pub fn check(formula: &str, proof: &str) -> Result<Accepted, Rejection>;
pub fn check_with_limit(formula: &str, proof: &str, max_vars: usize)
    -> Result<Accepted, Rejection>;
```

`Accepted` carries counts (lemmas checked, RAT lemmas, deletions). `Rejection`
says why, and for proof errors which step and line.

## 4. Proof emission in `cdcl-sat`

Proof logging is off by default and switched on per solver
(`Solver::enable_proof`). The proof is collected in memory, so the solver
still needs no capabilities; `Solver::proof()` returns it and
`DratProof::to_drat_text()` renders it. Writing it to a file is the caller's
job.

The four events VER09 §6 reserved are logged as follows:

| Event | Logged step |
|---|---|
| `add_clause` stores a clause shorter than given (level-0 false literals removed), or reduces it to a unit or to nothing | `add` of the simplified clause (RUP: the removed literals are false by level-0 units) |
| a learned clause, unit or longer | `add` of the clause |
| database reduction deletes a learned clause | `d` of the clause |
| a conflict at level 0 (in search or while adding a unit) | `add` of the empty clause |

Original clauses are not logged: they are the formula, and the checker
reads them from the formula file. Clauses added between incremental solves
are originals too. A proof checked against the final formula stays valid,
because adding clauses to a formula never invalidates a RUP lemma.

UNSAT **under assumptions** does not end in the empty clause, because the
formula alone may be satisfiable. Such proofs are not certificates of
anything yet. A later part of VER11 will let the kernel check that a proof
derives a given clause, namely the negated failed-assumption core, so that
BMC and k-induction results can be certified too.

## 5. Verification

| Test | What it shows |
|---|---|
| Every UNSAT answer in the VER09 differential corpus (random formulas, pigeonhole) is re-solved with logging on, and `drat-check` accepts its proof | emission and checking agree |
| The kernel against a naive reference checker (full-scan propagation, written separately in the tests) on valid proofs and on thousands of random mutations | the watched-literal kernel accepts exactly what the obvious algorithm accepts |
| Negative corpus, all rejected: truncated proofs (cut mid-step, cut before the empty clause); the empty clause moved first; lemmas reordered so that one comes before its premises; tampered literals; deletions of absent clauses; valid proofs replayed against a *satisfiable* weakening of their formula | the kernel rejects what it must |
| Soundness: no proof of any kind is accepted for a satisfiable formula (random proofs against brute-force-SAT formulas) | the kernel cannot be talked into a false UNSAT |
| Parser error paths, RAT acceptance and rejection, limits | §3.1 holds |

Line coverage of `drat-check` is held at 95% or more, as is the
proof-logging code in `cdcl-sat`.

## 6. Capabilities

Both crates are pure computation over strings and in-memory data. Their
`required_capabilities.json` files list no capabilities.

## 7. Divergences

1. **Default variable cap 2^20, not 2^24.** The VER09 security review
   measured the per-variable cost of a 2^24 cap at about 2 GB of solver
   state. The kernel costs about 60 bytes per variable, so it follows the
   solver down to 2^20 (about 60 MB). Explicit limits stay available.
2. **`enable_proof` returns `false` once a clause has been added**, rather
   than starting a proof with level-0 simplifications already missing.
3. **A work budget** (security review). Checking is polynomial, but a
   hostile proof can make it slow: one RAT step costs a RUP check per
   partner clause, and a 75 KB formula plus a short repeating proof ran for
   minutes. Every literal visit is now charged against `Limits::max_work`
   (default 2³²). Exhaustion makes checks answer "no conflict", so it can
   only reject (`ResourceLimit`), never accept. Soundness is untouched.
4. **Kernel size.** `check.rs` is 373 lines before its tests, about a
   third of them comments, inside the "few hundred lines" VER00 §7.5 asks
   for. The readers in `parse.rs` are about 200 more.
