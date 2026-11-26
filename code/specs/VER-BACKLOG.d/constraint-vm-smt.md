# Constraint VM as Warrant's SMT solver (VER34)

**Authority:** current plan, decomposition tree, queue and status for growing the
Constraint VM into the solver Warrant uses. The main
[VER-BACKLOG](../VER-BACKLOG.md) keeps a short pointer and the milestone
headings; this shard owns the rows. The design is
[VER34](../VER34-constraint-vm-smt-solver.md); its section 15 is the review
record (Fable, round 1) that shaped this revision.

Owner direction, 2026-11-24: "We have our own constraint VM. I would love to build
that out as a full fledged Z3 equivalent so that Warrant can use that." And:
"Any big problem can be decomposed into smaller problems… we should be able to break
it down into smaller and smaller chunks until it can fit in as instructions in our
constraint VM architecture."

Structural rule (VER34 section 2): the solver is assembled from small units that
fit the LANG24 layers, and **this table is the decomposition tree**: every row is
one leaf unit and one PR. Slot (the "fits in" layer) is in brackets: **INSTR**
opcode, **TACTIC** theory component or search step, **PASS** preprocessing
rewrite, **CERT** certificate emitter, **RECON** kernel-side library or
reconstructor, **COMB** combinator, **HARN** validation harness, **DOC**
spec-only or data. Size is the target in source lines, tests extra; a row that
overruns by more than a quarter is split in its PR. The **certificate column**
names the fragment kind a unit emits (VER34 6.2) or `none: model-preserving` /
`none: harness` / `n/a`; a unit that can contribute to UNSAT and says `none`
without a reason is not a unit. The solver is untrusted: UNSAT counts toward
`proved` only through a PIF proof the kernel checks (VER34 section 6). Order is
dependency order inside a milestone; spec before code; nothing in M1 starts before
CVM-S1 reports and the owner answers VER34 Q7 (**Q13 was decided 2026-11-24, conditional on CVM-S2**; M0 needs only Q1 to Q3).
**Follow-up 2026-11-24:** VER34 13.1 is applied to VER32, VER33, VER11a and VER31; proof libraries sit on the **proof side** of VER32's chain (E7), which needs the reader class `ProofSide` (VER32 PIF-002b) and the `proof` chain role (PIF-003e) before any claim file that imports one can be signed.
ADL-xxx ids are VER33's (A0 to A4 ladder); PIF-xxx and KRN-xxx are VER32's and
VER11a's.

### CVM-M0: foundations

Gate: VER34 section 10, M0. Unlocks VER33 ADL-010 on Bool and BV.

