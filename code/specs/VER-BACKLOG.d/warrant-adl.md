# Warrant algorithm definition language (VER33): backlog shard

Authoritative queue for [VER33](../VER33-warrant-algorithm-definition-language.md), cut by the
owner's decomposition rule: **every row is one PR of about 100 to 400 lines** that fits the
existing architecture; a row that would be larger is split. Spec before code. Nothing starts
before the owner approves VER33 and a second adversarial review has been answered. Rung names
A0 to A4 are VER33 section 1; `PIF-*`, `KRN-*`, `G-M*` are VER32 and VER11a ids; `CVM-*` are
VER34 ids; `WX-*` are VER04 family ids. Buckets are VER31's `a|b|c`.

The main [VER-BACKLOG.md](../VER-BACKLOG.md) carries a pointer and the milestone headings only.

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
| P1 / ADL-002 | **`code/specs/logic-gates-model.warrant`**: the 11 gates over `Int`, `ref_g`, the 26 LG02 claims restated over `Int` as model claims, guard `to_bit = None`, oracle-versus-LG02 guard; **G-M1 waits on this** | after ADL-001a |
| P1 / ADL-002b | **`realizes` entries** for the 11 gate `op`s in the Rust binding file (embedding `int-by-value`, declared `panic_messages`) | after ADL-002 |
| P1 / ADL-004a | **ADL prelude layer `adl.core`** (`Adl.ite`, `Adl.Int.*`, `Adl.foldRange`, `Option`/`Pif.Result` helpers, `beq` combinators) with its specification lemmas; generator, allow-list entry | after PIF-004, KRN-002 slice 3a |
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
| P1 / ADL-008a | **Mandatory lemmas as gate conditions**: `beq_spec`, helper specification lemmas, exact-rejection goals (VER33 5.10) | after ADL-003c, ADL-004a |
| P1 / ADL-008b | **Non-vacuity witnesses as gate conditions**: generator for the witness lines (domain witness, one per outcome, one concrete run) | after ADL-008a |
| P1 / G-M1 | VER32's gate model proofs, with the ADL model file and goals list; signature conditions: ADL-005a to ADL-005c, ADL-008a and ADL-008b pass | after ADL-002, ADL-002b, ADL-003c, ADL-005c, ADL-008a, ADL-008b, VER32 PIF-006, PIF-020 |

## M3. Registration, the gate and the package-claim rule

| Priority / ID | Work | Prerequisite and acceptance |
|---|---|---|
| P1 / ADL-006a | **VER13 registration**: the ids of VER33 9.1 with bucket entries (owner signs the bucket file), with VER32 PIF-016a | with G-M1 |
| P1 / ADL-006b | **VER31 amendments**: model-theorem category and link status (with PIF-020), the §6.2 cap on lane packages, `solver-attested` evidence kind (with CVM-170), A-rung computation | after ADL-000 |
| P1 / ADL-006c1 | **Gate change: A-rung field**, computed by the gate (never declared) and printed beside the VER31 rung | after ADL-006b |
| P1 / ADL-006c2 | **Gate change: link status** with the **domain actually quantified** and the whole-unit marker (A3-source only) | after ADL-006c1 |
| P1 / ADL-006c3 | **Gate change: A2 is never rendered as a package-level word containing "proved"** | after ADL-006c1 |
| P1 / ADL-007 | **Package-claim goal rule (owns this gate change)**: the gate elaborates each package claim over `U8` into its own goal over `Int` with range hypotheses (K0, up to 255; only binders, in-range literals, comparisons and canonical helpers; `exists` takes the range as a conjunct; fixed-width arithmetic refused) and the signed goals-list line; the claim rises only with `Equiv` for every `op` it mentions | after ADL-003c, ADL-006c1 |
| P2 / ADL-009 | **Totality goals and the fuel-footprint gate rule (owns this gate change)** for measure-defined constants (`gen.<f>.total`, `.mono`); the corpus is **hand-written PIF** (fuel definitions written by hand), because the fuel elaboration is ADL-051, which comes later; the counterexample `g(n) = g(n)` is a case | after ADL-003c, ADL-001b, PIF-018 |

