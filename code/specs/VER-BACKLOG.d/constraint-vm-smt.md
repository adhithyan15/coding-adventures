# Constraint VM as Warrant's SMT solver (VER34)

**Authority:** current plan, decomposition tree, queue and status for growing the
Constraint VM into the solver Warrant uses. The main
[VER-BACKLOG](../VER-BACKLOG.md) keeps a short pointer and the milestone
headings; this shard owns the rows. The design is
[VER34](../VER34-constraint-vm-smt-solver.md).

Owner direction, 2026-11-24: "We have our own constraint VM. I would love to build
that out as a full fledged Z3 equivalent so that Warrant can use that." And:
"Any big problem can be decomposed into smaller problems… we should be able to break
it down into smaller and smaller chunks until it can fit in as instructions in our
constraint VM architecture."

[VER34](../VER34-constraint-vm-smt-solver.md) grows the
Constraint VM into the solver Warrant uses. Structural rule (VER34 section 2): the
solver is assembled from small units that fit the LANG24 layers, and **this table
is the decomposition tree**: every row is one leaf unit and one PR. Slot (the
"fits in" layer) is in brackets: **INSTR** opcode, **TACTIC** theory component or
search step, **PASS** preprocessing rewrite, **CERT** certificate emitter, **RECON**
kernel-side lemmas plus an untrusted per-kind reconstructor, **COMB** combinator,
**HARN** validation harness. Size target is source lines, tests extra; a row that
overruns by more than a quarter is split in its PR. Every row has its own tests and
a certificate story (a fragment it emits, or why none is needed). The solver is
untrusted: UNSAT counts toward `proved` only through a PIF proof the kernel checks
(VER34 section 6). Order is dependency order inside a milestone. Spec before code;
nothing starts before the owner answers VER34 section 12 (M0 may start on Q1 to Q3).
ADL-xxx ids are VER33's (A0 to A4 ladder); PIF-xxx and KRN-xxx are
VER32's and VER11a's.

### CVM-M0: foundations

Gate: VER34 section 10, M0. Unlocks VER33 ADL-007 on Bool and BV.

