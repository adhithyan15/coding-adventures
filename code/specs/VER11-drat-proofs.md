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

### 3.4 The contract (audit of 2026-11-09)

Everything above is the design. This section is the **testable contract**:
each rule is covered by at least one test in
`code/packages/rust/drat-check/tests/` or in the crate's unit tests. Where
the original text of §3.1 was vague, the rule below is binding.

#### 3.4.1 Soundness statement

> **S.** For all `formula`, `proof` and limits: if
> `check_with_limits(formula, proof, limits)` returns `Ok(_)`, then `formula`
> parses as strict DIMACS CNF (rules F1-F9) and, read as the conjunction of
> its clauses, is **unsatisfiable**.

Proof sketch. Let `F0` be the formula's clause multiset and `Fi` the database
after step `i`. A `d` step removes a clause, so `Fi` is a subset of `Fi-1`:
if `Fi-1` is satisfiable, so is `Fi`. An `add C` step is accepted only if
`C` is RUP (then `Fi-1` implies `C`, so adding it changes nothing) or RAT on
its first literal (then `Fi-1` satisfiable implies `Fi-1 + C` satisfiable,
by the standard RAT argument). So `F0` satisfiable implies every `Fi`
satisfiable. `Ok` is returned only when the empty clause is accepted at some
step `k` (it must be RUP: the empty clause has no pivot, so it is never
RAT), and then `Fk` contains the empty clause and is unsatisfiable. By
contraposition `F0` is unsatisfiable. Nothing about the work budget, the
variable limit or later proof lines enters this argument, which is why they
can only turn an acceptance into a rejection.

What `Ok` does **not** say: that the proof is minimal, that any particular
solver produced it, or that the formula the caller *meant* is the one that
was parsed. The kernel certifies the formula text it was given.

#### 3.4.2 Input formats

Only **text** formats are accepted. **Binary DRAT is not supported** and is
not detected: binary bytes fail as a bad proof token (or, by bad luck, parse
as something else), so callers must never feed binary proofs. Inputs are Rust
`&str` (valid UTF-8). There is no byte-order-mark handling: a BOM makes the
first line a bad formula line.

Lexical rules (both inputs):

- **L1.** Lines end at `\n`; a trailing `\r` is whitespace (CRLF works).
- **L2.** Tokens are separated by Unicode whitespace (space, tab, ...).
- **L3.** A line is a *comment line* if, after trimming whitespace, it
  starts with the character `c`; it is ignored whole. A `c` anywhere else
  (after tokens on the same line, say) is an ordinary, invalid token. Blank
  lines are ignored.
- **L4.** An integer token is `-?[0-9]+` read as a signed 64-bit value.
  **No `+` sign and no `-0`** (see divergence D6). Leading zeros are
  accepted (`007` is 7; `00` is the terminator). A token outside the `i64`
  range is not an integer and is rejected.

Formula (strict DIMACS CNF):

- **F1.** Exactly one header `p cnf <vars> <clauses>` (non-negative decimal
  integers, exactly four whitespace-separated fields) before any clause. A
  second header, a missing header, a malformed header, extra fields, or
  `<vars>` above the variable limit is rejected.
- **F2.** Clauses are sequences of non-zero integers each terminated by `0`.
  Clauses may span lines and several may share a line. A lone `0` is the
  empty clause (the formula is then trivially UNSAT).
- **F3.** Every literal satisfies `1 <= |l| <= <vars>`.
- **F4.** A clause left open at end of input (no terminating `0`) is
  rejected.
- **F5.** The number of clauses read must equal `<clauses>` exactly (fewer
  *and* more are rejected).
- **F6.** A line that is exactly `%` (after trim) ends the formula; every
  later line is ignored without being read (the SATLIB convention). This is
  the only way input can be ignored. A caller that also parses the formula
  itself must apply the same rule.
- **F7.** Duplicate literals and tautologies inside a clause are accepted
  and have their logical meaning (a clause is a *set* of literals).
- **F8.** Duplicate clauses count: a formula holding a clause twice needs it
  deleted twice before it is gone.
- **F9.** All errors are `ParseError::Formula { line, message }` with the
  1-based physical line; `line` is the last content line for end-of-input
  errors (F4, F5, missing header).

Proof (DRAT text):

- **P1.** A step is an optional leading `d`, then integers, then `0`. `d` is
  recognised only as the first token of a step; anywhere else it is a bad
  token. Steps may span lines; several may share a line.
- **P2.** `<lits> 0` adds, `d <lits> 0` deletes, a lone `0` adds the empty
  clause, `d 0` deletes an empty clause.
- **P3.** A proof ending inside a step (open literals, or a bare `d`) is
  rejected as truncated, **wherever** the damage is, including after the
  empty clause: the whole proof is parsed before any step is checked.
- **P4.** Any literal with `|l|` above the variable limit is rejected.
  Proof literals may use variables the formula does not have (RAT
  definitions).
- **P5.** Well-formed steps after the empty clause are parsed but never
  checked; a syntax error after it still rejects (see D3).
- **P6.** The step index in `Rejection` is 0-based over steps; `line` is the
  1-based line on which that step's terminating `0` stands.

#### 3.4.3 Checking semantics

- **C1. Database.** A multiset of clauses, each identified as a *set* of
  literals. Initially the formula's clauses.
