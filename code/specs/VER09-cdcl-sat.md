# VER09 — CDCL SAT: the Boolean engine under every proof

**Status:** Accepted. Implemented by `code/packages/rust/cdcl-sat`.

**Layer:** Proof machinery, layer 1 (VER00 §7.5, §7.6.6 items 1, 4 and 5).

**Builds on:** VER00 (the verified shipping program), LANG24
(`constraint-engine`, `constraint-vm`).

**Consumed by:** VER11 (DRAT proof certificates and the trusted kernel),
VER10 (Tseitin, and-inverter graphs, bit-blasting), VER18 (hardware
equivalence and k-induction), VER19 (bit-precise proofs over WIR), and
`constraint-engine`'s `SatTactic`.

---

## 1. Why this exists

VER00 §7.6 builds every first-generation proof on one question: *can this
Boolean formula be made true?* An UNSAT answer, backed by a certificate
(VER11), is a proof over all inputs. A SAT answer is a counterexample.

The repository already has a SAT solver: `constraint-engine/src/sat.rs`.
It is a recursive DPLL over string-named `Predicate`s, with `HashMap`
assignments, clause lists cloned on every branch, and a node budget. It is
fine for the O(10)-variable refinement checks of LANG23. It cannot carry
VER00:

| Need (VER00) | Old DPLL |
|---|---|
| Miter circuits with thousands of variables (§7.6.1) | Copies the whole clause list on every branch; exponential with no learning |
| Incremental solving for BMC and k-induction (§7.6.6 item 4) | Stateless: every call starts from nothing |
| DRAT certificates (§7.5 step 3, VER11) | Learns nothing, so it has no lemmas to log |
| A Cargo description that is true | Says "CDCL"; the code is DPLL |

The old solver is **not grown**. Its shape (string names, recursion,
cloning) is the problem, and fixing it in place would be a rewrite wearing
the old file's name. Instead a new crate, `cdcl-sat`, holds a conventional
conflict-driven clause-learning solver. `SatTactic` becomes a thin adapter
over it, and the old DPLL survives only as a test oracle.

## 2. Scope

**In:**

1. Integer variables and literals, MiniSat-style encoding.
2. Two watched literals for unit propagation, with blocker literals.
3. First-UIP conflict analysis with clause learning and conflict-clause
   minimisation.
4. Non-chronological backjumping.
5. VSIDS branching (activity heap with exponential decay) and phase saving.
6. Luby restarts.
7. Learned-clause database reduction.
8. Solving under assumptions, with a failed-assumption core, and adding
   clauses between solves (incremental).
9. A conflict budget, so callers that face hostile input can bound work.
10. A strict DIMACS CNF reader and a writer.

**Out (later specs):**

- (DRAT proof emission was listed here when VER09 was written. It has since
  landed in this crate, in memory, under VER11; §6 now states its contract.)
- Preprocessing (bounded variable elimination, subsumption). Every
  preprocessing step must also be logged for DRAT, so it waits for VER11 to
  land and to be measured against.
- Parallel or portfolio solving.
- Theory propagation (DPLL(T)) — VER10 follow-ups.

## 3. Data model

### 3.1 Variables and literals

A **variable** is a dense index `0..n`. A **literal** is a variable with a
sign, packed into one `u32` as `2·var + sign`, where `sign = 1` means
negated:

```
   var 0:  x0 → 0     ¬x0 → 1
   var 1:  x1 → 2     ¬x1 → 3
   var v:  xv → 2v    ¬xv → 2v+1
```

Negation is `code ^ 1`, the variable is `code >> 1`, and a literal indexes
arrays (watch lists, values) directly. DIMACS numbering is 1-based with a
minus sign for negation; conversion happens only at the boundary
(`Lit::from_dimacs`, `Lit::to_dimacs`).

### 3.2 Values

Each variable holds a three-valued `LBool`: `True`, `False` or `Undef`.
The value of a literal is its variable's value, flipped when the literal is
negated.

### 3.3 Clauses

Clauses live in an arena, addressed by a `ClauseRef` index. Each clause
records its literals, whether it is learned, its LBD ("literal block
distance": the number of distinct decision levels among its literals when
it was learned) and an activity. Deleted clauses are tombstoned and their
watches removed lazily.

### 3.4 Trail

