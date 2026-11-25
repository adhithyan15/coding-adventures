# VER34 — The Constraint VM as Warrant's SMT solver

**Status:** Draft for review (2026-11-24). Spec only; no code in this change.
**Layer:** Proof machinery. Extends LANG24 (the Constraint VM) from "a generic
executor with three small tactics" to "the solver Warrant uses".
**Builds on:** LANG24, LANG23, VER09 (`cdcl-sat`), VER10 (bit-blasting), VER11
(DRAT), VER11a (kernel; §9.4 to §10), VER32 (PIF), VER31 (claim ladder).
**Written alongside:** VER33 (Warrant as the algorithm definition language),
drafted in parallel. Where this spec names a VER33 rung or id, the name is
provisional until VER33 lands (section 12).
**Owner direction recorded here:**

> 2026-11-24: "We have our own constraint VM. I would love to build that out as
> a full fledged Z3 equivalent so that Warrant can use that."
>
> 2026-11-24 (structural rule, section 2): "Any big problem can be decomposed
> into smaller problems… we should be able to break it down into smaller and
> smaller chunks until it can fit in as instructions in our constraint VM
> architecture."

---

## 0. The design in one page

Warrant is becoming the repository's one algorithm definition language. From one
Warrant file the repository wants tests, fuzzing, property tests, solver-backed
checks, kernel proofs and idiomatic code in every lane. Several of those need a
decision procedure for "is there an input where this goes wrong?". Today the
repository has a Constraint VM that answers that for Booleans, bit-vectors and a
bounded corner of integer arithmetic. This spec says how it grows into a solver
that covers the theories real code needs, **without** becoming a 500,000-line
monolith and **without** ever asking anyone to trust it.

| # | Decision | Section |
|---|---|---|
| D1 | **Z3-class is defined by measurement**, not by line count or feature list: solved-percentage within a time budget, per logic, on pinned SMT-LIB families, with zero wrong answers. Targets are proposals until a baseline is measured (CVM-S0) | 1 |
| D2 | **The solver is built from small units** that fit LANG24's layers: an opcode, one tactic or theory component, a preprocessing pass, a certificate emitter, a kernel reconstructor, a combinator or a harness. Each unit is about 100 to 400 lines plus tests, one PR. A unit that does not fit is split | 2 |
| D3 | **Power comes from composition.** A small set of tactic combinators (sequence, try-else, budget, probe-conditional, theory combination, push/pop) assembles the units, as in Z3's tactic framework. The combinators are units too | 2.4, 4.7 |
| D4 | **CDCL(T) on `cdcl-sat`**, extended with a theory-hook interface. Offline lazy solving first (works with today's assumption API); online hooks second. The engine's own SAT adapter is a thin wrapper and stays one | 4.2 |
| D5 | **Theory combination by model-based theory combination**, with eager reduction (Ackermann) where a theory is not stably infinite or the instance is small. Nelson-Oppen is the textbook case and is not implemented as such | 4.4 |
| D6 | **The solver is untrusted, always.** SAT is checked by evaluating the model in an independent evaluator. UNSAT counts toward a VER31 `proved` claim only as a **certificate reconstructed into a kernel proof**, delivered as PIF | 6 |
| D7 | **Certificates decompose like the solver.** Every unit that can contribute to UNSAT emits one **fragment** of a fixed kind. Each fragment kind has one small reconstructor. The trusted proof path grows unit by unit with the solver | 6.2, 6.4 |
| D8 | **No new claim rung.** An UNSAT without a reconstructed certificate is `solver-attested` evidence: it can refute a claim (via a checked model) and can find bugs, but caps a claim at `tested`. VER31 gets a small clarifying amendment, not a rung | 6.6 |
| D9 | **Z3 and cvc5 are oracles in CI, never dependencies.** Their answers are cross-checked by our own evaluator and certificate checker; neither is in any trust path | 9 |
| D10 | **Rust only.** Other lanes call the solver as a binary or WASM module. The certificate, not the solver, is what must be portable | 10.4 |

**What this costs.** Honestly: tens of thousands of lines of Rust (section 2.5
estimates 45,000 to 65,000 for the program through M4, plus 5,000 to 8,000 lines
of kernel-side library), about 165 PRs, and a standing validation budget (section
9). M1 (bit-vectors with certificates) is a few weeks of focused work because the
circuits, the SAT core and the DRAT kernel already exist. M3 and M4 are months.
Nothing here promises Z3's speed; section 1 says what is promised.

**What it buys.** Warrant's solver-backed rungs stop being limited to what a
bounded enumerator can decide; every UNSAT that matters is checked by the kernel
that already checks everything else; and the whole thing is a set of small,
separately testable pieces a new reader can learn one at a time.

---

## 1. Purpose and honest scope

### 1.1 What "Z3-class" means here

Z3 is about 500,000 lines of C++, built by a research group over fifteen years,
with dozens of theory solvers, hundreds of rewrite rules, and tuning against
every benchmark library. cvc5 is of similar size. We will not match that
breadth, and saying "parity with Z3" would be a claim nobody could check.

**The target is defined by consumers and measured by benchmarks.**

1. **Theory coverage:** the theories Warrant, LANG23, `lang-refinement-checker`,
   the property-test generator, the package resolver and LEAN00's `bv_decide`
   actually need (section 4 lists them in priority order). Not every theory in
   SMT-LIB.
2. **Solving power:** on pinned SMT-LIB benchmark families for each logic, the
   fraction solved within a fixed time, compared with Z3 and cvc5 on the same
   machine.
3. **Soundness:** zero wrong answers, ever. A wrong SAT is caught by model
   evaluation; a wrong UNSAT is caught by the certificate check; a wrong answer
   found by the fuzzer or the differential test is a stop-the-line finding.

**The metric.** For a logic `L` and a time limit `T`:

    R(L, T) = (benchmarks in the pinned set solved by the CVM within T)
              / (benchmarks in the same set solved by Z3 within T)

measured on the same hardware, one thread each, with a deterministic resource
budget standing in for wall-clock time in CI (section 4.8). A milestone gate is
"`R(L, T) >= r`, zero wrong answers, and at least `c`% of UNSAT answers
certified". We also report PAR-2 (the SMT-COMP style score: solved time, with
twice the timeout charged for each unsolved one) so that a solver that is slow
on everything is visible, not hidden behind a solved-count.

**Non-goals, stated so nobody assumes them:** matching Z3's optimiser
(`z3.Optimize`, MaxSAT), its Horn-clause engine (Spacer), its fixed-point engine,
its tactic zoo, its parallel mode, its API surface, or its performance on
industrial hard-combinatorial instances. Any of these can be added later as
units if a consumer needs them (section 4.9).

### 1.2 Targets are proposals until a baseline exists