| Priority / ID | Work | State / prerequisite and acceptance |
|---|---|---|
| P1 / CVM-000 | VER34 spec, adversarial review, owner answers to section 12 | in review |
| P1 / CVM-S0 | [HARN, ~300] Baseline spike: run the current `constraint-vm` (QF_Bool, QF_BV, QF_LIA) and Z3 and cvc5 on pinned SMT-LIB families on one machine; record solved counts, PAR-2 and the `R(L,T)` table; re-base VER34 section 10 | after CVM-000; needs CVM-007 for the runner (a throwaway script first) |
| P1 / CVM-001 | [docs, ~50] Correct `constraint-engine` README: the LIA tactic is bounded enumeration, not Cooper, and not "Complete" (VER34 section 3); lesson | none; first |
| P1 / CVM-002 | [core, ~350] `Value::Int` and `Rational` on `bignum-core`; breaking change handled across consumers in one PR | after CVM-001 |
| P1 / CVM-003 | [INSTR, ~250] Named assertions (`:named`) and `check-sat-assuming`; assumption literals | after CVM-002 |
| P1 / CVM-004 | [TACTIC, ~250] Real `GetUnsatCore`: failed assumptions, minimised by deletion; replaces the stub; `adj-constraint-solver` follow-up filed | after CVM-003 |
| P1 / CVM-005 | [INSTR, ~300] `define-fun` and `let` (reader, writer, DAG-sharing expansion); rejection tests for recursion and arity | after CVM-002 |
| P1 / CVM-005a | [INSTR, ~250] Indexed identifiers as real terms (`(_ extract i j)`, `zero_extend`, `(_ bvN w)`); retires the flattened `Apply "_"` shape in `bv.rs` | after CVM-005 |
| P1 / CVM-006 | [COMB, ~300] `Meter`: deterministic counters (conflicts, propagations, nodes, pivots, rounds, bits, depth) and typed `Unknown` reasons; moves existing caps to data | after CVM-001 |
| P1 / CVM-007 | [HARN, ~350] `cvm-bench`: pinned SMT-LIB manifest (digests), runner, solved count, PAR-2, per-logic `R(L,T)`, certified fraction; nightly job | after CVM-006 |
| P1 / CVM-007a | [HARN, ~250] Oracle adapter: Z3 and cvc5 as pinned subprocesses; disagreement triage (evaluator checks oracle models; certificate re-check); never in a trust path | after CVM-007, CVM-008; owner Q1 |
| P1 / CVM-008 | [HARN, ~400] Independent model evaluator for every sort in SMT-LIB terms, sharing no code with any tactic; engine self-check: a model that fails evaluation becomes `Unknown` plus a record | after CVM-002 |
| P1 / CVM-009 | [COMB, ~250] `Goal`, `Tactic` trait (`Reduced`, `Sat`, `Unsat(Fragment)`, `Failed`), `Fragment` record types, `ModelConv` | after CVM-006 |
| P1 / CVM-010 | [COMB, ~120] `then` | after CVM-009 |
| P1 / CVM-011 | [COMB, ~120] `or_else` | after CVM-009 |
| P1 / CVM-012 | [COMB, ~120] `with_budget` | after CVM-009 |
| P1 / CVM-013 | [COMB, ~200] `when` and probes (`has_quantifiers`, `max_bv_width`, `num_theories`, `size`) | after CVM-009 |
| P1 / CVM-014 | [COMB, ~350] `smt(theories)` skeleton: stage-1 offline lazy CDCL(T) loop over registered theory components on `cdcl-sat` assumptions and clause addition; theory-component trait (VER34 4.3) | after CVM-009, CVM-016 |
| P1 / CVM-015 | [COMB, ~150] Default strategy per logic as a data table; the three existing tactics re-expressed through the trait with unchanged answers | after CVM-010 to CVM-013 |
| P1 / CVM-016 | [TACTIC, ~350] Incremental push/pop: scope selector literals on `cdcl-sat` assumptions, theory `backtrack` marks; replaces clone-per-push; test unit running every tactic's tests inside push/pop | after CVM-009 |
| P1 / CVM-017 | [INSTR, ~300] `check-sat-using` and the tactic-expression parser and printer | after CVM-010 to CVM-013 |
| P1 / CVM-018 | [INSTR, ~250] `get-value`, `set-option` (`produce-certificates`, `rlimit`), `get-certificate` | after CVM-009 |
| P1 / CVM-020 | [INSTR, ~250] `minimize` and `maximize` objective opcodes (single term, lexicographic list), parser, printer, VM arm; result is a model plus an "optimal" flag the evaluator can check only for feasibility (VER33 6.3 boundary points; extremum answers are test inputs, not claims) | after CVM-018 |
| P1 / CVM-021 | [TACTIC, ~250] Extremum search for Bool and BV: bit-by-bit descent under push/pop and assumptions | after CVM-016, CVM-020 |
| P2 / CVM-022 | [TACTIC, ~250] Extremum search for `Int`: bound bisection under push/pop; unbounded is reported as unbounded, never guessed (needs CVM-218 for LIA to be complete) | after CVM-021; full strength after CVM-221 |
| P1 / CVM-023 | [COMB, ~200] Seeded determinism: `:random-seed` drives tie-breaking and phase; same input, seed and meter give byte-identical models and traces; reproducibility test (VER33 6.3, VER04e) | after CVM-006, CVM-016 |
| P1 / CVM-019 | [HARN, ~350] Fuzz generator, QF_Bool and QF_BV (well-sorted terms; evaluator, brute-force and certificate checks) | after CVM-008 |
| P2 / CVM-019a | [HARN, ~200] Metamorphic relation family: renaming, permutation, double negation, tautology addition, conjunction splitting | after CVM-019 |
| P2 / CVM-019b | [HARN, ~200] Metamorphic relations: `push; A; pop` equals not asserting `A`; `check-sat-assuming` equals push/check/pop; "assert the model, still SAT" | after CVM-016, CVM-019 |

### CVM-M1: certified QF_BV (gates and ct-compare VCs)

Gate: VER34 section 10, M1. Unlocks VER33 ADL-008 and ADL-018 for fixed-width code, VER32 G-M2, VER11a K-M1c, LEAN00 L4c.