The **trail** is the stack of assigned literals in assignment order.
`trail_lim[d]` is the trail length when decision level `d + 1` began. Each
variable records its decision level and its **reason**: the clause that
forced it, or none for decisions and assumptions.

## 4. The algorithm

### 4.1 The search loop

```
loop:
    conflict = propagate()
    if conflict:
        if level == 0: return UNSAT
        (learnt, backjump_level) = analyze(conflict)
        backtrack(backjump_level)
        add learnt; assert its first literal (it is unit now)
        decay activities
        if budget exhausted: return UNKNOWN
    else:
        if restart due (Luby): backtrack(0)
        if too many learned clauses: reduce_db()
        if an assumption is pending: decide it (or fail, §4.6)
        else pick = branch(); if none: return SAT
        new decision level; assign pick
```

### 4.2 Unit propagation with two watched literals

Every clause of length ≥ 2 watches its first two literals. A clause can only
become unit or conflicting when one of its watched literals becomes false,
so when literal `p` becomes true, only the clauses watching `¬p` are
visited. For each one:

1. If the **blocker** literal (a cached literal of the clause) is true, the
   clause is satisfied: skip it without touching its memory.
2. Make sure the false literal is in position 1.
3. If position 0 is true, the clause is satisfied: keep watching.
4. Look for a replacement watch among positions 2.. that is not false. If
   one exists, swap it into position 1 and move the watch.
5. Otherwise the clause is unit (assign position 0, reason = this clause)
   or, if position 0 is false, conflicting.

Backtracking does not touch the watches. The invariant "a watched literal
is false only if every other literal in the clause is false, or the clause
is satisfied at a lower level" survives unassignment.

Unit clauses are never watched. They are assigned at level 0 when added.

### 4.3 Conflict analysis: the first UIP

When a clause is falsified, walk the trail backwards from the conflict,
resolving the conflict clause with the reasons of the current-level
literals, until exactly one current-level literal remains: the **first
unique implication point** (1UIP). The learned clause is that literal's
negation together with every lower-level literal seen.

```
   conflict:  (¬a ∨ ¬b)        trail at level 3:  d@3  a←(¬d ∨ a)  b←(¬d ∨ b)
   resolve on b with (¬d ∨ b):  (¬a ∨ ¬d)
   resolve on a with (¬d ∨ a):  (¬d)          one level-3 literal left → 1UIP
```

The learned clause is implied by the formula (resolution is sound), and it
is **RUP** (reverse unit propagation): assigning the negation of all its
literals and propagating reaches a conflict. VER11 relies on that.

**Minimisation.** A literal in the learned clause whose reason clause's
other literals are all already in the clause (or are level-0 facts) is
redundant and removed. The recursive form from MiniSat is used, bounded by
the trail. The result is still RUP.

### 4.4 Non-chronological backjumping

After learning `C = (u ∨ l₁ ∨ … ∨ lₖ)` with `u` the UIP literal, the solver
backtracks to the **highest level among `l₁…lₖ`** (0 if `C` is unit), not
just one level up. At that level `C` is unit and immediately asserts `u`.
Whole levels of decisions that had nothing to do with the conflict are
undone at once.

### 4.5 Heuristics

- **VSIDS.** Every variable in a learned clause (and every variable resolved
  on during analysis) gets its activity bumped by an increment that grows
  by `1/0.95` per conflict, which is the same as decaying every other
  activity by 0.95. When an activity exceeds `1e100`, all activities are
  rescaled. The next decision is the unassigned variable of highest
  activity, taken from a binary max-heap keyed on activity.
- **Phase saving.** A decision assigns the polarity the variable last had.
  Initially every variable is decided `false`, matching MiniSat.
- **Luby restarts.** The `i`-th restart happens after
  `100 · luby(i)` conflicts, where `luby` is 1, 1, 2, 1, 1, 2, 4, 1, 1, 2, 1,
  1, 2, 4, 8, …. Restarts backtrack to level 0 and keep learned clauses.
- **Clause database reduction.** When the learned clauses outnumber a limit
  (initially `max(n_clauses / 3, 2000)`, growing by 10% on each reduction),
  half of them are deleted, worst first: higher LBD, then lower activity.
  Clauses that are currently the reason for an assignment are kept, as are
  binary clauses and clauses with LBD ≤ 2 ("glue" clauses).

### 4.6 Assumptions

