# Warrant algorithm definition language (VER33): backlog shard

Authoritative queue for [VER33](../VER33-warrant-algorithm-definition-language.md), cut by the
owner's decomposition rule: **every row is one PR of about 100 to 400 lines** that fits the
existing architecture; a row that would be larger is split. Spec before code. Nothing starts
before the owner approves VER33 and a second adversarial review has been answered. Rung names
A0 to A4 are VER33 section 1; `PIF-*`, `KRN-*`, `G-M*` are VER32 and VER11a ids; `CVM-*` are
VER34 ids; `WX-*` are VER04 family ids. Buckets are VER31's `a|b|c`.

The main [VER-BACKLOG.md](../VER-BACKLOG.md) carries a pointer and the milestone headings only.

**Follow-up 2026-11-24** (rows changed by the cross-spec follow-up: ADL-002, ADL-004a, ADL-006a,
ADL-006b, ADL-008a, ADL-010, ADL-012a/b, ADL-013 to ADL-015, ADL-031, ADL-060 split into ADL-060a
and ADL-060b, G-M2; each says so in its own row). **K1 now means native words and `lits` with no
`Init`**; the `Init` tier (named K2 in VER33 4.2) is ADL-060b only.

**Named follow-up specifications (backlog items, not files yet):** **VER33a** (lifters:
interface, subset registry, the one kernel-side grammar interpreter with each lane's grammar as
data, export and whole-unit rules, build profiles; ADL-030) and **VER33b** (the ADL-WIR value
encoding, `outcome`, `Refines`/`Equiv`, certificate format, trap identity; ADL-020).

## M0. Spec and decisions

| Priority / ID | Work | Prerequisite and acceptance |
|---|---|---|
| P1 / ADL-000 | VER33 round 1 revision merged; second adversarial review answered; owner answers to VER33 section 12 | review answered |
| P1 / ADL-020 | **Write VER33b**: ADL-WIR encoding, total `outcome`, trap identity and `panic_messages`, `Refines` and `Equiv` with witnesses, certificate format; spec only | after ADL-000 |
| P1 / ADL-030 | **Write VER33a**: lifter interface, subset registry, **decide the single kernel-side grammar interpreter** (finding 31), export map, whole-unit and opaque-item rules, build-profile rule, refusal protocol; spec only | after ADL-000 |

## M1. Profile and the gate models (A2 preparation; unblocks G-M1)

| Priority / ID | Work | Prerequisite and acceptance |
|---|---|---|
| P1 / ADL-001a | Warrant **`profile algorithm` header and `realizes` binding-schema field**: tokens and grammar changes, drift tests, checker diagnostics, positive and negative corpus | after ADL-000 |
| P2 / ADL-001b | **`measure` recursion and `Nat` peeling**: syntax and checker; the evaluator **refuses `measure` models** (VER33 4.1) | after ADL-001a, WX-R1 (VER04s) |
| P2 / ADL-001c | **`fold` over `range` admitted in the profile** (totality rule) | after ADL-001a, WX-D2 (VER04k) |
| P1 / ADL-002 | **`code/specs/logic-gates-model.warrant`**: the 11 gates over `Int`, `ref_g`, the 26 LG02 claims as model claims **under LG02's names, stated over `Bool`, plus separate `<name>_over_int` and `<name>_matches_ref` claims (naming decided 2026-11-24)**, guard `to_bit = None`, **`Pif.GateError` declared once in this model file, not in a prelude**, oracle-versus-LG02 guard; **G-M1 waits on this** | after ADL-001a |
| P1 / ADL-002b | **`realizes` entries** for the 11 gate `op`s in the Rust binding file (embedding `int-by-value`, declared `panic_messages`) | after ADL-002 |
| P1 / ADL-004a | **ADL prelude layer `adl.core`** (**`Adl.ite.{u} alpha c t e`** with the result type explicit, **`Adl.Int.{beq, le, lt}`** (no `Int.ble`/`blt`), **`Adl.Nat.{beq, ble, blt}`**, `>`/`>=` as `lt`/`le` with swapped arguments, `Adl.foldRange`, **`Adl.Option.isSome`**, `Option`/`Pif.Result` helpers, `beq` combinators; `↔` is emitted as an `And` of both directions, the PIF prelude having no `Iff`) with the specification lemmas it can prove at once; **deferred to ADL-008a**: `Int` order against add, add commutative and associative, `sub a a = 0`, `Nat.sub` against add, `beq_spec` for `Option`, `Result`, `Prod`; generator, allow-list entry | after PIF-004, KRN-002 slice 3a |
| P2 / ADL-004b | **Layer `adl.list`**, appended (never an edit of `adl.core`), with lemmas | after ADL-004a |