Nobody has measured the current stack against Z3. The first unit (CVM-S0) runs
the existing `constraint-vm` on the QF_BV, QF_LIA and QF_Bool SMT-LIB families
and Z3 and cvc5 on the same machine, and records the table. Every number in
section 10 is then re-based on it. Until then the numbers are guesses with a
stated shape (for example "half of what Z3 solves at 60 seconds on the
Warrant-near subset of QF_BV"), chosen so that a gate is a real test and not a
vague hope.

### 1.3 Who the solver is for

| Consumer | Question it asks | Needs |
|---|---|---|
| Warrant claim engine (VER33) | Input generation: "give me inputs at the boundary of this precondition" | SAT with models; BV, LIA, ADT |
| Warrant bounded checks | "Does this model equal this code for all inputs up to N?" | UNSAT with certificate; BV, arrays, UF |
| Verification conditions (VER11a §9, VER33) | "Is this implication valid?" | UNSAT with kernel-checked certificate; BV, LIA/LRA, EUF, arrays, ADT, some quantifiers |
| LANG23 refinement checker | "Does this predicate follow from these guards?" | Fast `PROVEN_SAFE`/`PROVEN_UNSAFE`/`UNKNOWN`; LIA, BV |
| Property-test generator (VER05) | "A value satisfying this invariant" | SAT with models, many fast calls, push/pop |
| Package resolver | Version constraints | SAT/LIA with unsat cores |
| LEAN00 L4c | `bv_decide` for the in-house Lean | BV, LRAT proof terms |

The first four are the reason this spec exists. The last three are why the
solver is a generic VM and not a Warrant-private tool.

---

## 2. The structural rule: everything is a small unit

### 2.1 The rule (owner, 2026-11-24)

> Any big problem can be decomposed into smaller problems… we should be able to
> break it down into smaller and smaller chunks until it can fit in as
> instructions in our constraint VM architecture.

In this spec that is a **rule about the shape of the solver**, not a remark. The
rule has teeth:

- **No monolithic component is specified.** "The LIA solver" is not a unit. A
  tableau, a pivoting rule, a bound propagator, a branch step, a cut generator
  and a Farkas extractor are units.
- **Every unit fits one of seven slots** in the LANG24 layering (2.2).
- **Every unit is one PR**, sized to be read in one sitting (2.3).
- **The backlog is the decomposition tree** (section 11): the rows are the leaf
  units, grouped under milestone headings. There is no row for a thing that is
  not a leaf.
- **If a unit does not fit the size target, it is split.** A PR that exceeds the
  target needs a recorded reason in its description; two such exceptions in one
  milestone mean the decomposition below it was wrong and gets revised here.

### 2.2 The seven slots

LANG24 has four crates: `constraint-core` (data), `constraint-instructions` (the
IR), `constraint-engine` (tactics), `constraint-vm` (the executor). VER34 adds
no crate to that stack. It fills the stack with units of these kinds:

| Slot | What it is | Lives in | Example |
|---|---|---|---|
| **INSTR** | One VM opcode, or one operand form, with its parser, printer, validator and executor arm | `constraint-instructions` + `constraint-vm` | `check-sat-assuming`; `declare-datatype` |
| **TACTIC** | One theory component or one search step: a function from a goal to subgoals or a verdict | `constraint-engine` | The tableau; congruence closure; one cut generator |
| **PASS** | A preprocessing rewrite: a goal to an equivalent (or equisatisfiable) goal, plus a way to map models back | `constraint-engine` (`pass/`) | BV word-level rewrites for shifts; constant folding |
| **CERT** | A certificate emitter: given a tactic's run, a *fragment* in the trace format (6.3) | beside its tactic | The Farkas coefficients of a conflicting row |
| **RECON** | A kernel-side reconstructor: a library lemma set in `Warrant.SMT` (or the native equivalent) and a small untrusted program that turns one fragment kind into PIF terms | `warrant-smt-recon` (new, untrusted, one module per kind) + kernel library | `blast_add_sound` and the `blast` fragment reconstructor |
| **COMB** | A tactic combinator or the combination glue | `constraint-engine` (`strategy/`) | `then`, `or_else`, `with_budget`, model-based theory combination |
| **HARN** | A validation harness | `constraint-engine/tests`, `cvm-bench` | A metamorphic relation; the oracle adapter |

A unit is named by its slot and a one-line interface. Section 5 gives every
theory as a tree of such units; section 11 turns the trees into rows.

### 2.3 The unit card

Every backlog row is a unit and must be able to fill in this card. A row that
cannot is not yet a unit.

    Unit            CVM-nnn
    Fits in         one of the seven slots
    Interface       inputs and outputs, as VM instructions or as one trait call
    Size            about 100 to 400 lines of source (tests and comments extra)
    Tests           its own: unit, property, and (for TACTIC/PASS) a soundness
                    check against the independent evaluator
    Certificate     the fragment it emits, or the reason it needs none
                    (PASS and TACTIC units that preserve models need none; units
                    that can contribute to UNSAT always need one)
    Depends on      other units by id

The card also gives the **rejection test** for the unit: the case it must refuse
or answer `Unknown` on. A unit that can only succeed has not been specified.

### 2.4 Composition is the scaling argument

"Z3 is 500,000 lines" is true and misleading. What makes Z3 strong is not one
large algorithm; it is many small ones (a rewrite rule, a propagator, a
heuristic) combined by a **strategy layer** that decides what to run, in what
order, under what budget. A tactic is small. The combination is where the
power is, and combinations can be written, tested and tuned without touching the
parts.

Our strategy layer is five combinators, each one unit (section 4.7):

| Combinator | Meaning | Why it is enough |
|---|---|---|
| `then(a, b)` | Run `a`; run `b` on each subgoal it leaves | Pipelines: simplify, then blast, then SAT |
| `or_else(a, b)` | Run `a`; if it fails or answers `Unknown`, run `b` on the original goal | Fallbacks: cheap try, then the heavy method |
| `with_budget(a, n)` | Run `a` with a deterministic step budget; over budget is a failure | Makes `or_else` safe and CI reproducible |
| `when(probe, a, b)` | Choose by a property of the goal (has quantifiers; widest BV; mix of theories) | One default strategy per logic |
| `smt(theories)` | The CDCL(T) loop over a registered set of theories, with model-based theory combination | The one non-trivial combinator, because search itself is a combination |

Push and pop are not a combinator; they are the VM's `PushScope`/`PopScope`,
implemented once in the engine as an undo trail (4.5) that every unit respects.

The fifth combinator is where "theory combination" lives, which is why section
4.4 is about it and not about any one theory.

### 2.5 What the decomposition costs, honestly

Estimates, by unit count and the size target, **before** any slip. Error bars
are a factor of two either way.

| Part | Units | Source lines |
|---|---|---|
| Foundations (instructions, meter, combinators, evaluator, harness) | ~18 | 4,000 to 6,000 |
| QF_BV (rewrites, blasting groups, SAT improvements, certificates) | ~38 | 9,000 to 13,000 |
| EUF, arrays, UFBV | ~20 | 5,000 to 7,000 |
| LRA, LIA | ~22 | 6,000 to 9,000 |
| Datatypes, quantifiers, nonlinear | ~26 | 7,000 to 10,000 |
| Strings, sequences, floating point (later) | ~25 | 8,000 to 12,000 |
| Kernel-side library (Lean-style source or generated PIF) | ~12 | 5,000 to 8,000 |

Roughly 165 units (the backlog rows). That is not a small program; it is a bounded one, because
each unit is reviewable and each milestone ships something that works.

---

## 3. Audit of the current Constraint VM (main at 45c025edf11)

Measured by reading each crate's README, CHANGELOG and source on this commit.
Line counts are source plus unit tests inside `src`.

| Area | What exists | Quality | Gap against section 4 |
|---|---|---|---|
| **Predicate AST, sorts** (`constraint-core`, 1,620 lines) | Bool, Int, Real, `BitVec(w)`, Array, uninterpreted sorts, `Apply` (UF), `Select`/`Store`, `Forall`/`Exists`, `Ite`, CNF conversion, `Rational` on `i128` | Good, well tested, hardened against hostile sizes | `Int` and `Rational` are `i128`: arbitrary-precision LIA is not possible. No datatypes, no strings, no FP, no `let`, no indexed identifiers as real terms (the BV tactic takes a flattened `Apply "_"` shape) |
| **IR** (`constraint-instructions`, 2,121 lines) | `declare-var`, `declare-fn`, `assert`, `check-sat`, `get-model`, `get-unsat-core`, `push`, `pop`, `reset`, `set-logic`, `set-option`; text round trip; scope balance check | Good | No `define-fun`, `check-sat-assuming`, `get-value`, `declare-datatype`, named assertions, no certificate or tactic opcodes |
| **Engine** (`constraint-engine`, 5,876 lines) | Three tactics (SAT, LIA, BV) and a dispatcher; `SolverResult::{Sat, Unsat, Unknown(String)}`; models for Bool, Int, Real, BV | Sound by contract (LANG24 §"LIA as built"); `Unknown` is the honest fallback | One tactic is chosen per query. **No theory combination** (BV mixed with Int is `Unknown`). Real, arrays, UF, quantifiers: `Unknown`. No strategy layer |
| **SAT tactic** | Adapter over `cdcl-sat`; 1,000,000-conflict budget | Good | Offline only; no theory hook |
| **LIA tactic** (`lia.rs`, 1,267 lines) | **Bounded enumeration**, not Cooper (the README still says Cooper; LANG24's divergence note is the truth). Exhaustive only for bounded variables; `Unknown` otherwise | Sound after the 2026-11 fix; deliberately incomplete | `x < 0 ∧ 2x = 4000` style unbounded problems are `Unknown`. No simplex, no branch-and-cut, no real Cooper. No certificate. Values are `i128` |
| **BV tactic** (`bv.rs` + `bit-blaster`, ~1,900 + `bit-blaster`) | Bit-blasting to an and-inverter graph, Tseitin, `cdcl-sat`; every UNSAT passes `drat-check` or becomes `Unknown`; exhaustive lowering audit at widths 1 to 8, seeded checks at 16/32/64, 18 mutants killed; caps of 250,000 AIG nodes and 2,000,000 gate calls; refusals before building oversize multipliers and dividers | **The strongest part.** Faithfulness is checked, hostile inputs are bounded | No word-level preprocessing (every query is blasted as written). No certificate beyond DRAT: bit-blasting correctness is the open (c) id `tcb.bitblast.encoding-correct`. No BV with Int, no UF over BV |
| **VM** (`constraint-vm`, 1,364 lines) | Scope stack; limits (10,000 instructions, 100 scope depth, 10,000 assertions); `GetModel` | Good limits | **`GetUnsatCore` returns an empty core** (stub; "requires clause provenance"). `PushScope` clones the whole engine, so push/pop is **correct but not incremental**: nothing learned survives a pop, and nothing is reused across checks |
| **SMT-LIB** (`smt-lib-format`, 1,937 lines) | Strict v2 reader/writer onto `Program`; hostile-input hardening (`Rational::new(_,0)`, `i128::MIN`) | Good | No indexed identifiers, `define-fun`, `let`, datatypes, `check-sat-assuming`; no Z3 bridge (LANG24 "PR 24-H" never landed) |
| **SAT core** (`cdcl-sat`, 4,372 lines) | 2-watched literals with blockers, first-UIP and recursive minimisation, VSIDS, phase saving, Luby restarts, LBD clause reduction, **assumptions, failed-assumption cores, incremental clauses**, conflict budget, DRAT logging; differential, brute-force and PHP/XOR corpora | Good for a from-scratch core; **never benchmarked against MiniSat/CaDiCaL-class solvers or SAT-competition instances** | **DRAT only, no LRAT** (KRN-009 is the planned fix). No theory hook. No inprocessing (vivification, BVE, subsumption), no preprocessing |
| **DRAT kernel** (`drat-check`, 2,601 lines) | Forward RUP/RAT checker, bounded work and variable count, negative corpus | Good; a few hundred lines in `check.rs` | Forward checking is slow on large proofs. It is a *trusted-by-us* tool (`tcb.drat.checker-correct`), not a kernel proof. No LRAT checker |
| **Other consumer** (`adj-constraint-solver`, 3,556 lines) | Exact linear equalities via `cas-solve`; feasibility through `LiaTactic` with a **minimal infeasible subset** by deletion | Good for its slice | Its IIS comes from deletion, not from the solver. It will move onto real unsat cores (CVM-004) |
| **Tests** | `qfbv` corpus of 15 hand-written `.smt2` files with brute-force and evaluator cross-checks; random small problems; LIA soundness tests | Honest and independent | **No SMT-LIB benchmark runs. No differential test against Z3/cvc5** (the READMEs say "CI runs the SMT-LIB suite"; it does not). No fuzzing for soundness. No performance tracking |
| **Proof output** | DRAT for BV UNSAT inside the engine | Not exported | No certificate format, no kernel link; the engine returns only `Unsat` |

**Net.** The repository has a sound, hardened, small Bool/BV/bounded-LIA solver
and a good SAT core. It does not have: theory combination, a strategy layer,
real LIA/LRA, EUF, arrays, datatypes, quantifiers, unsat cores, real
incremental solving, certificates beyond DRAT, or any measurement against an
industrial solver. The decomposition in section 5 is written to close exactly
these gaps and no others.

**Finding to fix first (found in this audit; CVM-001).** `constraint-engine`'s
README says the LIA tactic is Cooper's algorithm and "Complete"; the source and
LANG24 say it is bounded enumeration, incomplete by design. A false completeness
claim in a README is exactly the kind of statement VER31 distrusts. The README
is corrected before any other work, and the correction is a lesson.

---

## 4. Architecture

### 4.1 How the four LANG24 layers carry this

    constraint-core          data: terms, sorts, values (bignum), models,
                             goals, fragments (the certificate record types)
          |
    constraint-instructions  IR: the opcodes (existing opcodes + the new ones of section 8.2)
          |
    constraint-engine        tactics, passes, theory components,
                             combinators, the CDCL(T) loop,
                             certificate emitters
          |
    constraint-vm            executes programs; scopes; limits; asks the engine
                             for a verdict, a model, a core or a certificate
          |
    (untrusted, separate)    warrant-smt-recon: fragments -> PIF
          |
    (trusted)                pif-check (VER32): the kernel

No crate is added to the stack except `warrant-smt-recon`, which sits outside it
because it is not part of solving: it consumes a finished certificate. That
separation is deliberate. The solver crates never import the reconstructor, so
a bug in reconstruction cannot change an answer, and a bug in the solver cannot
be hidden by the reconstructor (the kernel will reject the proof).

### 4.2 Decision: CDCL(T) on `cdcl-sat`, in two stages

**Decision.** The Boolean engine is `cdcl-sat`. The engine's `SatTactic` stays a
thin adapter; the dispatcher, the passes and the theory components are built
around it. We extend `cdcl-sat` in two steps, each a unit:

1. **Offline lazy (stage 1).** Abstract the formula's theory atoms to Boolean
   variables, ask the SAT solver for a model, ask the theory components whether
   that set of atoms is consistent, and if not add a **theory lemma** (a clause
   explaining the conflict) and solve again, using today's incremental clause
   addition and assumption API. This needs no change to `cdcl-sat` and works for
   every theory. It is slower than online solving on conflict-heavy instances
   because every round restarts propagation from the clauses.
2. **Online hook (stage 2).** A trait in `cdcl-sat` with `assert(literal)`,
   `propagate() -> implied literals + reasons`, `backtrack(level)` and
   `final_check()`, called from inside `propagate` and at full assignment. The
   same theory components implement it. Lemmas still land as clauses, so the
   proof story does not change.

**Alternatives considered.**

- *The engine's own SAT.* The engine has none: its SAT tactic is an adapter and
  its DPLL survives only as a test oracle. Writing a second CDCL would duplicate
  VER09's tested core and its DRAT logging.
- *Eager reduction only (everything to SAT).* This is what BV does, and it is
  the right choice for BV. It is the wrong choice for unbounded integers and
  for arrays and UF at scale.
- *Wrap Z3 or cvc5.* Rejected by the owner's direction ("our own"). Section 9 uses
  them as oracles only.

**Why two stages.** Stage 1 lets EUF, LRA and arrays ship before the hook exists,
each measured honestly; stage 2 is an optimisation measured against stage 1
(CVM-S0 records the cost of offline solving). A theory component is written once
against the stage-2 trait and driven by a stage-1 loop until the hook lands.

### 4.3 The theory component interface

A theory component (a TACTIC unit that implements the trait) is small by design:
it owns its own state and speaks to the engine only through these calls.

| Call | Meaning |
|---|---|
| `register(atom) -> id` | Tell the component about an atom it owns (it owns the atoms whose head symbol is in its signature) |
| `assert(id, polarity, level)` | The SAT solver assigned this atom |
| `propagate() -> [(atom, polarity, explanation)]` | Atoms the component can now deduce; each with a reason (a set of already-asserted atoms) |
| `conflict() -> Option<explanation>` | The asserted atoms are inconsistent; the explanation is an inconsistent subset, which becomes a clause |
| `backtrack(level)` | Undo to a decision level (the undo trail, 4.5) |
| `final_check() -> Consistent / Lemmas(Vec<Clause>) / Unknown` | At a full Boolean assignment: consistent (with a model), or new lemmas (splits, cuts, instantiations), or give up |
| `model() -> partial model` | A value for each symbol the component owns |
| `fragment(explanation) -> Fragment` | The certificate for one conflict or lemma (6.3) |

The last call is the certificate hook: **a component that cannot explain a
conflict with a fragment must say `Unknown` rather than return an unexplained
conflict in certified mode.** That one rule is what keeps D7 honest.

### 4.4 Decision: theory combination

**Problem.** A program verification condition mixes theories: BV arithmetic with
uninterpreted functions over it, arrays of integers, datatypes holding integers.
A combination procedure decides the mix from deciders for the parts.

**Decision.** **Model-based theory combination** (de Moura and Bjorner's, as in
Z3), implemented as the COMB unit `smt(theories)`:

1. *Purify* terms so that each atom belongs to exactly one theory (PASS).
2. Each theory solves its part and produces a partial model.
3. For every pair of **shared** terms the theories have not been told to equate
   or distinguish, but whose values **coincide in the model of one theory**, the
   engine adds the equality `s = t` as a new Boolean atom, to be decided like any
   other. The loop ends when no unforced coincidence remains.

Because the new equalities are ordinary atoms, **the proof of a combined problem
is the proof of a propositional problem plus per-theory lemmas.** Combination
contributes no certificate fragment of its own, which is a large part of why
this choice is better for us than the alternatives.

**Alternatives.**

- *Nelson-Oppen with equality propagation.* Complete for stably-infinite theories
  with disjoint signatures; needs each theory to enumerate all implied equalities
  (convex) or case-split (non-convex). LIA is non-convex; that case split is
  what model-based combination does lazily and more cheaply.
- *Eager Ackermannisation.* Replace each UF application by a fresh variable and
  add all pairwise congruence constraints. Exact and trivially certifiable, but
  quadratic. We keep it as a PASS (`ackermann`) for **QF_UFBV** and for small
  instances, because **BV is not stably infinite** (a finite domain can force
  equalities among shared variables), so Nelson-Oppen's completeness theorem
  does not apply and bit-blasting handles BV directly. For BV the rule is
  *blast, with UF eliminated by Ackermann*.
- *Delayed theory combination.* A published variant of the same idea; we get
  its behaviour from stage 2 of 4.2.

### 4.5 Incremental solving, models and unsat cores

**Push and pop.** Today `PushScope` clones the engine. The target: an **undo
trail** in the engine, so that `push` records a mark and `pop` undoes assertions,
theory state and learned lemmas that depended on them. The practical design is
the one MiniSat and Z3 both use: every assertion made inside a scope is guarded
by a fresh **scope selector literal** (`s_k`); the clause is stored as
`(¬s_k ∨ C)`; the solver is run under the assumption `s_k` for every open scope;
`pop` retires `s_k` by adding the unit `¬s_k`. Learned clauses stay valid (they
carry the selectors they depend on); nothing is recomputed from scratch. This
uses `cdcl-sat`'s existing assumption support. Theory components implement
`backtrack` to the same marks. One unit (CVM-016), plus one test unit that runs
every other unit's tests inside a push/pop wrapper.

**Models.** A model is a total assignment for declared symbols, built from the
partial models of the theories and then **checked by an independent evaluator**
(CVM-008) before being returned. The evaluator is written from the SMT-LIB
definitions, shares no code with any tactic, and is the same one the fuzzer uses.
A model that does not evaluate every live assertion to true is a solver bug and
becomes `Unknown` plus an error record (fail closed; never a wrong SAT). `Value`
moves to arbitrary precision (`bignum-core`) and gains datatype values, array
values (finite maps with a default), and, later, strings and floats.

**Unsat cores.** Each named assertion (`(! t :named n)`) gets an assumption
literal; the SAT solver's failed-assumption set, minimised by deletion (cheap
with incremental solving), is the core. The stubbed `GetUnsatCore` becomes real
(CVM-003, CVM-004). Theory lemmas are explained by the named assertions they depend on,
which is also what the certificate needs, so core extraction and certificate
tracking share one dependency record.

### 4.6 Preprocessing and simplification

Preprocessing is a pipeline of PASS units run by `then`. Each PASS:

- takes a goal and returns a goal plus a **model converter** (how to extend a
  model of the output to a model of the input; for example, eliminating `x = t`
  by substitution records `x := t`);
- is **equivalence-preserving** or **equisatisfiable** (declared, and the
  difference matters to the certificate, 6.4);
- has a rule table: **each rewrite rule has a stable name**, and a rule that is
  not in the table cannot be applied in certified mode (6.2, kind `rw`).

Word-level rewrites are grouped one unit per operator family (5.1). The default
pipeline per logic is data (CVM-015) so that tuning is a PR to a table, not to
code.

### 4.7 The tactic layer as units

The strategy layer is its own small set of units:

| Unit | Interface | Notes |
|---|---|---|
| `Goal` and `Tactic` trait (CVM-009) | `apply(goal, &mut Meter) -> Result` where `Result` is `Reduced(subgoals, ModelConv, Fragment)`, `Sat(model)`, `Unsat(Fragment)`, or `Failed(reason)` | The one trait every TACTIC and PASS implements |
| `then` (CVM-010) | sequence; subgoals flow left to right | Fragments are chained |
| `or_else` (CVM-011) | try `a`; on `Failed` or `Unknown` run `b` on the same goal | Needs `with_budget` to terminate fast |
| `with_budget` (CVM-012) | wrap a tactic with a deterministic step budget (4.8) | Over budget is `Failed("budget")`, never a guess |
| `when` and probes (CVM-013) | branch on a goal property: `has_quantifiers`, `max_bv_width`, `num_theories`, `size` | Probes are pure functions of the goal |
| `smt` (CVM-014, CVM-245 to 247) | the combination loop of 4.4 | The only combinator with search in it |
| `check-sat-using` opcode (CVM-017) | a tactic expression in the program text, Z3-like: `(check-sat-using (then simplify (or_else (with_budget 1e6 bv_blast) smt)))` | So strategies are data a consumer can write |
| default strategy per logic (CVM-015) | a table | The tuning surface |

**Determinism.** Everything is single-threaded and deterministic: the same
input and budget give the same answer, byte for byte, which is what makes
`or_else` and CI comparisons meaningful. A parallel portfolio is a later
optional unit (4.9) and is never allowed to change a verdict, only the time.

### 4.8 Resource limits (fail closed)

The solver reads hostile input (VER27: nothing about a Warrant file is trusted
to be small). Every unit therefore draws from a **Meter** (CVM-006) that counts,
deterministically: conflicts, propagations, rewrite steps, AIG nodes, tableau
pivots, instantiation rounds, e-graph nodes, bignum bits, term-DAG nodes, and
recursion depth (all traversals iterative). Exhausting any counter is
`Unknown("<which counter>")`, a typed reason, not a timeout. Wall-clock limits
exist only as an external kill in the harness; **no verdict depends on a clock.**
Parsers cap term depth, bignum size, identifier length, number of declarations;
bit-vector widths are capped (today 512 for blasted multiplication; the table
moves to data). Allocation sizes come from the meter before allocation, not
after, as the `bvmul` fix (LH-006c) did. A unit that cannot state its worst-case
meter cost has not been specified.

### 4.9 Not now

Optimisation (MaxSAT), Horn clauses, parallel portfolio, interpolation, proof
minimisation, quantifier elimination beyond Cooper. Each would be a new set of
units if a consumer asks; none is on the path to Warrant.

---

## 5. Theories, in priority order, each as a tree of units

Priority is set by Warrant (section 1.3): fixed-width integers are the core of
real code, so bit-vectors come first; the specification side of a claim is
usually over `Int`, so integer arithmetic is second; EUF and arrays are what
loads, stores and calls turn into. Each subsection gives, in this order: the
decision procedure, completeness, the certificate fragment kinds, benchmark
families, and the unit tree. Ids are the backlog's leaf rows (section 11); sizes
are in the rows.

SMT-LIB benchmark family names below are the ones to **start from**; CVM-S0 pins
the exact subsets and release (SMT-LIB's library changes), and records the
choice.

### 5.1 QF_BV (first)

- **Procedure.** Word-level rewrites, then bit-blasting to an and-inverter graph,
  Tseitin, CDCL. Complete (a finite problem), but multiplication, division and
  wide shifts blow up: complexity is the circuit size, and the existing caps
  (4.8) turn a blow-up into `Unknown`.
- **Certificate.** Two parts. *Pre-blasting:* `rw` fragments (named rules,
  6.2). *Blasting and SAT:* a `blast` fragment (the term DAG and an encoding tag;
  the kernel recomputes the CNF itself, 6.4) plus an `lrat` fragment. The
  encoding's correctness is proved **once** in the kernel library; closes
  `tcb.bitblast.encoding-correct` and `tcb.drat.checker-correct` for kernel
  proofs (VER11a §9.5).
- **Families.** `sage`, `brummayerbiere`, `spear`, `stp`, `core`, `mcm`,
  `bmc-bv`, plus the repository's own: every logic-gate VC and ct-compare VC.
- **Units.**
  - *Word-level rewrites, one PASS per operator family* (CVM-100 to 108):
    constant folding; Boolean and `ite`; bitwise (`bvand/or/xor/not`);
    additive and multiplicative (`bvadd/sub/neg/mul`, multiplication by constant
    to shifts); shifts; `extract/concat/extend`; comparisons; division and
    remainder by constants; variable elimination by definition (`x = t`).
  - *One RECON per family* (CVM-111 to 119): the kernel lemmas for exactly that
    family's named rules and the `rw` reconstructor that applies them.
  - *Bit-blaster groups* (CVM-120 to 128): bitwise/`ite`/`=`; add/sub/neg;
    comparisons; shifts; multiplication; division and remainder;
    `extract/concat/extend`; Tseitin. These exist today in `bit-blaster` and
    `bv.rs`; the unit is the **refactor into a group with its own
    reference-order emission and tests**, not a rewrite.
  - *Certificate side of each group* (CVM-130 to 137): emit clauses in the
    *reference order* (below) and a differential test that they equal the
    kernel library's encoder on all small widths.
  - *Kernel lemmas per group* (CVM-140 to 148): `blast_add_sound` and so on,
    then `tseitin_sound`, then `blast_correct` (structural induction over
    terms).
  - *SAT side* (CVM-150 to 153): LRAT semantics and `rup_step_sound`;
    `checkLRAT_sound` with reflection; the `lrat` reconstructor; the term-route
    fallback for small proofs. `cdcl-sat` gains LRAT emission through KRN-009.
  - *Speed* (CVM-160, 161): bounded variable elimination with subsumption and
    vivification in `cdcl-sat`, both proof-logged.

**Reference order (decision).** The kernel library contains the encoder as a
function written in the kernel's language: the same circuits the Rust blaster
builds, with sharing by term-DAG node, in a fixed order. The Rust blaster has a
*certified mode* that emits the clauses in exactly that order. The LRAT proof is
a proof about those clauses, so the kernel recomputes the CNF (by reduction),
runs `checkLRAT` on it, and needs nothing from the solver but the LRAT bytes.
*Alternatives:* (a) the solver ships its CNF as data and the kernel checks it
equals the encoder's output up to renaming (needs a canonical form; harder to
make small); (b) the kernel trusts the Rust encoder (this is today's open id).
*Why this one:* the trusted claim is "this circuit means this term", and it
should be proved against a definition we can read, in the kernel. *Cost:* the
fast path (AIG hashing, rewriting) and the certified path differ; the certified
path is slower and its CNF larger. The fast path runs first and is the answer
for SAT. For UNSAT we measure the cost (CVM-174) before deciding whether certified
mode re-solves or the optimisations are lifted into the kernel as further
lemma units. This is the biggest performance risk in the spec.

### 5.2 EUF (uninterpreted functions and equality)

- **Procedure.** Congruence closure over a term DAG: a union-find with an
  **explanation forest** (each merge remembers why), a congruence table keyed by
  `(f, class of args)`. Polynomial; complete for QF_UF.
- **Certificate.** A conflict explanation is a chain of equalities: assumptions,
  symmetry, transitivity and `congrArg`. Fragment `euf`: a list of merge steps.
- **Families.** `QF_UF` (`eq_diamond`, `NEQ`, `PEQ`, `SEQ`), `QF_UFBV`
  (`Ackermann` style), plus Warrant's function-heavy VCs.
- **Units** (CVM-200 to 207): term DAG and union-find with explanations;
  congruence table; theory-hook adapter (assert, propagate, conflict); `distinct`
  and disequalities; the `euf` CERT emitter; the RECON (`Eq.trans`/`congrArg`
  chains); Ackermann PASS for UFBV; model construction (fresh values per class).

### 5.3 Linear real and integer arithmetic (LRA, LIA)

- **Procedure.** The **general simplex** of Dutertre and de Moura (the one in Z3
  and Yices): a tableau of slack variables, bounds per variable, Bland's rule for
  termination, bound propagation, conflict from a row whose bounds cannot all
  hold. Complete for LRA. For LIA: **branch and bound** and **cutting planes**
  (Gomory mixed-integer cuts) over the same tableau, with a GCD test and
  equality elimination; complete for QF_LIA given enough budget (branch and
  bound alone does not terminate on unbounded problems; cuts plus budgets make
  it an answer or `Unknown`). **Cooper's algorithm** for *quantified* LIA
  (Presburger), as a separate PASS, because it is exact where simplex is not.
- **Certificate.** *LRA:* a **Farkas certificate**: non-negative rational
  multipliers on the conflicting bound atoms whose weighted sum is `0 < 0` (or
  `0 <= -c`). Fragment `farkas`. *LIA:* the same, plus the lemma kinds
  `branch` (`x <= k ∨ x >= k+1`, tautology in `Int`), `cut` (a Gomory cut,
  derived from a tableau row by a Farkas-like argument with integer rounding)
  and `gcd` (a divisibility contradiction). Each is a lemma the kernel can
  prove from `Int` axioms; `KRN-012a` (the linear-arithmetic certificate
  procedure over `Nat`) is the reconstructor's engine.
- **Families.** QF_LRA (`sc`, `uart`, `clock_synchro`, `spider_benchmarks`),
  QF_LIA (`cut_lemmas`, `CAV_2009`, `pidgeons`, `convert`), plus LANG23's
  refinement obligations.
- **Units.**
  - *Numbers:* arbitrary-precision rational with a symbolic infinitesimal (for
    strict inequalities), on `bignum-core` (CVM-210).
  - *Simplex, one unit per part* (CVM-211 to 215): tableau and slack
    introduction; assert-bound and immediate conflicts; pivoting and `check`
    with Bland's rule; cheap bound propagation; model construction (resolving
    the infinitesimal).
  - *LRA certificates* (CVM-216, 217): Farkas extraction from a conflicting
    row (CERT); the kernel reconstructor over `Int` after clearing denominators
    (RECON).
  - *Integers* (CVM-220 to 226): normalisation (integer coefficients, GCD
    tightening); `branch`; Gomory cuts; GCD test and equality elimination; each
    with its own CERT and RECON where the lemma kind is new; Cooper elimination
    in two units; **retire the bounded enumerator** from the default path (it
    stays as a test oracle).
  - *Equality sharing* (CVM-219): the theory-propagated equalities that
    model-based combination (4.4) asks for.

### 5.4 Arrays

- **Procedure.** The decision procedure for the extensional theory of arrays by
  **lazy axiom instantiation**: read-over-write (`select(store(a,i,v), j)`),
  extensionality (a disequal pair gets a witness index), and array models as
  finite maps with a default. Complete for QF_AX; with index and element
  theories it is complete when those are, plus the usual restrictions on
  nested arrays.
- **Certificate.** Fragment `inst`: each lemma instantiated from the three array
  axioms. The kernel models an array as a function (`Idx → Val`), `store` as
  `Function.update`, so the three axioms are library lemmas.
- **Families.** `QF_AX`, `QF_ABV`, `QF_AUFBV`, `QF_ALIA`, plus memory models from
  VER11a §6.3.
- **Units** (CVM-230 to 236): read-over-write instances; extensionality
  witnesses; array model construction; `const-array` and equality of stores; the
  `inst` CERT; the RECON; a weak-equivalence propagation unit that is an
  optimisation and can be dropped.

### 5.5 Algebraic datatypes

- **Procedure.** Constructor, selector and tester axioms handled in the EUF
  component: injectivity, disjointness, **acyclicity** (no term equals a proper
  subterm), exhaustiveness, and selectors total (SMT-LIB's underspecified
  selectors are modelled as uninterpreted on the wrong constructor). Complete
  for QF_DT.
- **Certificate.** Fragments `euf` plus `dt` (an instance of injectivity,
  disjointness or acyclicity). The kernel side needs `noConfusion`-style lemmas
  for the generated inductive types; VER32 §3.4 (the kernel derives recursors
  only) means these are **library lemmas generated per datatype by an untrusted
  producer and checked**.
- **Families.** `QF_DT`, `QF_UFDT`, `QF_DTLIA`; Warrant's own ADTs (VER04o
  collections, VER04s recursive types).