`solve_with_assumptions(&[Lit])` solves the formula plus the assumptions,
without adding them as clauses. Assumption `i` occupies decision level
`i + 1`, before any free decision. If an assumption is already false when
its turn comes, the formula plus the assumptions is UNSAT, and the solver
computes the **failed-assumption core**: the subset of assumptions that,
by following reasons back from the falsified assumption, forced it
(MiniSat's `analyzeFinal`). `failed_assumptions()` returns that subset,
sorted. The clause made of their negations is implied by the formula.

Learned clauses never mention an assumption as a fact, only as a literal,
so they stay valid for later calls with different assumptions. That is what
makes the solver incremental: BMC and k-induction (VER18) add clauses and
change assumptions between calls and keep everything learned so far.

### 4.7 Adding clauses

`add_clause` is allowed only between solves (the solver backtracks to
level 0 first). At level 0 it:

1. sorts and deduplicates the literals;
2. drops the clause if it contains `x` and `¬x` (a tautology) or a literal
   already true at level 0;
3. removes literals already false at level 0;
4. records UNSAT if nothing remains, assigns the literal if one remains,
   and otherwise stores and watches the clause.

The original clause, before steps 2–3, is what VER11's checker sees, and
each simplified clause is RUP with respect to it.

### 4.8 Budgets and results

`solve` returns `Sat`, `Unsat` or `Unknown`. `Unknown` is returned only
when one of the caller's budgets runs out: the **conflict budget**
(`set_conflict_budget`) or the **work budget** (`set_work_budget`, §10.6).
With no budget the solver always terminates with `Sat` or `Unsat`. CDCL is complete, and restarts do
not break termination because the Luby sequence is unbounded and learned
clauses are never all deleted.

After `Sat`, `model()` returns one Boolean per variable, and the model
satisfies every clause ever added. After `Unsat` under assumptions,
`failed_assumptions()` returns the core. After a plain `Unsat`, the solver
stays UNSAT: every later call answers `Unsat` at once.

## 5. DIMACS CNF

The reader accepts the DIMACS CNF format of the SAT competitions:

```
c comment lines start with c
p cnf <vars> <clauses>
1 -3 0
2 3 -1 0
```

It is strict, because a solver whose input is misread proves the wrong
thing:

- Exactly one `p cnf` header, before any clause.
- Literals are integers with absolute value in `1..=vars`. Values outside
  that range are an error, not a silent extension.
- A clause may span lines and ends at `0`. A trailing clause without `0` is
  an error.
- The number of clauses must equal the header's count.
- A line containing only `%` ends the input (the SATLIB convention); what
  follows it is ignored.
- The header's variable count is capped by the caller's limit (default
  `1 << 20`, never more than `1 << 31`, the encoding's ceiling), so a
  hostile header cannot force a huge allocation. The cap is set by what a
  solver loaded from the formula spends: about 100 bytes per declared
  variable, so 2^20 variables cost about 100 MB.
- Errors carry the 1-based line number.

The writer emits a header and one clause per line, terminated by `0`. A
round trip (`write` then `read`) gives the same clauses in the same order.

## 6. Proof emission (DRAT, VER11 §4)

The solver keeps an optional **in-memory** DRAT proof (`enable_proof`,
`from_cnf_with_proof`; read it with `proof`, `take_proof`). It is a
`Vec<ProofStep>` of `Add(clause)` and `Delete(clause)`; `to_drat_text` renders
the DRAT text that `drat-check` reads. Nothing is written to disk: the solver
stays free of capabilities. Every change to the clause database is one of
these events, each produced in exactly one function:

| Event | Where | Logged as |
|---|---|---|
| original clause | `add_clause` | **not logged**: the checker reads it from the formula |
| clause shortened by level-0 facts (§4.7) | `add_clause` | `Add(shortened)` (RUP) |
| empty clause from `add_clause` (empty after simplification, or a unit that contradicts level-0 facts) | `add_clause` | `Add([])` |
| learned clause, units included | `record_learnt` | `Add(clause)` (RUP) |
| learned clause deleted | `reduce_db` | `Delete(clause)` |
| conflict at level 0 | `search` | `Add([])` |

Original clauses are never deleted from the proof, and a deleted clause is
always a learned clause of length >= 3 with LBD >= 3 that is not currently a
reason.

### 6.1 Testable contracts

These are the statements the test suite (`tests/audit.rs`, `tests/drat.rs`,
`tests/differential.rs`) holds the implementation to. The solver is untrusted
search; the first four are exactly what makes it safe to trust `drat-check`'s
verdict on its output.

**C1. Sat is sound.** If `solve`/`solve_with_assumptions` returns `Sat`,
`model()` has one value per variable that exists at that moment, satisfies
every clause added so far (as given, before simplification), and makes every
assumption true.

**C2. Unsat comes with a checkable proof.** If the solver was created with
proof logging enabled before the first `add_clause`, and `solve()` returns
`Unsat` with no assumptions (or `is_ok()` is false), then
`DratProof::to_drat_text()` is accepted by `drat_check::check` against the
DIMACS text of **every clause ever added, in the form given**. This holds across
incremental `add_clause`/`solve` sequences and after earlier `Unknown` and
`Sat` answers, as long as the proof was not taken (`take_proof`) in between.
The proof also stays accepted when every `Delete` step is removed.

**C3. Unsat under assumptions is a claim about formula and assumptions, and
is certifiable.** `failed_assumptions()` is a sorted, duplicate-free subset of
the assumptions, and formula AND failed core is unsatisfiable. Appending the
core as unit clauses to the formula and `0` to the proof makes `drat-check`
accept: the proof so far is not a refutation of the formula alone (the formula
may be satisfiable), but it carries every lemma the conflict needed.

**C4. Unknown is honest.** `Unknown` is returned only when a budget ran out.
Whatever the budget, the answer is never a wrong `Sat` or `Unsat`, the solver
remains usable, and removing the budget and solving again gives the correct
answer. A proof recorded across the `Unknown` call is still accepted (C2).

**C5. Complete on small instances.** With no budget every call terminates with
`Sat` or `Unsat`, agreeing with exhaustive enumeration (formula AND
assumptions). Exhaustively tested: every formula of at most 3 clauses of width
<= 2 over 3 variables (including empty, duplicate-literal and tautological
clauses), every formula of at most 4 clauses of width <= 3 over 3 variables,
and every formula of at most 3 clauses of width <= 2 over 4 variables.

**C6. Determinism.** The solver has no randomness, clock, address-dependent
or hash-ordered behaviour. The same sequence of `add_clause`, budget and solve
calls gives the same results, the same model, the same `Stats` and
byte-identical proof text.

**C7. Budget semantics.** The conflict budget is a running countdown shared by
all later `solve` calls until reset (`set_conflict_budget(Some(n))` replaces
the remaining count). Each analysed conflict spends one. `Some(0)` yields
`Unknown` at once for any formula that is not already known UNSAT (a
known-UNSAT solver answers `Unsat` before looking at budgets). The work budget
is relative to the work done so far when it is set.

**C8. Model/core hygiene.** `model()` is empty after any answer but `Sat`;
`failed_assumptions()` is empty after any answer but `Unsat`, and empty after
an `Unsat` that does not depend on assumptions.

**C9. DIMACS.** Every malformed input listed in section 5 returns `Err`, never
panics and never allocates more than the header cap allows.

## 7. The `SatTactic` adapter

`constraint-engine::sat::SatTactic::solve(&[Predicate], &[String])` keeps
its signature and its behaviour:

1. The depth guard and CNF conversion (`to_cnf`) are unchanged. A new
   size guard runs before them: a predicate whose negation normal form
   would exceed `MAX_NNF_NODES` (100 000) nodes is answered `Unknown`
   without being expanded. `Iff` copies both operands, so nested `Iff`s
   grow as 2^k, and that work happened before any existing budget applied.
2. Each distinct variable name gets a `cdcl_sat::Var`, in first-appearance
   order.
3. The clauses are added; the solver runs with a conflict budget of
   1 000 000 (the old DPLL's node budget had the same order of magnitude),
   and `Unknown` carries the reason.
4. The model maps each requested `bool_var` to its value, and variables
   that appear in no clause default to `false`, as before.

The DPLL moves to `constraint-engine/src/sat/dpll_oracle.rs`, compiled only
under `cfg(test)`, and serves as a second oracle in the adapter's tests.

The Cargo description of `constraint-engine` is corrected: it now says the
SAT tactic is CDCL *via `cdcl-sat`*, which is true.

## 8. Verification

The solver is not part of the trusted base. VER11's checker is. Even so, a
solver that is wrong wastes proofs, so it is tested heavily.

| Test | What it shows |
|---|---|
| Brute-force cross-check: thousands of random 3-CNF and mixed-width formulas, 1–12 variables, densities around the 4.26 phase transition | SAT/UNSAT agrees with exhaustive enumeration |
| Every SAT answer | The model is re-checked against every clause |
| Assumptions vs brute force | Same answer as adding the assumptions as units; the failed core is a subset of the assumptions, and formula ∧ core is UNSAT by brute force |
| Incremental: random clause batches added between solves | Each answer agrees with brute force on the clauses so far |
| Pigeonhole PHP(n+1, n), n = 1…7 | Famously hard for resolution, always UNSAT |
| Old DPLL cross-check (in `constraint-engine`) | Random predicate formulas give the same answer from both |
| DIMACS reader: every error path; writer round trip | §5 holds |
| Conflict budget | `Unknown` comes back and the solver stays usable |
| All existing `constraint-engine` and `constraint-vm` tests | The adapter preserves behaviour |

Line coverage of `cdcl-sat` is held at 95% or more.

A development-time differential run against an external solver on SAT
competition benchmarks is allowed by VER00 §7.5 as an oracle, offline only.
No external solver is a dependency, a dev-dependency or a CI step.

## 9. Capabilities

`cdcl-sat` is pure computation: no filesystem, network, process,
environment or clock access. The DIMACS reader takes a `&str`; reading the
file is the caller's job and the caller's capability. Its
`required_capabilities.json` lists no capabilities.

## 10. Divergences

Recorded while implementing:

1. **`Var::positive` / `Var::negative`**, not `pos` / `neg`. Clippy rejects a
   method called `neg` because it reads like `std::ops::Neg::neg`.
2. **`add_clause` propagates units immediately.** §4.7 does not say when a
   level-0 unit is propagated. Propagating at once means the next clause is
   simplified against everything already implied, and a contradiction among
   units is found when it is added.
3. **A `constraint-core` fix rides along.** The adapter's differential test
   found that `Predicate::to_nnf` left `¬(a ⇔ b)` wrapped around a compound,
   so `to_cnf` returned a formula that was not CNF, and the tactic answered
   `Unknown`. `to_nnf` now rewrites it to `(a ∨ b) ∧ (¬a ∨ ¬b)`.
4. **The distributive CNF conversion keeps its clause cap** (10 000 clauses).
   Formulas past the cap still answer `Unknown`, as before. The Tseitin
   encoding of VER10 removes the cap; re-pointing `SatTactic` at it is a
   follow-up to VER10, not part of this spec.
5. **Security review hardening.** The DIMACS default cap went from 2^24
   to 2^20 and limits are clamped to 2^31. The adapter gained the NNF size
   guard of §7, which bounds an exponential case that predates this spec.
   `add_clause` documents that variable indices are the caller's to bound.
6. **A work budget** (`Solver::set_work_budget`, added with VER10). It
   counts watch-list visits, because a conflict budget alone does not bound
   time on formulas with millions of clauses.
7. **DRAT proof emission landed inside VER09's crate** (VER11 §4), as an
   in-memory `DratProof` rather than a text sink as §6 originally described;
   §6 and §6.1 now state what is logged and what is promised. The audit that
   wrote §6.1 found the old §6 table slightly wrong: a learned clause is also
   logged as a unit, and `add_clause` logs shortened clauses and the empty
   clause.
8. **`failed_assumptions`.** §4.6 says "sorted"; the code also removes
   duplicates, includes the falsified assumption itself, and returns both `x`
   and `¬x` when both were assumed. A level-0-false assumption `p` gives the
   core `[p]` (not empty): formula AND `p` is UNSAT although the formula alone
   is not. An empty core means the formula is UNSAT without assumptions.
9. **Budget scope.** §4.8 said "conflict budget" only and did not say that the
   budget persists across calls. §6.1 C7 fixes both.
10. **Restart/phase/reduction constants** (`RESTART_UNIT` 100, `VAR_DECAY`
    0.95, `CLAUSE_DECAY` 0.999, initial phase false, `max(n_clauses/3, 2000)`
    learned-clause limit growing 1.1x) match §4.5. Restarts are part of the
    contract only through C6 determinism; tests do not pin the numbers.