## M2. Elaborator and the evidence for its meaning (A2)

| Priority / ID | Work | Prerequisite and acceptance |
|---|---|---|
| P1 / ADL-003a | **Elaborator crate, non-recursive K0**: types, literals, `if`, flat `match`, calls; deterministic naming (`Pif.*`/`Adl.*`); model-file emission; printer round-trip hook (`tcb.pif.printer`); this is **PIF-017's first slice** | after ADL-004a, ADL-001a |
| P1 / ADL-003b | **Pattern compilation and derived definitions**: final `_` expansion, refusals of nested/tuple/literal patterns, derived `beq_T`, records via `T.rec` | after ADL-003a |
| P1 / ADL-003c | **Goals-list emitter**: claims to goals, hypotheses, `Eq T` for a top-level `==`, claim ids in VER32's charset, the generated lines of 5.10 as stubs | after ADL-003b |
| P1 / ADL-005a | **Independent value encoder** (a separate crate, sharing no code with the elaborator) and the **vector-to-`Eq`-theorem generator** | after ADL-003a |
| P1 / ADL-005b | **Per-function coverage gate**, maximum skip ratio, claim-instance checks (hypothesis, `=>`, quantifier) | after ADL-005a, ADL-003c |
| P1 / ADL-005c | **Mutation corpora**: elaborator mutants (swap `ite`, drop arm, reorder constructors) and goal-level mutants (drop or add hypothesis, swap sides) | after ADL-005b |
| P2 / ADL-005d | **Second independent elaborator**, written without reading the first; agreement on model-file and goals-list bytes (VER33 5.8 (1)) | after ADL-003c |
| P2 / ADL-005e | **Evaluator runs `measure` models under its meter for the differential only** (never for A0/A1 generation), so `elaboration-faithful` has vectors for them (VER33 4.1, 5.8) | after ADL-005a, ADL-001b |
| P1 / ADL-008a | **Redesigned 2026-11-25 (derive, don't detect; VER33 5.10, Fable design review); implementation pending.** **Mandatory lemmas as gate conditions**: `beq_spec`, helper specification lemmas, exact-rejection goals (VER33 5.10), **including the lemmas ADL-004a deferred** (`Int` order against add, add commutative and associative, `sub a a = 0`, `Nat.sub` against add, `beq_spec` for `Option`, `Result`, `Prod`) | after ADL-003c, ADL-004a |
| P1 / ADL-008b | **Redesigned 2026-11-25; implementation pending.** The witnesses are now `(Π Γ. False) → False` per claim with a binder, outcome witnesses per `Result` model, and a run witness (VER33 5.10). **Non-vacuity witnesses as gate conditions**: generator for the witness lines (domain witness, one per outcome, one concrete run) | after ADL-008a |
| P1 / G-M1 | VER32's gate model proofs, with the ADL model file and goals list; signature conditions: ADL-005a to ADL-005c, ADL-008a and ADL-008b pass | after ADL-002, ADL-002b, ADL-003c, ADL-005c, ADL-008a, ADL-008b, VER32 PIF-006, PIF-020 **G-M1 status 2026-11-25: implemented, PRs pending (not on `main` yet).** The gate model file, the 89-line goals list and the claim file are produced. All 89 goals are kernel-checked with empty footprints, end to end through `pif-check`, **with a test-fixture sign-off, not a real signature**. The vectors are 1021/1021. Remaining: the owner's signatures (the `pif-signoff 1` file and `buckets-v1.json` revision 2; VER31 13.8), ADL-005b/c, the ADL-008a/b gate conditions, and the push queue (the implementation branches). After the ADL-008 redesign (VER33 5.10) the goals list is 120 lines: the added lines are the derived G1, G3 and G4 obligations. |
| P2 / ADL-008d | **`rejects P with E` as a Warrant source construct** that elaborates to the single `<module>.contract.<f>.<n>` goal `P x ↔ f x = Err E`; until it lands the gate's exact-rejection check is a lint (VER33 5.10) | after ADL-008a, ADL-008b (the gate redesign) |
| P2 / ADL-008e | **Totality as gate conditions**: `<f>.total` and `<f>.mono` for every `measure` model (VER33 5.5), moved out of 5.10's gate list by the redesign | after ADL-008a, ADL-008b |

## M3. Registration, the gate and the package-claim rule

| Priority / ID | Work | Prerequisite and acceptance |
|---|---|---|
| P1 / ADL-006a | **VER13 registration**: the ids of VER33 9.1 with bucket entries (owner signs the bucket file), with VER32 PIF-016a **and PIF-016d**; **includes `tcb.vcgen.claim-faithful`** (the claim-to-VC generator, owned by ADL-013) **and `tcb.drat.checker-correct` while the DRAT route lasts** | with G-M1 **Drafted 2026-11-25, pending owner signature.** The ids VER33 9.1 marks new that G-M1 opens or relies on are proposed in `code/assumptions/catalogue-v2.proposed.json` and `buckets-v2.proposed.json` (with PIF-016a; `gates.model.faithful` is the instantiation of `<pkg>.model.faithful`, so `logic_gates.model.faithful` is not registered; `tcb.vcgen.claim-faithful` and `adl.wir-abi.faithful` included). **The owner signs `buckets-v1.json` revision 2.** The per-lane and A3 ids wait for G-M2 (PIF-016b). |
| P1 / ADL-006b | **VER31 amendments**: model-theorem category and link status (with PIF-020), the §6.2 cap on lane packages, `solver-attested` evidence kind (**cites VER34 CVM-170b, which owns the amendment text**: `pif:sha256:` scheme, `solver-attested` capped at `tested`, the DRAT certificate record), A-rung computation | after ADL-000 **Drafted 2026-11-25, pending owner signature.** Written as proposed text in VER31 section 13.4 to 13.6: the section 6.2 cap lifted per claim by computation, the A-rung computation with "A2 is never rendered as proved", and `solver-attested` **cited** (not copied) from CVM-170b, as this row says. Not in force until the owner signs VER31 section 13; ADL-006c1 to c3 (gate code) wait for it. |
| P1 / ADL-006c1 | **Gate change: A-rung field**, computed by the gate (never declared) and printed beside the VER31 rung | after ADL-006b |
| P1 / ADL-006c2 | **Gate change: link status** with the **domain actually quantified** and the whole-unit marker (A3-source only) | after ADL-006c1 |
| P1 / ADL-006c3 | **Gate change: A2 is never rendered as a package-level word containing "proved"** | after ADL-006c1 |
| P1 / ADL-007 | **Package-claim goal rule (owns this gate change)**: the gate elaborates each package claim over `U8` into its own goal over `Int` with range hypotheses (K0, up to 255; only binders, in-range literals, comparisons and canonical helpers; `exists` takes the range as a conjunct; fixed-width arithmetic refused) and the signed goals-list line; the claim rises only with `Equiv` for every `op` it mentions | after ADL-003c, ADL-006c1 |
| P2 / ADL-009 | **Totality goals and the fuel-footprint gate rule (owns this gate change)** for measure-defined constants (`gen.<f>.total`, `.mono`); the corpus is **hand-written PIF** (fuel definitions written by hand), because the fuel elaboration is ADL-051, which comes later; the counterexample `g(n) = g(n)` is a case | after ADL-003c, ADL-001b, PIF-018 |

## M4. Solver-backed testing (A1)

| Priority / ID | Work | Prerequisite and acceptance |
|---|---|---|
| P1 / ADL-010 | **Boundary-vector generation** from input contracts and preconditions into portable vectors (`tested`): extremum search, `c +- 1`, length limits | after CVM-020 to CVM-023 (min/max, seeded determinism), ADL-001a (**no certificate prerequisite**: boundary vectors are `tested`, so CVM-170a/170b and CVM-173 are not cited here) |
| P1 / ADL-011 | **Regression-example store in the corpus** (not the `.warrant` file) and the counterexample-confirmation loop (evaluate model and code; non-reproducing model is a solver defect) | after ADL-010 |
| P1 / ADL-012a | **WIR-to-SMT encoder, scalars**: Bool and fixed-width scalars for the Rust WIR subset; differential-checked against the evaluator | after VER34 CVM-M1 (certified QF_BV), CVM-173, `warrant-ir` `sym.rs`; **registers the encoder's ids** (`tcb.symexec.engine-correct`, `tcb.bitblast.encoding-correct`: bucket entries and owner) with ADL-006a |
| P2 / ADL-012b | **WIR-to-SMT encoder, arrays plus a length** (VER33 6.3) | after ADL-012a, VER34 CVM-M2 (arrays); **extends the encoder's id entries to arrays plus a length** |
| P1 / ADL-013 | **Claim-to-VC generator**: the bounded statement with `k` in the VC text, per-type `k` (bit length, element count, node count), the **non-vacuity check** (constraints without the negated goal are SAT, model confirmed) | after ADL-012a, VER34 CVM-170b (the record that binds the VC digest and `k`); **owns `tcb.vcgen.claim-faithful`**, registered by ADL-006a |
| P2 / ADL-014 | **Bounded model-versus-code driver**: `drat:` certificate path, the two `tcb.*` ids, `bounded(k)` for the Rust lane only | after ADL-013, **CVM-170a (the `pif:` path), CVM-170b (record and amendment) and CVM-173**; arrays need ADL-012b |
| P2 / ADL-015 | **VC discharge for A2/A3 goals**: bit-level obligations as certificates reconstructed into PIF | after **CVM-170a, CVM-170b**, CVM-173, KRN-009 |

## M5. ABI, refinement and the A3-WIR link for Rust (G-M2)

| Priority / ID | Work | Prerequisite and acceptance |
|---|---|---|
| P1 / ADL-021 | **`adl.wir-abi` file**: total `outcome`, `enc` as a partial function, reason tags from `panic_messages`; signed (`adl.wir-abi.faithful`) | after ADL-020, PIF-019 |
| P1 / ADL-022 | **`Refines`/`Equiv` goal templates with witnesses**; `Dom(h)` read from source parameter types | after ADL-021 |
| P2 / ADL-023 | **Certificate format and checker glue** | after ADL-022 |
| P1 / ADL-032 | **Lane-independent whole-unit, export-map and build-profile framework** (VER33a) | after ADL-030 |
| P1 / ADL-033 | **Rust subset additions for `logic-gates`**: `&str` parameter, format-string `assert!`, `and_n(&[u8])` walled off or admitted; export map for the crate | after ADL-032, VER25a |
| P1 / G-M2 | VER32's Rust gates lifted at WIR level (A3-WIR); scope per VER32 Q2, **decided: option D** (all 11 gates, after G-M2-D); `Dom(h)` is `u8` from source | after G-M1, ADL-022, ADL-033, PIF-019 |

## M6. Generated code (A4)

| Priority / ID | Work | Prerequisite and acceptance |
|---|---|---|
| P1 / ADL-040 | **Backend framework**: style-profile format, provenance header, CI **regenerate-and-diff** check | after ADL-003c |
| P1 / ADL-041 | **Rust style profile and backend** for K0 scalar models | after ADL-040 |
| P1 / ADL-042 | **`rust/logic-gates-adl` package** and the full-domain differential against the hand-written lane | after ADL-041 |
| P1 / G-M4 | One lane per PR in the order of VER33 8.4: style-profile review, backend, package with the LG02 corpus tests (rung `tested`); **a row per lane, listed under Per-lane rows below** | after ADL-042 |
| P2 / ADL-044 | **Sabotage corpus** for backends and certificates: wrong output must fail the reverse check | after G-M2, ADL-041 |
| P2 / ADL-045 | **Round trip, Rust, A3-WIR**: lift the generated Rust back; trades `rust.backend.code-link` for `wir.lowering.faithful` | after ADL-042, G-M2 |
| P2 / ADL-046 | **WIR compiler, K0 straight-line (optional)**: scalar and `Option`/`Result` models; certificate emitter; cross-check oracle | after ADL-021, ADL-003c |
| P2 / ADL-047 | **Loop templates T1 and T2 (optional)** with derived invariants | after ADL-046, KRN-012 |

## M7. Recursive algorithms, adders (A2, A3)

| Priority / ID | Work | Prerequisite and acceptance |
|---|---|---|
| P2 / ADL-050 | **Primitive-recursion elaboration** on direct fields | after ADL-003b, WX-R1 |
| P2 / ADL-051 | **Fuel elaboration for `measure` models** (`Option`-returning `f_fuel`) | after ADL-050, ADL-009 |
| P2 / G-M3a | Adders at the model level in the profile, for every width | after ADL-050, PIF-018 |
| P2 / G-M3b | Adder code level (generated, then lifted) | after G-M2, ADL-047, KRN-012 |

## M8. A3-source and per-lane lifters (all depend on K1; PIF-009 stays listed until ADL-030 decides whether the lane semantics need `Init`)

| Priority / ID | Work | Prerequisite and acceptance |
|---|---|---|
| P1 / ADL-031 | **The one kernel-side grammar interpreter**, as kernel definitions, reviewed once; lane grammars as data; measured on Rust | after ADL-030, KRN-002 slice 4 (VER32 section 11, P1: literals, quotient package, `prim` validation; needed by PIF-009), PIF-009 (kept conservatively; K1 itself needs no `Init`, ADL-030 decides) |
| P2 / ADL-035 | **A3-source for Rust**: goal restated over the source text and the Rust semantics (VER25c) | after ADL-031, KRN-016 |
| P3 / ADL-060a | **K1 elaboration** (split 2026-11-24 from ADL-060): `lits`, **native `Adl`/`Pif` fixed-width words**, `Vec`, bytes; layer `adl.fixed` appended; **no `Init`**; the word representation of VER32 PIF-019 | after KRN-002 slice 4, PIF-019, ADL-004b |
| P3 / ADL-060b | **The `Init` tier** (named K2 in VER33 4.2), for the **Lean escape hatch only** (VER33 6.2): Lean's `Init` in the base chain (the model is re-elaborated and re-signed), or PIF-010's PIF-to-Lean import with a closure check; **no third bridge** between K1 words and `Init`'s `BitVec` on the main path | after PIF-009, ADL-060a |
| P3 / ADL-061 (placeholder, not a leaf: to be split when scheduled) | Later profiles: floats (needs a kernel IEEE model), stateful objects (VER04m), opaque types (VER04n) | after VER04p FL1, VER04m S1 |

### Per-lane rows (lifters and backends)

Rows marked *P3 placeholder* group several lanes and are **not leaves**: each lane in them is split into its own five rows when scheduled.

**Lifter rows**: for each lane, **five PRs**, each one PR-sized: (a) the lane's subset
semantics spec (VER25c style); (b) the lane's grammar as data plus an instance in the kernel
interpreter (ADL-031); (c) the semantics file in `warrant-lib`; (d) the lifter; (e) the
differential harness against the real toolchain (version pin, build-profile matrix). Row ids
`ADL-L<nn>-a` to `-e`. **The second lane is chosen after KRN-015/016 report measured cost and
K1 is scheduled**; the order below is the recommendation, not a commitment. Every row depends
on ADL-031.

| Lane id | Lane | Target | Notes |
|---|---|---|---|
| ADL-L01 | Rust (A3-source) | WIR | the calibration; with ADL-035 |
| ADL-L02 | to be decided: Go or OCaml | WIR or direct | after the measurement |
| ADL-L03 (P3 placeholder, not a leaf) | the other of Go/OCaml; then F#, Haskell | direct for the functional lanes | |
| ADL-L04 | Java | WIR | its own integer semantics |
| ADL-L05 | Kotlin | WIR | **not** shared with Java |
| ADL-L06 | C# | WIR | **not** shared with Java |
| ADL-L07 (P3 placeholder, not a leaf) | Swift, Dart | WIR | |
| ADL-L08 (P3 placeholder, not a leaf) | C, C++ | WIR | undefined behaviour and memory |
| ADL-L09 (P3 placeholder, not a leaf) | TypeScript, Python, Ruby, Lua, Perl, Elixir | WIR | dynamic lanes last; may stay at A0/A1 |

**Backend rows (G-M4)**: for each lane, **three PRs**: (a) the style profile and its review;
(b) the backend for the 11 gates; (c) the package with the LG02 corpus tests and the
regenerate-and-diff CI entry. Row ids `G-M4-<lane>-a` to `-c`, in the order of VER33 8.4. C and
C++ have no gates package today, so their generated code is the lane (VER33 7.5).