- **Units** (CVM-300 to 306): `declare-datatype` opcode (INSTR);
  constructor/selector/tester axioms; acyclicity; finite-enumeration handling;
  datatype values in models; the `dt` CERT; the RECON that maps a Warrant ADT to
  the PIF inductive block and proves its no-confusion lemmas.

### 5.6 Quantifiers

- **Procedure.** Three techniques, three units groups: **skolemisation and
  normal forms** (the easy part); **E-matching** on the e-graph from EUF, with
  triggers and generation/cost caps; **model-based quantifier instantiation**
  (MBQI: build a candidate model, ask the ground solver for a counterexample to
  each quantified formula, instantiate with it); plus **finite expansion** for
  quantifiers over small BV sorts and **Cooper** for LIA. Incomplete in general
  (first-order logic is undecidable); complete on stated fragments (BV with
  finite expansion; LIA by Cooper; the "essentially uninterpreted" fragment by
  MBQI).
- **Certificate.** Fragment `inst`: the list of **instantiations** (a quantified
  hypothesis, a term list). The kernel specialises the hypothesis and the ground
  proof proceeds; the instantiation list is the only thing the solver supplies.
  Skolemisation is *equisatisfiable*, not equivalent, so its certificate is an
  implication fragment with the usual `Classical.choice` footprint, which is a
  **named axiom in VER32's sense** and shows in the claim's axiom footprint (a
  consequence the owner should see: section 12, Q8).