| Priority / ID | Work | State / prerequisite and acceptance |
|---|---|---|
| P1 / CVM-100 | [PASS, ~250] Constant folding for every BV operator, via the evaluator | after CVM-009, CVM-008 |
| P1 / CVM-101 | [PASS, ~200] Boolean and `ite` simplification with BV conditions | after CVM-100 |
| P1 / CVM-102 | [PASS, ~250] Bitwise family rules (`and/or/xor/not`: identities, `x^x`, `x&0`) | after CVM-100 |
| P1 / CVM-103 | [PASS, ~300] Additive and multiplicative rules (`add/sub/neg/mul`, multiplication by a constant to shifts) | after CVM-100 |
| P1 / CVM-104 | [PASS, ~250] Shift rules | after CVM-100 |
| P1 / CVM-105 | [PASS, ~300] `extract/concat/extend` normalisation | after CVM-005a, CVM-100 |
| P1 / CVM-106 | [PASS, ~250] Comparison rules (`ult/ule/slt/sle`, reflexive and constant cases) | after CVM-100 |
| P2 / CVM-107 | [PASS, ~250] Division and remainder by constants | after CVM-100 |
| P1 / CVM-108 | [PASS, ~300] Variable elimination by definition (`x = t`), with model converter and `red` fragment | after CVM-100 |
| P1 / CVM-110 | [RECON, ~350] SMT library as an allow-listed **base-chain file in the `Pif.*` namespace** (VER32 6.4, E6): `Pif.BV` as a bit-list (LSB first, length invariant, no literals), `Pif.Smt.Expr` and `eval`; reconstructor-supplied reification checked by definitional equality against the signed goal; faithfulness id `cvm.bv.semantics-faithful` registered with VER32's VER13 slice (parallel to PIF-016) | after PIF-004, ADL-004 (the base chain is PIF prelude, ADL prelude, this file); VER32 request in VER34 section 13 |
| P1 / CVM-111 | [RECON, ~300] `rw` fragment kind: the reconstructor (congruence chain from a named rule application) and the trace line | after CVM-110, CVM-009 |
| P1 / CVM-112 | [RECON, ~300] Lemmas for CVM-100 and CVM-101's named rules | after CVM-111 |
| P1 / CVM-113 | [RECON, ~300] Lemmas for CVM-102's rules | after CVM-111 |
| P1 / CVM-114 | [RECON, ~350] Lemmas for CVM-103's rules | after CVM-111 |
| P1 / CVM-115 | [RECON, ~250] Lemmas for CVM-104's rules | after CVM-111 |
| P1 / CVM-116 | [RECON, ~300] Lemmas for CVM-105's rules | after CVM-111 |
| P1 / CVM-117 | [RECON, ~250] Lemmas for CVM-106's rules | after CVM-111 |
| P2 / CVM-118 | [RECON, ~250] Lemmas for CVM-107's rules | after CVM-111 |
| P1 / CVM-119 | [RECON, ~250] `red` fragment kind and `solve_eqs_sound` (equisatisfiable passes) | after CVM-111, CVM-108 |
| P1 / CVM-120 | [TACTIC, ~250] Blast group: bitwise, `ite`, `=`, `distinct` (exists in `bit-blaster`; refactor to a group with a named entry point) | after CVM-009 |
| P1 / CVM-121 | [TACTIC, ~250] Blast group: add, sub, neg | after CVM-120 |
| P1 / CVM-122 | [TACTIC, ~250] Blast group: comparisons | after CVM-120 |
| P1 / CVM-123 | [TACTIC, ~300] Blast group: shifts (constant and barrel) | after CVM-120 |
| P1 / CVM-124 | [TACTIC, ~300] Blast group: multiplication, keeping the refusal-before-build estimates | after CVM-120 |
| P1 / CVM-125 | [TACTIC, ~350] Blast group: division and remainder family | after CVM-124 |
| P1 / CVM-126 | [TACTIC, ~200] Blast group: `extract/concat/extend` wiring | after CVM-120 |
| P1 / CVM-128 | [TACTIC, ~250] Tseitin as a unit with a stable variable and clause order | after CVM-120 |
| P1 / CVM-130 | [CERT, ~200] Reference-order emission for group CVM-120 and the `blast` trace line (term DAG plus encoding tag); differential test against the kernel encoder at small widths | after CVM-120; encoder spec from CVM-148 |
| P1 / CVM-131 | [CERT, ~150] Reference-order emission, group CVM-121 | after CVM-121, CVM-130 |
| P1 / CVM-132 | [CERT, ~150] Reference-order emission, group CVM-122 | after CVM-122, CVM-130 |
| P1 / CVM-133 | [CERT, ~200] Reference-order emission, group CVM-123 | after CVM-123, CVM-130 |
| P1 / CVM-134 | [CERT, ~200] Reference-order emission, group CVM-124 | after CVM-124, CVM-130 |
| P1 / CVM-135 | [CERT, ~250] Reference-order emission, group CVM-125 | after CVM-125, CVM-130 |
| P1 / CVM-136 | [CERT, ~150] Reference-order emission, group CVM-126 | after CVM-126, CVM-130 |
| P1 / CVM-137 | [CERT, ~200] Reference-order Tseitin and clause numbering; VER10 addendum for certified mode | after CVM-128, CVM-130 |
| P1 / CVM-140 | [RECON, ~300] `blast_*_sound` lemmas for the bitwise group | after CVM-110 |
| P1 / CVM-141 | [RECON, ~350] Lemmas: add, sub, neg (ripple-carry) | after CVM-140 |
| P1 / CVM-142 | [RECON, ~300] Lemmas: comparisons | after CVM-140 |
| P1 / CVM-143 | [RECON, ~350] Lemmas: shifts | after CVM-140 |
| P1 / CVM-144 | [RECON, ~400] Lemmas: multiplication (shift-add) | after CVM-141 |
| P2 / CVM-145 | [RECON, ~400] Lemmas: division and remainder | after CVM-144 |
| P1 / CVM-146 | [RECON, ~250] Lemmas: `extract/concat/extend` | after CVM-140 |
| P1 / CVM-147 | [RECON, ~350] `tseitin_sound`: definitional CNF equisatisfiable with the circuit | after CVM-140 |
| P1 / CVM-148 | [RECON, ~400] `encode` (the reference encoder, as a kernel function) and `blast_correct` by structural induction over terms | after CVM-141 to CVM-147 |
| P1 / CVM-150 | [RECON, ~350] CNF and LRAT semantics; `rup_step_sound`; one resolution lemma set | after CVM-110; KRN-009's format decision |
| P1 / CVM-151 | [RECON, ~350] `checkLRAT` as a kernel function and `checkLRAT_sound`; reflection glue (`Eq.refl true`); no `Lean.ofReduceBool` | after CVM-150 |
| P1 / CVM-152 | [RECON, ~300] `lrat` fragment reconstructor: LRAT bytes into a data constant, goal assembly; `cdcl-sat` LRAT emission is KRN-009 | after CVM-151, KRN-009 |
| P2 / CVM-153 | [RECON, ~300] Term-route fallback: LRAT steps as proof terms for small proofs (VER11a 9.5 option 1) | after CVM-150, CVM-152 |
| P2 / CVM-160 | [TACTIC, ~350] `cdcl-sat`: bounded variable elimination and subsumption, proof-logged (DRAT and LRAT) | after KRN-009 |
| P2 / CVM-161 | [TACTIC, ~250] `cdcl-sat`: vivification, proof-logged | after CVM-160 |
| P1 / CVM-170 | [RECON, ~350] `bv-decide`: end-to-end reconstructor from a trace and a signed goal to a PIF file; VER31 clarifying amendment for `solver-attested` (VER34 6.6) in the same PR | after CVM-152, CVM-148, PIF-003; serves VER33 ADL-008 and ADL-018; the claim file emits no axiom and no `goal` record (VER34 6.8) |
| P1 / CVM-171 | [HARN, ~300] Corpus: every logic-gate VC at 8/16/32/64 bits through `pif-check`; the worked example of VER34 section 7 first | after CVM-170; G-M2 |
| P1 / CVM-172 | [HARN, ~300] Corpus: the four ct-compare VCs; sabotage controls (VER27) | after CVM-170 |
| P1 / CVM-173 | [HARN, ~250] M1 benchmark gate: pinned QF_BV families, `R`, certified fraction; A1 `bounded(k)` demo: DRAT-checked and PIF-checked bounded VC (VER34 6.7) | after CVM-007, CVM-170; ADL-008 consumes it |
| P1 / CVM-174 | [HARN, ~250] Certified-mode cost report: kernel reduction time per UNSAT, sizes; decides owner Q6 and Q7 | after CVM-171 |