| Priority / ID | Work | Certificate | State / prerequisite and acceptance |
|---|---|---|---|
| P1 / CVM-000 | VER34 spec, adversarial review (round 1 answered), owner answers to section 12 | n/a | in review |
| P1 / CVM-001 | [DOC, ~50] Correct `constraint-engine` README: the LIA tactic is bounded enumeration, not Cooper, and not "Complete" (VER34 section 3); lesson | n/a | none; first |
| P1 / CVM-S0 | [HARN, ~300] **Self-contained baseline script** (`code/scripts`, no dependency on CVM-007): run the current `constraint-vm` (QF_Bool, QF_BV, QF_LIA) and Z3 and cvc5 on pinned SMT-LIB families on one machine; **freeze first** the Warrant-near predicate, the counting rule and `c_trace`/`c_kernel` definitions (VER34 1.1); record solved counts, PAR-2, the meter-to-seconds calibration and the `R(L,T)` table; re-base VER34 section 10 | none: harness | after CVM-000, CVM-001 |
| P1 / CVM-002a | [core, ~250] Add `BigInt` and `BigRat` value types on `bignum-core` beside `i128` (additive; nothing migrated) | none: model-preserving | after CVM-001; owner Q11 |
| P1 / CVM-002b | [core, ~300] Migrate `constraint-engine` (LIA, models) to bignum; `i128` still accepted at the boundary | none: model-preserving | after CVM-002a |
| P1 / CVM-002c | [core, ~300] Migrate `smt-lib-format` reader and writer and `constraint-vm` to bignum | none: model-preserving | after CVM-002b |
| P1 / CVM-002d | [core, ~250] Migrate `adj-constraint-solver` and `lang-refinement-checker` | none: model-preserving | after CVM-002c |
| P1 / CVM-002e | [core, ~150] Remove `Value::Int(i128)` and `Rational(i128,i128)`; changelog and lesson | none: model-preserving | after CVM-002d |
| P1 / CVM-003 | [INSTR, ~250] Named assertions (`:named`) and `check-sat-assuming`; assumption literals | none: model-preserving | after CVM-002c |
| P1 / CVM-004 | [TACTIC, ~250] Real `GetUnsatCore`: failed assumptions, minimised by deletion; replaces the stub; `adj-constraint-solver` follow-up filed | none: model-preserving (a core is used by CVM-158's vacuity check, which is itself a check) | after CVM-003 |
| P1 / CVM-005 | [INSTR, ~300] `define-fun` and `let` (reader, writer, DAG-sharing expansion); rejection tests for recursion and arity | none: model-preserving | after CVM-002c |
| P1 / CVM-005a | [INSTR, ~250] Indexed identifiers as a real term form (`(_ extract i j)`, `zero_extend`, `(_ bvN w)`) in `smt-lib-format` and the AST, **additive**: the flattened `Apply "_"` shape is still accepted | none: model-preserving | after CVM-005 |
| P1 / CVM-005b | [TACTIC, ~300] Migrate `bv.rs` to the real indexed form and remove the flattened shape | none: model-preserving | after CVM-005a |
| P1 / CVM-006 | [COMB, ~300] `Meter`: deterministic counters (conflicts, propagations, nodes, pivots, rounds, bits, depth) and typed `Unknown` reasons; moves existing caps to data | none: harness | after CVM-001 |
| P1 / CVM-006a | [COMB, ~200] A meter for **untrusted tools**: the trace reader, the reconstructor and the independent reifier get their own budgets and fail closed | none: harness | after CVM-006 |
| P1 / CVM-006b | [COMB, ~200] Named blow-up counters: Ackermann, `distinct`, finite expansion (2^w), Cooper lcm growth, cut coefficient growth, NNF over shared DAGs; `rlimit` and tactic budgets may only lower the global meter | none: harness | after CVM-006 |
| P1 / CVM-007 | [HARN, ~350] `cvm-bench`: pinned SMT-LIB manifest (digests), runner, solved count, PAR-2, per-logic `R(L,T)` from the CVM-S0 calibration, `c_trace` and `c_kernel` accounting; nightly job | none: harness | after CVM-006, CVM-S0 |
| P1 / CVM-007a | [HARN, ~250] Oracle adapter: Z3 and cvc5 as pinned subprocesses, fetched and verified offline; disagreement triage (evaluator checks oracle models; independent-reifier certificate re-check); oracle agreement never counted in `c`; never in a trust path | none: harness | after CVM-007, CVM-008; owner Q1 |
| P1 / CVM-008 | [HARN, ~400] Independent model evaluator for Bool, BV and Int in SMT-LIB terms, sharing no code with any tactic; engine self-check: a model that fails evaluation becomes `Unknown` plus a record | none: harness | after CVM-002c |
| P1 / CVM-008a | [HARN, ~350] **Second, deliberately different evaluator** (bit-lists and `BigInt`) and a differential against CVM-008 on generated terms; a disagreement is stop-the-line | none: harness | after CVM-008 |
| P1 / CVM-009 | [COMB, ~250] `Goal`, `Tactic` trait (`Reduced`, `Sat`, `Unsat(Fragment)`, `Failed`), `Fragment` record types, `ModelConv` | none: the trait carries fragments | after CVM-006 |
| P1 / CVM-010 | [COMB, ~120] `then` | none: composes fragments | after CVM-009 |
| P1 / CVM-011 | [COMB, ~120] `or_else` | none: composes fragments | after CVM-009 |
| P1 / CVM-012 | [COMB, ~120] `with_budget` | none: harness | after CVM-009 |
| P1 / CVM-013 | [COMB, ~200] `when` and probes (`has_quantifiers`, `max_bv_width`, `num_theories`, `size`) | none: pure functions of the goal | after CVM-009 |
| P1 / CVM-014a | [TACTIC, ~250] **Boolean skeleton and atom table**: abstract theory atoms to Boolean variables (moved here from M2: the offline loop needs it) | `enc` (the skeleton's encoding, later, via CVM-147) | after CVM-009 |
| P1 / CVM-014 | [COMB, ~350] `smt(theories)` skeleton: stage-1 offline lazy CDCL(T) loop over registered theory components on `cdcl-sat` assumptions and clause addition; the theory-component trait (VER34 4.3) | lemma clauses carry their fragment (6.2) | after CVM-014a, CVM-016a |
| P1 / CVM-015a | [COMB, ~150] Default strategy per logic as a data table | none: model-preserving | after CVM-010 to CVM-013 |
| P1 / CVM-015b | [TACTIC, ~200] Re-express the SAT tactic through the trait, answers unchanged | none: model-preserving | after CVM-015a |
| P1 / CVM-015c | [TACTIC, ~200] Re-express the bounded LIA tactic through the trait, answers unchanged | none: model-preserving | after CVM-015a |
| P1 / CVM-015d | [TACTIC, ~250] Re-express the BV tactic through the trait, answers unchanged | none: model-preserving (DRAT as today) | after CVM-015a |
| P1 / CVM-016a | [TACTIC, ~250] Incremental push/pop at the Boolean level: scope selector literals on `cdcl-sat` assumptions; replaces clone-per-push | none: model-preserving | after CVM-009 |
| P1 / CVM-016b | [TACTIC, ~150] Theory `backtrack` marks (the trait part only) | none: model-preserving | after CVM-016a |
| P1 / CVM-016c | [HARN, ~200] Test unit running every tactic's tests inside a push/pop wrapper | none: harness | after CVM-016a |
| P1 / CVM-017a | [INSTR, ~250] `code/grammars/cvm/tactic.tokens` and `.grammar` and AST (grammar-driven, with a depth cap) | none: model-preserving | after CVM-010 to CVM-013 |
| P1 / CVM-017b | [INSTR, ~250] `check-sat-using` opcode, printer, validator and VM arm; budgets may only lower the global meter | none: model-preserving | after CVM-017a, CVM-006b |
| P1 / CVM-018a | [INSTR, ~150] `get-value` | none: harness | after CVM-009 |
| P1 / CVM-018b | [INSTR, ~200] `set-option` (`produce-certificates`, `rlimit` lower-only) | none: model-preserving | after CVM-006b |
| P1 / CVM-018c | [INSTR, ~200] `get-certificate` opcode (returns the trace) | none: carries fragments | after CVM-024b |
| P1 / CVM-019 | [HARN, ~350] Fuzz generator, QF_Bool and QF_BV (well-sorted terms; checked by both evaluators and brute force on small domains; certificate checks join in M1) | none: harness | after CVM-008a |
| P2 / CVM-019a | [HARN, ~200] Metamorphic relations: renaming, permutation, double negation, tautology addition, conjunction splitting | none: harness | after CVM-019 |
| P2 / CVM-019b | [HARN, ~200] Metamorphic relations: `push; A; pop` equals not asserting `A`; `check-sat-assuming` equals push/check/pop; "assert the model, still SAT" | none: harness | after CVM-016a, CVM-019 |
| P1 / CVM-020 | [INSTR, ~250] `minimize` and `maximize` objective opcodes (single term, lexicographic list), parser, printer, VM arm; extremum answers are test inputs, not claims (VER33 6.3 boundary points) | none: extremum is not evidence | after CVM-018a |
| P1 / CVM-021 | [TACTIC, ~250] Extremum search for Bool and BV: bit-by-bit descent under push/pop and assumptions | none: not evidence | after CVM-016a, CVM-020 |
| P2 / CVM-022 | [TACTIC, ~250] Extremum search for `Int`: bound bisection under push/pop; unbounded is reported as unbounded, never guessed | none: not evidence | after CVM-021; complete only after CVM-221 |
| P1 / CVM-023 | [COMB, ~200] Seeded tie-break and phase (`:random-seed`); same input, seed, meter and build give identical answers; vectors are committed with (solver version, seed) provenance and **checked by the evaluator, not re-run** (VER34 4.7) | none: harness | after CVM-006, CVM-016a |
| P1 / CVM-024a | [DOC, ~200] `code/grammars/cvm/trace.tokens` and `.grammar` (`cvm-trace-1`): the grammar is the whole language; corpus of valid and hostile traces | n/a | after CVM-009 |
| P1 / CVM-024b | [INSTR, ~250] Trace writer and reader on the shared grammar-driven parser, with the untrusted-tool meter | n/a: carries fragments | after CVM-024a, CVM-006a |
| P1 / CVM-025 | [DOC, ~150] Register the ids VER34 opens in VER32's VER13 slice (including `tcb.vcgen.claim-faithful` with VER33), and the **rule: no proof-side file declares a constant occurring in a signed goal** (VER34 6.8; signed goals only; **the VER32 E3/E4 proof-side amendment is approved and written as VER32 E7**); the rule is **enforced** by the reader class `ProofSide` (R-GOL-4 on every proof-side file), so until PIF-002b lands no VER34 claim file may be signed; spec-only | n/a | with VER32 PIF-016d; enforcement needs PIF-002b and PIF-003e |
| P1 / CVM-026 | [HARN, ~200] `solver-attested` evidence kind in `ca-assurance-report` (solver, version, query digest), capped at `tested` | none: report field | after CVM-025 |

### CVM-M1: certified QF_BV (P1 operators only)

Gate: VER34 section 10, M1. Prerequisites: CVM-S1 reported, `lits` and `cvm-smt-v1` approved (Q7, CVM-S3), authoring route: **Q13 decided 2026-11-24, Lean as an untrusted producer, conditional on CVM-S2**. P1 operators: constants, bitwise, `=`, `distinct`, `ite`, add/sub/neg, comparisons, extract/concat/extend. Shifts, multiplication and division are P2 and sit in M1b.

| Priority / ID | Work | Certificate | State / prerequisite and acceptance |
|---|---|---|---|
| P1 / CVM-S1 | [HARN, ~300] **Kernel-fit spike** (may use throwaway Lean-generated or Rust-built PIF): a hand-written `checkLRAT` in PIF and a synthetic proof (the 4-NAND XOR miter, a size ladder, not a claim) through `pif-check` at 8, 32 and 64 bits; record steps, depth and new nodes; try chunked data, binary numerals, the term route; **decide** the data encoding, the limits configuration, and reflection versus term route per size | none: measurement | after PIF-003, PIF-004 (library files on the proof side also need PIF-003e); before every row of this milestone |
| P1 / CVM-S2 | [HARN, ~300] **Authoring spike** (owner Q13 option a): prove one real lemma, `blast_add_sound`, in Lean `prelude` mode over a mirror of the PIF prelude (no `Init`), convert with PIF-008 and check with `pif-check`; report **which tactics survive**, whether the auto-generated auxiliaries (`casesOn`, `recOn`, `below`, `brecOn`, `noConfusion`, `match_n`, `PProd`) pass and avoid collisions; Q13 is **decided (2026-11-24) conditional on it**: if too few tactics survive, option (b) (native term builders, PIF-018) becomes M1's route and the library estimate is redone | none: measurement | after CVM-S1, PIF-008 |
| P1 / CVM-S3 | [DOC, ~150] Record `lits` and the named configuration `cvm-smt-v1` (raised steps and nodes, same depth cap) in the gate's named configuration (VER32 section 3.5, the section 7 limits); owner-signed; spec-only | n/a | after CVM-S1; owner Q7 |
| P1 / CVM-109a | [RECON, ~250] Authoring pipeline (owner Q13 option a, conditional on CVM-S2): a Lean `prelude`-mode **mirror of the PIF prelude** with a **mirror-identity check** (declaration-identical to the PIF prelude), a regeneration script producing committed, digest-pinned PIF files via PIF-008, Lean needed only to regenerate | none: authoring | after CVM-S2, PIF-008; owner Q13 (decided 2026-11-24, conditional on CVM-S2) |
| P1 / CVM-109b | [HARN, ~150] **Footprint lint**: fails the build if a `Pif.BV` or `Pif.Lrat` library file's generated PIF contains an axiom (`propext`, `Quot.sound`, `Classical.choice`) | none: lint | after CVM-109a |
| P1 / CVM-110a | [RECON, proof side, ~300] Library file `Pif.Lrat` definitions: `Cnf`, `Clause`, assignments, satisfaction, chunked certificate data type | definitions only | after CVM-109a, ADL-004a |
| P1 / CVM-110b | [RECON, proof side, ~300] Library file `Pif.BV` definitions for the P1 operators: bit-list (LSB first, length invariant), SMT-LIB semantics; **no literals**; proof-internal (VER34 6.8) | definitions only | after CVM-110a |
| P1 / CVM-110c | [RECON, proof side, ~250] Library file `Pif.Prop`: formulas over opaque atoms (`Nat -> Prop` valuations), evaluation, skeleton encoding definitions | definitions only | after CVM-110a |
| P1 / CVM-111 | [RECON, ~300] `rw` fragment kind: reconstructor (congruence chain from a named rule application) and the trace line | `rw` | after CVM-110b, CVM-009, CVM-024b |
| P1 / CVM-111a | [RECON, ~200] `Impl.trans` fold: compose `rw`/`red` implications and leaf kinds into one proof | none: composition | after CVM-111 |
| P1 / CVM-100 | [PASS, ~250] Constant folding for every P1 BV operator, **with its own arithmetic** (not the evaluator), differentially tested against both evaluators | `rw` | after CVM-009, CVM-008a |
| P1 / CVM-101 | [PASS, ~200] Boolean and `ite` simplification with BV conditions | `rw` | after CVM-100 |
| P1 / CVM-102 | [PASS, ~250] Bitwise family rules (identities, `x^x`, `x&0`) | `rw` | after CVM-100 |
| P1 / CVM-103 | [PASS, ~200] Additive rules (`add/sub/neg`) | `rw` | after CVM-100 |
| P1 / CVM-105 | [PASS, ~300] `extract/concat/extend` normalisation | `rw` | after CVM-005a, CVM-100 |
| P1 / CVM-106 | [PASS, ~250] Comparison rules (`ult/ule/slt/sle`, reflexive and constant cases, e.g. `ule a 1`) | `rw` | after CVM-100 |
| P1 / CVM-108 | [PASS, ~300] Variable elimination by definition (`x = t`), with model converter | `red` | after CVM-100 |
| P1 / CVM-112 | [RECON, ~300] Lemmas for CVM-100 and CVM-101's named rules | `rw` lemmas | after CVM-111 |
| P1 / CVM-113 | [RECON, ~300] Lemmas for CVM-102's rules | `rw` lemmas | after CVM-111 |
| P1 / CVM-114 | [RECON, ~250] Lemmas for CVM-103's rules | `rw` lemmas | after CVM-111, CVM-141b |
| P1 / CVM-116 | [RECON, ~300] Lemmas for CVM-105's rules | `rw` lemmas | after CVM-111 |
| P1 / CVM-117 | [RECON, ~250] Lemmas for CVM-106's rules | `rw` lemmas | after CVM-111, CVM-142b |
| P1 / CVM-119 | [RECON, ~250] `red` fragment kind and `solve_eqs_sound` (equisatisfiable passes) | `red` | after CVM-111a, CVM-108 |
| P1 / CVM-120 | [TACTIC, ~250] Blast group: bitwise, `ite`, `=`, `distinct` (exists in `bit-blaster`; refactor to a group with a named entry point) | `enc` (via CVM-130) | after CVM-009 |
| P1 / CVM-121 | [TACTIC, ~250] Blast group: add, sub, neg | `enc` | after CVM-120 |
| P1 / CVM-122 | [TACTIC, ~250] Blast group: comparisons | `enc` | after CVM-120 |
| P1 / CVM-126 | [TACTIC, ~200] Blast group: `extract/concat/extend` wiring | `enc` | after CVM-120 |
| P1 / CVM-128 | [TACTIC, ~250] Tseitin as a unit with a stable variable and clause order | `enc` | after CVM-120 |
| P1 / CVM-140 | [RECON, ~300] `blast_sound` lemmas for the bitwise, `=`, `ite` group | lemma | after CVM-110b |
| P1 / CVM-141a | [RECON, ~250] Half adder and full adder correctness | lemma | after CVM-140 |
| P1 / CVM-141b | [RECON, ~350] Ripple-carry adder, subtract and negate by induction on width (**the first calibration unit**: size and kernel time recorded) | lemma | after CVM-141a |
| P1 / CVM-142a | [RECON, ~250] Equality and disequality circuits | lemma | after CVM-140 |
| P1 / CVM-142b | [RECON, ~300] Unsigned comparison circuits (`ult/ule`) | lemma | after CVM-141b |
| P1 / CVM-142c | [RECON, ~250] Signed comparison circuits | lemma | after CVM-142b |
| P1 / CVM-146 | [RECON, ~250] `extract/concat/extend` wiring lemmas | lemma | after CVM-140 |
| P1 / CVM-147a | [RECON, ~250] Per-gate Tseitin clauses sound (AND, NOT) | lemma | after CVM-110c |
| P1 / CVM-147b | [RECON, ~300] `tseitin_sound` for a circuit DAG (definitional CNF equisatisfiable with the circuit); also the Boolean skeleton's encoding (closes CVM-014a's `enc`) | lemma | after CVM-147a |
| P1 / CVM-148a | [RECON, ~350] `encode` (the reference encoder as a kernel function) for the P1 groups, and `blast_correct` for the P1 term forms (**second calibration unit**) | lemma | after CVM-141b, CVM-142c, CVM-146, CVM-147b |
| P1 / CVM-148b | [RECON, ~250] `blast_correct` as a structural induction over typed BV terms and the reflection statement `lrat_sound` plus `blast_correct` composed | lemma | after CVM-148a, CVM-151b |
| P1 / CVM-130 | [CERT, ~300] **Rust port of the kernel encoder**: core, bitwise, `=`, `ite`, extract/concat; emits the CNF in kernel order and the `enc` trace line | `enc` | after CVM-148a, CVM-120, CVM-128 |
| P1 / CVM-131 | [CERT, ~250] Rust port: add/sub/neg and comparisons | `enc` | after CVM-130, CVM-121, CVM-122 |
| P1 / CVM-135 | [HARN, ~250] Differential: the Rust encoder against the kernel's encoder (through the kernel's reducer) on small widths for every P1 group | none: harness | after CVM-131 |
| P1 / CVM-150a | [RECON, ~250] Unit propagation lemmas over `Pif.Lrat` | lemma | after CVM-110a |
| P1 / CVM-150b | [RECON, ~250] `rup_step_sound` and the resolution lemma set | lemma | after CVM-150a |
| P1 / CVM-151a | [RECON, ~300] `checkLRAT` as a kernel function over chunked data (design per CVM-S1) | definitions only | after CVM-150b, CVM-S1 |
| P1 / CVM-151b | [RECON, ~300] `checkLRAT_sound` (RUP-only) and the reflection glue (`Eq.refl true`); **no `Lean.ofReduceBool`** | lemma | after CVM-151a |
| P1 / CVM-152 | [RECON, ~300] `lrat` fragment reconstructor: certificate data into chunked constants, goal assembly. `cdcl-sat` LRAT emission is KRN-009, **P1 as of 2026-11-24** (VER11a request applied) | `lrat` | after CVM-151b, KRN-009 |
| P1 / CVM-153 | [RECON, ~300] **Term-route fallback** (P1): LRAT steps as proof terms for sizes the spike shows reflection cannot reach (VER11a 9.5 option 1) | `lrat` | after CVM-150b, CVM-152 |
| P1 / CVM-155a | [RECON, ~250] Bridge: signed WIR word operations (bitwise, `=`, `ite`) to `Pif.BV` | lemma | after CVM-110b, PIF-019 (which **now states** the word representation for widths above 8, VER32 section 11) |
| P1 / CVM-155b | [RECON, ~250] Bridge: add, sub, neg | lemma | after CVM-155a, CVM-141b |
| P1 / CVM-155c | [RECON, ~250] Bridge: comparisons | lemma | after CVM-155a, CVM-142c |
| P1 / CVM-155d | [RECON, ~200] Bridge: extract, concat, extend | lemma | after CVM-155a, CVM-146 |
| P1 / CVM-156 | [RECON, ~300] Bridge: ADL K0 (`Int` with range guards, as in G-M1) to `Pif.BV` for in-range values | lemma | after CVM-110b; VER33 ADL-003a to c |
| P1 / CVM-157 | [HARN, ~300] **Independent reifier**: `.smt2` to a library term, written without the reconstructor's code (**common-mode risk: it shares the `smt-lib-format` parser with the solver**; `c_kernel` here is relative to `Pif.BV`); goal-less runs state the theorem the reifier produced and the reconstructor must prove exactly that | none: harness | after CVM-110b, CVM-005 |
| P1 / CVM-158 | [HARN, ~250] **Vacuity check** for bounded VCs: a SAT witness for the preconditions at size exactly k (evaluator-confirmed) and the UNSAT core must include the negated conclusion; else `vacuous` | none: harness | after CVM-004, CVM-008a |
| P1 / CVM-159 | [CERT, ~250] **DRAT-route certificate record**: `(VC digest, k, CNF digest, DRAT digest)`, the `blast, sat` evidence strategy with formula-level passes off, and the id list; retired by CVM-159a once CVM-170a exists; ids `tcb.vcgen.claim-faithful`, `tcb.symexec.engine-correct`, `tcb.drat.checker-correct`, `tcb.bitblast.encoding-correct` | `drat` + record | after CVM-158, CVM-015d |
| P1 / CVM-159a | [DOC, ~150] **DRAT sunset**: inventory every DRAT-backed `bounded(k)` claim; each is re-evidenced through PIF by the release after the CVM-174 report or drops to `tested`; spec-only, with CVM-170b | n/a | after CVM-170a, CVM-174 |
| P1 / CVM-170a | [RECON, ~350] `bv-decide`: end-to-end reconstructor from a trace and a signed goal to a PIF claim file (no axioms, no `goal` record) | `rw`, `red`, `enc`, `lrat` | after CVM-152, CVM-148b, CVM-111a, PIF-003 |
| P1 / CVM-170b | [DOC, ~150] **Owns the VER31 amendment text** (ADL-006b cites it), **spec-only**: `pif:sha256:` scheme accepted, `solver-attested` refused as certificate, the DRAT certificate record (VER34 6.6, 6.7) | n/a | after CVM-159 |
| P1 / CVM-171 | [HARN, ~300] Obligation corpus from **VER32 G-M2** (e.g. `validate_bit`'s `ule a 1` fact) through `pif-check`, widths set W; the VC-builder side is KRN-012 | none: harness | after CVM-170a, CVM-155c; G-M2 |
| P1 / CVM-172 | [HARN, ~300] Obligation corpus: the four ct-compare claims' bit-level obligations; sabotage controls (VER27) | none: harness | after CVM-170a, KRN-012 |
| P1 / CVM-173 | [HARN, ~250] M1 gate: pinned QF_BV families, `R`, `c_trace`, `c_kernel`; the A1 `bounded(k)` demo on the PIF route and on the DRAT route under VER34 6.7 | none: harness | after CVM-007, CVM-170a; ADL-012 to 014 consume it |
| P1 / CVM-174 | [HARN, ~250] Certified-mode cost report: kernel reduction time per UNSAT, sizes, against the budget CVM-S1 set; decides W and any widening | none: measurement | after CVM-171 |
| P1 / CVM-177 | [HARN, ~250] **Independent checkers run the proofs**: KRN-019 (second kernel) and the Lean replay harness (PIF-010) run reflective proofs under `cvm-smt-v1` with `lits`, so `kernel.checker.correct` can close for these claims | none: harness | after CVM-170a, KRN-019, PIF-010 |
| P1 / CVM-176 | [HARN, ~200] **Operator coverage report**: every operator in a signed goal's VC must be in the certified set or the claim is `solver-attested`; signed-operation list per milestone | none: harness | after CVM-170a |
| P2 / CVM-154 | [RECON, ~300] RAT soundness for `checkLRAT` (needed before bounded variable elimination may appear in certified mode) | lemma | after CVM-151b |
| P2 / CVM-160 | [TACTIC, ~350] `cdcl-sat`: bounded variable elimination and subsumption, proof-logged; **fast path only** until CVM-154 | `lrat` (RAT) | after KRN-009 |
| P2 / CVM-161 | [TACTIC, ~250] `cdcl-sat`: vivification, proof-logged (RUP-checkable) | `lrat` | after CVM-160 |

### CVM-M1b: BV widening (shifts, multiplication, division)

Gate: VER34 section 10, M1b. Each operator group lands whole: solver, port, lemmas, bridge.

| Priority / ID | Work | Certificate | State / prerequisite and acceptance |
|---|---|---|---|
| P2 / CVM-104 | [PASS, ~250] Shift rules | `rw` | after CVM-100 |
| P2 / CVM-115 | [RECON, ~250] Lemmas for CVM-104's rules | `rw` lemmas | after CVM-111 |
| P2 / CVM-103m | [PASS, ~250] Multiplication by a constant to shifts and adds | `rw` | after CVM-104 |
| P2 / CVM-114m | [RECON, ~250] Lemmas for CVM-103m | `rw` lemmas | after CVM-111, CVM-144b |
| P2 / CVM-107 | [PASS, ~250] Division and remainder by constants | `rw` | after CVM-100 |
| P2 / CVM-118 | [RECON, ~250] Lemmas for CVM-107's rules | `rw` lemmas | after CVM-111, CVM-145b |
| P2 / CVM-123 | [TACTIC, ~300] Blast group: shifts (constant and barrel) | `enc` | after CVM-120 |
| P2 / CVM-124 | [TACTIC, ~300] Blast group: multiplication, keeping the refusal-before-build estimates | `enc` | after CVM-120 |
| P2 / CVM-125 | [TACTIC, ~350] Blast group: division and remainder family | `enc` | after CVM-124 |
| P2 / CVM-132 | [CERT, ~250] Rust port: shifts | `enc` | after CVM-131, CVM-123 |
| P2 / CVM-133 | [CERT, ~250] Rust port: multiplication | `enc` | after CVM-131, CVM-124 |
| P2 / CVM-134 | [CERT, ~250] Rust port: division and remainder | `enc` | after CVM-133, CVM-125 |
| P2 / CVM-143a | [RECON, ~250] Constant shifts lemma | lemma | after CVM-140 |
| P2 / CVM-143b | [RECON, ~300] Barrel shifter lemma | lemma | after CVM-143a |
| P2 / CVM-144a | [RECON, ~300] Partial products | lemma | after CVM-141b |
| P2 / CVM-144b | [RECON, ~350] Shift-add multiplier by induction | lemma | after CVM-144a |
| P2 / CVM-144c | [RECON, ~250] Truncation to width w | lemma | after CVM-144b |
| P2 / CVM-145a | [RECON, ~300] Restoring-division step lemma | lemma | after CVM-144b |
| P2 / CVM-145b | [RECON, ~350] `udiv`/`urem` correctness | lemma | after CVM-145a |
| P2 / CVM-145c | [RECON, ~300] Signed division, remainder, modulus | lemma | after CVM-145b |
| P2 / CVM-155e | [RECON, ~300] Bridge: shifts **with the WIR amount masking** (SMT shifts by at least the width give 0, Rust masks) | lemma | after CVM-143b, CVM-155a |
| P2 / CVM-155f | [RECON, ~300] Bridge: division and remainder **with the side condition** `y /= 0` (SMT total, WIR stuck) | lemma | after CVM-145c, CVM-155a |

### CVM-M2: EUF, arrays, LRA, LIA, combination

Gate: VER34 section 10, M2. Unlocks ADL-010, ADL-012 to 015 for `Int`, arrays and UF.

| Priority / ID | Work | Certificate | State / prerequisite and acceptance |
|---|---|---|---|
| P1 / CVM-200 | [TACTIC, ~300] Term DAG and union-find with an explanation forest | none: model-preserving (explanations feed `euf`) | after CVM-014 |
| P1 / CVM-201 | [TACTIC, ~250] Congruence table (signature hashing) and merge propagation | `euf` | after CVM-200 |
| P1 / CVM-202 | [TACTIC, ~250] EUF as a theory component: assert, propagate, conflict with explanation | `euf` | after CVM-201 |
| P1 / CVM-203 | [TACTIC, ~200] `distinct` and disequalities (a conflict is a merge chain between the two sides) | `euf` | after CVM-202 |
| P1 / CVM-204 | [CERT, ~200] `euf` fragment emitter (merge chain) | `euf` | after CVM-202 |
| P1 / CVM-205 | [RECON, ~250] `euf` reconstructor (`Eq.trans`, `congrArg`) | `euf` | after CVM-204, CVM-111 |
| P1 / CVM-206 | [PASS, ~250] Ackermann reduction for UFBV, with its `red` lemma | `red` | after CVM-119 |
| P2 / CVM-207 | [TACTIC, ~200] EUF model construction (fresh value per class) | none: model-preserving | after CVM-202 |
| P1 / CVM-208 | [HARN, ~300] Evaluators extended to arrays, UF and reals (both implementations) | none: harness | after CVM-008a |
| P1 / CVM-209a | [RECON, ~300] `Pif.Rat` library file: pairs of `Int` and a positive denominator, operations | definitions only | after CVM-110a |
| P1 / CVM-209b | [RECON, ~350] Ordered-field lemma set (add, mul, order) for `Pif.Rat` | lemma | after CVM-209a |
| P1 / CVM-210 | [core, ~300] Solver-side rationals with a symbolic infinitesimal | none: model-preserving | after CVM-002a |
| P1 / CVM-211 | [TACTIC, ~300] Simplex tableau: sparse rows, slack introduction from linear atoms | none: model-preserving | after CVM-210 |
| P1 / CVM-212 | [TACTIC, ~250] Bounds: assert-bound and immediate conflicts | `farkas` | after CVM-211 |
| P1 / CVM-213 | [TACTIC, ~300] Pivoting and `check` with Bland's rule | `farkas` | after CVM-212 |
| P2 / CVM-214 | [TACTIC, ~200] Cheap bound propagation (each propagated bound carries its explanation) | `farkas` | after CVM-213 |
| P1 / CVM-215 | [TACTIC, ~200] Simplex model construction (resolving the infinitesimal) | none: model-preserving | after CVM-213 |
| P1 / CVM-216 | [CERT, ~200] Farkas extraction from a conflicting row | `farkas` | after CVM-213 |
| P1 / CVM-217 | [RECON, ~350] `farkas` reconstructor: the weighted sum over `Int`/`Pif.Rat` after clearing denominators (this spec's own `Int` engine; KRN-012a is not reused) | `farkas` | after CVM-216, CVM-209b, CVM-111 |
| P1 / CVM-218 | [TACTIC, ~250] LRA theory component (strict inequalities, theory-hook adapter) | `farkas`, `tri` | after CVM-213, CVM-014 |
| P1 / CVM-219 | [TACTIC, ~200] Theory-propagated equalities for combination | `tri` | after CVM-218 |
| P1 / CVM-220a | [PASS, ~200] LIA normalisation: integer coefficients | `rw` | after CVM-218 |
| P1 / CVM-220b | [PASS, ~250] GCD tightening (a Chvátal–Gomory step with a divisor, conditional on its bound atoms) | `tight` | after CVM-220a |
| P1 / CVM-221 | [TACTIC, ~250] Branch step | `branch` | after CVM-220a |
| P1 / CVM-222a | [TACTIC, ~250] Gomory cut: tableau row to multipliers and divisor | `cg` | after CVM-221 |
| P1 / CVM-222b | [TACTIC, ~250] Gomory cut: fractional parts and rounding, strengthening | `cg` | after CVM-222a |
| P2 / CVM-223 | [TACTIC, ~300] GCD test and equality elimination | `gcd` | after CVM-220a |
| P1 / CVM-224a | [RECON, ~250] `branch` reconstructor and lemma | `branch` | after CVM-217, CVM-221 |
| P1 / CVM-224b | [RECON, ~350] `tight` and `cg` reconstructors: Chvátal–Gomory derivations as conditional lemma clauses | `tight`, `cg` | after CVM-224a, CVM-222b |
| P2 / CVM-224c | [RECON, ~250] `gcd` reconstructor and divisibility lemmas | `gcd` | after CVM-223 |
| P1 / CVM-224d | [RECON, ~250] `tri` trichotomy and equality-atom lemmas | `tri` | after CVM-209b |
| P2 / CVM-225a | [PASS, ~350] Cooper: eliminate one variable (inequalities) | `cooper` (solver-attested until CVM-226a is measured; excluded from the M3 `c` gate) | after CVM-220a |
| P2 / CVM-225b | [PASS, ~250] Cooper: divisibility atoms | `cooper` (solver-attested until CVM-226a is measured; excluded from the M3 `c` gate) | after CVM-225a |
| P2 / CVM-225c | [PASS, ~300] Cooper: case split and lcm growth (meter counter) | `cooper` (solver-attested until CVM-226a is measured; excluded from the M3 `c` gate) | after CVM-225b, CVM-006b |
| P2 / CVM-226a | [RECON, ~350] Cooper step lemma and reconstructor | `cooper` (solver-attested until CVM-226a is measured; excluded from the M3 `c` gate) | after CVM-225c, CVM-224b |
| P2 / CVM-226b | [RECON, ~250] `Int` divisibility lemmas for Cooper | lemma | after CVM-226a |
| P2 / CVM-226c | [DOC, ~100] Retire the bounded enumerator from the default path (it stays a test oracle); README and spec note | n/a | after CVM-225c |
| P1 / CVM-230 | [TACTIC, ~300] Arrays: read-over-write lemma instances | `inst` | after CVM-202 |
| P1 / CVM-231 | [TACTIC, ~250] Arrays: extensionality witnesses | `inst` | after CVM-230 |
| P1 / CVM-232 | [TACTIC, ~200] Arrays: model construction (finite maps with a default) | none: model-preserving | after CVM-230 |
| P2 / CVM-233 | [TACTIC, ~200] `const-array` and store equality | `inst` | after CVM-231 |
| P1 / CVM-234 | [CERT, ~150] `inst` emitter for array axiom instances | `inst` | after CVM-230 |
| P1 / CVM-235 | [RECON, ~300] `inst` reconstructor; array axioms as library lemmas (array as a function, `store` as `Function.update`) | `inst` | after CVM-234, CVM-111 |
| P3 / CVM-236 | [TACTIC, ~250] Weak-equivalence propagation (optimisation, no certificate needed beyond the lemmas it replaces) | none: optimisation over `inst` | after CVM-231 |
| P1 / CVM-241a | [PASS, ~250] Purification (one theory per atom, fresh definitions) | `red` | after CVM-119 |
| P1 / CVM-241b | [RECON, ~250] Purification lemma | lemma | after CVM-241a |
| P1 / CVM-240 | [COMB, ~350] Model-based theory combination: shared-term equality atoms from coinciding model values; one shared kernel proposition per equality atom | none: new atoms only (the split is axiom-free under the clause semantics, VER34 6.8) | after CVM-014, CVM-219, CVM-241a |
| P1 / CVM-242 | [COMB, ~200] Combined model merge | none: model-preserving | after CVM-240 |
| P1 / CVM-245 | [TACTIC, ~300] Theory dispatch (assert, propagate, backtrack, final-check) across registered components | none: composes fragments | after CVM-014 |
| P1 / CVM-246a | [TACTIC, ~250] `cdcl-sat` theory-hook trait: `assert` and `backtrack` hooks in propagation | none: model-preserving | after CVM-245 |
| P1 / CVM-246b | [TACTIC, ~300] Theory propagation with lazy reasons (explanation clauses on demand) | the explanation's fragment | after CVM-246a |
| P1 / CVM-246c | [TACTIC, ~300] Conflict analysis over theory explanations (learning a clause from a theory conflict) | the explanation's fragment | after CVM-246b |
| P1 / CVM-246d | [TACTIC, ~250] Lemma records for LRAT: a theory lemma enters the proof as an input clause with its fragment | `lrat` input clauses | after CVM-246c, KRN-009 |
| P1 / CVM-250 | [PASS, ~250] BV and Int bridge: `bv2nat` and `int2bv` as linear sums | `red` | after CVM-218, CVM-141b |
| P1 / CVM-251 | [RECON, ~250] Bridge lemmas for `bv2nat`/`int2bv` | lemma | after CVM-250 |
| P1 / CVM-252a | [HARN, ~250] M2 gates: QF_UF and QF_UFBV | none: harness | after CVM-007, CVM-205 |
| P1 / CVM-252b | [HARN, ~250] M2 gates: QF_LRA and QF_LIA | none: harness | after CVM-007, CVM-224b |
| P1 / CVM-252c | [HARN, ~250] M2 gate: QF_AX | none: harness | after CVM-007, CVM-235 |
| P1 / CVM-252d | [HARN, ~300] Fuzz generator: LIA and LRA | none: harness | after CVM-208 |
| P1 / CVM-252e | [HARN, ~300] Fuzz generator: EUF and arrays | none: harness | after CVM-208 |
| P1 / CVM-252f | [TACTIC, ~250] Move `adj-constraint-solver` onto real unsat cores and the new engine | none: consumer | after CVM-004, CVM-224b |
| P1 / CVM-252g | [TACTIC, ~250] Move LANG23's refinement checker to the new engine and meter | none: consumer | after CVM-015a, CVM-224b |

### CVM-M3: datatypes, quantifiers, nonlinear, classical

Gate: VER34 section 10, M3. Unlocks ADL-015 for ADTs, lists as datatypes and `forall` claims (classical claims reach `checked` until VER11a Q5).

| Priority / ID | Work | Certificate | State / prerequisite and acceptance |
|---|---|---|---|
| P1 / CVM-300 | [INSTR, ~300] `declare-datatype(s)`: reader, writer, validator, VM arm | none: model-preserving | after CVM-005 |
| P1 / CVM-301 | [TACTIC, ~300] Constructor, selector and tester axioms in the EUF component (injectivity, disjointness) | `dt` | after CVM-202, CVM-300 |
| P1 / CVM-302 | [TACTIC, ~200] Acyclicity | `dt` | after CVM-301 |
| P2 / CVM-303 | [TACTIC, ~200] Finite-enumeration datatypes, exhaustiveness | `dt` | after CVM-301 |
| P1 / CVM-304 | [TACTIC, ~150] Datatype values in models; total selectors | none: model-preserving | after CVM-301 |
| P1 / CVM-305 | [CERT, ~150] `dt` fragment emitter | `dt` | after CVM-301 |
| P1 / CVM-306a | [RECON, ~300] `dt` reconstructor core: injectivity and disjointness instances | `dt` | after CVM-305 |
| P1 / CVM-306b | [RECON, ~300] Per-datatype no-confusion lemma **generator** (untrusted producer, output checked) | lemma | after CVM-306a, PIF inductive support |
| P1 / CVM-306c | [RECON, ~250] Acyclicity lemmas | `dt` | after CVM-306b |
| P1 / CVM-306d | [RECON, ~250] Mapping of Warrant ADTs to the PIF inductive block (VER32 3.3; VER33 ADL-003a to c, ADL-004a) | none: elaboration | after CVM-306b |
| P1 / CVM-308 | [HARN, ~300] Evaluators extended to datatypes and quantifiers (enumerate a finite domain, else `Unknown`) | none: harness | after CVM-208 |
| P1 / CVM-310a | [PASS, ~250] NNF | `rw` | after CVM-009 |
| P1 / CVM-310b | [PASS, ~250] Prenex, with the NNF-over-DAG counter | `rw` | after CVM-310a, CVM-006b |
| P1 / CVM-311a | [PASS, ~250] Skolemisation: Skolem functions defined by `Classical.choice` | `red` | after CVM-310b, CVM-119 |
| P1 / CVM-311b | [RECON, ~250] `skolem_sound` lemma | `red` lemma | after CVM-311a, CVM-311c |
| P1 / CVM-311c | [RECON, ~300] **`Pif.Classical`**: derive excluded middle and `byContradiction` from the three axioms (Diaconescu; needs the `quot` dependency for `Quot.sound`) | lemma | after the axioms-file unit CVM-311d |
| P1 / CVM-311d | [DOC, ~150] The **axioms file after the model** (an optional last member of the **signed base**; binds `eq`, `iff`, `nonempty`, `quot`, then the `axiom.*` roles structurally, VER32 A1); chain rule for classical claims; spec-only with PIF-002 | n/a | after CVM-025; owner Q8 |
| P1 / CVM-312 | [TACTIC, ~250] Trigger selection | none: heuristic over `inst` | after CVM-310b |
| P1 / CVM-313a | [TACTIC, ~300] E-graph term index and pattern compilation | none: model-preserving | after CVM-202, CVM-312 |
| P1 / CVM-313b | [TACTIC, ~300] The matching loop (code-tree or backtracking) | `inst` | after CVM-313a |
| P1 / CVM-314 | [TACTIC, ~250] Instantiation loop with generation and cost caps on the meter | `inst` | after CVM-313b |
| P2 / CVM-315 | [TACTIC, ~300] MBQI part 1: candidate model extraction for quantified formulas | none: model-preserving | after CVM-314, CVM-308 |
| P2 / CVM-316a | [TACTIC, ~300] MBQI part 2a: build the counterexample query (the negated body at the candidate model) | none: model-preserving | after CVM-315 |
| P2 / CVM-316b | [TACTIC, ~250] MBQI part 2b: instantiate with the counterexample | `inst` | after CVM-316a |
| P1 / CVM-317a | [PASS, ~150] Finite expansion of quantifiers over small BV sorts (2^w counter) | `fe` | after CVM-310b, CVM-006b |
| P1 / CVM-317b | [RECON, ~250] `fe` lemma: a quantifier over a bit-list sort equals the conjunction over its values | `fe` | after CVM-317a |
| P1 / CVM-318 | [CERT, ~150] `inst` emitter for quantifier instantiations | `inst` | after CVM-314 |
| P1 / CVM-319 | [RECON, ~250] `inst` reconstructor for quantified hypotheses (`specialize`) | `inst` | after CVM-318, CVM-235 |
| P2 / CVM-320 | [HARN, ~250] Fuzz generators for quantified logics and DT; M3 benchmark gates | none: harness | after CVM-316b |
| P2 / CVM-330 | [PASS, ~250] Nonlinear monomial abstraction (fresh variables per monomial) | `red` | after CVM-220a |
| P2 / CVM-331 | [TACTIC, ~250] Linearisation lemmas: sign and zero | `nl` | after CVM-330 |
| P2 / CVM-332 | [TACTIC, ~300] Linearisation lemmas: tangent planes | `nl` | after CVM-331 |
| P2 / CVM-333 | [TACTIC, ~250] Linearisation lemmas: monotonicity | `nl` | after CVM-331 |
| P2 / CVM-334 | [CERT, ~150] `nl` fragment emitter | `nl` | after CVM-331 |
| P2 / CVM-335 | [RECON, ~350] `nl` reconstructor over `Int.mul` lemmas | `nl` | after CVM-334, CVM-217 |
| P2 / CVM-336 | [HARN, ~200] QF_NIA gate and fuzz generator | none: harness | after CVM-333 |

### CVM-M4: strings, sequences, floating point (as Warrant asks)

Gate: VER34 section 10, M4. Unlocks ADL-061 once a kernel IEEE model exists.

| Priority / ID | Work | Certificate | State / prerequisite and acceptance |
|---|---|---|---|
| P3 / CVM-407a | [RECON, ~350] Kernel IEEE 754 binary-format semantics file `Pif.FP`: unpack and pack (jointly with VER33's float note and ADL-061, so it has an owner) | definitions only | after VER33 float note, VER04p |
| P3 / CVM-407b | [RECON, ~350] `Pif.FP`: rounding modes and rounding | definitions only | after CVM-407a |
| P3 / CVM-400 | [TACTIC, ~350] FP: classification, unpack and pack circuits | `enc` | after M1, CVM-407a |
| P3 / CVM-401 | [TACTIC, ~350] FP: rounding circuits | `enc` | after CVM-400, CVM-407b |
| P3 / CVM-402 | [TACTIC, ~300] FP: add and subtract | `enc` | after CVM-401 |
| P3 / CVM-403 | [TACTIC, ~350] FP: multiply | `enc` | after CVM-401 |
| P3 / CVM-404 | [TACTIC, ~350] FP: divide and square root | `enc` | after CVM-403 |
| P3 / CVM-405 | [TACTIC, ~300] FP: fused multiply-add and conversions | `enc` | after CVM-403 |
| P3 / CVM-406a | [RECON, ~400] IEEE lemmas: classification, unpack, pack | lemma | after CVM-407a, CVM-400 |
| P3 / CVM-406b | [RECON, ~400] IEEE lemmas: rounding | lemma | after CVM-407b, CVM-401 |
| P3 / CVM-406c | [RECON, ~400] IEEE lemmas: add and subtract | lemma | after CVM-406b, CVM-402 |
| P3 / CVM-406d | [RECON, ~400] IEEE lemmas: multiply | lemma | after CVM-406b, CVM-403 |
| P3 / CVM-406e | [RECON, ~400] IEEE lemmas: divide and square root | lemma | after CVM-406d, CVM-404 |
| P3 / CVM-406f | [RECON, ~400] IEEE lemmas: fused multiply-add and conversions | lemma | after CVM-406d, CVM-405 |
| P3 / CVM-410 | [TACTIC, ~300] Strings: length abstraction and word-equation unfolding | `red` | after M2; VER04l has a consumer |
| P3 / CVM-411 | [TACTIC, ~350] Strings: regular membership by automata | `inst` | after CVM-410 |
| P3 / CVM-412 | [TACTIC, ~300] Sequences: concatenation, extract, nth | `inst` | after CVM-410 |
| P3 / CVM-413 | [CERT, ~200] String and sequence fragments | `inst`, `red` | after CVM-410 |
| P3 / CVM-414 | [RECON, ~350] String and sequence reconstructors | `inst`, `red` | after CVM-413 |
| P3 / CVM-415 | [HARN, ~250] Gates for strings and FP | none: harness | after CVM-414, CVM-406f |

### CVM-M5: hardening and speed

Gate: VER34 section 10, M5. **There are no placeholder rows**: the profiling report of CVM-500 adds each chosen unit as a real leaf row.

| Priority / ID | Work | Certificate | State / prerequisite and acceptance |
|---|---|---|---|
| P2 / CVM-500 | [HARN, ~250] Re-baseline against Z3 and cvc5 and profile; the report names the leaf rows to add; set the M5 `R` margin with the owner | none: harness | after M3 |
| P2 / CVM-503 | [COMB, ~300] Deterministic portfolio: sequential budgeted strategies; never changes a verdict | none: composes fragments | after CVM-500 |
| P3 / CVM-504 | [COMB, ~350] Optional parallel portfolio, owner Q9 | none: composes fragments | after CVM-503 |
| P2 / CVM-505 | [HARN, ~200] Nightly standing report (per-logic `R`, `c_trace`, `c_kernel`, PAR-2 trend) | none: harness | after CVM-007 |