- **C2. RUP.** `C` is RUP in `F` iff, from no assignment, assigning every
  literal of `C` false, assigning the literal of every unit clause of `F`
  true, and propagating by unit resolution to fixpoint, some clause of `F`
  becomes all-false (an empty clause in `F` counts, and so does a clash
  between two assignments). A tautological `C` is therefore RUP. Propagation
  is complete, so the verdict does not depend on clause or literal order.
- **C3. RAT.** A non-empty `C` is RAT iff, for its *first literal as
  written* `p`, every live clause `D` containing `-p` yields a RUP resolvent
  `C + (D - {-p})`. It is checked against the database *before* `C` is
  added. If no clause contains `-p`, `C` is RAT. The empty clause is never
  RAT. Literal order in an *added* lemma is therefore significant; the
  stored clause is a set.
- **C4. Order of tests.** RUP first; RAT only if RUP fails.
  `Accepted::rat_lemmas` counts lemmas that were RAT and not RUP.
- **C5. Deletion.** `d C` removes one live copy of a clause whose literal
  *set* equals `C`'s (permutations and duplicate literals still match). No
  such clause: `DeleteAbsent`. Any clause may be deleted: units, clauses
  that would be propagation reasons, the empty clause. There is no trail to
  corrupt (§3.2), and a deleted unit stops propagating at once. (This
  differs from `drat-trim`, which ignores unit deletions.)
- **C6. Termination.** The first accepted empty-clause lemma ends the check
  with `Ok`. If every step passes without one: `NoEmptyClause`. **An empty
  proof is never accepted**, even for a formula that is UNSAT by unit
  propagation or that contains the empty clause: the proof must contain the
  `0` step.
- **C7. Failure vocabulary and order.** `Rejection` is one of
  `Parse(ParseError)`, `NotImplied` (lemma neither RUP nor RAT),
  `DeleteAbsent`, `NoEmptyClause`, `ResourceLimit`. The formula is parsed
  first, then the proof, then steps are replayed in order; the first
  failing step wins.
- **C8. Resource limits.** `Limits::max_vars` (default 2^20, capped at 2^30)
  bounds the header's `<vars>` and every proof literal; tables cost about 60
  bytes per variable up to the largest index mentioned, *including indices
  that appear only in `d` steps*. `Limits::max_work` (default 2^32) bounds
  work in literal visits. Once exceeded, every further RUP check answers "no
  conflict", so a failing lemma is reported as `ResourceLimit`. A budget can
  only turn what an unlimited run would accept into a rejection, never the
  reverse. Every charge is made *before* the work it pays for, and a
  conflict is only reported after its charge succeeded, so an `Ok` always
  has `Accepted::work <= Limits::max_work`. Callers passing `usize::MAX` to
  `check_with_limit` accept up to 2^30 variables (tens of GB of tables).

#### 3.4.4 What "verified" means

A caller may label a result *proved* iff `check`/`check_with_limits`
returned `Ok` **on the exact formula text that encodes the negation of the
property**. `Ok` is evidence that this text is unsatisfiable, and nothing
more: not that the encoding is faithful (VER10's burden), and a
`ResourceLimit` is neither proof nor refutation.

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
   (default 2³²), including building RAT resolvents and walking the trail.
   Exhaustion makes checks answer "no conflict", so it can
   only reject (`ResourceLimit`), never accept. Soundness is untouched.
4. **Kernel size.** `check.rs` is 385 lines before its tests, about a
   third of them comments, inside the "few hundred lines" VER00 §7.5 asks
   for. The readers in `parse.rs` are about 200 more.
5. **Audit of 2026-11-09 (spec/code reconciliation).** Found by reading the
   code line by line against the text above. "fixed" means the code changed
   to match the spec; "spec" means the spec was tightened to match the code.
   - **D1 (spec).** The spec did not say whether binary DRAT is accepted. It
     is not (3.4.2).
   - **D2 (spec).** "Formula fails the DIMACS rules of VER09 section 5" was
     not self-contained. Rules F1-F9 now state them, including the `%`
     terminator, the comment-line rule and the exact clause count.
   - **D3 (spec).** 3.1 says that after the empty clause "later steps are not
     checked". True of *checking*, but the proof is parsed completely first,
     so a malformed or truncated tail still rejects (P3, P5).
   - **D4 (spec).** Empty-clause requirement: an empty proof, or one with no
     `0` step, is `NoEmptyClause` even if the formula contains an empty
     clause (C6).
   - **D5 (spec).** `d` steps naming huge variables allocate tables like any
     other literal (C8); the spec only mentioned lemmas.
   - **D6 (fixed).** Tokens `+1` and `-0` were accepted because the lexer
     used Rust's `i64::from_str`. `-0` acted as a clause terminator, so
     `p cnf 1 1` / `-0 1 0` read as *two* clauses where every other DIMACS
     reader sees a syntax error. A parser differential on the trusted side is
     exactly what the kernel's own-parser rule (3.1) exists to avoid. Rule L4
     now rejects both.
   - **D7 (spec).** The relation between `Accepted.work` and `max_work` was
     unstated. It is now a contract: `work <= max_work` on every `Ok` (C8).
   - **D8 (spec).** `Accepted::{empty_clause_line, work}`, `Limits` and
     `check_with_limits` were undocumented; they are API contract now.
   - **D9 (spec).** Section 5 claimed 95% coverage and "thousands of random
     mutations" without recording a measurement; the measured figure is in
     the crate CHANGELOG.