### CVM-M2: EUF, arrays, LRA, LIA, combination

Gate: VER34 section 10, M2. Unlocks ADL-007, ADL-008, ADL-018 for `Int`, arrays and UF.

| Priority / ID | Work | State / prerequisite and acceptance |
|---|---|---|
| P1 / CVM-200 | [TACTIC, ~300] Term DAG and union-find with an explanation forest | after CVM-014 |
| P1 / CVM-201 | [TACTIC, ~250] Congruence table (signature hashing) and merge propagation | after CVM-200 |
| P1 / CVM-202 | [TACTIC, ~250] EUF as a theory component: assert, propagate, conflict with explanation | after CVM-201 |
| P1 / CVM-203 | [TACTIC, ~200] `distinct` and disequalities | after CVM-202 |
| P1 / CVM-204 | [CERT, ~200] `euf` fragment emitter (merge chain) | after CVM-202 |
| P1 / CVM-205 | [RECON, ~250] `euf` reconstructor (`Eq.trans`, `congrArg`) | after CVM-204, CVM-111 |
| P1 / CVM-206 | [PASS, ~250] Ackermann reduction for UFBV, with its `red` lemma | after CVM-119, CVM-202 |
| P2 / CVM-207 | [TACTIC, ~200] EUF model construction (fresh value per class) | after CVM-202 |
| P1 / CVM-210 | [core, ~300] Arbitrary-precision rationals with a symbolic infinitesimal | after CVM-002 |
| P1 / CVM-211 | [TACTIC, ~300] Simplex tableau: sparse rows, slack introduction from linear atoms | after CVM-210 |
| P1 / CVM-212 | [TACTIC, ~250] Bounds: assert-bound and immediate conflicts | after CVM-211 |
| P1 / CVM-213 | [TACTIC, ~300] Pivoting and `check` with Bland's rule | after CVM-212 |
| P2 / CVM-214 | [TACTIC, ~200] Cheap bound propagation | after CVM-213 |
| P1 / CVM-215 | [TACTIC, ~200] Simplex model construction (resolving the infinitesimal) | after CVM-213 |
| P1 / CVM-216 | [CERT, ~200] Farkas extraction from a conflicting row | after CVM-213 |
| P1 / CVM-217 | [RECON, ~300] `farkas` reconstructor over `Int` after clearing denominators | after CVM-216, KRN-012a, CVM-111 |
| P1 / CVM-218 | [TACTIC, ~250] LRA theory component (strict inequalities, theory-hook adapter) | after CVM-213, CVM-014 |
| P1 / CVM-219 | [TACTIC, ~200] Theory-propagated equalities for combination | after CVM-218 |
| P1 / CVM-220 | [PASS, ~250] LIA normalisation: integer coefficients, GCD tightening | after CVM-218 |
| P1 / CVM-221 | [TACTIC, ~250] Branch step, with CERT (`branch` fragment) | after CVM-220 |
| P1 / CVM-222 | [TACTIC, ~350] Gomory mixed-integer cuts, with CERT (`cut`) | after CVM-221 |
| P2 / CVM-223 | [TACTIC, ~300] GCD test and equality elimination, with CERT (`gcd`) | after CVM-220 |
| P1 / CVM-224 | [RECON, ~350] `branch`, `cut`, `gcd` reconstructors and `Int` lemmas | after CVM-217, CVM-221 to CVM-223 |
| P2 / CVM-225 | [PASS, ~350] Cooper elimination of one variable (quantified LIA) | after CVM-220 |
| P2 / CVM-226 | [PASS, ~300] Cooper: divisibility atoms and the case split; retires the bounded enumerator to a test oracle | after CVM-225 |
| P1 / CVM-230 | [TACTIC, ~300] Arrays: read-over-write lemma instances | after CVM-202 |
| P1 / CVM-231 | [TACTIC, ~250] Arrays: extensionality witnesses | after CVM-230 |
| P1 / CVM-232 | [TACTIC, ~200] Arrays: model construction (finite maps with a default) | after CVM-230 |
| P2 / CVM-233 | [TACTIC, ~200] `const-array` and store equality | after CVM-231 |
| P1 / CVM-234 | [CERT, ~150] `inst` emitter for array axiom instances | after CVM-230 |
| P1 / CVM-235 | [RECON, ~300] `inst` reconstructor; array axioms as library lemmas (array as a function, `store` as `Function.update`) | after CVM-234, CVM-111 |
| P3 / CVM-236 | [TACTIC, ~250] Weak-equivalence propagation (optimisation) | after CVM-231 |
| P1 / CVM-240 | [COMB, ~350] Model-based theory combination: shared-term equality atoms from coinciding model values | after CVM-014, CVM-219 |
| P1 / CVM-241 | [PASS, ~250] Purification (one theory per atom) | after CVM-240 |
| P1 / CVM-242 | [COMB, ~200] Combined model merge | after CVM-240 |
| P1 / CVM-245 | [TACTIC, ~300] Theory dispatch (assert, propagate, backtrack, final-check) across registered components | after CVM-014 |
| P1 / CVM-246 | [TACTIC, ~350] `cdcl-sat` theory-hook trait (stage 2 online CDCL(T)); lemma proof records | after CVM-245, KRN-009 |
| P1 / CVM-247 | [PASS, ~300] Boolean skeleton abstraction and `tseitin_sound` use for atoms | after CVM-147 |
| P1 / CVM-250 | [PASS, ~250] BV and Int bridge: `bv2nat` and `int2bv` as linear sums, with CERT | after CVM-218, CVM-141 |
| P1 / CVM-251 | [RECON, ~250] Bridge lemmas | after CVM-250 |
| P1 / CVM-252 | [HARN, ~300] M2 benchmark gates; fuzz generators for LIA, LRA, EUF, arrays (one unit each: 252a to 252d); `adj-constraint-solver` and LANG23 moved to the engine | after CVM-007, CVM-224, CVM-235 |