- **Families.** `UFLIA`, `AUFLIA`, `UFBV` with quantifiers, `LIA` (Presburger),
  and Warrant's `forall`-style claims.
- **Units** (CVM-310 to 322): NNF and prenex; skolemisation; trigger selection;
  e-matching engine; the instantiation loop with meter; MBQI in two units
  (candidate model; counterexample and instantiation); BV finite expansion; the
  `inst` RECON; Cooper (CVM-224/225) reused.

### 5.7 Nonlinear arithmetic (incomplete, incrementally)

- **Procedure.** Treat each nonlinear monomial as a fresh variable and add
  **linearisation lemmas on demand**: sign, zero, monotonicity, tangent planes,
  following the incremental-linearisation line of work. Incomplete; answers
  `Unknown` often. NIA over bounded integers can instead blast through BV.
- **Certificate.** Each lemma is an instance of a multiplication fact
  provable in the kernel from `Int.mul` lemmas (fragment `nl`).
- **Families.** `QF_NIA`, `QF_NRA` (the latter only to the extent the
  linearisation handles it; no CAD, no `nlsat`, section 4.9).
- **Units** (CVM-330 to 336): monomial abstraction PASS; one unit per lemma
  family (sign, tangent plane, monotonicity, zero); `nl` CERT and RECON; the
  mixed BV/Int bridge `bv2nat` / `int2bv` (CVM-250, 251; needed earlier, in M2,
  because Warrant's specifications are over `Int` and the code is over
  fixed-width words).

