# VER34 — The Constraint VM as Warrant's SMT solver

**Status:** Draft for review (2026-11-24). Spec only; no code in this change.
**Cross-spec follow-up applied 2026-11-24** (section 15, "Follow-up 2026-11-24"): the
list of section 13.1 is applied to VER32, VER33, VER11a and VER31; the owner decisions
are recorded in section 12.1 (including Q13).
**Layer:** Proof machinery. Extends LANG24 (the Constraint VM) from "a generic
executor with three small tactics" to "the solver Warrant uses".
**Builds on:** LANG24, LANG23, VER09 (`cdcl-sat`), VER10 (bit-blasting), VER11
(DRAT), VER11a (kernel; §9.4 to §10), VER32 (PIF), VER31 (claim ladder).
**Aligned with:** VER33 (Warrant as the algorithm definition language; its
adoption ladder A0 to A4 and its section 6.3, "the solver") and VER32 as revised
as merged (d01ad9a9ab8: `Pif.*` namespace, one linear base chain signed per
claim, separate goals lists, structural validation of logic axioms, `kernel.logic.consistent`
as bucket (c)). Section 8.3 maps VER33's needs to
milestones; section 6.7 reconciles VER33's `bounded(k)` with the untrusted-solver
rule; section 6.8 fits the reconstructors to PIF; section 15 is the review record
(Fable, round 1, 18 findings).
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
| D11 | **Measure before building.** A spike (CVM-S1) runs a hand-written LRAT checker through `pif-check` and measures steps, depth and nodes at 8, 32 and 64 bits *before* any library work; `lits` and a named raised-limits configuration are prerequisites of M1; the term route is a P1 fallback | 6.4 |
| D12 | **How library proofs are authored is an owner question (Q13)**, decided before M1; M1's P1 scope is cut to the operators the gates and ct-compare use | 12 |
| D10 | **Rust only.** Other lanes call the solver as a binary or WASM module. The certificate, not the solver, is what must be portable | 10.4 |

**What this costs.** Honestly: tens of thousands of lines of Rust (section 2.5
estimates 45,000 to 65,000 for the program through M4), **10,000 to 25,000 lines
of kernel-side library for M1's P1 scope alone** (the review showed the first
draft's 5,000 to 8,000 was too low), a few hundred PRs, and a standing
validation budget (section 9). M1 is **months**, not weeks: the circuits, the SAT
core and the DRAT kernel exist, but the kernel-checked proofs that make the UNSAT
count do not, and whether the kernel can run them at 32 and 64 bits is unmeasured
(6.4). Nothing here promises Z3's speed; section 1 says what is promised.

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

measured on the same hardware, one thread each. Four definitions are **frozen
before CVM-S0 runs**, so a number cannot be tuned after it is seen:

1. **The pinned sets** are fixed by SMT-LIB release and benchmark digest.
   **The Warrant-near subset** of QF_BV is a *predicate*, not a hand-picked list:
   every operator is in M1's P1 set (bitwise, `=`, `ite`, add/sub/neg, comparisons,
   extract/concat/extend), width at most 64, no multiplication or division (P2
   operators) until they land. The predicate and the resulting manifest are
   committed together.
2. **CI measures in meter units** (4.8), never wall-clock, so `R(L, 60 s)` is
   undefined until a **recorded calibration** maps meter units to seconds on a
   named machine. The calibration is part of CVM-S0's output and is re-recorded
   when the machine changes.
3. **Counting rule:** instances the CVM solves and Z3 does not are *reported but
   do not raise `R`* (it is a ratio against Z3's solved set; capping at 1 avoids a
   few easy extras hiding many losses); wrong answers make `R` undefined.
4. **`c` is two measures**, not one:
   **`c_trace`**, the fraction of solver UNSATs for which a trace was emitted and
   passes a fast, untrusted structural check (it measures emitter coverage), and
   **`c_kernel`**, the fraction of solver UNSATs **kernel-checked within a stated
   kernel budget**, over instances whose statement comes from the independent
   reifier or a signed goal (6.1). A gate that names `c` means `c_kernel` on the
   Warrant corpus and `c_trace` on the pinned families. There is **no "or
   labelled" escape**: an in-scope UNSAT that is not kernel-checked counts against
   `c_kernel`.

A milestone gate is "`R(L, T) >= r`, zero wrong answers, and `c_kernel`/`c_trace`
at the stated levels". We also report PAR-2 (the SMT-COMP style score: solved
time, with twice the timeout charged for each unsolved one) so that a solver that
is slow on everything is visible, not hidden behind a solved-count.

**Non-goals, stated so nobody assumes them:** matching Z3's optimiser
(`z3.Optimize`, MaxSAT), its Horn-clause engine (Spacer), its fixed-point engine,
its tactic zoo, its parallel mode, its API surface, or its performance on
industrial hard-combinatorial instances. Any of these can be added later as
units if a consumer needs them (section 4.9).

### 1.2 Targets are proposals until a baseline exists

Nobody has measured the current stack against Z3. The first unit (CVM-S0) runs
the existing `constraint-vm` on the QF_BV, QF_LIA and QF_Bool SMT-LIB families
and Z3 and cvc5 on the same machine, and records the table. **S0 is
self-contained** (a script under `code/scripts` with no dependency on the
benchmark runner of CVM-007, which comes later), so it really is first. Every number in
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
| Foundations (instructions, grammars, meters, combinators, evaluators, harness) | ~55 | 10,000 to 14,000 |
| QF_BV solver side (rewrites, blast groups, encoder port, SAT improvements) | ~30 | 7,000 to 10,000 |
| EUF, arrays, LRA, LIA, combination, hook (solver side and certificate emitters) | ~60 | 14,000 to 20,000 |
| Datatypes, quantifiers, nonlinear | ~38 | 9,000 to 13,000 |
| Strings, sequences, floating point (later) | ~25 | 8,000 to 12,000 |
| **Kernel-side library and reconstructors**, M1 P1 scope | ~45 | **10,000 to 25,000** (planning figure; re-estimate after CVM-141 and CVM-148a) |
| Kernel-side library, M2 to M4 | ~40 | 10,000 to 20,000 |

Roughly 260 units (the backlog rows; the first draft's 169 undercounted by merging
units that are several PRs). That is not a small program; it is a bounded one,
because each unit is reviewable and each milestone ships something that works.

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
`backtrack` to the same marks. Three units (CVM-016a to 016c), the last a test unit that runs
every other unit's tests inside a push/pop wrapper.

**Models.** A model is a total assignment for declared symbols, built from the
partial models of the theories and then **checked by an independent evaluator**
(CVM-008) before being returned. The evaluator is written from the SMT-LIB
definitions and shares no code with any tactic, **including constant folding**
(CVM-100 has its own arithmetic, tested against the evaluators). It is also the
fuzzer's SAT oracle, which is why it must not share a bug with the thing it
checks: a **second, deliberately different implementation** (CVM-008a: bit-lists
and `BigInt`, where the first uses machine words) is run against it on every
generated term and every folded constant, and a disagreement is a stop-the-line
finding. The evaluator grows with the sorts: arrays, UF and reals in M2 (CVM-208),
datatypes and quantifiers in M3 (CVM-308), the latter by **enumerating a finite
domain and answering `Unknown` otherwise**. A model that does not evaluate every
live assertion to true is a solver bug and becomes `Unknown` plus an error record
(fail closed; never a wrong SAT). `Value`
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