### CVM-M3: datatypes, quantifiers, nonlinear

Gate: VER34 section 10, M3. Unlocks ADL-018 for ADTs, lists as datatypes and `forall` claims.

| Priority / ID | Work | State / prerequisite and acceptance |
|---|---|---|
| P1 / CVM-300 | [INSTR, ~300] `declare-datatype(s)`: reader, writer, validator, VM arm | after CVM-005 |
| P1 / CVM-301 | [TACTIC, ~300] Constructor, selector and tester axioms in the EUF component (injectivity, disjointness) | after CVM-202, CVM-300 |
| P1 / CVM-302 | [TACTIC, ~200] Acyclicity | after CVM-301 |
| P2 / CVM-303 | [TACTIC, ~200] Finite-enumeration datatypes, exhaustiveness | after CVM-301 |
| P1 / CVM-304 | [TACTIC, ~150] Datatype values in models; total selectors | after CVM-301, CVM-008 |
| P1 / CVM-305 | [CERT, ~150] `dt` fragment emitter | after CVM-301 |
| P1 / CVM-306 | [RECON, ~350] `dt` reconstructor and per-datatype no-confusion lemma generation, tied to Warrant ADTs (VER32 3.3, VER33 ADL-003/ADL-004 elaboration) | after CVM-305, PIF inductive-block support |
| P1 / CVM-310 | [PASS, ~250] NNF and prenex | after CVM-009 |
| P1 / CVM-311 | [PASS, ~250] Skolemisation with `red` fragment and `skolem_sound`, reconstructed by `Exists.elim` per instance (no choice needed; confirm here); classical goals use the base-chain variant of owner Q8 | after CVM-310, CVM-119 |
| P1 / CVM-312 | [TACTIC, ~250] Trigger selection | after CVM-310 |
| P1 / CVM-313 | [TACTIC, ~350] E-matching over the e-graph | after CVM-202, CVM-312 |
| P1 / CVM-314 | [TACTIC, ~250] Instantiation loop with generation and cost caps on the meter | after CVM-313 |
| P2 / CVM-315 | [TACTIC, ~300] MBQI part 1: candidate model extraction for quantified formulas | after CVM-314, CVM-008 |
| P2 / CVM-316 | [TACTIC, ~350] MBQI part 2: counterexample check and instantiation | after CVM-315 |
| P1 / CVM-317 | [PASS, ~150] Finite expansion of quantifiers over small BV sorts | after CVM-310 |
| P1 / CVM-318 | [CERT, ~150] `inst` emitter for quantifier instantiations | after CVM-314 |
| P1 / CVM-319 | [RECON, ~250] `inst` reconstructor for quantified hypotheses (`specialize`) | after CVM-318, CVM-235 |
| P2 / CVM-320 | [HARN, ~250] Fuzz generators for quantified logics and DT; benchmark gates | after CVM-316 |
| P2 / CVM-330 | [PASS, ~250] Nonlinear monomial abstraction | after CVM-220 |
| P2 / CVM-331 | [TACTIC, ~250] Linearisation lemmas: sign and zero | after CVM-330 |
| P2 / CVM-332 | [TACTIC, ~300] Linearisation lemmas: tangent planes | after CVM-331 |
| P2 / CVM-333 | [TACTIC, ~250] Linearisation lemmas: monotonicity | after CVM-331 |
| P2 / CVM-334 | [CERT, ~150] `nl` fragment emitter | after CVM-331 |
| P2 / CVM-335 | [RECON, ~350] `nl` reconstructor over `Int.mul` lemmas | after CVM-334, KRN-012a |
| P2 / CVM-336 | [HARN, ~200] QF_NIA gate and fuzz generator | after CVM-333 |