### 5.8 Later: strings, sequences, floating point

- **Floating point.** IEEE 754 by **bit-blasting each operation to a circuit**
  (classification, unpack/pack, rounding, add, multiply, divide, fused
  multiply-add, square root, conversions), one unit per operation. The kernel
  side needs an IEEE semantics; that is **VER33's float note's** subject (and
  VER04p's floats); the FP units wait for it, and the certificate story is the
  BV one over the blasted circuits.
- **Strings and sequences.** Length abstraction, then word equations by
  the usual unfolding with regular-membership by automata; incomplete.
  One unit per rule family. Not started until Warrant's text work (VER04l) has a
  real consumer.

---

## 6. Trust: the solver is untrusted

### 6.1 The rule

The owner's rule (VER31, "trust the compiler, verify everything else") applies
to the solver as to anything else we wrote. The solver is a heap of heuristics;
it will have bugs; the design assumes it does.

| Answer | What is checked | By what | Counts toward |
|---|---|---|---|
| **SAT** + model | The model is evaluated against **every live assertion** | The independent evaluator (CVM-008), which shares no code with any tactic, and in CI the kernel-adjacent `pif-check` model-check mode for generated goals | A *refutation* of a claim (a counterexample) and `tested` evidence |
| **UNSAT** + certificate | The certificate is reconstructed into a PIF proof of the goal and **checked by the kernel** | `warrant-smt-recon` (untrusted) then `pif-check` (VER32; trusted) | A VER31 `proved` claim, subject to the (c) ids it still depends on |
| **UNSAT** without a certificate | Nothing | nothing | Only `tested` (6.6) |
| **Unknown** | n/a | n/a | Nothing |

A solver that answers wrongly can therefore cause at most **a false rejection**
(the kernel refuses a proof) or **a false counterexample that fails evaluation**
(caught); never a false proof of the signed goal. That is VER32 §1.3 applied to a
new producer.

### 6.2 Fragment kinds

One fragment kind per kind of reasoning. A fragment is a record in the trace
format (6.3). Each kind has exactly one small reconstructor (6.4).

| Kind | Emitted by | Asserts | Kernel justification |
|---|---|---|---|
| `rw` | a PASS | goal `G` equals `G'` by a named rule at a position | the rule's library lemma plus congruence |
| `red` | a PASS that is only equisatisfiable | `G'` unsat implies `G` unsat | the pass's library lemma (for example `solve_eqs_sound`, `skolem_sound`) |
| `blast` | the BV tactic | the CNF is the reference encoding of the term DAG | `blast_correct` + `Eq.refl` by reduction |
| `lrat` | `cdcl-sat` | the clause set is refuted by this LRAT proof | `checkLRAT_sound` + reduction |
| `farkas` | simplex | a theory lemma `¬l1 ∨ … ∨ ¬lk` from weighted bound atoms | linear arithmetic lemma (KRN-012a) |
| `branch`, `cut`, `gcd` | LIA | integer lemma | `Int` axioms |
| `euf` | congruence closure | a theory lemma from a merge chain | `Eq.trans`, `congrArg` |
| `inst` | arrays, quantifiers, datatypes | an instance of a library axiom or a quantified hypothesis | `specialize` |
| `dt` | datatypes | injectivity, disjointness, acyclicity instance | per-datatype no-confusion lemmas |
| `nl` | nonlinear | a multiplication-fact instance | `Int.mul` lemmas |
| `input` | the loader | an original assertion | goal binding (VER32 §3.6) |

The propositional glue is **always LRAT**: every theory lemma is a clause the
proof may use as an extra axiom clause, each backed by a fragment from the table.
So the proof of a mixed problem is one LRAT proof plus a list of lemma proofs;
adding a theory means adding fragment kinds, never changing the glue.

### 6.3 The trace format and its relation to PIF

**Decision.** The solver writes a **trace** (`cvm-trace-1`), a line-oriented text
file with one fragment per line, in dependency order, including the LRAT proof's
steps as `lrat` lines. The trace is **not** a trust boundary and carries no
semantics beyond what a reconstructor gives it; it may change between releases
(its version is in its header, and a reconstructor refuses an unknown version).
The **certificate that matters is PIF**: `warrant-smt-recon` reads a trace and
the goal's PIF statement and emits a PIF file whose theorem the kernel checks.

    cvm-trace 1
    input   a1  (distinct (xor4 a b) (bvxor a b))
    rw      r1  from a1  rule bv.xor.def-nand  at 1.0
    blast   b1  from r1  width 8  enc ref1
    lrat    p1  from b1  steps 212   -- the next 212 lines are LRAT steps
    close   c1  from p1             -- the empty clause: goal refuted

**Alternatives.** (a) The solver emits PIF directly. Rejected: it would put PIF
construction, name tables and universe levels inside the solver, which couples
two fast-moving formats and makes every fragment emitter a kernel-term author.
(b) A separate, versioned solver-certificate format that the kernel reads
natively (as Z3's proof format, or LFSC). Rejected: it enlarges the trusted
reader, against VER32's budget. (c) **Chosen:** an untrusted trace plus an
untrusted per-kind reconstructor, so the trusted path is PIF alone. The
reconstructors are in the "untrusted producer" family VER32 §1 already
describes, next to the case-split producer (PIF-006) and the arithmetic
procedure (KRN-012a).

### 6.4 One small reconstructor per fragment kind

Each kind has a library part (trusted, in the kernel, checked) and a producer
part (untrusted Rust). The producer part is small because the library part does
the work, by reflection or by direct term construction:

| Kind | Library (proved once) | Producer does | Mode |
|---|---|---|---|
| `rw` | one lemma per named rule | build `Eq` chain via congruence | direct terms |
| `blast` | `blast_correct`, `tseitin_sound`, one lemma per group | state the goal as `eval (term) = …` and `Eq.refl` | reflection |
| `lrat` | `checkLRAT_sound` | write the LRAT bytes in a data constant | reflection (or term route) |
| `farkas` | linear arithmetic over `Int` | the weighted sum and a `Nat`/`Int` check | direct terms via KRN-012a |
| `euf` | `Eq.trans`, `congrArg` (core) | the merge chain | direct terms |
| `inst` | per-axiom lemmas | `specialize h t1 … tn` | direct terms |

**Reflection** means the kernel *evaluates* a checker on the data, as in
VER11a §9.5 option 2: `checkLRAT cnf cert = true` by `Eq.refl`. It avoids
`Lean.ofReduceBool` and any native trust, at the price of kernel reduction time
(measured, KRN-009 and CVM-174). Reflection on big numbers wants the kernel's
literal acceleration (`lits`, VER32 §3.5), which the gate must allow for these
files; without it the checker works on a unary or list `Nat`, which is
slower. That decision is Q7.

**Composition.** The tactic layer's reductions give *implications* between
goals (`G' unsat → G unsat`, kinds `rw`, `red`), and leaf kinds close a goal
(`blast`+`lrat`, or a theory conflict). The reconstructor folds the chain with
`Impl.trans`, so the whole proof is built by the same structure that ran the
strategy. The trusted proof path therefore grows **one kind at a time**: M1
ships `rw`, `blast`, `lrat` and nothing else is reachable in a certified proof;
a query that needs any other kind is `solver-attested` until that kind lands.

### 6.5 What each answer is worth on the VER31 ladder

Using VER31 §4 as it stands:

| Evidence | Rung it can support | Open (c) ids it carries |
|---|---|---|
| Model checked by evaluator (a counterexample) | refutes; `tested` evidence for the claim it supports | none |
| UNSAT, no certificate | `tested` at most (`solver-attested`, 6.6) | n/a |
| UNSAT, DRAT checked by `drat-check` (today's BV path) | `bounded(N)`/`checked`, as VER31 already computes | `tcb.drat.checker-correct`, `tcb.bitblast.encoding-correct` |
| UNSAT, per-kind certificate checked by a non-kernel checker we wrote (for example a Farkas checker in Rust) | `checked` | a new (c) id per such checker (`tcb.cvm.<kind>-checker`) |
| UNSAT, reconstructed to PIF and kernel-checked (6.4) | `proved` if no other (c) id is open | none from the solver; the kernel's ids (VER11a §13) |

### 6.6 Decision: "solver-attested" is a label, not a rung

The brief asked whether VER31 needs a "solver-attested" rung. **No new rung.**
The ladder already has `tested` (behaviour checked on chosen inputs) and
`checked` (a certificate with open own-tool ids). An UNSAT with no certificate is
neither a certificate nor a test: it is an *unverified statement by our own
tool*, which VER31 treats as (c) tool trust. So:

- It is recorded in the report as evidence kind `solver-attested`, with the
  solver, version and query digest, **so a reader sees it**.
- The gate **never** accepts it as `proof.certificate` (scheme `solver-attested`
  is refused by the gate's certificate parser), and never lets it raise a claim
  above `tested`. It can *lower* confidence by refuting (a checked model), and it
  is useful for finding bugs and for prioritising work.
- A claim whose only UNSAT evidence is attested is labelled in the notes
  "solver-attested, not certified", the way VER31 §8 prints open ids.

**Amendment needed (small, for VER31 §4 and §8, to be made with CVM-170):**
(1) certificate schemes: `pif:sha256:<64 hex>` (VER32's PIF-020 aggregate) is
accepted for `checked`/`proved`; `solver-attested` is refused as a certificate;
(2) the report's evidence list gains the `solver-attested` kind, never
contributing above `tested`. Not a new rung; the owner may still prefer one
(Q5).

---

## 7. Worked example: one logic gate

**The gate.** From the logic-gates program (VER32 §9.2 and the NAND
constructions): XOR built from four NANDs, on 8-bit words, one lane per bit.

    nand(x, y)  = bvnot(bvand(x, y))
    n1 = nand(a, b)
    n2 = nand(a, n1)
    n3 = nand(b, n1)
    xor4(a, b) = nand(n2, n3)

**The verification condition.** "`xor4` equals `bvxor`, for every `a`, `b`":

    (set-logic QF_BV)
    (declare-const a (_ BitVec 8))
    (declare-const b (_ BitVec 8))
    (define-fun nand ((x (_ BitVec 8)) (y (_ BitVec 8))) (_ BitVec 8)
      (bvnot (bvand x y)))
    (define-fun xor4 ((x (_ BitVec 8)) (y (_ BitVec 8))) (_ BitVec 8)
      (let ((n1 (nand x y)))
        (nand (nand x n1) (nand y n1))))
    (assert (distinct (xor4 a b) (bvxor a b)))   ; the negated claim
    (check-sat)                                  ; expected: unsat

(`define-fun` and `let` are CVM-005; today's reader would reject this file.)

**1. Parse, expand.** Definitions are inlined, sharing `n1` by DAG node.

**2. Word-level preprocessing (PASS units, `then`).** Rule `bv.not.and-def`
rewrites `bvnot(bvand ·)` as written (no change); the Boolean pass has nothing
to do; the solver does *not* find `xor4 = bvxor` by rewriting alone, which is
as it should be: the equality is the content of the claim. Fragment: an `rw`
line per rule that fired (here, none that change the term).

**3. Bit-blast (TACTIC, group "bitwise").** Eight lanes, independent. In each
lane, four NAND nodes `n1, n2, n3, out` and the reference `a ⊕ b` (three
AND nodes in an AIG), a lane-difference, then an OR over the eight lanes' difference
bits. By hand: about 4 + 3 + 3 = 10 AND nodes per lane, 7 for the OR, so about
87 AND nodes, 103 CNF variables (16 inputs plus 87 gate outputs), 3 clauses per
gate plus the unit asserting "some lane differs": about 262 clauses. (The tool
prints its own counts; the hand count is only to show the size is tiny.)

**4. CDCL (TACTIC, `cdcl-sat`).** The eight lanes share no variables, so the
refutation is eight small case analyses on two inputs each. A few conflicts per
lane; a proof of the order of a few hundred LRAT steps. UNSAT. Time: milliseconds.
`drat-check` accepts the DRAT form today. In certified mode `cdcl-sat` emits LRAT
(KRN-009).

**5. The trace.** Fragments `input`, `blast b1 (width 8, enc ref1)`, `lrat p1
(N steps)`, `close`. Nothing else is needed: preprocessing did nothing.

**6. Reconstruction into PIF (untrusted producer, one reconstructor per kind).**
The goal in the kernel's language is the signed statement (VER32 §3.6):

    theorem xor4_ok : ∀ a b : BitVec 8, xor4 a b = a ^^^ b

The producer writes, as PIF declarations:

1. `reify`: the VC as a library term `vc : SExpr` and `eval vc env = ¬(xor4 a b = a ^^^ b)`
   by `Eq.refl` (the kernel unfolds both sides). This is where the **goal binding**
   makes the proof about *this* statement: `vc` is computed from the goal by a
   library function, not supplied by the solver.
2. `cnf : Cnf := encode vc` — the library's reference encoder, evaluated by the
   kernel. Not data from the solver.
3. `cert : Lrat := <bytes from the solver>`. The only solver-supplied data.
4. `chk : checkLRAT cnf cert = true := Eq.refl true` — kernel reduction.
5. Assembly, using only theorems proved once in the library:
   `lrat_sound chk : ¬ ∃ env, cnf_sat cnf env`, then `blast_correct vc` turns
   that into `¬ ∃ a b, eval vc env`, then `Classical`-free double negation on a
   decidable equality gives `xor4_ok`.

A wrong cert makes step 4 fail to reduce to `true`: **a rejection, never a false
proof**. A wrong encoder in Rust makes the clauses differ from `cnf`, so the LRAT
steps do not check against the kernel's CNF: again a rejection. The things that
must be right are the library theorems (`lrat_sound`, `blast_correct`) and the
kernel, exactly the items VER11a §12 already makes the trusted base.

**7. What it closes.** `tcb.bitblast.encoding-correct` and
`tcb.drat.checker-correct` for this claim; the claim's report lists
`certificate: pif:sha256:…`. **What it does not close:** the model faithfulness
ids for the gate's Warrant-to-Lean elaboration and for the Rust-to-WIR step
(VER32 §9.2, VER11a §13); those are others' work.

**Cost.** For 8 bits this is trivial. For a 64-bit multiplier-containing VC the
CNF has about 10^5 gates and the LRAT proof has millions of steps; kernel
reduction of `checkLRAT` over that is the open performance question, measured in
CVM-174, and the reason the term route (6.4) and a certified-mode
re-solve are kept as fallbacks.

---

## 8. Front ends and consumers

### 8.1 SMT-LIB v2 (`smt-lib-format`)

The reader grows by units: `define-fun`, `let`, indexed identifiers as real
terms (retiring the flattened `Apply "_"` shape), `declare-datatype(s)`,
named assertions, `check-sat-assuming`, `get-value`, `set-option`, and the
SMT-LIB `check-sat-using`-style extension for tactics. The writer round-trips
all of it. **Strictness stays**: an unsupported command is a parse error, never
silently ignored.

### 8.2 New instructions

`DefineFun`, `NamedAssert`, `CheckSatAssuming`, `GetValue`, `DeclareDatatype`,
`CheckSatUsing`, `SetOption(produce-certificates, rlimit)`, `GetCertificate`.
Each is one INSTR unit: the variant, parser, printer, validator and VM arm, with
a rejection test.

### 8.3 Warrant (VER33), provisional rung names

| Warrant need | Solver mode | Milestone |
|---|---|---|
| **A1:** generate inputs at the boundary of a precondition, and satisfying a claim's `requires` | SAT with models, many small calls with push/pop; BV and LIA | M1, M2 |
| **A2:** bounded model-versus-code check (the model and the compiled code agree for inputs up to N) | UNSAT with certificate; BV, arrays, UF | M1 (BV), M2 |
| **A3:** verification-condition discharge to a kernel proof | UNSAT certified by PIF; all theories as they land | M1 for BV, then per theory |
| Counterexample explanation | unsat cores, models | M0 |

(Rung names A1 to A3 follow the owner's use of "A1" and are provisional: VER33 owns
them.)

### 8.4 Other consumers

- **LANG23 refinement types.** The checker keeps its three-way outcome. It
  benefits from LRA/LIA completeness and real unsat cores; it **stays
  `UNKNOWN → runtime check`** when the solver answers `Unknown`. A solver
  budget replaces its current ad hoc caps.
- **Property-test generation (VER05).** Models for preconditions; push/pop for
  incremental enumeration of distinct models (block the last model, solve
  again).
- **Package resolver.** LIA with unsat cores (to explain a conflict).
- **LEAN00 L4c.** A `bv_decide`-like tactic is the `blast` + `lrat` path with
  the reflection glue: the same units, called from the Lean elaborator when it
  lands; nothing in this spec is specific to Warrant for it.
- **`adj-constraint-solver`.** Replaces its deletion-based IIS with
  `GetUnsatCore` once CVM-004 lands.

---

## 9. Validation of the solver itself

The solver is untrusted, but it must still be *good*. Validation is a set of
harness units (HARN), each small, which run in CI at stated cadences.

1. **Differential testing against Z3 and cvc5 (oracles, never trusted).** A
   harness runs each pinned SMT-LIB benchmark through the CVM, Z3 and cvc5
   (binaries pinned by SHA-256, run as subprocesses, never linked). A
   disagreement is triaged mechanically: if the oracle says SAT, **our evaluator
   checks the oracle's model**; if we said UNSAT, our certificate is
   re-checked; if the oracle says UNSAT and we said SAT, our model is re-checked
   by the evaluator. Whichever side fails its own check is wrong. A genuine
   disagreement with both sides self-consistent is a *stop-the-line
   finding* (it means the evaluator and the oracle disagree on semantics, which
   is exactly what needs a human). Oracle answers are never evidence for a claim.
2. **Soundness fuzzing.** A grammar-driven generator per logic (one HARN unit
   each) produces well-sorted terms; the solver's answer is checked by the
   independent evaluator (SAT), by brute force on small domains (UNSAT up to
   16 bits of free variables, as the `qfbv` test already does), and by the
   certificate checker. A wrong answer is a finding with a regression case.
3. **Metamorphic relations** (one HARN unit per family): renaming variables,
   permuting assertions, double negation, adding a tautology, splitting an
   assertion into a conjunction, `push; assert A; pop` equals not asserting
   `A`, `check-sat-assuming` equals `push; assert; check; pop`, widening a
   bit-vector and zero-extending the claim, and "assert the model, still SAT".
   Each relation is a property test over generated formulas.
4. **Certificate check on every UNSAT in CI.** In certified mode every UNSAT in
   the benchmark and fuzz runs is reconstructed and checked by the kernel; the
   fraction of UNSATs that are certified is a tracked number per logic
   (the `c` of the gates in section 10).
5. **Performance tracking per logic.** `cvm-bench` records, per benchmark and
   per commit: result, deterministic meter totals, wall-clock (informational),
   and per-logic `R(L,T)` and PAR-2. A regression in solved-count or a
   certified-fraction below the milestone value fails the nightly job. Wall
   time never decides a verdict.
6. **Mutation controls.** Per VER27's sabotage protocol: a seeded bug in a
   rewrite rule, a Farkas coefficient, a congruence merge, an LRAT hint must be
   **rejected by the kernel** or **caught by the evaluator**; the corpus keeps
   the mutants (as the `bv_faithfulness` audit already does for 18 lowering
   mutants).

Cadence: units 2, 3, 4 run on every PR that touches a tactic (small budgets);
1 and 5 run nightly; the full benchmark gate runs per milestone.

---

## 10. Milestones

Each is a measurable gate. `R(L,T)` and the other numbers below are
**proposals** to be re-based on the CVM-S0 baseline (1.2) and approved by the
owner (Q3). Every gate also requires: zero wrong answers across the fuzz and
differential corpora, every PR's units merged with their certificate story, and
the "unit card" (2.3) filled in for each.

| Milestone | Scope | Gate | Unlocks |
|---|---|---|---|
| **CVM-M0** Foundations | Meter, bignum values, instructions (8.2), tactic trait and combinators, real push/pop, real unsat cores, evaluator, `cvm-bench`, oracle adapter, fuzz base | Baseline table recorded; combinators tested; every existing test green through the new trait; push/pop equivalence relation passes | Everything; Warrant A1 on Bool/BV (already works, now measured) |
| **CVM-M1** Certified QF_BV | Section 5.1 complete | (i) **100%** of the logic-gate VCs (all 11 gates, widths 8/16/32/64) and the four ct-compare VCs: UNSAT **certified through `pif-check`**, each in at most 10 s solver time plus the measured kernel time recorded; (ii) QF_BV pinned Warrant-near subset (width at most 64, no variable divisors, no wide multipliers): `R(QF_BV, 60 s) >= 0.5`; (iii) full pinned QF_BV: `R >= 0.25`; (iv) 100% of QF_BV UNSATs certified or labelled `Unknown`/attested with a reason | Warrant **A3** for fixed-width code (gates and ct-compare, VER32 G-M2 and VER11a K-M1c), A2 for BV, A1; LEAN00 L4c can start |
| **CVM-M2** Arithmetic, EUF, arrays | 5.2 to 5.4, plus BV/Int bridge and the CDCL(T) stage-2 hook | `R(QF_UF, 30 s) >= 0.6`; `R(QF_LRA, 60 s) >= 0.5`; `R(QF_LIA, 60 s) >= 0.4`; `R(QF_AX, 60 s) >= 0.4`; `R(QF_UFBV, 60 s) >= 0.4`; `R(QF_AUFBV)` recorded; >= 95% of UNSAT certified; `adj-constraint-solver` and LANG23 on the new engine | Warrant A3 for Int specifications, memory models, function-heavy code; LANG23 solver tier at full strength; property-test generator on Int |
| **CVM-M3** Datatypes, quantifiers, nonlinear | 5.5 to 5.7 | `R(QF_DT, 60 s) >= 0.5`; `R(UFLIA/AUFLIA with quantifiers, 60 s) >= 0.25`; Presburger LIA: all pinned instances solved or `Unknown` with reason, none wrong; `R(QF_NIA) >= 0.2`; >= 90% UNSAT certified | Warrant A3 for ADT-valued algorithms and `forall` claims; VER04o/s collections |
| **CVM-M4** Strings, sequences, floating point | 5.8, as Warrant asks | per-logic `R >= 0.2`, FP certified through the BV path | Warrant floats (VER04p); text algorithms |
| **CVM-M5** Hardening and speed | Preprocessing breadth, SAT improvements, portfolio, certified-mode performance | Re-baselined `R` raised by the owner's chosen margin; kernel-time budget per certificate met | Scale; the standing nightly report |

---

## 11. The decomposition tree and the backlog

The leaf rows are in [VER-BACKLOG.md](VER-BACKLOG.md) under "Constraint VM as
Warrant's SMT solver (VER34)". Each row is one PR and states: the unit, its slot
(instruction, tactic, pass, certificate, reconstructor, combinator, harness),
its size target, and its dependencies. The tree:

| Milestone | Group | Units | Rows |
|---|---|---|---|
| M0 | Baseline, corrections, values | measure first; fix README; bignum | S0, 001, 002 |
| M0 | Instructions (INSTR) | named assertions and assumptions; `define-fun`/`let`; indexed identifiers; `check-sat-using`; `get-value`, certificates | 003, 005, 005a, 017, 018 |
| M0 | Engine plumbing | unsat cores; meter; trait and combinators; defaults; incremental push/pop | 004, 006, 009 to 016 |
| M0 | Harness | runner, oracle adapter, evaluator, fuzz generator, metamorphic relations | 007, 007a, 008, 019 to 019c |
| M1 | BV rewrites (PASS) and their RECON | one per operator family | 100 to 108, 110 to 119 |
| M1 | Blasting (TACTIC), CERT, RECON | one per operator group, plus Tseitin and `blast_correct` | 120 to 128, 130 to 137, 140 to 148 |
| M1 | SAT certificate and speed | LRAT semantics, checker soundness, reconstructors, BVE, vivification | 150 to 153, 160, 161 |
| M1 | Closing | end-to-end reconstructor, gate and ct-compare corpora, benchmark gate, kernel-time report | 170 to 174 |
| M2 | EUF | 200 to 207 | |
| M2 | Simplex and LRA | 210 to 219 | |
| M2 | LIA | 220 to 226 | |
| M2 | Arrays, bridge, combination, CDCL(T) hook | 230 to 236, 250, 251, 240 to 247 | |
| M3 | Datatypes | 300 to 306 | |
| M3 | Quantifiers | 310 to 322 | |
| M3 | Nonlinear | 330 to 336 | |
| M4 | Floating point, strings | 400 to 415 | |
| M5 | Hardening | 500 to 505 | |

The ordering inside a milestone is by dependency; a unit's PR is not started
until its dependencies' PRs have merged and their certificate stories exist
(spec before code, VER00's rule).

---

## 12. Owner questions

Each has a recommendation; none blocks the foundations (M0).

| # | Question | Recommendation |
|---|---|---|
| **Q1** | **Build-versus-reuse boundary.** Is Z3 and cvc5 as CI oracles acceptable? | **Yes, oracles only**: nightly job, binaries pinned by digest, run as subprocesses, never linked, never in a trust path, never evidence for a claim. Everything that ships and everything on the proof path is ours. Both are permissively licensed; confirm the licences at CVM-007a |
| **Q2** | **Which logic first.** | **QF_BV** (the repository already has the pieces and the gates and ct-compare need it); QF_LIA/LRA/EUF/arrays in M2, driven by Warrant's `Int` specifications. Confirm |
| **Q3** | **Performance targets.** | Adopt the metric `R(L,T)` and the certified fraction (1.1); approve the **shape** of section 10's numbers now, re-base the numbers after CVM-S0 |
| **Q4** | **Publication.** Standalone crate, or a package per lane? | **Rust crates only** (`constraint-core` through `constraint-vm`, and `warrant-smt-recon`); other lanes call a `cvm` binary or a WASM build (as KRN-018 plans for the kernel). The certificate is portable; the solver need not be, because it is untrusted. A lane that wants a native port can have one later by its own decision |
| **Q5** | **A "solver-attested" rung?** | **No rung**: a label that caps at `tested` (6.6), with the VER31 clarifying amendment. If the owner wants a visible rung, it would sit between `tested` and `bounded(N)`; I recommend against, because it dilutes the ladder with something that is not a certificate |
| **Q6** | **Certified-mode cost.** Re-solve with the reference encoding for UNSAT, or lift AIG optimisations into the kernel as lemmas? | Decide after CVM-174's measurements. Default: re-solve; lift only what the measurements justify |
| **Q7** | **Literal acceleration (`lits`) in SMT proof files.** May the gate-fixed environment for SMT reflection files enable `lits`? | Yes, if measurement shows reflection is impractical without it; the feature is already validated by VER32 §3.5 and stays out of the gates' G-M1 environment. The alternative is the term route, slower for large proofs |
| **Q8** | **Classical axioms in quantifier proofs.** Skolemisation needs choice | Accept `Classical.choice` as a named, pinned axiom in the footprint for claims that use skolemisation (VER32 §1.4), and show it in the report; claims that avoid it keep a smaller footprint. Confirm with VER31's buckets |
| **Q9** | **Determinism and parallelism.** | Single-threaded, deterministic, meter-budgeted; no parallel mode before M5; a portfolio may never change a verdict |
| **Q10** | **Strings and floating point.** | Defer to M4 and to when Warrant has a consumer; FP waits for VER33's float note |
| **Q11** | **Bignum.** | Use `bignum-core`; `constraint-core`'s `Value::Int(i128)` is replaced (CVM-002), a breaking change to its consumers handled in one PR |
| **Q12** | **Fate of the bounded LIA enumerator and `adj-constraint-solver`'s IIS.** | Keep the enumerator as a test oracle only; move `adj` onto real cores after M2 |

---

## 13. Relation to other specifications

| Spec | Relation | Note or amendment |
|---|---|---|
| **LANG24** | VER34 *is* LANG24's later phases, structured. The four-crate layering is unchanged; units fill it. LANG24's "LIA as built" divergence note remains correct | **Dated note added to LANG24 (2026-11-24)** pointing here. LANG24 PR 24-H (Z3 bridge) is superseded by section 9's oracle adapter |
| **LANG23** | Consumer. Keeps `PROVEN_SAFE/UNSAFE/UNKNOWN`; gets complete LIA/LRA and cores | None |
| **VER09** | `cdcl-sat` is the Boolean core | Needs LRAT (KRN-009), a theory hook (CVM-246), inprocessing (CVM-160/161); a note in VER09 when each lands |
| **VER10** | Bit-blasting circuits are the BV groups | The "certified mode" (reference order) is a VER10 addendum, made with CVM-130 |
| **VER11** | DRAT stays the fast-path check inside the engine and the `bounded`/`checked` evidence | `drat-check` is not replaced; the kernel route supersedes it for `proved` |
| **VER11a** | §9.4 to 9.5 are what M1 delivers; §10 "automation first" calls the Constraint VM as step 3 | Proposed: §9.5 records the *reflection* route as the default for BV with the term route as fallback, and names this spec |
| **VER32** | The certificate is PIF; reconstructors are untrusted producers | Requests: a native `BitVec` and `SExpr` library module in the PIF prelude (or the Init route through PIF-009); possibly `lits` for SMT files (Q7); a backlog cross-reference |
| **VER33** | Warrant is the front end and the main consumer | Rung names (A1 to A3) and ADL ids are provisional until VER33 lands; Warrant's `forall` and ADT forms drive M3 |
| **VER31** | The claim ladder is used as is | Clarifying amendment for `solver-attested` (6.6), made with CVM-170 |
| **LEAN00** | L4c is the `blast` + `lrat` path | No change |
| **VER27, VER15** | Hostile input, finding protocol, weakness registry | A soundness finding in the solver is a VER27 finding; weakness lists get a "constraint VM" row per lane that calls it (Rust only for now) |

---

## 14. Risks and what this spec does not promise

- **Speed.** `cdcl-sat` has never been compared with MiniSat or CaDiCaL. Our
  CDCL may be several times slower; theory solvers written fresh will be
  slower than ten-year-tuned ones. The gates are fractions of Z3's solved set
  for this reason.
- **Certified-mode cost** (Q6) is the largest unknown; the units to measure it
  come before the units that depend on it.
- **The kernel library is real work** (5,000 to 8,000 lines) and is on the
  critical path for any `proved` claim; it can lag the solver, which is why
  uncertified UNSAT is labelled, not hidden.
- **Decomposition can be wrong.** If a unit repeatedly overruns, the tree is
  revised here, not exceptions piled up (2.1).
- **No claim of completeness** for quantifiers, nonlinear arithmetic, strings.
  `Unknown` is a first-class answer.
- **This spec promises no dates.** Milestones are gates, not schedules.