## M4. Solver-backed testing (A1)

| Priority / ID | Work | Prerequisite and acceptance |
|---|---|---|
| P1 / ADL-010 | **Boundary-vector generation** from input contracts and preconditions into portable vectors (`tested`): extremum search, `c +- 1`, length limits | after CVM-020 to CVM-023 (min/max, seeded determinism), ADL-001a |
| P1 / ADL-011 | **Regression-example store in the corpus** (not the `.warrant` file) and the counterexample-confirmation loop (evaluate model and code; non-reproducing model is a solver defect) | after ADL-010 |
| P1 / ADL-012a | **WIR-to-SMT encoder, scalars**: Bool and fixed-width scalars for the Rust WIR subset; differential-checked against the evaluator | after VER34 CVM-M1 (certified QF_BV), `warrant-ir` `sym.rs` |
| P2 / ADL-012b | **WIR-to-SMT encoder, arrays plus a length** (VER33 6.3) | after ADL-012a, VER34 CVM-M2 (arrays) |
| P1 / ADL-013 | **Claim-to-VC generator**: the bounded statement with `k` in the VC text, per-type `k` (bit length, element count, node count), the **non-vacuity check** (constraints without the negated goal are SAT, model confirmed) | after ADL-012a |
| P2 / ADL-014 | **Bounded model-versus-code driver**: `drat:` certificate path, the two `tcb.*` ids, `bounded(k)` for the Rust lane only | after ADL-013, CVM-170 for the `pif:` path; arrays need ADL-012b |
| P2 / ADL-015 | **VC discharge for A2/A3 goals**: bit-level obligations as certificates reconstructed into PIF | after CVM-170, CVM-173, KRN-009 |

## M5. ABI, refinement and the A3-WIR link for Rust (G-M2)

| Priority / ID | Work | Prerequisite and acceptance |
|---|---|---|
| P1 / ADL-021 | **`adl.wir-abi` file**: total `outcome`, `enc` as a partial function, reason tags from `panic_messages`; signed (`adl.wir-abi.faithful`) | after ADL-020, PIF-019 |
| P1 / ADL-022 | **`Refines`/`Equiv` goal templates with witnesses**; `Dom(h)` read from source parameter types | after ADL-021 |
| P2 / ADL-023 | **Certificate format and checker glue** | after ADL-022 |
| P1 / ADL-032 | **Lane-independent whole-unit, export-map and build-profile framework** (VER33a) | after ADL-030. **Note (ADL-001a security review):** the lifter confirms every `panic_messages` template against a literal panic message in the lane source (the long-term form of VER33 R5; the schema's anchor rule and denylist are only a backstop) |
| P1 / ADL-033 | **Rust subset additions for `logic-gates`**: `&str` parameter, format-string `assert!`, `and_n(&[u8])` walled off or admitted; export map for the crate. **Note (ADL-001a security review):** the format-string `assert!` messages are the templates the binding lists; the lifter checks each against the literal in source | after ADL-032, VER25a |
| P1 / G-M2 | VER32's Rust gates lifted at WIR level (A3-WIR); scope per VER32 Q2; `Dom(h)` is `u8` from source | after G-M1, ADL-022, ADL-033, PIF-019 |

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

## M8. A3-source and per-lane lifters (all depend on K1 and PIF-009)

| Priority / ID | Work | Prerequisite and acceptance |
|---|---|---|
| P1 / ADL-031 | **The one kernel-side grammar interpreter**, as kernel definitions, reviewed once; lane grammars as data; measured on Rust | after ADL-030, KRN-002 slice 4 (VER32 section 11, P1: literals, quotient package, `prim` validation; needed by PIF-009), PIF-009 (K1) |
| P2 / ADL-035 | **A3-source for Rust**: goal restated over the source text and the Rust semantics (VER25c) | after ADL-031, KRN-016 |
| P3 / ADL-060 | **K1 elaboration**: `lits`, Lean `Init` `BitVec` fixed-width, `Vec`, bytes | after PIF-009 |
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