### CVM-M4: strings, sequences, floating point (as Warrant asks)

Gate: VER34 section 10, M4. Unlocks ADL-020 (floats) once a kernel IEEE model exists.

| Priority / ID | Work | State / prerequisite and acceptance |
|---|---|---|
| P3 / CVM-400 | [TACTIC, ~350] FP: classification, unpack and pack | after M1; VER33 float note, VER04p |
| P3 / CVM-401 | [TACTIC, ~350] FP: rounding modes and rounding | after CVM-400 |
| P3 / CVM-402 | [TACTIC, ~300] FP: add and subtract | after CVM-401 |
| P3 / CVM-403 | [TACTIC, ~350] FP: multiply | after CVM-401 |
| P3 / CVM-404 | [TACTIC, ~350] FP: divide and square root | after CVM-403 |
| P3 / CVM-405 | [TACTIC, ~300] FP: fused multiply-add and conversions | after CVM-403 |
| P3 / CVM-406 | [RECON, ~400] IEEE semantics lemmas per operation (one RECON PR per operation: 406a to 406f) | after CVM-402 to CVM-405 |
| P3 / CVM-410 | [TACTIC, ~300] Strings: length abstraction and word-equation unfolding | after M2; VER04l has a consumer |
| P3 / CVM-411 | [TACTIC, ~350] Strings: regular membership by automata | after CVM-410 |
| P3 / CVM-412 | [TACTIC, ~300] Sequences: concatenation, extract, nth | after CVM-410 |
| P3 / CVM-413 | [CERT, ~200] String and sequence fragments | after CVM-410 |
| P3 / CVM-414 | [RECON, ~350] String and sequence reconstructors | after CVM-413 |
| P3 / CVM-415 | [HARN, ~250] Gates for strings and FP | after CVM-414, CVM-406 |

### CVM-M5: hardening and speed

Gate: VER34 section 10, M5.

| Priority / ID | Work | State / prerequisite and acceptance |
|---|---|---|
| P2 / CVM-500 | [HARN, ~250] Re-baseline against Z3 and cvc5; set the M5 `R` margin with the owner | after M3 |
| P2 / CVM-501 | [PASS, ~300] Extra preprocessing found by profiling (each a separate row when chosen) | after CVM-500 |
| P2 / CVM-502 | [TACTIC, ~300] `cdcl-sat` tuning units found by profiling (restart policy, phase, clause database), one PR each | after CVM-500 |
| P2 / CVM-503 | [COMB, ~300] Deterministic portfolio: sequential budgeted strategies; never changes a verdict | after CVM-500 |
| P3 / CVM-504 | [COMB, ~350] Optional parallel portfolio, owner Q9 | after CVM-503 |
| P2 / CVM-505 | [HARN, ~200] Nightly standing report (per-logic `R`, certified fraction, PAR-2 trend) | after CVM-007 |