**Determinism.** Everything is single-threaded and deterministic **per solver
build**: the same input, seed and budget give the same answer, which is what makes
`or_else` and CI comparisons meaningful. It is *not* a promise across builds: a
refactor that perturbs the search changes which model is found. So **vectors the
solver produces are committed artifacts** that record (solver version, seed) as
provenance, and the reproduction check is that **each vector satisfies its
predicate under the evaluator**, not that a re-run matches (VER31's byte-for-byte
rule for committed evidence is then about the artifact, and no solver PR
invalidates a package's vectors). A parallel portfolio is a later optional unit
(4.9) and is never allowed to change a verdict, only the time.

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
meter cost has not been specified. Further rules the review added:

- `set-option :rlimit` and a budget inside `check-sat-using` may only **lower**
  the VM's global meter, never raise it.
- The tactic-expression parser has a **depth cap**; the trace reader and the
  reconstructor have **their own meter** (CVM-006a): untrusted tools are bounded too.
- The known blow-ups are **named counters** on the meter (CVM-006b): Ackermann
  (quadratic in applications), `distinct` (quadratic), finite expansion of a
  quantifier over a w-bit sort (2^w), Cooper's lcm growth, cut coefficient growth,
  and NNF/prenex over shared DAGs.

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
- **Certificate.** Two parts. *Pre-blasting:* `rw` and `red` fragments (named
  rules, 6.2). *Blasting and SAT:* an `enc` fragment (the term DAG; the **kernel
  recomputes the CNF** by running its own encoder) plus an `lrat` fragment. The
  encoding's correctness is proved **once** in the kernel library; this closes
  `tcb.bitblast.encoding-correct` and `tcb.drat.checker-correct` for kernel proofs
  (VER11a §9.5), **if the kernel can run it** (6.4, CVM-S1).
- **Families.** `sage`, `brummayerbiere`, `spear`, `stp`, `core`, `mcm`,
  `bmc-bv`, plus the repository's own obligations from the logic gates and ct-compare.
- **Scope cut (review finding 2).** M1's **P1 operators** are exactly those the
  gates and ct-compare use: constants, bitwise, `=`, `distinct`, `ite`,
  add/sub/neg, comparisons, extract/concat/extend. **Shifts, multiplication and
  division are P2** (CVM-M1b): they are where the circuits and the proofs are
  large, and they join when the spike and the first two lemma units (CVM-141b,
  CVM-148a) report real costs.
- **Units.**
  - *Word-level rewrites, one PASS per operator family* (CVM-100 to 108):
    constant folding (its own arithmetic); Boolean and `ite`; bitwise; additive
    (and multiplicative with constants, P2); shifts (P2); `extract/concat/extend`;
    comparisons; division by constants (P2); variable elimination by definition.
  - *Kernel lemmas per family* (CVM-112 to 119) and the `rw`, `red` reconstructors
    (CVM-111, 119) and the `Impl.trans` fold (CVM-111a).
  - *Bit-blaster groups* (CVM-120 to 128): bitwise/`ite`/`=`; add/sub/neg;
    comparisons; extract/concat/extend; Tseitin (P1); shifts, multiplication,
    division (P2). They exist today in `bit-blaster` and `bv.rs`; the unit is the
    **refactor to a group with a named entry point and tests**, not a rewrite.
  - *The kernel encoder and its Rust port* (CVM-130 to 135, below).
  - *Kernel lemmas per group* (CVM-140 to 148): half and full adder, ripple-carry
    induction, comparisons, `tseitin_sound`, `blast_correct`, each split to a
    reviewable size (the first draft hid these in 350-to-400-line rows).
  - *LRAT* (CVM-150 to 154): `Pif.Lrat` semantics and `rup_step_sound`;
    `checkLRAT` and its soundness; reflection glue; the `lrat` reconstructor; the
    **term-route fallback (P1)**; RAT soundness (P2, needed before bounded
    variable elimination may appear in certified mode). `cdcl-sat` LRAT emission
    is KRN-009, which VER11a's backlog ranks P2 and this program needs at P1.
  - *Bridges* (CVM-155, 156): section 6.9.
  - *Speed* (CVM-160, 161): variable elimination and vivification, proof-logged,
    **fast path only** until CVM-154.

**The encoder is the kernel's; Rust ports it (review finding 17).** The kernel
library contains the encoder as a function in the kernel's language: the circuits
for each P1 group, with sharing by term-DAG node, in a fixed order. The solver's
**certified re-solve** uses a **direct Rust port of that encoder** (CVM-130 to
134), not a "certified mode" of the optimised AIG blaster. The LRAT proof is
about exactly the CNF the kernel will recompute, so the kernel needs from the
solver only the LRAT bytes; a wrong port makes the clauses differ, so the proof
does not check: a rejection, never a false proof. A differential (CVM-135) runs
the Rust port against the kernel's encoder on small widths. *Alternatives:*
(a) a certified mode of the fast blaster with matching order (what the first
draft specified: eight units and an order-matching risk for no soundness gain);
(b) the solver ships its CNF as data and the kernel checks it equals the encoder's
output up to renaming (needs a canonical form; larger). *Cost:* the fast path
(AIG hashing, rewriting) answers SAT and finds UNSAT first; for UNSAT, certified
mode **re-solves** over the reference CNF, which is slower and larger. That is
the default (Q6); whether it is affordable is the spike's second question.

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
  `branch` (`x <= k ∨ x >= k+1`, tautology in `Int`), `cg` (a **Chvátal–Gomory derivation**: multipliers, a divisor and a
  rounding, **conditional on the bound atoms it used**, so it is a lemma clause,
  not an unconditional fact), `tight` (GCD tightening, the same shape), `tri`
  (trichotomy and the two-inequality form of an equality atom) and `gcd` (a
  divisibility contradiction). Each is a lemma the kernel proves from `Int` (and,
  for LRA, `Pif.Rat`) axioms. The engine is **CVM-217's own `Int` linear
  arithmetic**: `KRN-012a` is a `Nat` procedure and is not reused as one. QF_LRA
  has **no kernel type to be certified against** until `Pif.Rat` exists
  (CVM-209a, 209b: pairs of `Int` and a positive denominator, with an ordered-field
  lemma set); real-sorted goals wait for it.
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
  Skolemisation is *equisatisfiable*, not equivalent, so its certificate is a
  `red` fragment; the Skolem **function** is defined by `Classical.choice`
  (6.8), so such a claim is classical (a footprint the owner sees: Q8) and, with
  `kernel.logic.consistent` open, reaches `checked`, not `proved`.
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
| **SAT** + model | The model is evaluated against **every live assertion**. A quantified assertion is evaluated by enumeration when its domain is finite and small; otherwise the answer is `Unknown`, not SAT (an evaluator cannot decide it) | The independent evaluator (CVM-008, with the second implementation CVM-008a) | A *refutation* of a claim (a counterexample) and `tested` evidence |
| **UNSAT** + certificate | The certificate is reconstructed into a PIF proof of a **signed goal** and checked by the kernel | `warrant-smt-recon` (untrusted) then `pif-check` (VER32; trusted) | A VER31 `proved` claim, subject to the (c) ids it still depends on |
| **UNSAT** without a certificate | Nothing | nothing | Only `tested` (6.6) |
| **Unknown** | n/a | n/a | Nothing |

A solver that answers wrongly can therefore cause at most **a false rejection**
(the kernel refuses a proof) or **a false counterexample that fails evaluation**
(caught); never a false proof of the signed goal. That is VER32 §1.3 applied to a
new producer.

**What "certified" is relative to (finding 4 of the Fable review).** A kernel
proof is a proof *of a statement*. For a claim, the statement is the signed goal,
so the proof is about the claim. For a **benchmark file with no signed goal**
(CI, fuzzing, the `c_kernel` metric of 1.1), nothing binds the proof to the file
unless we bind it: the statement is produced by an **independent reifier**
(CVM-157: `.smt2` to a library term, written without the reconstructor's code) and
the reconstructor must prove exactly the statement the reifier produced. A
reconstructor that reified some other, trivially false formula then fails the
statement match instead of counting as "certified". The same rule governs the
fuzz oracle of section 9. Three limits, stated
(confirmation pass, R3): (a) **`c_kernel` on goal-less benchmarks is relative to
`Pif.BV`'s semantics**, because the independent reifier produces a `Pif.BV` term,
not to SMT-LIB's own semantics; the gap between them is closed by the two
evaluators' differential (CVM-008a) and, for code-level goals, by the bridges
(6.9). (b) The CVM-025 rule applies to **signed goals only**. (c) **Common-mode
risk:** the reifier and the solver share the `smt-lib-format` parser, so a parser
bug is invisible to both; the mitigations are the oracle's own parser (Z3 and cvc5
read the same file, 9) and the hostile corpus.

### 6.2 Fragment kinds

One fragment kind per kind of reasoning. A fragment is a record in the trace
format (6.3). Each kind has exactly one small reconstructor (6.4), **and every
unit that can contribute to UNSAT names its kind in the backlog's certificate
column**; a unit that emits nothing must say `none: model-preserving` and why.

| Kind | Emitted by | Asserts | Kernel justification |
|---|---|---|---|
| `input` | the loader | an original assertion | goal binding (VER32 §3.6) or the independent reifier (6.1) |
| `rw` | a PASS (including NNF/prenex, constant folding, rule rewrites) | goal `G` equals `G'` by a named rule at a position | the rule's library lemma plus congruence |
| `red` | a PASS that is only equisatisfiable or introduces fresh symbols (purification, monomial abstraction, variable elimination, Skolemisation, Ackermann) | `G'` unsat implies `G` unsat | the pass's library lemma |
| `enc` | the BV tactic and the Boolean skeleton | the CNF is the **kernel encoder's** output on this skeleton and term DAG | `blast_correct`, `tseitin_sound`, and `Eq.refl` by reduction |
| `lrat` | `cdcl-sat` | the clause set (kernel CNF plus lemma clauses) is refuted by this LRAT proof | `checkLRAT_sound` and reduction. **Certified mode is RUP-only**; RAT steps (bounded variable elimination) need a RAT soundness unit (CVM-154) before they may be used |
| `farkas` | simplex, including propagated bounds (an explanation) | a theory lemma `¬l1 ∨ … ∨ ¬lk` from weighted bound atoms | linear arithmetic over `Int` after clearing denominators |
| `tri` | arithmetic equality atoms | `t = c`  iff `t <= c ∧ t >= c`, and trichotomy | order lemmas on `Int`/`Pif.Rat` |
| `tight`, `cg` | LIA normalisation, cuts | a Chvátal–Gomory derivation: multipliers, a divisor, rounding, **conditional on the bound atoms used** (so it is a lemma clause, not a fact) | `Int` division and rounding lemmas |
| `branch`, `gcd`, `cooper` | LIA | integer lemma; each Cooper elimination step is a lemma. **`cooper` is `solver-attested` until CVM-226a is measured, and is excluded from the M3 `c` gate** (R6) | `Int` axioms and divisibility lemmas |
| `euf` | congruence closure, disequalities | a theory lemma from a merge chain | `Eq.trans`, `congrArg` |
| `inst` | arrays, quantifiers | an instance of a library axiom or a quantified hypothesis | `specialize` |
| `dt` | datatypes (injectivity, disjointness, acyclicity, exhaustiveness) | an instance | per-datatype no-confusion lemmas |
| `fe` | finite expansion of quantifiers | a quantifier over a small bit-list sort equals the conjunction over its values | an induction lemma over bit-lists |
| `nl` | nonlinear | a multiplication-fact instance | `Int.mul` lemmas |

**Propositional glue is always LRAT** over the kernel-recomputed CNF of the
Boolean skeleton **plus the theory lemma clauses**; each lemma clause is proved
by its fragment and enters as an extra axiom clause. Lazily found lemmas (arrays,
quantifier instances, splits) reach the kernel as follows: **certified mode
re-solves once** with the final lemma set added as input clauses, so one LRAT
proof covers skeleton plus lemmas. Theory combination adds no fragment of its own
because each shared equality `s = t` is an ordinary atom and the split `s = t ∨ s ≠ t`
is a tautology of the skeleton (axiom-free under the clause semantics of 6.8), but the
fresh symbols purification introduces are `red` fragments, and the equality atom
must be **one shared kernel proposition** for all theories.

### 6.3 The trace format, and who reads it

**Decision.** The solver writes a **trace** (`cvm-trace-1`), line-oriented, one
fragment per line in dependency order, including the LRAT steps. The trace is
**not** a trust boundary; it may change between releases (its version is in its
header; a reconstructor refuses an unknown one). **The certificate that matters
is PIF**: `warrant-smt-recon` reads a trace and the goal, and writes a PIF claim
file the kernel checks. Illustration (not normative; the grammar is):

    cvm-trace 1
    input   a1  (and (bvule a #x01) (distinct b #x00))
    rw      r1  from a1  rule bv.ule.const  at 0
    enc     b1  from r1  width 8
    lrat    p1  from b1  steps 212
    close   c1  from p1

**Grammar, not a hand-written parser.** The repository's rule is that new
formats are defined by `.tokens`/`.grammar` files and read by the shared
grammar-driven parser (the Kiln decision). So the trace format
(`code/grammars/cvm/trace.*`) and the tactic-expression language of
`check-sat-using` (`code/grammars/cvm/tactic.*`) are grammar-driven (CVM-024a,
CVM-017a). `smt-lib-format` is a hand-written, hostile-input-hardened reader of
a standard format; migrating it is owner question Q14, not an assumption.

**Alternatives.** (a) The solver emits PIF directly: rejected, it would put PIF
construction inside the solver and couple two moving formats. (b) A separate
solver-certificate format the kernel reads natively (Z3's proof format, LFSC):
rejected, it enlarges the trusted reader against VER32's budget. (c) **Chosen**:
untrusted trace, untrusted per-kind reconstructor, trusted PIF alone.

### 6.4 Reflection, the kernel's limits, and the term route

**The problem the review found.** `lean-kernel`'s default limits are term depth
1,000, 2,000,000 reduction steps and 4,000,000 new nodes, and VER32 §7 says a
workload that needs more must use a **named, reviewed, raised-limits
configuration**. A naive design (the LRAT bytes and the CNF as one right-nested
list of unary numbers) already approaches depth 1,000 for the 8-bit gate example
of the first draft, and Lean's own `bv_decide` uses `Lean.ofReduceBool` exactly
because kernel reduction of an LRAT checker does not scale. So reflection is a
**hypothesis to be measured**, not a design.

**What the design now commits to.**

1. **Measure first.** CVM-S1, a spike before any library work (it may use
   throwaway Lean-generated or Rust-built PIF): a hand-written
   `checkLRAT` in PIF and a synthetic proof (the 4-NAND XOR miter at 8, 32 and 64
   bits) through `pif-check`, recording reduction steps, depth and new nodes. Its
   output decides the data encoding, the limits configuration, and whether
   reflection or the term route is the default for each size.
2. **Data encodings that respect depth:** certificate data is **chunked** (each
   chunk a separate constant of bounded size, so no term is deeper than the cap)
   and folded by the checker; the CNF is **never data**: the kernel recomputes it
   (`enc`). Numbers are **binary** via the `lits` feature (Q7, now a
   prerequisite of M1) so indices and variable ids cost one node, not their
   value.
3. **A named configuration** `cvm-smt-v1`: raised steps and nodes, the same depth
   cap, `lits` enabled, recorded in the pin file, reviewed and owner-signed; never
   by disabling a check (VER32 §7). The gate's named configuration fixes it (VER32 §3.5); the chain comes from the sign-off (E1).
4. **The term route (LRAT as proof terms, VER11a §9.5 option 1) is a P1
   fallback**, not an afterthought (CVM-153): it is the route for small proofs
   and for any size the spike shows reflection cannot reach.
5. **Per-kind reconstructors** stay small because the library does the work:

| Kind | Library (proved once) | Reconstructor does | Mode |
|---|---|---|---|
| `rw`, `red` | one lemma per named rule or pass | build the `Eq`/implication chain, fold with `Impl.trans` (CVM-111a) | direct terms |
| `enc` | `blast_correct`, `tseitin_sound`, encoder | state the goal, `Eq.refl` | reflection |
| `lrat` | `checkLRAT_sound` | write chunked certificate data | reflection or term route |
| `farkas`, `tri`, `tight`, `cg`, `branch`, `gcd`, `cooper` | `Int` and `Pif.Rat` lemmas (KRN-012a is **a different thing**: a `Nat` procedure; the `Int` engine is CVM-217's own) | the weighted sum, multipliers | direct terms |
| `euf` | `Eq.trans`, `congrArg` | the merge chain | direct terms |
| `inst`, `dt`, `fe` | per-axiom lemmas | `specialize`, instances | direct terms |

**Composition.** Reductions give implications between goals (`G' unsat → G
unsat`, kinds `rw`, `red`); leaf kinds close a goal. The reconstructor folds the
chain with `Impl.trans`, the same structure that ran the strategy. The trusted
proof path grows one kind at a time: M1 ships `rw`, `red`, `enc`, `lrat`; any
query that needs another kind is `solver-attested` until that kind lands. The
reconstructor and trace reader have their **own meter** (CVM-006a): untrusted
does not mean unbounded. **The independent checkers must run these proofs too:**
KRN-019 (the second kernel) and the Lean replay harness (PIF-010) must run
reflective proofs under `cvm-smt-v1` with `lits` (CVM-177); otherwise
`kernel.checker.correct` cannot close for claims that use them.

### 6.5 What each answer is worth on the VER31 ladder

| Evidence | Rung it can support | Open (c) ids it carries |
|---|---|---|
| Model checked by evaluator (a counterexample) | refutes; `tested` evidence | none |
| UNSAT, no certificate | `tested` at most (`solver-attested`, 6.6) | n/a |
| UNSAT, DRAT checked by `drat-check` under the conditions of 6.7 | `bounded(k)`, until CVM-170b retires this route | `tcb.drat.checker-correct`, `tcb.bitblast.encoding-correct`, `tcb.symexec.engine-correct`, the VC-generator id |
| UNSAT, reconstructed to PIF and kernel-checked (6.4) | `proved` if no other (c) id is open | none from the solver; the kernel's ids (VER11a §13), and `kernel.logic.consistent` **when classical axioms are in the footprint** (6.8) |

**There is no row for a Rust per-kind checker.** The first draft offered one
("`checked` with a `tcb.cvm.<kind>-checker` id"); the review showed that nothing
would then check the composition (skeleton, purification, preprocessing, lemma-to-
clause mapping), which is a whole trace checker. If the owner ever wants a
non-kernel route, it is one `tcb.cvm.trace-checker` with that scope, specified
then; it is not specified here (Q15).

### 6.6 Decision: "solver-attested" is a label, not a rung

An UNSAT with no certificate is neither a certificate nor a test: an unverified
statement by our own tool, which VER31 treats as (c) tool trust. So:

- It is recorded in the report as evidence kind `solver-attested` (solver,
  version, query digest), a new field in `ca-assurance-report` (CVM-026).
- The gate **never** accepts it as `proof.certificate` and never lets it raise a
  claim above `tested`. It can *lower* confidence by refuting (a checked model).
- A claim whose only UNSAT evidence is attested is labelled "solver-attested, not
  certified" the way VER31 §8 prints open ids.

The VER31 amendment (CVM-170b, a spec-only PR separate from the reconstructor code):
(1) certificate schemes `pif:sha256:` accepted, `solver-attested` refused;
(2) the `solver-attested` evidence kind, capped at `tested`; (3) for the DRAT
route, a certificate **record** binding the VC and the bound (6.7). Not a new rung;
the owner may prefer one (Q5).

### 6.7 The `bounded(k)` rule (VER33 rung A1), with the DRAT route made honest

VER33 maps its rung A1 to VER31's `bounded(N)`. The question: is the "bounded"
evidence the solver's UNSAT, or executed tests? **Neither alone.**

- VER31 rung 2 is a **certificate rung**: rung 1 (`tested`) plus a checked
  certificate that no counterexample exists up to N. Not tests; not a bare answer.
- **Executed tests are `tested`**, including the boundary vectors the solver chose.
- **A solver UNSAT with no certificate is `solver-attested`**, capped at `tested`,
  including a UNSAT in a logic or fragment kind whose reconstructor has not landed.
- A **counterexample** (SAT) is confirmed by evaluation of the model and the code
  and refutes the claim.
- On the **PIF route**, the bound `k` is part of the signed goal's statement, so
  the certificate proves *that* bounded statement (**`k` is signed on `pif:`,
  record-bound on `drat:`**).

**The DRAT route, which the first draft overclaimed.** A DRAT proof certifies a
CNF. Three things sit between the claim and that CNF, and none of them is
certified by DRAT: the claim-to-VC generator and the symbolic execution that
produced the VC (VER31 always adds `tcb.symexec.engine-correct`; the generator needs its own id, `tcb.vcgen.claim-faithful`, owned by VER33's ADL-013); the **formula-level preprocessing**
(rewrites, variable elimination: DRAT certifies the CNF of the *rewritten*
formula, so a wrong rewrite is silently trusted); and the **bound itself**, which
lives in an unsigned query, with `drat:sha256:` hashing the proof and not the VC.
A generator that emitted `k = 0`, or a contradictory precondition, would produce a
valid DRAT proof and the report would say `bounded(128)`. Rules until CVM-170b
retires this route:

1. **Evidence strategy, preprocessing off.** The DRAT route runs the minimal
   strategy `blast, sat` only. SAT-level preprocessing (bounded variable
   elimination, vivification) is fine, because DRAT checks it against the CNF;
   formula-level passes are not allowed on this route unless they bring a (c) id.
2. **A certificate record binds** `(VC digest, k, CNF digest, DRAT digest)`
   (CVM-159). The gate recomputes the CNF digest from the VC.
3. **Non-vacuity** (CVM-158): (a) a SAT witness for the preconditions **at size
   exactly k**, confirmed by the evaluator; (b) the UNSAT core (assumption-based,
   4.5) must include the negated conclusion; otherwise the result is `vacuous` and
   refused as bounded evidence.
4. **The ids** `tcb.symexec.engine-correct` and `tcb.vcgen.claim-faithful` (and `tcb.drat.checker-correct`, `tcb.bitblast.encoding-correct`) are listed with
   the claim; §8.3 names who owns the generator.
5. **Retire it.** Once CVM-170a exists, DRAT stops being rung evidence for new
   claims; it stays as the fast internal check inside the engine (Q5).
6. **Sunset for existing DRAT-backed claims** (CVM-159a). After CVM-170a every
   claim whose `bounded(k)` rests on a DRAT certificate is **re-evidenced through
   PIF by the release that follows the CVM-174 report**, or it drops to `tested`
   in that release's notes. The deadline is a milestone, not a date (this spec
   promises none), and the inventory of such claims is part of CVM-159a.

| Evidence | Claim rung (VER31) |
|---|---|
| Tests, including solver-generated boundary vectors | `tested` |
| UNSAT + DRAT under rules 1 to 4 | `bounded(k)`, with the (c) ids above |
| UNSAT + PIF checked, bounded statement | `bounded(k)`, no solver (c) ids |
| UNSAT + PIF checked, statement for every input | `checked`/`proved` per VER31 |
| UNSAT, no certificate | `tested` at most; printed as `solver-attested` |

### 6.8 Fit with PIF as merged (VER32 at d01ad9a9ab8)

**Two sides of one chain (Fable confirmation pass, R1).** VER32 E1 to E6: the
sign-off supplies the ordered base chain per claim, the chain is linear (every
file imports the digests of *all* files before it, E2), every base file is
allow-listed (E4), and the signature binds the base environment digest. The first
revision put the SMT libraries *after the model*, "only the ones a claim needs".
Under E2 that makes a different chain, hence a different digest, allow-list entry
and signature, for every combination of model and libraries. The libraries are
**proof plumbing, not meaning**, so they do not belong in the signed base at all:

    SIGNED BASE  (chain from the sign-off, E1; allow-listed, E4; bound by G6)
      PIF prelude -> ADL layers (VER33's prefix rule) -> WIR semantics (PIF-019)
        -> program constant / model -> [axioms file, classical claims only]

    PROOF SIDE   (unsigned, not allow-listed; checked by the kernel; no axioms)
      Pif.Lrat -> Pif.Prop -> Pif.BV -> (Pif.Rat, ...) -> bridges -> claim file

The base is the meaning: the prelude, the ADL layers, the semantics of WIR, the
program under proof (its constant, by value) and the model, plus the optional
axioms file. The proof side is a chain of several files **after the signed base,
ending in the claim file**. Each proof-side file is read by the kernel like any
other (it declares definitions and theorems and no axiom, A2), but it is neither
signed nor allow-listed: the only way it can matter is to prove the goal, and
the goal is signed. **Rule (CVM-025, signed goals only): no proof-side file
declares a constant that occurs in a goal.** (Benchmark statements of 6.1 are not
signed goals, so the rule does not apply to them.) That is what lets a library
theorem need no id: a wrong library *definition* cannot make a goal true, and at
worst a proof fails. Growing a library (a new theory file, a new lemma) touches
**no signature and no allow-list**.

*This needed a VER32 E3/E4 amendment. **Applied 2026-11-24 and owner-approved**: it is
VER32 E7 and E8, where the proof-side role is `proof` and the chain file lists it between
the signed base and the claim.* (The text below is the original statement of the need.)
*This needs a VER32 E3/E4 amendment* (a proof side that is not part of the base
environment digest and not subject to the allow-list), which goes in the
cross-spec follow-up (13.1); this spec does not edit VER32. **Fallback if the
owner rejects the amendment:** VER33's prefix rule, with the libraries placed
*before* the model in the signed base and the re-signing cost accepted each time a
library changes.

**What is meaning and what is plumbing.**

- **Meaning** lives in files that are *already signed*: the WIR semantics file
  (PIF-019, VER11a §6), and VER33's ADL fixed-width definitions. A goal mentions
  these (or `Int`, `Bool`).
- **Plumbing** is `Pif.Lrat`, `Pif.Prop`, `Pif.BV`, the encoder and their lemmas:
  the proof side. **The CVM-025 rule above** (no proof-side file declares a
  constant occurring in a goal) is what makes "library theorems need no id" true.
- **Bridges** (6.9) connect the two. They are theorems stated over both, and they
  are *how* `Pif.BV` is shown faithful to the signed semantics. Hence no
  `cvm.bv.semantics-faithful` id (the first draft had one: finding 6); faithfulness
  is a kernel theorem, not a differential test.
- **Opaque atoms.** The skeleton library reflects only propositional structure over
  **atoms** (`Nat → Prop` valuations). **Clause semantics (R5):** a clause holds
  iff **not all its literals are false**, `¬(¬l1 ∧ … ∧ ¬lk)`, not `l1 ∨ … ∨ lk`.
  That makes RUP intuitionistically sound, and the shared-equality split
  `s = t ∨ s ≠ t` provable **without classical axioms even for abstract sorts**
  (`¬(¬(s = t) ∧ ¬¬(s = t))` is a theorem). Only non-stable *conclusions* stay
  classical (below). Each theory is reified **separately** (a typed
  term language per theory: bit-vector terms, integer terms), so no single `Expr`
  has to close over all sorts and constructors.

**What a reconstructor emits.** A **claim file**: `theorem` (named and typed as
the goals list says), `def` data constants (chunked certificate data) and helper
lemmas; **no axiom** (VER32 A2) and **no `goal` record or goals list** (those come
from the elaborator, signed under G6). It is *given* the theorem name and goal and
fails if its proof's type differs. **Roles:** the BV path binds `eq` and the
prelude's `nat`/`list` roles; `lits` (Q7) comes from the gate's named configuration (VER32 §3.5: features and
limits come from the configuration, the chain from the sign-off; a claim file
cannot enable it: `FeatureNotAllowed`). No `Init`, no Lean `BitVec`, no
`Decidable` (it is not in the PIF prelude): stability of a conclusion is proved by
`Bool` case analysis on the decision procedure's `Bool` result.

**Classical reasoning (Q8, corrected).**

- A refutation of `H ∧ ¬C` proves `C` **intuitionistically whenever `C` is stable**
  (`¬¬C → C`). Every decidable conclusion is stable: bit-vector and `Int` (in)equalities,
  `≤`, and Boolean combinations of them, **including claims over uninterpreted
  functions or arrays whose conclusion is such a fact**. These need **no axiom**.
- Only conclusions that are not stable need classical reasoning: equality of an
  abstract sort's elements, equality of function-typed or array-typed terms
  (extensionality), existentials, quantified conclusions. That needs
  `Classical.choice`, `propext`, `Quot.sound`.
- **`kernel.logic.consistent` is bucket (c), open, until the owner decides** (VER32
  §9.2, VER11a Q5). A claim whose footprint includes the three axioms therefore
  **cannot be `proved`** today, and VER34 does not pre-empt that decision (the
  first draft called it bucket (a)). A classical claim reaches `checked` at best
  until Q5 of VER11a is decided.
- **Where the axioms come from:** the **axioms file placed after the model**
  (above), which binds `eq`, `iff`, `nonempty`, `quot` and then the `axiom.*` roles
  structurally (VER32 A1, 3.5). The model's and libraries' digests are unaffected;
  footprints are per claim, so no prelude variant is needed.
- **Units that must exist** (CVM-311c, 311d): deriving excluded middle and
  `byContradiction` from the three axioms (Diaconescu; needs `Quot.sound` for
  function extensionality) as library lemmas.
- **Skolem functions use `Classical.choice`.** The first draft's "`Exists.elim` per
  instance, no choice" fails once congruence closure merges `sk(t1)` and `sk(t2)` for
  provably equal `t1`, `t2`: per-instance witnesses carry no such equality. Such
  goals are already classical; define the Skolem function by choice.

### 6.9 Three fixed-width semantics, and the bridges (findings 5 and 14)

Real code has three notions of a machine word, and the first draft connected them
by a differential test. They are:

| Where | What | Source |
|---|---|---|
| **WIR** (code level) | the straight-line semantics's word operations: `wrapping_shl` masks the shift amount; division by zero is `stuck`/panic | PIF-019, VER11a §6 |
| **ADL** (model level) | VER33's fixed-width types: K0 is `Int` with explicit range guards (as in G-M1); K1 is `lits` plus **native `Adl`/`Pif` fixed-width words, no `Init`** (VER33 ADL-060) | VER33 |
| **Solver** | `Pif.BV`, SMT-LIB semantics: shift by at least the width gives 0; `bvudiv` by 0 is all ones; signed remainder follows the dividend | VER34 |

They differ. A VC encoder must therefore **state the WIR semantics in SMT terms**
(a WIR shift becomes `bvshl x (bvurem amt w)`; a WIR division carries the side
condition `y /= 0`), and the kernel must be told, by theorem, that this encoding
is the semantics. That is a **bridge theorem per operation group** (CVM-155):

- `WIR op = Pif.BV op` on the group's domain (bitwise, `=`, `ite`: P1; add, sub,
  neg: P1; comparisons: P1; extract/concat/extend: P1; shifts with amount masking:
  P2; division with its side condition: P2).
- `ADL K0` (`Int` with range guards) `<->` `Pif.BV` for in-range values (CVM-156).
- **K1** is `lits` plus native `Adl`/`Pif` fixed-width words with no `Init`. If
  their representation is not `Pif.BV`'s, the bridge has the same shape as
  CVM-155 and is added with VER33's ADL-060; there is **no third, `Init`-based
  bridge**. An `Init` import (PIF-009) is a separate, later tier for the Lean
  escape hatch only. (The first revision said "VER32 forbids Init"; VER32 says
  only that the native prelude and `Init` are not interchangeable, and PIF-009
  does import `Init`. Corrected.)

**Who proves "VC valid implies goal"?** Not this spec. A claim over WIR `run` is
proved by the VC builder (KRN-012, VER11a §9) with per-step obligation lemmas; the
bit-level ones are universally quantified statements over the semantics's word
operations. **The solver discharges an obligation**, whose statement is over the
signed semantics; the bridge turns it into a `Pif.BV` statement; the library
proves that. Obligations are *lemmas inside the claim file*: a wrong obligation
cannot prove the claim, so nothing needs to trust the VC builder for soundness
(only for completeness), and the signed goal remains the only thing bound.

---

## 7. Worked example: a real obligation from the logic gates (G-M2)

The first draft used "XOR from four NANDs on 8-bit words". That is not a gate
claim (the gates take `{0,1}` and reject the rest) and not a code-level statement.
The honest example is the obligation behind `validate_bit`, which every shipped
gate calls (VER32 §9.3): the code is one comparison and an `assert!`.

**The obligation.** Straight-line WIR semantics of `validate_bit(a : U8)`: the
step relation takes the `assert!` branch exactly when `ule a 1` is false. The
claim "`no_panic` on every input in `{0,1}`, and a panic on every other byte"
needs the bit-level fact (a lemma inside the claim file, from the step lemma's
side condition):

    obl : ∀ a : Wir.U8,  Wir.ule a 1 = true  ↔  (a = 0 ∨ a = 1)

(`Wir.U8`, `Wir.ule`, `0` and `1` are the **signed semantics file's** constants,
PIF-019.) Its negation is the VC: `∃ a, ¬(ule a 1 ↔ a = 0 ∨ a = 1)`.

**1. The VC as SMT.** The encoder of ADL-012 to 014 states it over `(_ BitVec 8)`:

    (declare-const a (_ BitVec 8))
    (assert (distinct (bvule a #x01) (or (= a #x00) (= a #x01))))
    (check-sat)                                   ; expected: unsat

**2. Rewrite.** Rule `bv.ule.const` rewrites `bvule a #x01` to `(or (= a #x00) (= a #x01))`
at once, the two sides become syntactically equal, and `(distinct t t)` folds to `false`.
The solver finishes in the preprocessor; the trace is `input`, one `rw`, one `red`
(or `rw`) for `distinct t t`, and `close` with **no `enc` and no `lrat`**. This is
the cheap, common case, and it shows why `rw` is the first kind to certify.

(If the rewrite is switched off, the blast path runs: 8 lanes of a comparison
circuit against two equality circuits, roughly 40 AND nodes and 90 clauses by hand
count, a few conflicts, an LRAT proof of a few dozen steps. The CVM-S1 spike uses
a 4-NAND XOR miter at 8, 32 and 64 bits as its synthetic size ladder, **not as a
claim**.)

**3. Reconstruction.** `warrant-smt-recon` emits, in the claim file (library
constants in `Pif.*`, chain 6.8):

1. the `rw` step as a library lemma instance `Pif.BV.ule_one : ule a 1 = true ↔ (a = 0 ∨ a = 1)`;
2. the bridge instance `Wir.ule a 1 = Pif.BV.ule (Pif.BV.ofWord a) (Pif.BV.const 1)`
   (CVM-155c) and the word-to-bit-list correspondence for `a = 0`, `a = 1`;
3. `Impl.trans` of the two, closing `obl` by `Iff.intro`, with no `Classical`: the
   conclusion is an `Iff` of decidable facts (stable, 6.8).

If the reconstructor reified `a` at the wrong width, or used the wrong rule, the
proof's type does not match `obl` and the kernel rejects. The signed goal is the
claim; `obl` is a lemma the claim's proof uses.

**4. For a case that needs SAT** (for example the bit-level facts behind
`wasm-leb128`), the trace continues `enc` then `lrat`: the kernel recomputes the CNF
by running the encoder on the skeleton and term DAG, runs `checkLRAT` on the
chunked certificate data, and `lrat_sound` plus `blast_correct` turn
"no satisfying assignment" into the obligation. Whether that reduction fits the
kernel's limits at 32 and 64 bits is **exactly what CVM-S1 measures**; this spec
does not claim it does.

**5. What it closes.** For the SAT path, `tcb.bitblast.encoding-correct` and
`tcb.drat.checker-correct` for that claim. For the rewrite-only path, nothing
solver-related: the solver is not in the trusted set at all. **What it does not
close:** `wir.semantics.lean-faithful` and the lowering ids (VER32 §9.3 lists them);
those are others' work.

**Cost.** The rewrite-only case is trivial. A 64-bit multiplier VC is a different
animal (millions of LRAT steps); the term route and `cvm-smt-v1` exist for it, and
M1's P1 scope excludes multiplication until the spike and the first two lemma
units report real numbers.

---

## 8. Front ends and consumers

### 8.1 SMT-LIB v2 (`smt-lib-format`)

The reader grows by units: `define-fun`, `let`, indexed identifiers as real
terms (retiring the flattened `Apply "_"` shape), `declare-datatype(s)`,
named assertions, `check-sat-assuming`, `get-value`, `set-option`, and the
SMT-LIB `check-sat-using`-style extension for tactics. The writer round-trips
all of it. **Strictness stays**: an unsupported command is a parse error, never
silently ignored. `smt-lib-format` is a hand-written, hardened reader of a
standard format; whether it becomes grammar-driven is Q14. The trace format and
the tactic-expression language are grammar-driven from the start (6.3).

### 8.2 New instructions

`DefineFun`, `NamedAssert`, `CheckSatAssuming`, `GetValue`, `DeclareDatatype`,
`CheckSatUsing`, `Minimize`/`Maximize`, `SetOption(produce-certificates, rlimit; lower-only)`, `GetCertificate`.
Each is one INSTR unit: the variant, parser, printer, validator and VM arm, with
a rejection test.

### 8.3 Warrant (VER33): rungs and what its section 6.3 needs

VER33's ladder is A0 (tests from a Warrant file), **A1 (the solver: vectors and
bounded model-versus-code checks)**, A2 (kernel proofs of the model), A3 (lift
code and check it against the model), A4 (generated code). The solver serves A1
and the verification-condition discharge of A2 and A3 (VER33 6.3). Its backlog
rows that call the solver are **ADL-010** (A1 vector generation), **ADL-012a/b, ADL-013 and ADL-014** (A1
bounded model-versus-code: the encoder, the claim-to-VC generator, the bounded
driver), **ADL-015** (VC discharge) and, for later profiles, ADL-051 with ADL-009
(recursive-model obligations) and ADL-061 (floats).

VER33 6.3 asks for five things "from VER34". Each is covered:

| VER33 6.3 need | Covered by | Milestone |
|---|---|---|
| Models with **minimisation and maximisation** (boundary points) | `minimize`/`maximize` opcodes and extremum search for BV and Int (CVM-020 to 022). *Added in this revision; the first draft had no unit for it* | M0 (BV, Bool), M2 (Int) |
| Incremental **push/pop** | CVM-016, plus the metamorphic check CVM-019b | M0 |
| A bounded QF fragment over **Bool, Int, fixed-width, list and ADT** operations | Bool and BV: M1. Int: M2. Lists and ADTs: bounded **lists as arrays plus a length** in M2 (CVM-230 and the bridge), as **datatypes** in M3 (CVM-300 to 306). VER33 should say which it models lists as (section 13) | M1, M2, M3 |
| **Proof-producing mode**: LRAT for the SAT core, proof terms or checkable lemmas per theory | `lrat`, `blast`, `rw` fragments and the `bv-decide` reconstructor in M1; per-theory fragment kinds (6.2) as they land | M1 (BV), then per theory |
| **Deterministic output for a fixed seed**, so vectors reproduce byte for byte (VER04e) | Meter-based determinism (4.8) plus a seeded tie-break and phase option; vectors record (solver version, seed) as provenance and are checked by the evaluator, not by re-running (4.7, CVM-023) | M0 |

Also covered, from VER33's text rather than its list: the encoder's faithfulness
check ("differential harness of 5.8") is the independent evaluator CVM-008 plus the
fuzzer; counterexample confirmation is the model check of 6.1. What VER33's
consumers must supply, and this spec does not: the **WIR-to-SMT encoder** (symbolic
execution, `warrant-ir`'s `sym.rs`) and the claim-to-VC generator, which sit with
**ADL-012 to 014**: they are not VER34's, but **their faithfulness id must be registered
there**, because on the DRAT route they are in the trusted set (6.7). The word
semantics they encode and the bridges to `Pif.BV` are 6.9.

Extremum answers are *test-input choices*, not claims: "this is the smallest model"
is not evidence for any claim, so it needs no certificate; a minimum is only ever
used as a boundary vector, which is then run (rung `tested`).

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
   (binaries pinned by SHA-256, run as subprocesses, never linked; **oracle
   agreement is never counted in `c`**). A
   disagreement is triaged mechanically: if the oracle says SAT, **our evaluator
   checks the oracle's model**; if we said UNSAT, our certificate is
   re-checked (the kernel route, with the statement from the independent
   reifier); if the oracle says UNSAT and we said SAT, our model is re-checked
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
   the benchmark and fuzz runs is reconstructed and checked by the kernel **against
   a statement produced by the independent reifier** (6.1), so a reconstructor that
   proved some other formula cannot pass; `c_kernel` and `c_trace` (1.1) are tracked
   per logic.
5. **Performance tracking per logic.** `cvm-bench` records, per benchmark and
   per commit: result, deterministic meter totals, wall-clock (informational),
   and per-logic `R(L,T)` and PAR-2. A regression in solved-count or a
   certified-fraction below the milestone value fails the nightly job. Wall
   time never decides a verdict.
6. **Mutation controls.** Per VER27's sabotage protocol: a seeded bug in a
   rewrite rule, a Farkas coefficient, a congruence merge, an LRAT hint must be
   **rejected by the kernel** or **caught by the evaluator**, and **the vacuity
   check** (CVM-158: a bounded VC with a satisfiable-at-size-k precondition) is
   itself mutation-tested; the corpus keeps
   the mutants (as the `bv_faithfulness` audit already does for 18 lowering
   mutants).

Cadence: units 2, 3, 4 run on every PR that touches a tactic (small budgets);
1 and 5 run nightly; the full benchmark gate runs per milestone.

---

## 10. Milestones

Each is a measurable gate. Numbers are **proposals**: they are re-based on the
CVM-S0 baseline and approved by the owner (Q3) after the **definitions** of `R`,
the Warrant-near subset and `c` (1.1) are frozen. Every gate also requires: zero
wrong answers across the fuzz and differential corpora, each unit merged with its
certificate story (the backlog's certificate column), and the unit card (2.3).

| Milestone | Scope | Gate | Unlocks |
|---|---|---|---|
| **CVM-M0** Foundations | Baseline, bignum (staged), instructions (8.2), trace and tactic grammars, meters, tactic trait and combinators, real push/pop, real unsat cores, two evaluators, `cvm-bench`, oracle adapter, fuzz base, boundary points and seeded provenance | Baseline table recorded; combinators tested; every existing test green through the new trait; push/pop equivalence relation passes; VER13 ids registered | VER33 ADL-010 (A1 vector generation) on Bool and BV |
| **CVM-M1** Certified QF_BV, P1 scope | The operators the gates and ct-compare use: bitwise, `=`, `ite`, add/sub/neg, comparisons, extract/concat/extend. **Shifts, multiplication, division are P2** and join when the first two lemma units and CVM-S1 report real costs | **Prerequisites:** CVM-S1 done; `lits` and the named configuration `cvm-smt-v1` approved (Q7); authoring route chosen (Q13). **Gate:** (i) the obligation corpora of CVM-171 (gates, G-M2) and CVM-172 (ct-compare) are **certified through `pif-check` under `cvm-smt-v1`** at the widths in set W (proposal {8, 16, 32, 64}; S1 may shrink W, never silently), each within the kernel-time budget S1 sets; (ii) pinned Warrant-near subset: `R(QF_BV, 60 s) >= 0.5`; (iii) full pinned QF_BV: `R >= 0.25`; (iv) **`c_kernel` is 100%** of solver UNSATs on the Warrant corpus and `c_trace >= 90%` on pinned QF_BV, **with no "or labelled" escape**: an UNSAT in scope that is not kernel-checked fails the gate | VER33 **ADL-012 to 014** (A1 `bounded(k)`, PIF route) and **ADL-015** (VC discharge) for fixed-width obligations; VER32 G-M2; VER11a K-M1c; LEAN00 L4c can start |
| **CVM-M1b** BV widening | Shifts, multiplication, division, with their bridges and lemma units | Each operator group's `c_kernel` 100% at widths the measurements justify; `R` re-measured | The remaining gate and ct-compare operators |
| **CVM-M2** Arithmetic, EUF, arrays | 5.2 to 5.4, `Pif.Rat`, BV/Int bridge, the CDCL(T) stage-2 hook | `R(QF_UF, 30 s) >= 0.6`; `R(QF_LRA, 60 s) >= 0.5`; `R(QF_LIA, 60 s) >= 0.4`; `R(QF_AX, 60 s) >= 0.4`; `R(QF_UFBV, 60 s) >= 0.4`; **`c_trace >= 95%`** for kinds that have landed and **`c_kernel` 100%** on the Warrant corpus; `adj-constraint-solver` and LANG23 on the new engine | ADL-010/008/018 for `Int`, arrays and UF |
| **CVM-M3** Datatypes, quantifiers, nonlinear | 5.5 to 5.7, the classical library units | `R(QF_DT, 60 s) >= 0.5`; `R` for quantified UFLIA/AUFLIA `>= 0.25`; Presburger LIA: each pinned instance solved or `Unknown` with a reason, none wrong; `R(QF_NIA) >= 0.2`; `c_trace >= 90%`; `c_kernel` 100% on the Warrant corpus | ADL-015 for ADT algorithms, lists as datatypes and `forall` claims (classical claims reach `checked`, not `proved`, until VER11a Q5) |
| **CVM-M4** Strings, sequences, floating point | 5.8, as Warrant asks, after a kernel IEEE model exists (CVM-407a, 407b) | per-logic `R >= 0.2`; FP through the BV path | ADL-061 |
| **CVM-M5** Hardening and speed | Rows chosen from CVM-500's profile | Re-baselined `R` raised by the owner's margin; kernel-time budget met | Scale; the nightly report |

---

## 11. The decomposition tree and the backlog

The leaf rows are in the backlog shard
[VER-BACKLOG.d/constraint-vm-smt.md](VER-BACKLOG.d/constraint-vm-smt.md); the main
[VER-BACKLOG.md](VER-BACKLOG.md) carries a pointer and the milestone headings.
Each row is one PR and states: the unit, its slot, its size target, **its
certificate** (a fragment kind from 6.2, or `none: model-preserving`, `none:
harness`), and its dependencies. A row that turned out to be several PRs was
split (review record, finding 15); there are **no placeholder rows** (a
"profiling-driven" row is added by CVM-500's report, each as a real leaf).

| Milestone | Group | Units |
|---|---|---|
| M0 | Spike-free groundwork | baseline (S0), README correction, staged bignum (002a to 002e), named assertions and cores, `define-fun`, staged indexed identifiers, meters (including the untrusted tools' meter and the blow-up counters), `cvm-bench`, oracle adapter, two evaluators, trace and tactic grammars, boundary points, VER13 ids, the report field |
| M0 | Engine plumbing | `Goal`/`Tactic`, combinators, atom table and offline `smt` loop, default strategies and re-expressed tactics, incremental push/pop, `check-sat-using`, options |
| M1 | Spike and prerequisites | CVM-S1, `cvm-smt-v1`, the authoring pipeline and its footprint lint |
| M1 | Libraries (`Pif.Lrat`, `Pif.Prop`, `Pif.BV`) | definitions, rule lemmas per family, `red`, `Impl.trans`, blast lemmas per group, encoder, LRAT checker and soundness, term route, RAT soundness |
| M1 | Solver side | rewrites per family, blast groups, Rust port of the kernel encoder per group, differential |
| M1 | Bridges, binding, DRAT route | WIR and ADL bridges, independent reifier, vacuity, certificate record, operator coverage |
| M1 | Closing | reconstructor, corpora, gate, cost report, VER31 amendment text |
| M1b | BV widening | shifts, multiplication, division: solver, port, lemmas, bridges |
| M2 | EUF, simplex and LRA, LIA, arrays, combination, hook | as in 5.2 to 5.4, each with its CERT and RECON, `Pif.Rat`, trichotomy, CG, Cooper |
| M3 | Datatypes, quantifiers, nonlinear, classical library | as in 5.5 to 5.7 |
| M4, M5 | Floating point and strings; hardening | as in 5.8; rows from the profile |

The ordering inside a milestone is by dependency; a unit's PR does not start until
its dependencies' PRs have merged and their certificate stories exist.

---

## 12. Owner questions

Each has a recommendation and the adversarial reviewer's view (Fable, round 1).
None blocks M0.

| # | Question | Recommendation | Fable's view |
|---|---|---|---|
| **Q1** | **Build-versus-reuse.** Are Z3 and cvc5 acceptable as CI oracles? | **Yes, oracles only**: nightly, pinned by digest, subprocess, never linked, never in a trust path, never evidence for a claim. Oracle agreement is never counted in `c` | Agree; binaries are fetched and verified offline at test time |
| **Q2** | **Which logic first.** | **QF_BV**, with M1's P1 scope cut to the operators the gates and ct-compare use | Agree |
| **Q3** | **Performance targets.** | Approve the **metric and its definitions** (1.1: `R`, the frozen Warrant-near predicate, meter calibration, the counting rule, `c_trace`, `c_kernel`). **Do not approve the numbers yet**; re-base them after CVM-S0 | Agree on the metric's shape; **disagree** with approving the numbers' shape now: fix the subset predicate, calibration and `c` first. Adopted |
| **Q4** | **Publication.** | **Rust crates only**; other lanes call a `cvm` binary or WASM | Agree |
| **Q5** | **A "solver-attested" rung?** And the DRAT route. | **No rung**: a label capped at `tested` (6.6). `bounded(k)` needs a certificate (6.7). **Retire DRAT as rung evidence for new claims once CVM-170a exists**; until then only under 6.7's four rules | Agree; DRAT `bounded(k)` needs the ids and bindings first. Adopted |
| **Q6** | **Certified-mode cost.** | **Measure first, as a spike (CVM-S1), not "after CVM-174".** Default: **re-solve** with a **Rust port of the kernel encoder** (finding 17), not a "certified mode" of the optimised blaster | Disagreed with "decide after CVM-174". Adopted |
| **Q7** | **Literal acceleration (`lits`) and limits.** | **Yes, and a prerequisite of M1**: `lits` and the named raised-limits configuration (both from the gate's named configuration, VER32 §3.5) `cvm-smt-v1` (6.4), reviewed and owner-signed. Decide before the library is written | "Yes, and not optional": without them M1 gate (i) is probably unreachable. Adopted |
| **Q8** | **Classical axioms.** | Decidable (stable) conclusions are axiom-free, even over UF and arrays. Only non-stable conclusions are classical, via an **axioms file as the last member of the signed base** (6.8). The id `kernel.logic.consistent` is **(c), open**, so such claims reach `checked`, not `proved`, until VER11a Q5. Skolem functions by `Classical.choice`. Diaconescu and `quot` units | **Disagree as first worded** (id is (c); one chain; narrow the set; choice for Skolems). Adopted |
| **Q9** | **Determinism and parallelism.** | Single-threaded, deterministic **per solver build**; committed vectors record (solver version, seed) as provenance and are checked by the evaluator, not by re-running; no parallel mode before M5 | Agree, with the provenance fix. Adopted |
| **Q10** | **Strings and floating point.** | Defer to M4; FP needs a kernel IEEE model (CVM-407a, 407b, jointly with VER33's float note and ADL-061) | Agree: fix the missing owner of the IEEE model. Adopted |
| **Q11** | **Bignum.** | `bignum-core`, **staged**: add the type, migrate each consumer, remove `i128` (CVM-002a to 002e) | Agree on `bignum-core`; **disagree on "one PR"**. Adopted |
| **Q12** | **Fate of the bounded LIA enumerator and `adj`'s IIS.** | Keep the enumerator as a second oracle; move `adj` onto real cores after M2 | Agree |
| **Q13** | **How are kernel-library proofs authored?** (new; it decides M1's size and whether Lean is on the build path) | **(a)** for M1, with conditions. See below. **Decided 2026-11-24: see 12.1** | New question; Fable asked for it |
| **Q14** | **Is `smt-lib-format` to become grammar-driven?** | The trace and tactic formats are grammar-driven (6.3). Keep `smt-lib-format` hand-written for now (it is hardened and standard); revisit when it needs the next large feature | Asked for an explicit answer under the no-hand-written-parser rule |
| **Q15** | **A non-kernel route for `checked`** (a Rust trace checker)? | **No.** If ever wanted, one `tcb.cvm.trace-checker` with an explicit scope | Deleted the per-kind-checker row; asked for either deletion or this |

**Q13 in full: how the kernel library's proofs get authored.** The library
(`blast_correct`, `tseitin_sound`, `checkLRAT_sound`, the arithmetic and bridge
lemmas) is thousands of lines of proofs, and `Init` is not available in the chain.
Three options:

| Option | What | For | Against |
|---|---|---|---|
| **(a)** Lean source in `prelude` mode over a **mirror of the PIF prelude**, elaborated by the pinned official Lean toolchain as an **untrusted data producer** | The owner already approved this role for the toolchain (VER11a, LEAN00 QA). The generated PIF files are **committed artifacts**; Lean is needed only to regenerate them, **not in CI's critical path**. The kernel checks the output; the gate checks axiom footprints, so any `propext` or `Quot.sound` that `simp`/`omega` introduces appears honestly | Tactics make several-thousand-line proof development feasible; fastest to M1 | The toolchain is on the *authoring* path; tactic output can pull axioms into footprints the library promises empty |
| **(b)** Native Rust term builders, extending PIF-018 | No Lean anywhere | Fully ours | Hand-built terms for induction proofs are very expensive; PIF-018 is a P1 builder for simple inductions only |
| **(c)** Wait for LEAN00 L3 (our own elaborator) | Our own tactic framework | Ours, eventually | Blocks M1 for a long time |

**Recommendation: (a) for M1, conditional on the spike CVM-S2 (below), with conditions:** (1) a **footprint lint** (CVM-109b)
that fails the build when a `Pif.BV` or `Pif.Lrat` library file's generated PIF
contains an axiom, so those footprints stay empty and tactic use is restricted to
what does not need `propext`/`Quot.sound`; (2) generated files are committed and
digest-pinned, regenerated by a script (CVM-109a), and a regeneration is a
reviewed change; (3) move to (b) or (c) later, library file by library file, since
a file's PIF is independent of how it was authored. *Fable's view:* asked that
the route be an owner question decided before M1, and that M1's scope be cut to
what the gates use. Adopted.

**The condition (confirmation pass, R2).** Lean in `prelude` mode has **no
`Init`**: `simp`, `omega` and `decide` are unavailable or reference `Init`, and
Lean auto-generates auxiliaries (`casesOn`, `recOn`, `below`, `brecOn`,
`noConfusion`, `match_n`, `PProd`) that must pass PIF-008 and must not collide with
the PIF prelude's own declarations. So the recommendation is **conditional on
CVM-S2**: prove one real lemma (`blast_add_sound`) this way and report **which
tactics survive** and what the generated PIF looks like. If too few survive, (b)
becomes M1's route and the library estimate (14) is redone. CVM-109a also gains a
**mirror-identity check**: the Lean mirror must be declaration-identical to the PIF
prelude.

### 12.1 Owner decisions, 2026-11-24

Recorded as **decided**. The recommendation and Fable's view above are kept for the record.

| # | Decision | Condition |
|---|---|---|
| All | **Every recommendation that the drafter and Fable agreed on is accepted as a working decision**: Q1 to Q12, Q14 and Q15 as recommended (including Q3: the metric and its definitions are approved, **the numbers are not**, and are re-based after CVM-S0; Q5: no rung, DRAT retired as rung evidence for new claims once CVM-170a exists; Q8: decidable conclusions axiom-free, classical ones through the axioms file, `kernel.logic.consistent` stays (c) open) | "Working decision": revisited only on evidence from an implementation or a spike |
| **Q13** | **The Lean toolchain is an untrusted producer for kernel-library proofs**: Lean source in **`prelude` mode over a mirror of the PIF prelude (no `Init`)**, the generated PIF files **committed and digest-pinned**, regenerated by a script (CVM-109a), Lean needed only to regenerate and **not in CI's critical path** (option (a)) | **Conditional on the spike CVM-S2**: it must prove `blast_add_sound` this way and report which tactics survive. If too few survive, option (b) (native term builders, PIF-018) becomes M1's route and the library estimate (section 14) is redone. The conditions of the recommendation stand: the footprint lint (CVM-109b), committed digest-pinned files, the mirror-identity check, and a later move to (b) or (c) file by file |
| (VER32 Q1, Q2, E3/E4) | The VER32 decisions that touch this spec: the hand-written trusted reader is approved; **option D** (integer switch-level `evaluate_digital`) is the route for G-M2, so CVM-171's obligation corpus covers all 11 gates; **the unsigned proof side is approved**, so 6.8's primary route stands and its fallback (VER33's prefix rule) is not needed | None |

No new owner question arises from this follow-up in VER34.

---

## 13. Relation to other specifications

| Spec | Relation | Note or amendment |
|---|---|---|
| **LANG24** | VER34 *is* LANG24's later phases, structured. The four-crate layering is unchanged; units fill it. LANG24's "LIA as built" divergence note remains correct | Dated note added to LANG24 (2026-11-24). LANG24 PR 24-H (Z3 bridge) is superseded by the oracle adapter (CVM-007a) |
| **LANG23** | Consumer. Keeps `PROVEN_SAFE/UNSAFE/UNKNOWN` | None |
| **VER09** | `cdcl-sat` is the Boolean core | Needs LRAT (KRN-009), a theory hook (CVM-246a to d), inprocessing (CVM-160/161); a VER09 note when each lands |
| **VER10** | Bit-blasting circuits are the BV groups | The kernel encoder is the reference; the Rust port (CVM-130 to 134) follows it; VER10 addendum when CVM-130 lands |
| **VER11** | DRAT stays the fast-path check inside the engine | Not rung evidence for new claims after CVM-170a (Q5) |
| **VER11a** | §9.4 to 9.5 are what M1 delivers | Requests in 13.1 (KRN-009 to P1; its LRAT checker superseded by CVM-150/151; KRN-012a is not the `Int` Farkas engine; reflection is measured, KRN-019 and the replay harness must run `cvm-smt-v1`): **applied 2026-11-24** |
| **VER32** (d01ad9a9ab8) | The certificate is PIF; reconstructors are untrusted producers of **claim files** on an unsigned **proof side** (6.8) | Agrees with: `Pif.*`; the sign-off supplies the chain (E1); no axioms or goals in claim files (A2, G1); structural validation of logic axioms (A1); `kernel.logic.consistent` as (c); features and limits from the gate's named configuration (§3.5). ~~Needs the E3/E4 proof-side amendment~~ **Applied 2026-11-24 (VER32 E7, E8; owner-approved)**, with the other items of 13.1 |
| **VER33** (409a671977f) | Warrant is the front end and the main consumer; this spec serves its section 6.3 (8.3) | No conflict of substance. **Edits VER33 needed (13.1): applied 2026-11-24** |
| **VER31** | The claim ladder is used as is | Amendment text (CVM-170b, spec-only): `pif:` scheme, `solver-attested` kind, DRAT certificate record (6.7). **Pointer added to VER31 2026-11-24; CVM-170b still owns the text** |

### 13.1 The follow-up list (for a separate PR after the merges; taken verbatim)

VER34 edits none of these. They are listed so the follow-up can apply them.

**Status: all applied in the cross-spec follow-up of 2026-11-24** (VER32 "Follow-up
2026-11-24" F1 to F6; VER33 G1 to G9; VER11a and VER31 follow-ups in their own records;
backlog rows in `VER-BACKLOG.md`, `VER-BACKLOG.d/warrant-adl.md` and
`VER-BACKLOG.d/constraint-vm-smt.md`). Two items needed a reading: VER32 item 1 says
proof-side files declare no axiom, while VER32 A2 lets a claim file declare registered
`Warrant.Assume.*` axioms (the G-M2 contract is one); VER32 E7 resolves it by giving the
**claim file** R-AX-1 and every **other** proof-side file no axiom at all. VER33 item 2 left
the name of the `Init` tier open; VER33 4.2 calls it K2.

**VER32**
1. **E3/E4 proof-side amendment.** A **proof side** after the signed base: files that are not part of the base environment digest, not allow-listed and not signed, may declare definitions and theorems but no axiom (A2), and **may declare no constant occurring in a signed goal** (CVM-025). The claim file is the last proof-side file. Fallback if the owner rejects this: VER33's prefix rule with the libraries before the model and the re-sign cost accepted.
2. **Named configuration `cvm-smt-v1`** (raised steps and nodes, same depth cap, `lits`) recorded in §3.5 and the §7 limits; features and limits come from the configuration, the chain from the sign-off.
3. The **axioms file** as the optional last member of the signed base.
4. **PIF-019 states the word representation for widths above 8** (CVM-155 depends on it).
5. Register the ids VER34 opens in the VER13 slice (parallel to PIF-016), including `tcb.vcgen.claim-faithful` with VER33.
6. Check that no text says `Init` is forbidden (VER32 says only that the native prelude and `Init` are not interchangeable; PIF-009 imports `Init`).

**VER33**
1. **§2.2 and §9.1: the SMT library is not meaning.** It is proof plumbing on the proof side.
2. **K1 split:** K1 is `lits` plus native `Adl`/`Pif` fixed-width words, no `Init`; an `Init` import is a separate later tier for the Lean escape hatch only (ADL-060); no third bridge.
3. Register **`tcb.vcgen.claim-faithful`** under ADL-013 and ADL-006a (the claim-to-VC generator), and the encoder under ADL-012a/b.
4. **Add `tcb.drat.checker-correct`** (with `tcb.bitblast.encoding-correct` and `tcb.symexec.engine-correct`) to rung A1's id list while the DRAT route lasts.
5. **§6.3 cites VER34 §6.7's DRAT rules** (formula-level passes off, certificate record, non-vacuity, sunset) and the wording "`k` is signed on `pif:`, record-bound on `drat:`".
6. Name the certificate schemes (`drat:`, `pif:`) and the `solver-attested` kind capped at `tested`; **ADL-006b cites CVM-170b**, which owns the VER31 amendment text.
7. §6.3 "list": state whether Warrant lists are datatypes or arrays with a length (decides M2 versus M3).
8. ADL-010 and ADL-012 to 015 cite CVM-020 to 023, **CVM-170a/170b** (not "CVM-170") and CVM-173 as prerequisites.
9. §13.1 item 3 (environment chain): signed base then proof side, as in 6.8.

**VER11a**
1. **KRN-009 (LRAT emission) to P1**; CVM-152 and CVM-246d need it.
2. KRN-009's "verified LRAT checker in `warrant-lib`" is **superseded by CVM-150/151** (`Pif.Lrat`).
3. **KRN-012a** is a `Nat` procedure; it is not the `Int`/`Pif.Rat` Farkas engine (CVM-217).
4. §9.5 records that reflection is a hypothesis measured by CVM-S1, and that **KRN-019 and the replay harness run `cvm-smt-v1` proofs** (CVM-177).

**VER31**
1. CVM-170b owns the amendment text: `pif:sha256:` scheme, `solver-attested` kind capped at `tested`, the DRAT certificate record.

| **LEAN00** | L4c is the `enc` + `lrat` path | None |
| **VER27, VER15** | Hostile input, finding protocol, weakness registry | A soundness finding in the solver is a VER27 finding |

---

## 14. Risks and what this spec does not promise

- **Reflection may not fit.** The kernel's limits and the data encodings are
  unproven (6.4); CVM-S1 measures them before any library work, and the term route
  is a P1 fallback. If both fail at 32 and 64 bits, M1's gate (i) shrinks W and the
  report says so.
- **The library is the critical path.** Its size is not known: the first draft's
  5,000 to 8,000 lines was too low (Lean's own bit-blaster and LRAT checker are
  several times that, with full tactics). **Planning figure: 10,000 to 25,000
  lines of library source for M1's P1 scope**, re-estimated after the first two
  lemma units (CVM-141b, CVM-148a). M1 is **months, not weeks**.
- **Speed.** `cdcl-sat` has never been compared with MiniSat or CaDiCaL. Our CDCL
  may be several times slower; fresh theory solvers will be slower than tuned ones.
- **Decomposition can be wrong.** If a unit repeatedly overruns, the tree is
  revised here (2.1).
- **No completeness claim** for quantifiers, nonlinear arithmetic, strings.
  `Unknown` is a first-class answer.
- **This spec promises no dates.** Milestones are gates, not schedules.

---

## 15. Review record

### Fable, round 1 (adversarial review of a3fa78ef314): 18 findings

Verdict: not ready as drafted; the trust architecture holds (no way to get a false
UNSAT accepted as a kernel proof of a signed goal). Every finding is adopted unless
a deviation is stated.

| # | Finding | Severity | Disposition |
|---|---|---|---|
| 1 | Reflection does not fit the kernel's limits | critical | **Fixed** (6.4, CVM-S1, S2): spike first; chunked data, binary numerals via `lits`; named `cvm-smt-v1`; term route P1 |
| 2 | Nothing decides how library proofs are authored; estimates too low | critical | **Fixed** (Q13; 14; scope cut to P1 operators; CVM-109a/b; estimate raised) |
| 3 | DRAT `bounded(k)` overclaims | critical | **Fixed** (6.7: ids, strategy off, certificate record, non-vacuity, retire). Deviation: DRAT is retired *after* CVM-170a rather than removed now (Q5) |
| 4 | Certified fraction `c` can be met without certifying | critical | **Fixed** (6.1 independent reifier CVM-157; `c_kernel`/`c_trace`, 1.1; no "or labelled"; M1 gate iv) |
| 5 | Three fixed-width semantics bridged by a test | critical | **Fixed** (6.9, CVM-155a to 155f, 156; section 7 redone on a G-M2 obligation) |
| 6 | `cvm.bv.semantics-faithful` misplaced; one monolithic library file | critical | **Fixed** (6.8: libraries split, opaque atoms, goal-may-not-mention rule). **Deviation:** instead of a new small signed semantics file, the signed meaning is the *existing* WIR and ADL files and `Pif.BV` becomes proof-internal, so the id disappears and faithfulness is a bridge theorem, not an id |
| 7 | Certificate gaps per fragment kind | major | **Fixed** (6.2 new kinds `tri`, `tight`, `cg`, `fe`; certificate column on every row; CG with conditional bound atoms; RUP-only with CVM-154; `Pif.Rat` rows 209a/b) |
| 8 | Combination under-specified; Skolem claim wrong; quantified SAT check | major | **Fixed** (6.2 purification `red`, shared equality proposition, re-solve once with lemma clauses; 6.8 Skolem by choice; 6.1 enumerate-or-`Unknown`) |
| 9 | Classical goals inconsistent with VER32 as merged | major | **Fixed** (6.8: id is (c); one chain with axioms file after the model; classical set narrowed to non-stable conclusions; Diaconescu and `quot` units; no `Decidable`) |
| 10 | Evaluator not independent | major | **Fixed** (4.5: CVM-008a second implementation; folding uses its own code, CVM-100; extension rows CVM-208, 308) |
| 11 | A Rust per-kind checker is an unspecified second trust path | major | **Fixed** (6.5: row deleted; Q15) |
| 12 | Determinism versus byte-for-byte reproduction; hostile input | major | **Fixed** (4.7 provenance; 4.8 lower-only budgets, depth cap, blow-up counters, reconstructor meter; CVM-006a/b, 017b, 023) |
| 13 | `R(L,T)` and targets | major | **Fixed** (1.1: frozen predicate, calibration, counting rule, `c` split; M2/M3 targets; CVM-S0 self-contained) |
| 14 | Mis-prioritised dependencies on other backlogs | major | **Fixed in this spec, requests recorded** (13: KRN-009 to P1; duplication resolved; KRN-012a distinguished; K1/Init conflict stated for VER33) |
| 15 | Rows that are not leaf units | decomposition | **Fixed**: CVM-002, 005a, 015, 016, 017, 018, 170, 222, 224, 246, 252, 306, 313, 315/316, 406 split into sub-rows; CVM-501/502 removed |
| 16 | Wrong or circular dependencies | decomposition | **Fixed**: atom table moved to CVM-014a (M0); purification before combination; kernel encoder before the Rust port; Ackermann dependency; M0 rows no longer cite certificates; CVM-022 prerequisite |
| 17 | Reference-order CERT rows mostly unnecessary | decomposition | **Fixed**: a Rust port of the kernel encoder (CVM-130 to 134) plus a differential (CVM-135); re-solve is the default |
| 18 | Missing rows | decomposition | **Fixed**: trace and tactic grammars, VER13 registration, report field, vacuity, `Impl.trans`, reconstructor meter, operator coverage |

### Fable, confirmation pass (on de34480c674): ready to merge as a draft

| # | Finding | Severity | Disposition |
|---|---|---|---|
| R1 | Libraries after the model, "only the ones needed", break under VER32 E2 (per-combination chains, allow-list entries and signatures) | major | **Fixed** (6.8): unsigned, non-allow-listed **proof side** after a signed base (prelude, ADL layers, WIR semantics, program constant/model, optional axioms file); the chain picture now includes the WIR semantics and the program constant; CVM-025 rule "no proof-side file declares a constant occurring in a goal"; VER32 E3/E4 amendment in 13.1; fallback is VER33's prefix rule |
| R2 | Lean in `prelude` mode has no `Init` (`simp`/`omega`/`decide`, auxiliaries may collide) | major | **Fixed**: spike CVM-S2 (`blast_add_sound`, report which tactics survive); CVM-109a mirror-identity check; Q13 (a) is **conditional on CVM-S2** |
| R3 | `c_kernel` is relative to `Pif.BV`; CVM-025 for signed goals only; shared parser | minor | **Fixed** (6.1): all three stated, including the common-mode risk |
| R4 | CVM-S1 may use throwaway PIF; independent checkers must run `cvm-smt-v1` with `lits` | minor | **Fixed** (6.4, CVM-177) |
| R5 | Opaque-atom clause semantics; no classical note on the equality split | minor | **Fixed** (6.8 `¬(all literals false)`; notes removed from 6.2 and CVM-240) |
| R6 | Quantified LIA (`cooper`) | minor | **Fixed**: `solver-attested` until CVM-226a is measured; excluded from the M3 `c` gate |
| R7 | `Pif.BV.ule_one` statement | minor | **Fixed** (7): `ule a 1 = true ↔ (a = 0 ∨ a = 1)` |
| D | DRAT sunset | | **Added** (6.7 rule 6, CVM-159a): re-evidence through PIF by the release after the CVM-174 report, or drop to `tested` |
| S | Shard: CVM-159 names a real row; stale ADL ids | | **Fixed**: ADL ids moved to the final VER33 (ADL-010, 012a/b to 015, 051, 060, 061, 003a to c, 004a); CVM-159 now points at CVM-159a |
| X4 | K1 | | **Fixed** (6.9): native words, no `Init`, no third bridge; wording about VER32 corrected |
| X10 to X12 | CVM-170b owns the VER31 text; CVM-155 depends on PIF-019's word representation; VER32's wording on chain versus configuration | | **Fixed** (13.1, 6.8, CVM-155a) |


### Follow-up 2026-11-24 (cross-spec follow-up; owner decisions)

VER34 edits no other spec's text in its own body; its section 13.1 listed the edits for a
separate PR, which this follow-up is. The list is **applied in full** (statuses below).
Changes to VER34 itself are only status notes and the owner decisions.

| # | Change | Source | Where |
|---|---|---|---|
| H1 | **Owner decisions recorded** (12.1): every recommendation both the drafter and Fable agreed on is a working decision; **Q13 decided: the Lean toolchain as an untrusted producer for kernel-library proofs** (prelude mode, no `Init`, committed digest-pinned PIF), **conditional on the CVM-S2 spike**; the VER32 decisions that touch VER34 (hand-written reader, option D, proof side) noted | owner, 2026-11-24 | 12, 12.1 |
| H2 | 13.1 marked **applied**; the two readings it needed are stated there: proof-side files carry no axiom except that the claim file keeps R-AX-1 (the G-M2 contract is a registered `Warrant.Assume.*` axiom); the `Init` tier is VER33's K2 | VER34 13.1 against VER32 A2 and VER33 4.2 | 13.1 |
| H3 | 6.8's "needs a VER32 E3/E4 amendment" now points at VER32 E7 and E8 (role `proof`); the fallback (VER33's prefix rule) is not needed | owner decision, VER32 E3/E4 amendment approved | 6.8 |
| H4 | The 13 table rows for VER32, VER33, VER11a and VER31 say "applied" | this follow-up | 13 |
| H5 | **Backlog rows** (`VER-BACKLOG.d/constraint-vm-smt.md`): M1's prerequisites reflect Q13 decided (conditional on CVM-S2); CVM-025 depends on PIF-016d and on PIF-002b and PIF-003e (the reader class and the chain role that enforce its rule); KRN-009 is P1 (VER11a request 1); CVM-152, CVM-246d and CVM-160 cite it | VER34 13.1 (VER11a 1), VER32 follow-up F5, F18 | backlog |
| H6 | **New finding, for CVM-025**: the rule "no proof-side file declares a constant occurring in a signed goal" is enforced by the reader class `ProofSide` (VER32 5.3, R-GOL-4), not by a gate-only check; until PIF-002b lands the rule has no enforcement, so no VER34 claim file may be signed | VER32 follow-up F18 | CVM-025 |

**Not applied, and why.** Nothing in VER34 was left unapplied. No new owner question.

### Follow-up / G-M1 registration 2026-11-25

**Status: a note only; no VER34 rule changes.** VER31 section 13.6 (proposed, pending owner
signature) **cites** CVM-170b for the `solver-attested` kind and does not copy its text, as the
ADL-006b row says, so CVM-170b still owns that text. Two requests follow for CVM-170b:

| # | Request | Source | Where |
|---|---|---|---|
| K1 | Read the clause "`pif:sha256:` accepted" as "as defined in VER31 13.2": PIF-020 defines what the certificate is (file digest, both environment digests, kernel crate version and commit, sign-off digest), CVM-170b says when the DRAT route may claim `bounded(k)` and refuses `solver-attested` as a certificate | VER31 13.2, 13.6; VER32 8.3 | CVM-170b |
| K2 | `tcb.vcgen.claim-faithful` is registered as a proposal (bucket (c)) alongside the G-M1 ids, while `tcb.drat.checker-correct`, `tcb.bitblast.encoding-correct` and `tcb.symexec.engine-correct` are already in the catalogue; the per-scheme rule of VER31 13.2 does not add the three solver ids to a `pif:` claim whose proof uses no solver, which agrees with 6.5 | VER33 9.1; VER31 13.2 | 6.5 |
