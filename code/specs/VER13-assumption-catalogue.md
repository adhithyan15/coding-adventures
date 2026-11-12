# VER13: Assumption Catalogue and Discharge Ladder

**Status:** Draft specification. Implemented so far: the catalogue data and its integrity checker (AS-001a part 1) and the report-side rules AC-1..AC-14 in `ca-assurance-report` (AS-001a part 2); see the implementation clarifications in section 5.2.
**Authority:** VER00 §7.5 (assume-guarantee), §7.5.2, §7.5.6, §7.6.1, §7.6.5; VER01, VER04h §9-10, VER15, VER18 (planned), VER20, VER26 §3, §8.1
**Backlog:** AS-001 (this spec), AS-002..AS-005 (VER-BACKLOG, "Assumption reduction, programs to logic gates")
**Direction (user, 2026-11-09):** slowly pare the assumptions back, from programs down to logic gates, until a claim such as "this runs in constant time" is proved on a simulated netlist rather than assumed below WIR.

---

## 1. Why this document exists

Every proof label in this repository is an assume-guarantee statement
(VER00 §7.5.1): *if* the listed assumptions hold, *then* the claim holds.
Today the assumption lists are short strings scattered across four
places:

| Where | What lives there |
|---|---|
| `ca-assurance-report/src/evidence.rs` | `compiler.preserves_timing`, `memory.unaliased`, `<kind>:unchecked` rows |
| `ca-assurance-report/src/matrix.rs`, `lib.rs` | nine `unverified::` attestation ids, `EXEMPT-CWE-208-GATE-LEVEL`, four `EXEMPT-CWE-<n>-UNSAFE-EXCEPTION` ids, `analyzer::` / `capability-audit::` / `effect-certificate::` evidence ids |
| specs (VER08a G-1..G-9, VER08b §14, VER23a, VER26 §3, CT01, VER04h) | prose caveats nobody can cite |
| `warrant-ir` | the cost schedule, the `bounded(k)` unroll cap, the "trivial-miter" certificate kind, name-matched lowering of `volatile_barrier::*` |

Nothing ties a string to a meaning, to the claims that rely on it, or to
what would remove it, and nothing stops a report from citing an
assumption that does not exist or from omitting one that does. VER13
fixes that in three steps:

1. an **inventory** (§3): every assumption currently used or implied,
   with a stable id, a layer, a meaning, its dependants and its discharge
   evidence;
2. a **discharge ladder** for constant time (§4): the concrete route from
   today's WIR proof to a simulated netlist, with the evidence artefact
   at each rung and how the report shows the shrinking assumption set;
3. a **machine-readable catalogue** (§5) that reports cite by id, checked
   by `ca-assurance-report --check` (§6), and a starting backlog (§7).

An assumption is not a defect. VER00 §7.5.1 is explicit that "we prove
our logic, not the platform's". The defect is an assumption that is
unnamed, unchecked, or silently dropped from a label. The goal of this
program is to make the set *named, minimal and shrinking*.

---

## 2. Vocabulary

**Assumption.** A proposition `A` that a proof or a piece of evidence
relies on but that no check in the cited evidence establishes. It has a
*layer* (where in the stack it is a statement about).

**Layers** (top is nearest the author, bottom is nearest physics). The
order matters: a label `proved-to-L` (§5.4) means every assumption at a
layer *above* `L` has been discharged.

| # | Layer key | Statements about |
|---|---|---|
| 1 | `spec` | what the Warrant file / CT01 say and what the observation model includes |
| 2 | `src` | the source language, its subset, its unsafe regions, the capability analyzers |
| 3 | `wir` | the Rust-to-WIR lowering, WIR semantics, the symbolic engine's model and its proof objects |
| 4 | `cc` | the compiler and its IR (LLVM IR for the host build; our own IR for an owned build) |
| 5 | `mc` | machine code, the ISA and its model |
| 6 | `uarch` | microarchitecture: caches, predictors, speculation, fetch |
| 7 | `gate` | the gate/netlist model and the sequential/clocking abstraction |
| 8 | `phys` | silicon, power, EM, faults: the floor that software cannot discharge |

Three cross-cutting families sit beside the layers because they are not
about one stack level: `tcb` (the trusted computing base of the
verification tools themselves), `os` (allocator, `std`, kernel, vDSO,
runtime gate limits) and `gov` (approval and exemption governance). Each
entry still carries a `layer` field in the catalogue (the layer at which
it is *consumed*); the id prefix is only a family.

**Status** of an assumption in the catalogue:

| Status | Meaning |
|---|---|
| `assumed` | Nothing in the repository checks it. |
| `attested` | A static claim recorded by an audit step that no tool re-checks (the nine `unverified::` ids). |
| `evidenced` | Tested or measured on instances (tests, differential runs, a pinned-toolchain gate); not proof. |
| `discharged` | Established by cited evidence ids that the checker resolves. Only valid with a `rung` and evidence (§6). |
| `floor` | Cannot be discharged by software; it is the declared bottom of the ladder (`phys.*`). |

**Discharge.** Replacing an assumption by evidence the checker can
validate: a proof object, a translation-validation certificate, a Miri or
differential run bound to a digest, or a lower layer's own claim (VER00
§7.5.6 rule 1: an assumption and the lower layer's claim are the same
formula).

**Non-claim.** Something a package says it does *not* provide, such as
`gf256` multiplication being table-indexed and therefore not
constant-time (MA01, "Constant-time status": the CWE-208 row is
"applicable and unaddressed"). A non-claim is not an assumption and gets
no catalogue entry; it is a missing claim. The checker never lets a
non-claim be reported as `proved` (rule AC-12, §6).

---

## 3. Inventory

Sources read for this inventory: VER00, VER01, VER04, VER04h, VER15,
VER20, VER23a, VER25/25a/25b, VER26, CT01, VER-BACKLOG; `ca-assurance-report`
(`evidence.rs`, `matrix.rs`, `lib.rs`, `checker.rs`), `warrant-ir`
(`ct_proof.rs`, `cost.rs`, `lower.rs`), `rust/ct-compare` (`assurance_report.json`,
`effect_certificate.json`, `tests/optimized_ir.rs`, `ct-compare.warrant`); and, from
branches not yet merged, VER08a (G-1..G-9, branch `worktree-agent-a274a2c279c1b0d70`),
VER08b §14 (branch `worktree-agent-a9f1811c0377a9acf`) and MA01 (branch
`worktree-agent-a88d5407e27ca898b`).

Columns: **Used by** names the labels/rows/packages that rely on the
assumption today. **Discharge** is the evidence that would remove it.
**Tools** are existing repository packages (or "new" where nothing
exists). **Diff** is S (days), M (1-2 weeks), L (several weeks),
XL (a quarter or more), N/A (not dischargeable by software). **Ord** is the
attack tier of §3.9 (`P` = parallel capability/runtime track, `floor` = declared).

Legacy ids that reports already emit are kept verbatim as catalogue ids
and marked *(legacy)*; umbrella ids list the finer ids that refine them.

### 3.1 Layer `spec`

| Id | Meaning | Used by | Discharge | Tools | Diff | Ord |
|---|---|---|---|---|---|---|
| `spec.secret-labels.complete` | The `secret` labels in `ct-compare.warrant` cover every input whose value must not influence the observation (contents of `a`,`b`, `choice`; `a`,`b` of `eq_u64`). The vacuity check only rejects the case of *no* secret input; it cannot tell that one secret was left unlabelled. | every `constant-time` obligation of ct-compare | Derive the label set from CT01's corpus table and compare; negative control that drops one label and must fail | `ct-compare` fixtures, `warrant-grammar`, VER04h manifests | S | O1 |
| `spec.obs-model.complete` | The observable event set of VER20 §2.1 (branch decision, memory provenance+offset, step count) contains everything the attacker model for CWE-208 can see. Not in it: operand-dependent latency, store *values*, instruction-fetch addresses, indirect-call targets. | all `constant-time` proofs | Extend `ObsEvent` with fetch address and indirect-target events, re-prove; push the rest into `mc.*` / `uarch.*` | `warrant-ir` `ct_proof.rs`, `trace.rs` | M | O2 |
| `spec.obs-model.public-length` | Lengths and widths are public (CT01: a length mismatch is observable). The proof ties them equal across the two runs. | `eq`, `eq_fixed`, `select_bytes` proofs | Not dischargeable: it is part of the claim. Keep as a *declared* assumption in every report so a consumer who treats lengths as secret is warned | n/a | N/A | floor |
| `spec.platform.declared-none` | `ct-compare.warrant` says `assume platform: none` and `assume environment: none`, yet the proofs rely on platform assumptions (`compiler.preserves_timing`, ...). VER04 §7 says `none` cannot erase a relied-on assumption; the Warrant file and the report disagree. | VER04 W0401/W0601 hygiene, pilot acceptance | Make `assume platform:` cite catalogue ids; the report's list must equal the file's list | `warrant-grammar`, `ca-assurance-report` | M | O1 |
| `spec.corpus.ct01-faithful` | The Warrant claims formalise CT01's prose; CT01 itself disclaims compiled-code behaviour and cache/speculation immunity. | every claim of ct-compare | Conformance corpus + sabotage controls exist (E1); add a spec-sync test mapping each CT01 normative sentence to a claim or a catalogue id | `code/specs/fixtures/ct-compare-v1/` | M | O2 |

### 3.2 Layer `src`

| Id | Meaning | Used by | Discharge | Tools | Diff | Ord |
|---|---|---|---|---|---|---|
| `src.rust.subset-conformance` | The audited source stays inside the Rust subset VER25a lowers; constructs outside it are rejected, not mis-lowered. | `not-yet-provable` boundary, all WIR proofs | Subset conformance suite with negatives; make the lowering total-or-error | `warrant-ir/src/rust_subset`, VER25a | M | O2 |
| `src.rust.int-semantics` | `u8`/`u64` arithmetic wraps or panics as WIR models it (no debug-vs-release divergence in the audited code). | functional claims, `bounded(k)` proofs | Run the differential harness with overflow checks on and off; forbid implicit overflow in the subset | `warrant-ir` interpreter | S | O2 |
| `src.rust.safe-bounds-checks` | An out-of-range slice index panics instead of reading memory. Used to *discharge* CWE-125/787 from bounded proofs (VER04h §9 check 7), i.e. at `bounded(k)`, not as a Miri result. | weakness rows CWE-125, CWE-787 | Memory-safety obligation in WIR (index in range on all paths) at `proved`, not `bounded(k)` | `warrant-ir`, Miri | M | O2 |
| `src.rust.unsafe-region.sound` | The one audited `unsafe` region (`volatile_barrier`) does exactly a bounded, in-range volatile read/write of one `u8` and violates no aliasing, lifetime or null rule. | exemptions `EXEMPT-CWE-416/415/476/190-UNSAFE-EXCEPTION` | Miri run on the region's tests, bound to the source digest (WD-008); later a WIR proof of the region | Miri (`code/scripts/miri-twig-vm.sh` is the model), `ct-compare/tests` | M | O2 |
| `src.panic.paths-public` | Panic/unwind paths (length-mismatch `assert!`, bounds checks) branch only on public values. | `select_bytes`, `eq_fixed` proofs | Lowering keeps panics as explicit branches (see lesson on `assert_eq!` lowered to a no-op); prove they sit under public conditions | `warrant-ir` lowering | M | O2 |
| `src.analyzer.G-1` | Names that do not resolve to `std`/`core`/`alloc` or the listed crates, including **third-party crates**, are assumed pure. | every E0 capability label | ct-compare: assert the dependency closure is empty (`[dependencies]` is empty today) as a checked fact; in general analyse dependencies or require certificates for them | `ca-capability-analyzer`, cargo metadata | S (ct-compare) / L (general) | O1 |
| `src.analyzer.G-2` | Implicit authority never named in source (hash-seed entropy, allocation, panic output) is not detected. | E0 | Pair with the VER08b runtime gate's armed-phase observation; list expected implicit authority in the catalogue | `capability-runtime-conformance` (branch) | M | P |
| `src.analyzer.G-3` | Capability `target` is documentation, not a verified bound. | E0 | Runtime path allowlists (Landlock); static constant-argument analysis | VER08b | L | P |
| `src.analyzer.G-4` | Manifest `package` is not matched to its directory/crate name. | E0 manifest binding | Trivial check in the analyzer | `ca-capability-analyzer` | S | O1 |
| `src.analyzer.G-5` | `tests/`, `benches/`, `examples/`, `[dev-dependencies]` are not analysed though `cargo test` executes them (and tests produce trusted evidence). | E0; evidence production | Analyse them under a separate "build-time authority" policy | `ca-capability-analyzer` | M | P |
| `src.analyzer.G-6` | Macros are not expanded; authority produced only by another crate's macro expansion is invisible. | E0 | Analyse expanded code / MIR (also feeds the WIR front end) | rustc, `warrant-ir` | L | P |
| `src.analyzer.G-7` | Dynamic behaviour (function pointers, trait objects, `dlopen` through any crate) is not tracked. | E0 | Closed-world call-graph proof; ct-compare has no trait objects (checkable fact) | `ca-capability-analyzer` | M | P |
| `src.analyzer.G-8` | Type-directed routes beyond the §3.3 method heuristic are missed. | E0 | Resolve through rustc type information | rustc API / MIR | L | P |
| `src.analyzer.G-9` | `cfg` is ignored (conservative); compiler flags, `.cargo/config.toml`, `RUSTFLAGS` are not examined. | E0, and `cc.version-pinned` | Examine and pin build configuration; record it in the effect certificate | `ca-capability-analyzer` | S | O1 |
| `src.analyzer.correct` | The analyzer, and the small checker that re-reads the program, are themselves correct (VER00 §8.6.5: "everything above rests on one assumption"). | `effect-certificate::proved`, every `capability-audit::*` id | Proof-carrying checker already reduces it; next: checker corpus with negatives, then a mechanised checker | `ca-capability-analyzer` checker | L | P |
| `src.effects.witness-token` | The effect module's "pure computation witness token" (VER23a) is a witness only; the Rust compiler enforces nothing about it. Authority is established by the analyzer, not by types. | VER23a labels | Type-level enforcement (sealed capability traits), or keep as a declared non-enforcement; low value for ct-compare | `layout-effects`, VER23a generator | M | P |
| `src.effects.certificate-binding` | The effect certificate's `source_digest`/`manifest_digest` cover every file that determines behaviour; a partial digest (e.g. omitting `build.rs`) would certify different code. | `effect-certificate::proved` | Digest all inputs incl. `Cargo.toml`, `build.rs`, lock file; CI regeneration gate (see the lesson on stale certificates) | `ca-capability-analyzer`, CI | S | O1 |
| `src.registry.excluded-rows` | A weakness row marked `excluded` (e.g. CWE-78) is excluded because capability evidence is empty, not because the class was proved absent. | 17 `excluded` rows of the ct-compare report | Each exclusion cites the catalogue ids of the analyzer caveats it depends on (G-1..G-9) | `ca-assurance-report/src/matrix.rs` | S | O1 |
| `att.analyzer.name-allowlist-regex-free` | *(attested: `unverified::analyzer::name-allowlist-regex-free`)* The analyzer's name allowlist uses no regex-like matching that could over-match. | row CWE-1333 | Property test of the matcher plus a code-level check | `ca-capability-analyzer` | S | O1 |
| `att.ast-audit.no-compression-routines` | *(attested)* No compression/decompression routine appears in the AST. | row CWE-409 | Banned-construct scan bound to a digest | `ca-capability-analyzer` | S | O1 |
| `att.ast-audit.no-deserialization` | *(attested)* No deserialization entry point appears. | row CWE-502 | same | same | S | O1 |
| `att.ast-audit.no-legacy-ciphers` | *(attested)* No legacy cipher or hash is used. | row CWE-327 | same | same | S | O1 |
| `att.ast-audit.no-protocol-sinks` | *(attested)* No protocol/network sink (ldap, xpath, ...) is reachable. | rows CWE-90/91 family | same | same | S | O1 |
| `att.ast-call-graph.acyclic` | *(attested)* The call graph is acyclic, so there is no unbounded recursion. | row CWE-674 | Compute the call graph from WIR; assert acyclicity (also discharges `wir.bound.depth-unchecked`) | `warrant-ir` | S | O1 |
| `att.language-type-system.no-prototype-pollution` | *(attested)* The language has no prototype mutation. | row CWE-1321 | Language-property fact in the per-language catalogue (VER00 §7.5.2); true by construction in Rust | per-language catalogue | S | O1 |
| `att.type-signatures.unconstrained-slices` | *(attested)* No public signature accepts an unconstrained slice beyond what the `bound` rows cover. | row CWE-1333/400 family | Check public signatures against `bound` rows | `warrant-grammar` | S | O1 |
| `att.warrant-ir.cost-semantics-linear` | *(attested)* The WIR cost semantics gives linear cost for the audited operations. | row CWE-407 | Cite the proved step-bound obligation (`steps <= c*n + d`, already produced) instead | `warrant-ir` `BoundProver` | S | O1 |

### 3.3 Layer `wir`

| Id | Meaning | Used by | Discharge | Tools | Diff | Ord |
|---|---|---|---|---|---|---|
| `wir.lowering.faithful` | The Rust-to-WIR lowering preserves the observable behaviour *and the observable trace* of the Rust source. It has been wrong before (lesson: `assert_eq!` lowered to a no-op; `vec![0u8; n]` stored past a zero-length buffer). | every WIR proof | Differential execution of WIR vs rustc on generated inputs (exists, partial), then translation validation against rustc's MIR for the audited functions | `warrant-ir` interpreter, rustc MIR dump | L | O2 |
| `wir.lowering.volatile-by-name` | `volatile_barrier::accumulate/read`, `read_volatile`, `write_volatile`, `black_box` are lowered **by name** to plain loads/stores/identity (`lower.rs`). The unsafe body is never analysed; WIR is told what it does. | ct-compare `constant-time` proofs; `src.rust.unsafe-region.sound` | Lower the unsafe body for real (volatile as ordered load/store) or validate the name-matched model against the body | `warrant-ir` | M | O2 |
| `wir.inline.sound` | The WIR inliner (needed to expose `ct_eq` to the proof) preserves semantics. | `ct_eq` proof | Differential test (exists), then per-run translation validation of inlined vs original | `warrant-ir/src/inline.rs` | M | O2 |
| `wir.interp.differential-only` | Trust in the WIR front end rests on differential testing against the real runtime (VER00 §7.6.2), not proof. | all WIR proofs | See `wir.lowering.faithful` | property engine | M | O2 |
| `wir.cost.step-model` | The cost schedule (ALU 1, branch 1, load/store 2, ...) gives equal cost to events that are equal under `spec.obs-model`. A step count is an abstract cost, not time. | every `steps-bound` and the `StepCount` event | Not dischargeable at WIR; discharged *by replacement*: at `mc` the step count becomes retired instructions, at `gate` clock cycles (§4) | `warrant-ir/src/cost.rs` | N/A at WIR | via ladder |
| `memory.unaliased` *(legacy)* | The buffers passed to an operation occupy disjoint, in-bounds provenance regions and are not mutated during the call. Emitted whenever an obligation has buffers and a label other than `proved`. | `ct_eq_soundness`, `ct_eq_fixed_*`, `select_bytes` claims | Prove 2-safety under both aliasing cases (alias / disjoint) or show the trace is alias-insensitive; for the `select_bytes` output prove freshness | `warrant-ir` symbolic memory (`sym.rs`) | M | O2 |
| `wir.memory.provenance-model` | WIR's `(provenance id, offset)` memory model matches what addresses mean on the target (no overlapping allocations, no out-of-provenance arithmetic). | memory-address events | Instantiated per target at `mc` (rung R2) | `warrant-ir` | M | O4 |
| `wir.loop.bounded-k` | `bounded(k)` proofs unroll loops to depth `k` (128 for ct-compare); the claim says nothing for lengths above `k`. | labels `bounded(128)` | Loop invariants then induction: `proved` for all lengths (VER04h §10 follow-up) | `warrant-ir` `BoundProver`, `cdcl-sat` | L | O2 |
| `wir.bound.heap-unchecked` *(legacy `heap-bound:unchecked`)* | No bound on heap use is proved; `heap` rows are recorded `unchecked`. | all four ct-compare claims | Heap bound from WIR allocation sites (`select_bytes` allocates `len`; `eq`, `eq_u64` allocate nothing) | `warrant-ir` cost semantics | S | O1 |
| `wir.bound.depth-unchecked` *(legacy `depth-bound:unchecked`)* | No bound on call depth is proved. | all four ct-compare claims | Acyclic call graph (also retires `att.ast-call-graph.acyclic`) | `warrant-ir` | S | O1 |
| `wir.proof.trivial-miter` | A "trivial-miter" certificate is a DRAT refutation of a formula that collapsed to constant false before search: valid, but weak evidence that the encoding is right. Marked `certificate_kind: "trivial-miter"` today. | ct-compare `constant-time` claims | Non-collapsed negative controls (exist: `proofneg/*`) plus an independent re-derivation: compare the miter CNF with an independently built one | `warrant-ir`, `cdcl-sat`, `drat-check` | M | O1 |
| `wir.proof.obligation-binding` | The recorded `obligation_sha256` is only shape-checked (VER04h WD-009): a valid but unrelated refutation with a plausible hash is not detected. | every certificate-backed label | Re-parse, re-lower, re-inline, rebuild the obligation text, recompute digest and compare each certificate's CNF (WD-009) | `ca-assurance-report`, `warrant-ir` | M | O1 |

### 3.4 Cross-cutting `tcb` (consumed at layer `wir`)

| Id | Meaning | Used by | Discharge | Tools | Diff | Ord |
|---|---|---|---|---|---|---|
| `tcb.drat.checker-correct` | `drat-check` accepts only valid RUP/RAT refutations. | all UNSAT verdicts | Corpus with negatives exists; next a smaller checker or a cross-check against an independent implementation (TypeScript solver, VER09a) | `drat-check`, TypeScript SAT lane | M | O3 |
| `tcb.bitblast.encoding-correct` | Bit-blasting and Tseitin encodings (VER10) are equisatisfiable with the Boolean semantics of the operations. | all SAT-backed proofs | Exhaustive differential tests per small width; equivalence-check the encoder against `arithmetic` gate-level adders | `arithmetic`, `logic-gates`, bit-blaster | M | O3 |
| `tcb.symexec.engine-correct` | The symbolic executor, miter construction and `TraceComparator` implement VER20 faithfully. | all CT and bound proofs | Sabotage controls exist (`proofneg/secret-branch`); add cross-checks by concrete two-run execution on the interpreter | `warrant-ir` | M | O3 |
| `tcb.report.checker-correct` | `ca-assurance-report --check` rejects what it claims to reject: JSON parsing, digest framing, manifest handling. | every report | Falsification tests per rule exist; add property-based fuzzing of the checker | `ca-assurance-report` | M | O3 |
| `tcb.evidence.manifest-honesty` | Evidence manifests are produced by the runs they describe, not hand-edited (lesson: a hard-coded report passes a gate that only checks consistency). | all derived evidence | Bind manifests to the producing run's digest; CI regenerates and diffs | `ct-compare/evidence`, CI | M | O1 |
| `tcb.rust-toolchain.trusted` | The `rustc`/`cargo`/linker that build the *verification tools* do not miscompile the prover (trusting trust). | the whole tool chain | Diverse double compilation of the small checker; far future | rustc, owned compiler | XL | O6 |

### 3.5 Layer `cc` (compiler and IR)

`compiler.preserves_timing` is today a single legacy id for everything
between WIR and the machine. VER13 makes it an **umbrella** and splits it,
because different pieces are dischargeable by different tools.

| Id | Meaning | Used by | Discharge | Tools | Diff | Ord |
|---|---|---|---|---|---|---|
| `compiler.preserves_timing` *(legacy umbrella)* | Refined by `cc.llvm-ir.no-secret-branch`, `cc.llvm-ir.addresses-public`, `cc.volatile-honoured`, `cc.version-pinned`, `cc.caller-context`, `mc.backend.no-secret-branch`, `mc.backend.addresses-public`. A report may keep citing the umbrella; it counts as discharged only when every refinement is. | all four ct-compare claims (always emitted) | see refinements | | | |
| `cc.llvm-ir.no-secret-branch` | In the optimised LLVM IR (`-C opt-level=3`) no `br`/`switch`/branch-lowered `select` condition is data-dependent on secret input. Today *evidenced* by pattern checks of the loop CFG (`tests/optimized_ir.rs`: every cycle passes through the volatile block, no exit depends on volatile-derived values). | `compiler.preserves_timing` | Taint analysis over the whole IR CFG with secret roots = argument loads; negative controls (branchy select, early-exit loop) | new `ir-taint` checker; `tests/optimized_ir.rs` as seed | M | O3 |
| `cc.llvm-ir.addresses-public` | No GEP/load/store address is secret-dependent in the optimised IR. | same | same taint pass over address operands | same | M | O3 |
| `cc.volatile-honoured` | LLVM keeps every volatile access and its order (what makes the accumulate loop non-elidable). | the `volatile_barrier` technique | Part of the IR gate (volatile loads on every CFG cycle); exists | `tests/optimized_ir.rs` | S | O3 |
| `cc.version-pinned` | Evidence is for one `rustc` version (1.97.0 in this repo's environment), target, `opt-level`, codegen flags and `RUSTFLAGS`/`.cargo/config.toml`; another toolchain may behave differently. | all `cc.*` and `mc.*` evidence | Record the full tuple in the evidence manifest; checker rejects evidence whose tuple differs from the pinned build | `rust-toolchain`, CI | S | O3 |
| `cc.caller-context` | A caller may inline or const-propagate the callee and re-introduce a branch the compiler can prove one-sided; the library cannot control this (CT01: "optimisers and JIT compilers can reintroduce a branch"). | any consumer of ct-compare | Out of ct-compare's scope; state as a boundary; offer a caller-side check | n/a | N/A | floor |

### 3.6 Layer `mc` (machine code and ISA)

| Id | Meaning | Used by | Discharge | Tools | Diff | Ord |
|---|---|---|---|---|---|---|
| `mc.backend.no-secret-branch` | Instruction selection and register allocation do not turn the IR's branch-free select/compare into a secret-dependent branch (e.g. `cmov`/`setcc` to `jcc`). Invisible at LLVM IR level. | `compiler.preserves_timing` | Taint check on the assembly/ELF of the *shipped* artefact, or an owned back end whose output is validated (rung R2) | disassembly taint checker (new); `x86-simulator` decode as a seed; for the owned lane `riscv-simulator` decode | L | O3/O4 |
| `mc.backend.addresses-public` | Spills, stack slots and table addresses are public. | same | same | same | L | O3/O4 |
| `mc.isa.timing-data-independent` *(VER26 "machine timing")* | The latency of every executed instruction does not depend on operand data (no early-out multiplier, variable-latency divide or data-dependent microcode). | CWE-208 floor; VER26 §3 | Choose an ISA whose gate-level implementation has this by construction, and prove it structurally: RV32I base has no `mul`/`div`, and `riscv-gatelevel` advances one instruction per step (§4) | `riscv-gatelevel`, `riscv-simulator` | M | O5 |
| `mc.isa.model-faithful` | The ISA model used as semantics (`riscv-simulator`) matches the ISA specification. | rungs R2, R3 | Python oracle vectors exist (`python_differential.rs`, `generate_python_oracle.py`); add riscv-arch-test style vectors | `riscv-simulator` | M | O4 |

### 3.7 Layers `uarch`, `gate` and `phys`

| Id | Meaning | Used by | Discharge | Tools | Diff | Ord |
|---|---|---|---|---|---|---|
| `uarch.cache.address-trace-suffices` | If the data-address trace is identical across secrets, cache-state evolution is identical (no secret-dependent cache effect). | cache-timing reading of CWE-208 | Holds trivially on a cache-less machine; for modelled caches use `cpu-cache` and extend the observation model | `cpu-cache`, `cpu-pipeline` | M | O5 |
| `uarch.fetch.pc-trace` | The instruction-fetch (pc) sequence is attacker-visible and is covered by the trace equality. | CWE-208 | Add the pc to `ObsEvent` (see `spec.obs-model.complete`) | `warrant-ir` | S | O2 |
| `uarch.speculation.none` | No speculative execution leaks secret-dependent state (Spectre class). CT01 disclaims it. | CWE-208 | By construction on an in-order non-speculative core (`riscv-gatelevel` has none); not dischargeable on a host CPU | `riscv-gatelevel` (structural) | M | O5 |
| `uarch.predictor.no-state-leak` | Branch-predictor state does not leak beyond the branch-decision trace. | CWE-208 | No predictor in the in-order model; otherwise not dischargeable | same | M | O5 |
| `uarch.shared-resources.none` | No SMT/shared-cache/shared-bus neighbour observes the program. | CWE-208 | Single-core simulated machine; otherwise a deployment statement | `riscv-gatelevel` | M | O5 |
| `gate.model.is-netlist` | The gate-level "simulator" is a *netlist*. Today `riscv-gatelevel` evaluates `logic_gates` primitive functions (`and_gate`, `mux2`, ...) from Rust code; there is no extractable netlist object a SAT solver can consume. | any `proved-to-gate-level` label | Extract a netlist (HNL, `gate-netlist-format`) from the engine; show by differential run that it computes the same step function | `gate-netlist-format`, `logic-gates`, `riscv-gatelevel` | L | O5 |
| `gate.primitives.correct` | `logic-gates`/`arithmetic` primitives implement the Boolean functions (AND, OR, NOT, XOR, MUX, adders). | netlist semantics | Exhaustive truth tables (complete) down to the `transistors` CMOS model | `transistors`, `logic-gates` | S | O6 |
| `gate.sequential.ideal-dff` | Flip-flops are ideal edge-triggered state elements: no metastability, setup/hold violation, glitch or ambiguous reset. | cycle semantics | Declared model; reset state checked by a structural test | `logic-gates/sequential.rs` | N/A | floor |
| `gate.clocking.one-instruction-per-cycle` | Every instruction retires in exactly one clock step whatever the operands (no multi-cycle FSM, no stall). This turns "step count" into "cycle count". | `StepCount` event at gate level | Structural proof on the netlist: no state encodes progress within an instruction; halt only on the explicit halt flag | `riscv-gatelevel`, HNL | M | O5 |
| `gate.memory.abstraction` | The 512 KiB `DffMemory` (4,194,304 flip-flops) is a flat byte array whose only data-dependent input is the address bus, and access takes the same time for any address. | memory-address events | Cut memory out of the netlist at the ports and prove the core exposes only `(addr, we, wdata)` | `riscv-gatelevel` | M | O5 |
| `gate.isa-equivalence` | The one-step gate-level next-state function equals the ISA model for **every** state and instruction, not only the sampled Python vectors. | composition R2 to R3 | VER18 one-step equivalence: a miter between the extracted netlist and a bit-blast encoding of the RV32I step, UNSAT with a DRAT certificate | `cdcl-sat`, `drat-check`, `riscv-simulator` | XL | O5 |
| `phys.silicon.realization` | A physical chip behaves as the netlist: same function and no data-dependent delay, glitching, power or EM visible to the attacker. | the floor of every constant-time label | None in software. State it; do not hide it. | `fab-process-simulation`, `spice-netlist-parser` are models, not proof | N/A | floor |
| `phys.side-channels.out-of-scope` | Power, EM, acoustic, thermal and fault-injection channels are outside the attacker model. | CWE-208 | Declared non-goal | n/a | N/A | floor |

### 3.8 Cross-cutting `os` and `gov`

| Id | Meaning | Used by | Discharge | Tools | Diff | Ord |
|---|---|---|---|---|---|---|
| `os.allocator.size-only` | `ct_select_bytes` allocates a `Vec`; allocator latency and address depend only on the public length, never on secret contents (the allocator never sees them). | `select_bytes` timing; `src.analyzer.G-2` | Owned lane: no allocator (caller-provided or static output buffer), or a checked allocator with a size-only trace (VER16) | VER16 allocator (planned) | M | O4 |
| `os.std.runtime-benign` | The `std`/`alloc` routines called (`Vec` drop, `memcpy`, panic machinery) do not branch on secret contents. | whole-binary timing | Owned lane is `core`-only (`no_std`) | owned lane | M | O4 |
| `os.time.vdso` | `Instant::now()`/`clock_gettime` through the vDSO make no syscall and are invisible to the seccomp and strace rings (VER08b §14; CAP-RT-006). | `time:read` capability evidence | Detect by unmapping the vDSO in the box, or declare as residual | `capability-runtime-conformance` (branch) | M | P |
| `os.env.memory-reads` | Environment reads are memory reads, not syscalls; only an explicitly built environment makes undeclared variables absent. | runtime E0 evidence | Launch-time environment allowlist (exists) | VER08b | S | P |
| `os.syscall.baseline` | The armed-phase baseline `rcg-baseline-1` (observed `{write, exit_group, futex, mmap, munmap}`; `brk` unobserved) is a measurement on glibc 2.39, not a proof. | runtime capability gate | Per-lane baselines, version string, spec-sync test (exist) | VER08b | S | P |
| `os.runtime-gate.exercised-paths-only` | The runtime gate shows the *tested* execution made no undeclared syscall, not that no path can. | runtime evidence | The static analyzer covers all paths; neither replaces the other | VER08a + VER08b | N/A | P |
| `os.kernel.trusted` | The kernel, `strace`, Docker, seccomp and Landlock behave as documented. | runtime gate | Two independent rings exist; otherwise declared | VER08b | N/A | P |
| `os.counters.not-live` | Retired-instruction counters are host-dependent; the E2/microbenchmark rung is `not-live` and no instruction-count evidence is claimed. | microbenchmark row, CWE-208 exemption text | Counters on a host that provides them; or replace by simulator step counts (owned lane: exact) | `cpu-simulator`, `riscv-simulator` | S | O4 |
| `os.platform.linux-only` | Runtime evidence exists only for Linux; other platforms are not covered at all. | VER08b | Per-platform gate | VER08b | L | P |
| `gov.approval.blocked` | Exemption approval is a hardware-token-backed step that VER00 §12.1 blocks; **every** exemption stays `approved: false` and the verdict `report-only`. | verdict of every report | Settle trust-root findings, implement approval (VER00 §9.1) | `chief-of-staff-hardware-key-approval` | L | gov |
| `gov.catalogue.integrity` | The catalogue (§5) is complete and honest: a tool-private assumption missing from it would not be seen. | every report | Hash-pin the catalogue in the report; spec-sync test; review rule | `ca-assurance-report` | S | O1 |

### 3.9 Size, and the order of attack

The catalogue has **84 entries** (generated count, rule AC-14). By
family: `spec` 5, `src` 18 (analyzer caveats G-1..G-9, analyzer/effects/
certificate trust, Rust-semantics entries) plus 9 `att.*` attestations,
`wir` 12, `tcb` 6, `cc` 6 (including the `compiler.preserves_timing`
umbrella), `mc` 4, `uarch` 5, `gate` 6, `phys` 2, and cross-cutting `os`
9 and `gov` 2. By consumption layer (tcb entries counted at `wir`): spec 5,
src 27, wir 18, cc 6, mc 4, uarch 5, gate 6, phys 2, os/gov 11. Of the 84,
**4 are legacy ids that reports emit today** (`compiler.preserves_timing`,
`memory.unaliased`, `heap-bound:unchecked`, `depth-bound:unchecked`), 9 are
the existing `unverified::` ids, and 5 are the existing computed exemptions
(derived records, below, not entries). The other ~70 are assumptions that
are relied upon today without being named anywhere in a report. Floor or
declared (not dischargeable by software): `spec.obs-model.public-length`,
`cc.caller-context`, `gate.sequential.ideal-dff`, the two `phys.*` entries,
`tcb.rust-toolchain.trusted` (practically) and the N/A rows of `os`.

**Exemptions as derived records.** The five computed exemptions in the
ct-compare report are not extra assumptions; each is a record *derived
from* catalogue ids and must list them (rule AC-13):

| Exemption | Derived from (catalogue ids) |
|---|---|
| `EXEMPT-CWE-208-GATE-LEVEL` | everything below the label's layer: today `compiler.preserves_timing` (all refinements), `mc.isa.timing-data-independent`, `uarch.*`, `gate.*`, `phys.*`, `wir.cost.step-model` |
| `EXEMPT-CWE-416-UNSAFE-EXCEPTION`, `-415-`, `-476-`, `-190-` | `src.rust.unsafe-region.sound`, `wir.lowering.volatile-by-name` |
| `EXEMPT-CWE-208-NO-SECRET-OPERANDS` | nothing: the class does not apply (every operand labelled `public`, no constant-time obligation; VER04h section 9 item 20). First used by `rust/wasm-leb128`. |

**Order of attack.** Tiers are ordered by leverage per unit effort and by
the rule "work starts at the leaves and climbs" (VER-BACKLOG, AS):

| Tier | Theme | Entries | Why this order |
|---|---|---|---|
| O1 | Bookkeeping and integrity | `gov.catalogue.integrity`, `spec.platform.declared-none`, `spec.secret-labels.complete`, `wir.proof.obligation-binding`, `wir.proof.trivial-miter`, `wir.bound.heap-unchecked`, `wir.bound.depth-unchecked`, the nine `att.*`, `src.analyzer.G-1/G-4/G-9` (for ct-compare), `src.effects.certificate-binding`, `src.registry.excluded-rows`, `tcb.evidence.manifest-honesty` | Cheap, remove `unchecked` and `unverified::` strings, and make every higher rung's evidence trustworthy. Nothing above is meaningful if a certificate may be unrelated to its obligation. |
| O2 | Source and WIR semantics | `memory.unaliased`, `src.rust.unsafe-region.sound`, `wir.lowering.*`, `wir.inline.sound`, `wir.loop.bounded-k`, `spec.obs-model.complete`, `uarch.fetch.pc-trace`, `src.rust.*` | Shrinks the exemption list (Miri) and the biggest silent risk (a wrong lowering, which has happened) before building on WIR. |
| O3 | Host compiler evidence | `cc.*`, `mc.backend.*` (host lane), `tcb.drat/bitblast/symexec/report` | Turns `compiler.preserves_timing` from "a test passes" into "an analysis of the whole IR/asm passes on a pinned build". Partial by nature (rustc/LLVM stay trusted for other builds). |
| O4 | Owned compile lane | owned back end WIR to RV32I, `mc.isa.model-faithful`, `wir.memory.provenance-model`, `os.allocator.size-only`, `os.std.runtime-benign`, `os.counters.not-live` | The only route where `compiler.preserves_timing` is *proved* (per-run translation validation), because the compiler is ours and tiny. |
| O5 | ISA to gates | `gate.*`, `uarch.*`, `mc.isa.timing-data-independent` | Needs the netlist-extraction work (largest single piece) and VER18. |
| O6 | The floor | `gate.primitives.correct` (to transistors), `tcb.rust-toolchain.trusted`, `phys.*` | What remains is declared, not hidden. |
| P | Capability/runtime track | `src.analyzer.G-2/3/5/6/7/8`, `src.analyzer.correct`, `src.effects.witness-token`, `os.time.vdso`, `os.*` runtime | Independent of the constant-time chain; runs in parallel under LH-002 / CAP-RT. |

---

## 4. The constant-time discharge ladder

The claim being climbed is the one in the ct-compare report:
"for every pair of runs whose public inputs agree, the observable trace
is identical" (VER20 §2.2). The ladder never changes the claim; it
changes **what the claim is a statement about** (WIR, then an optimised
IR, then machine code on an ISA model, then a netlist) and so which
assumptions remain.

```
 R0  WIR self-composition (today)        spec ─ src ─ WIR ━━━━ assumed: cc, mc, uarch, gate, phys
 R1  translation validation, host build  ... ─ WIR ═ IR/asm ━━ assumed: parts of mc, uarch, gate, phys (pinned toolchain only)
 R2  owned compile to RV32I, ISA trace   ... ─ WIR ═ RV32I ━━━ assumed: isa-model, uarch, gate, phys
 R3  gate-level netlist                  ... ─ RV32I ═ netlist assumed: gate model, phys     (label proved-to-gate-level)
 R4  transistor-level primitives         ... ─ netlist ═ CMOS   assumed: phys only
```

`━` = assumed; `═` = discharged by a checked artefact.

### 4.1 Rung R0: today (baseline)

- **Claim:** `proved` or `bounded(128)` on `eq`, `eq_fixed`, `select_bytes`,
  `eq_u64` (`rust/ct-compare/assurance_report.json`).
- **Evidence:** WIR self-composition miter; `drat:sha256:...` certificate;
  `certificate_kind: "trivial-miter"`; `tests/optimized_ir.rs`.
- **Remaining assumptions, in catalogue terms:** every row of §3 whose
  layer is `spec` through `phys` and that is not marked discharged.
  Reported today as only `compiler.preserves_timing`, `memory.unaliased`,
  `heap-bound:unchecked`, `depth-bound:unchecked`. **That short list under-reports**:
  at least `wir.lowering.faithful`, `wir.lowering.volatile-by-name`,
  `wir.proof.obligation-binding`, `spec.obs-model.complete`,
  `mc.isa.timing-data-independent`, `uarch.*`, `gate.*` and `phys.*` are
  silently relied upon. Moving to the catalogue makes the list *longer*
  before it gets shorter, and that is the point (AC-4).
- **Exemption:** `EXEMPT-CWE-208-GATE-LEVEL`, unapproved.

### 4.2 Rung R1: translation validation of the compiled artefact (user item a)

**Goal.** Replace the pattern test in `tests/optimized_ir.rs` with a
checker that covers the *whole* function, then extend it below LLVM IR.

**Artefact (host lane).** `rustc --emit=llvm-ir,asm -C opt-level=3
--target x86_64-unknown-linux-gnu` of `ct-compare`, with the toolchain tuple
recorded (`cc.version-pinned`).

**Deliverables.**

1. `ir-taint` (new crate, uses no `rustc` internals): parses the emitted LLVM
   IR for the four entry points, marks every load from a secret-argument
   pointer and the `choice`/`a`/`b` scalar arguments as secret, propagates
   through SSA data flow and `phi`, and **fails** if any `br` condition,
   `switch` operand, `select` that lowers to a branch, or GEP/load/store
   address is secret-tainted. Volatile loads are secret sources but their
   *addresses* are public (loop counter).
2. The same on the emitted assembly (`mc.backend.no-secret-branch`): a
   small x86-64 control-flow and taint pass over the `.s` text (instruction
   subset used by this crate is tiny), failing on a `jcc`/indirect jump whose flags
   derive from a secret and on secret-dependent address operands. Repository
   `x86-simulator` supplies a decoder if the text route proves too brittle.
3. **Negative controls**, which are the proof the checker works: a
   deliberately branchy `select_bytes` (`if choice { a } else { b }` on bytes),
   an early-exit `eq`, a secret-indexed table; each must fail, in the
   same style as the existing `proofneg/*` and sabotage guards.

**What is a "translation validation" here.** The checker is not a
compiler proof; it validates *this build's output* against the WIR
property (per run), which is exactly VER00 §7.6.2's use of translation
validation. Label for the discharged parts: `proved-per-run`
(VER00 §7.6.5), bound to the digest of the emitted IR/asm.

**Evidence artefact.** `evidence/ir-taint-manifest.json`:

```json
{ "id": "cc/ct_compare/ir-taint/eq",
  "toolchain": {"rustc": "1.97.0", "target": "x86_64-unknown-linux-gnu",
                "opt_level": 3, "rustflags": ""},
  "artefact_sha256": "…", "checker_sha256": "…",
  "entry": "ct_eq", "secret_roots": 3, "branches": 4, "tainted_branches": 0,
  "addresses": 9, "tainted_addresses": 0,
  "negative_controls": {"branchy-select": "fail", "early-exit-eq": "fail"},
  "verdict": "pass" }
```

**Assumptions discharged:** `cc.llvm-ir.no-secret-branch`,
`cc.llvm-ir.addresses-public`, `cc.volatile-honoured`, `mc.backend.*` (for
the pinned build only). **Remaining of the umbrella:** `cc.caller-context`
(declared), and *other* toolchains. `compiler.preserves_timing` therefore
becomes: *discharged for pinned build P; assumed otherwise*. The report
must say "for build P" (rule AC-9).

**Not covered:** `mc.isa.timing-data-independent`, `uarch.*`, `gate.*`,
`phys.*`: a host x86 CPU is not ours. Difficulty: M/L.

### 4.3 Rung R2: owned compile to a repository CPU simulator (user item b)

**Why a different lane, not just "run it on a simulator".** Rung R1 trusts
whatever `rustc` emitted. To *prove* preservation the compiler must be
small enough to validate per run. So R2 introduces an **owned
non-optimising back end** from WIR to RV32I, `wir-to-riscv` (new). Since WIR
is already the thing that was proved constant-time, and the back end maps
each WIR instruction to a fixed machine sequence, the validation is
local.

**Target choice.** RV32I on `riscv-simulator` (ISA model) with encoder
`riscv-encoder`. Rationale: base ISA has no `mul`/`div`/FP, so
operand-dependent latency cannot arise; byte loads/stores (`lbu`, `sb`)
exist, so the `u8` accumulators are natural; `riscv-gatelevel` is its
gate-level twin (rung R3). The existing `riscv-backend` consumes the dynamic-VM
`CIRInstr` and also emits `mul`/`div`; it is useful as a reference for
encoding, not as the lane. Nib/Oct targets (4004/8008) are *not* used here: no
arrays in v1 (VER26 §8.1).

**Deliverables.**

1. `wir-to-riscv`: one WIR instruction -> a fixed RV32I sequence;
   no optimisation, no register allocation cleverness (spill every temp to
   a public stack slot), no `mul`/`div`, ct-select lowered to `and/or/xor`
   masks. Output: a flat RV32I image plus a **source map** from WIR
   instruction id to the address range of its sequence.
2. A symbolic RV32I executor over the `riscv-simulator` decoder, reusing
   `warrant-ir`'s symbolic engine and bit-blaster (VER10/VER19): each
   machine path is a formula.
3. **Per-run translation validation.** For each function, for each WIR path
   `p` with condition `PC_p` and result `r_p`: the machine path under the
   mapped inputs has the same result, and the machine's observable events
   `(pc, memory address, instruction count)` equal the WIR events under the
   source map. Discharged by UNSAT of the miter, DRAT-checked.
4. **ISA-level constant-time proof**: self-composition over the machine
   trace for two runs with equal public inputs, events = pc sequence, load/store
   address+size, and the retired-instruction count. (Because the validated
   trace *is* the WIR trace through the source map, this is the WIR proof
   transported; the independent machine-level miter is run as a cross-check.)
5. A **concrete falsifier**: execute the image on `riscv-simulator`
   (`Rv32IStepTrace` gives `pc_before/after`, `raw`, `state_before/after`) for
   many secret pairs per public input class and compare full traces; the
   branchy controls must differ.

**Evidence artefacts.**

```json
{ "id": "isa/ct_compare/tv/ct_eq",
  "image_sha256": "…", "wir_sha256": "…", "isa": "rv32i",
  "isa_model": "riscv-simulator@<digest>", "paths": 130, "unsat_miters": 130,
  "certificate": "drat:sha256:…", "source_map_sha256": "…", "verdict": "pass" }
{ "id": "isa/ct_compare/trace/ct_eq",
  "pairs": 20000, "trace_equal": 20000, "negative_controls": {"branchy": "differ"},
  "verdict": "pass" }
```

**Assumptions discharged:** `compiler.preserves_timing` and all
refinements *for the owned lane*, `wir.memory.provenance-model` (instantiated),
`os.allocator.size-only` and `os.std.runtime-benign` (no allocator, `core`-only),
`os.counters.not-live` (exact retired-instruction count from the simulator),
`mc.backend.*`. Label: **`proved-to-isa`**.

**Remaining:** `mc.isa.model-faithful`, `mc.isa.timing-data-independent`,
`uarch.*`, `gate.*`, `phys.*`, plus the `wir.*`/`spec.*` assumptions not yet
discharged. Difficulty: XL (new back end + symbolic ISA executor; the solver
and bit-blast machinery already exist).

**Important framing.** The R2 guarantee is about a *different artefact* than the
host build that ships today. The report states which stack each guarantee is
for (VER00 §7.5.6 rule 2): `host-x86_64` stays `proved` + R1 evidence; `owned-rv32i`
gets `proved-to-isa`. Whether a package may retire the CWE-208 exemption on the
owned stack while its production build is the host build is an open question
(§9, Q1).

### 4.4 Rung R3: gate level on a simulated netlist (user item c)

**Chosen candidate: `riscv-gatelevel` (RV32I, spec 07a2).**

| Criterion | `riscv-gatelevel` | Why it wins / loses vs alternatives |
|---|---|---|
| ISA has no variable-latency instructions | RV32I base: none; single-cycle step | ARM1/ARM7 and MIPS R2000 have multipliers/dividers with commonly data-dependent latency (to verify per spec); `intel8008`/`intel4004` are fine but 8/4-bit |
| Shares ISA with R2's lane | Yes: same `riscv-simulator` oracle, same encoder | Switching ISA would re-do R2 |
| Datapath already through gate primitives | Decode, ALU, shifts, comparisons, addresses, branches, CSRs, traps via `logic_gates`/`arithmetic` (`engine.rs`); PC, registers, memory as D flip-flops | Engine is differential-tested against `riscv-simulator` on the Python oracle vectors (`python_differential.rs`) |
| Observable ports easy to expose | `Rv32IStepTrace` has `pc_before/after`; memory accessed through checked `load/store` helpers | A 4004/8008 would need Nib/Oct arrays (absent) |
| Size | ~1,076 lines of engine/state; modest gate count | `riscv-rv64i-gatelevel` is the same design at 64 bits (no benefit) |
| In-order, non-speculative, no caches | yes (in the model) | discharges `uarch.speculation.none`, `uarch.predictor.no-state-leak` structurally |

`intel4004-gatelevel` remains the target of the separate Busicom/Nib pilot
(VER26 §8.1), which proves a *different* program; it is not on the
constant-time ladder because `ct-compare` cannot be expressed in Nib v1.

**What is missing (the real work of R3).**

1. **A netlist, not a Rust evaluation.** `gate.model.is-netlist`. The engine
   calls `and_gate(a, b)` etc.; nothing records the wiring. Options:
   (i) a *tracing* gate backend in `logic-gates` (feature flag) that records
   every primitive application during one step as an HNL cell with symbolic
   input nets, giving the *step netlist* `N : (state, memory-read-data) -> (state', addr, we, wdata)` by
   running the real engine on symbolic bits; or (ii) re-synthesise through
   `hdl-ir`/`synthesis` from an RTL description of RV32I and then equivalence-check
   against the engine. Option (i) keeps the artefact identical to what
   the differential tests already trust, so it is preferred (open question Q2).
2. **A cut at the memory.** `gate.memory.abstraction`: 4.2 M flip-flops
   cannot enter a SAT instance. Cut at the port: the core netlist `N`
   has the address bus, write-enable and write-data as outputs and the read
   data as an input; the memory is a verified-by-inspection byte array.
3. **A bit-level step-function encoding of RV32I** to compare against
   (`gate.isa-equivalence`): the ISA model as a Boolean function. It
   is the same object rung R2 needs (the symbolic RV32I executor), so R3 reuses it.
4. **Structural facts**: `gate.clocking.one-instruction-per-cycle`
   (no multi-cycle state in `N`) and that the halt flag is the only
   state that stops progress.
5. **Observation mapping**: define the gate-level event for each ISA event:
   *branch decision* = `pc' != pc + 4` (a netlist output), *memory
   address* = the address-bus output, *step count* = number of clock steps.

**Two proof strategies, used together.**

- **(A) Compositional, complete (the proof).**
  (A1) one-step refinement: `N` computes the RV32I step function
  (and its `(pc', addr, we)` outputs equal the ISA events) for every state
  and instruction. Miter between `N` and the bit-blasted ISA step,
  UNSAT + DRAT. (A2) the R2 ISA-level constant-time proof of the image.
  (A3) composition: because every instruction takes exactly one cycle and the
  netlist's observable outputs are exactly the ISA events, ISA-level
  trace equality implies gate-level equality of `(pc, addr, we)` per cycle and
  of cycle count. Label: `proved-to-gate-level`.
- **(B) Monolithic bounded cross-check.** Unroll `N` for the machine steps of a
  *small* instance (say `ct_eq` on `n <= 4`: a few hundred steps times a few
  thousand gates), instantiate two copies with equal public inputs and free
  secrets, and assert that the output ports differ at some cycle; UNSAT means no
  distinguishing pair exists at those lengths. Size makes it `bounded(k)` only,
  but it is independent of A1-A3 and catches a composition mistake.

**Evidence artefacts.**

```json
{ "id": "gate/rv32i/step-equivalence",
  "netlist_sha256": "…", "isa_encoding_sha256": "…", "cells": 0,
  "miter_vars": 0, "certificate": "drat:sha256:…", "verdict": "pass" }
{ "id": "gate/rv32i/one-instruction-per-cycle", "method": "structural", "verdict": "pass" }
{ "id": "gate/ct_compare/ct_eq/bmc", "k_bytes": 4, "steps": 0,
  "certificate": "drat:sha256:…", "label": "bounded(4)", "verdict": "pass" }
```

**Assumptions discharged:** `mc.isa.timing-data-independent`,
`gate.model.is-netlist`, `gate.isa-equivalence`,
`gate.clocking.one-instruction-per-cycle`, `gate.memory.abstraction`,
`uarch.speculation.none`, `uarch.predictor.no-state-leak`,
`uarch.shared-resources.none`, `uarch.cache.address-trace-suffices`
(cache-less), `mc.isa.model-faithful` (no longer needed: the netlist is
compared with the *bit-level model*, and the model with the Python oracle).
**Remaining:** `gate.primitives.correct`, `gate.sequential.ideal-dff`, `phys.*`
and whatever `spec.*`/`wir.*` entries are still open.
`EXEMPT-CWE-208-GATE-LEVEL` retires **only if** the label reaches
`proved-to-gate-level` *and* the policy accepts the owned stack (Q1).

### 4.5 Rung R4: transistor primitives (optional floor-lowering)

`gate.primitives.correct`: simulate each primitive used by the netlist
(`and`, `or`, `not`, `xor`, `mux`, full adder) in the `transistors` CMOS
model over all inputs and compare with the truth table. Complete,
cheap, and makes `phys.silicon.realization` + `gate.sequential.ideal-dff`
the only entries left. Those are the declared floor.

### 4.6 How `ca-assurance-report` shows the shrinking set

A claim's `proof` object keeps `label` and `assumptions` (backwards
compatible with today's checker) and adds `stack`, `rung`, `discharged`
and `remaining_by_layer`:

```json
"proof": {
  "label": "proved-to-isa",
  "stack": "owned-rv32i",
  "rung": "R2",
  "certificate": "drat:sha256:…",
  "assumptions": ["wir.lowering.faithful", "mc.isa.model-faithful",
                  "mc.isa.timing-data-independent", "uarch.speculation.none",
                  "gate.isa-equivalence", "gate.primitives.correct",
                  "phys.silicon.realization"],
  "discharged": [
    {"id": "compiler.preserves_timing", "rung": "R2",
     "evidence": ["isa/ct_compare/tv/ct_eq", "isa/ct_compare/trace/ct_eq"]},
    {"id": "memory.unaliased", "rung": "R0+",
     "evidence": ["warrant/ct_compare/proof/eq/alias-split"]}
  ],
  "remaining_by_layer": {"wir": 1, "mc": 2, "uarch": 1, "gate": 2, "phys": 1}
}
```

Progression for the same claim (`ct_eq_soundness`, 4 claims alike):

| Rung | `label` | `assumptions` (ids, abridged) | `remaining_by_layer` | CWE-208 row |
|---|---|---|---|---|
| R0 | `bounded(128)` | every §3 entry whose `applies_to` matches (AC-4); the `os.*` runtime and `gov.*` entries apply to other claims, so far fewer than 84, but well over today's 4 legacy ids | spec 5, src (CT-relevant subset), wir 18, cc 6, mc 4, uarch 5, gate 6, phys 2 | exemption, unapproved |
| R1 | `bounded(128)` + `ir-taint`, `asm-taint` evidence | cc: `cc.caller-context`; mc: `isa.*`; uarch, gate, phys | cc 1, mc 2, uarch 5, gate 6, phys 2 | exemption, floor `proved-to-gate-level` unmet |
| R2 | `proved-to-isa` | `mc.isa.model-faithful`, `mc.isa.timing-*`, uarch, gate, phys | mc 2, uarch 5, gate 6, phys 2 | exemption |
| R3 | `proved-to-gate-level` | `gate.primitives.correct`, `gate.sequential.ideal-dff`, `phys.*` | gate 2, phys 2 | floor met; exemption retirable by policy |
| R4 | `proved-to-gate-level` | `gate.sequential.ideal-dff`, `phys.*` | gate 1, phys 2 | same |

(The abridged counts are illustrative; the checker computes them from the catalogue.)

The CWE-208 weakness-row logic (`matrix.rs`) changes from "the floor is
`proved-to-gate-level`, so exemption" to: "the floor is met iff the claim set
for the *declared deployment stack* reaches label rank `proved-to-gate-level`
and `remaining_by_layer` contains only `gate`/`phys` floor entries".

---

## 5. Machine-readable catalogue

### 5.1 Location and files

```
code/assumptions/catalogue-v1.json     the catalogue (this spec §3 as data)
code/assumptions/evidence-kinds.json   evidence id patterns and what proves what
code/assumptions/non-claims.json       declared non-claims (rule AC-12; added by AS-001a part 2)
code/assumptions/CHANGELOG.d/          one shard per change (lessons.d style)
```

The catalogue is data; §3 of this document is its human rendering. A
spec-sync test (rule AC-14) regenerates the §3 counts and ids from the JSON
and fails when they differ.

### 5.2 Entry schema

```json
{
  "schema": "ver13-assumption-catalogue-v1",
  "assumptions": [
    {
      "id": "memory.unaliased",
      "aliases": [],
      "refines": null,
      "refined_by": [],
      "layer": "wir",
      "family": "wir",
      "title": "Operation buffers are disjoint, in bounds and unmodified during the call",
      "statement": "For every obligation with buffer parameters, the provenance regions of distinct buffers are disjoint and no write to a buffer occurs during the call other than through the operation's own output.",
      "kind": "model",
      "status": "assumed",
      "applies_to": {"labels": ["proved", "bounded(k)"], "claims": ["constant-time", "functional"], "packages": ["rust/ct-compare"]},
      "depends_on": ["wir.memory.provenance-model"],
      "discharge": {
        "evidence_kinds": ["wir-alias-split-proof", "miri-run"],
        "rung": "R0+",
        "tools": ["warrant-ir"],
        "difficulty": "M",
        "backlog": "AS-002"
      },
      "since": "VER26",
      "notes": "Emitted today by ca-assurance-report evidence.rs whenever an obligation has buffers and label != proved."
    },
    {
      "id": "phys.silicon.realization",
      "layer": "phys", "family": "phys", "kind": "physical", "status": "floor",
      "title": "A physical chip behaves as the netlist",
      "statement": "…",
      "discharge": null
    }
  ]
}
```

Fields:

| Field | Type | Meaning |
|---|---|---|
| `id` | string `[A-Za-z0-9._-]+` | Stable, never reused, never renamed (rename = new id + `aliases`). Legacy ids from reports are valid ids or aliases. |
| `aliases` | string[] | Other spellings accepted in reports (`heap-bound:unchecked`). Resolve to this id. |
| `refines` / `refined_by` | id / id[] | Umbrella relation. A parent is discharged iff all children are. |
| `layer` | the eight layer keys, plus `os` and `gov` | Where it is a statement about; orders `proved-to-L`. `os` and `gov` entries have no stack position and are consumed at a layer of their own name (§3.9 counts them as one group). |
| `family` | `spec|src|wir|cc|mc|uarch|gate|phys|tcb|os|gov|att` | Prefix grouping. |
| `kind` | `model|trust|tool|environment|attestation|physical|governance|declared` | `declared` = part of the claim, not dischargeable (e.g. public lengths). |
| `status` | `assumed|attested|evidenced|discharged|floor` | See §2. |
| `statement` | string | Precise meaning, quantifier-explicit. |
| `applies_to` | object | Label kinds, claim kinds, packages (for the unused-assumption rule). |
| `depends_on` | id[] | This assumption holds only if those hold (acyclic). |
| `discharge` | object or `null` | `evidence_kinds`, `rung` (`R0+`, `R1`..`R4`), `tools`, `difficulty`, `backlog`. `null` only for `floor` and `declared`. |
| `discharged_by` | object[] | Present only when `status: discharged`: `[{"evidence": "<id>", "digest": "sha256:…", "stack": "owned-rv32i", "pinned": {…}}]`. |
| `since` | string | Spec/PR that introduced it. |
| `title` | string | One-line name (the first sentence of `statement`, at most about 110 characters). |
| `legacy` | boolean | True for the four ids reports emit today (§3.9). |
| `used_by` | string | The spec §3 "Used by" column, verbatim. |
| `tier` | string | The spec §3 "Ord" column (`O1`..`O6`, `O3/O4`, `P`, `floor`, `gov`, `via-ladder`, or empty for the umbrella). |
| `constrains_input_classes` | string[] | Input classes (`public`, `secret`, `trusted`, `hostile`) the assumption restricts; AC-15 forbids `hostile` here for `environment`-kind and `spec.*` entries. Empty for every entry today. |
| `discharge.plan` | string | The spec §3 "Discharge" column, verbatim. |
| `notes` | string, optional | Free text. |

Implementation clarifications (AS-001a part 1): every entry carries all of
the fields above (the example in this section elides some for brevity);
`status: floor` is used for every entry with `discharge: null`, including the
`kind: declared` ones; `discharge.difficulty` keeps the worst case when §3
gives two (`S (ct-compare) / L (general)` is `L`); and the catalogue-only
half of the checker (`code/scripts/assumption_catalogue.py`, AC-14 plus the
shape halves of AC-3, AC-8, AC-11 and AC-15) landed before the report-side
rules. `code/assumptions/evidence-kinds.json` belongs to the report-side part
and does not exist yet. The JSON is generated from §3 by
`code/scripts/generate_assumption_catalogue.py` and the unit tests fail if the
committed file is not exactly what that script produces.

`evidence-kinds.json` maps each evidence-id pattern to the manifest that
must contain it and the verdict it must carry (reusing VER04h's manifest
resolution: `warrant/...`, `bench/...`, `isa/...`, `gate/...`, `cc/...`).

Implementation clarifications (AS-001a part 2, `ca-assurance-report` 0.2.0).
Where the text above is underspecified or contradicts itself, the stricter
reading was taken; each item says which.

1. **Where the pin lives.** The report's `catalogue` object (section 5.3) pins
   all three data files: `{"schema", "path", "sha256",
   "evidence_kinds_sha256", "non_claims_sha256"}`. `--check` recomputes the
   digests of the files on disk and fails with `CatalogueDrift` (AC-14) on any
   difference. The tool loads `code/assumptions/` from the repository root
   at which it found the package's Warrant document, and that root must be a
   strict ancestor of the package directory, so a package cannot ship its own
   catalogue and be checked against it; nothing is embedded in the binary. CI (`code/scripts/assumption_catalogue.py`, run on every
   PR) also checks that every committed `code/packages/*/*/assurance_report.json`
   pins the current digests, so a catalogue edit cannot leave a report stale
   until the next PR that happens to rebuild that package.
2. **Fail closed on the files.** The tool re-validates the catalogue's
   structure, identity, graph and shape rules (the twin of the Python
   checker, minus spec sync and backlog lookup) and refuses a missing,
   unreadable, non-UTF-8 or malformed file. Size caps are enforced while
   reading: catalogue 1 MiB, `evidence-kinds.json` 256 KiB,
   `non-claims.json` 64 KiB; at most 4096 entries, 256 items per list, 4096
   evidence-id patterns in total and 4096 bytes per string. The Python
   checker applies the same file and count caps and the same whole-string
   patterns (no trailing newline).
3. **Aliases (AC-1).** Section 5.2 says aliases are "accepted in reports";
   section 5.3 says "reports cite catalogue ids only". Stricter reading: an
   alias in a claim's `assumptions`, `discharged`, an exemption's or the
   report's list is `UnknownAssumption` (the message names the id to cite).
   Aliases are resolved only for `unverified::` evidence ids (AC-11, the
   transition period) and, in the generator, for the legacy strings the
   evidence layer emits. An id listed twice, or both under `assumptions` and
   `discharged`, is also `UnknownAssumption`.
4. **What "applies_to matches" means (AC-4).** An entry matches a claim when
   it names the package (or `*`), the claim's label matches one of
   `applies_to.labels` (`bounded(k)` matches every ranked `bounded(N)`), and
   the claim's kinds intersect `applies_to.claims`. A claim's kinds come from
   its evidence: `functional` always, `constant-time` when its operation has a
   constant-time obligation, `bound` when it has any `*-bound` obligation.
   AC-4 says "assumed/attested", but AC-2 and the section 4.6 R0 row require
   floor entries to be listed: every matching entry that is not discharged
   (assumed, attested, evidenced or floor) is required. The closure walks
   `depends_on`, `refines` *and* `refined_by` (listing the umbrella forces its
   refinements), never expands past a discharged entry, and a discharged
   entry it reaches must appear under `discharged`. Consequence: the
   catalogue scopes `memory.unaliased` by label only, so `ct_eq_u64` (no
   buffers) now lists it too; narrowing that is a catalogue edit.
5. **Report-scope assumptions.** Entries whose `applies_to.claims` are
   `weakness-row`, `capability` or `report` belong to no claim, and section
   5.3 has no place for them. The report gains a top-level `assumptions`
   list with the same rule (no label test), plus every attestation a row,
   claim or exemption cites through an `unverified::` alias, even when the
   catalogue scopes that attestation to another package (gf256 cites
   ct-compare-scoped attestations), closed over the catalogue edges.
6. **The proof object.** `stack`, `rung`, `discharged` and
   `remaining_by_layer` (section 4.6) are required, never defaulted. The
   stack of today's claims is `host-rustc`, not `host-x86_64` (section 4.3):
   an R0 claim is about WIR and the host build of any target (CI also runs on
   arm64 macOS). Stacks are `host-rustc` and `owned-rv32i`; anything else is
   `StackMismatch` (AC-7). The R0 rung is `R0`, ranked below `R0+`; a
   discharge's rung must be at least `R0+` and at least the catalogue's
   `discharge.rung`. `remaining_by_layer` counts every listed assumption,
   floor entries included (section 4.6 counts `phys.silicon.realization`), and
   a value that disagrees with the list is `LabelOutranksAssumptions`.
7. **AC-5 and the WIR labels.** Reading `proved`/`bounded(k)` as
   "`proved-to-wir`, nothing above `wir` assumed" contradicts AC-4, which
   makes every R0 claim list `spec.*` and `src.*` entries. So AC-5 applies to
   `proved-per-run` and `proved-to-isa` (`L` = `mc`) and to
   `proved-to-gate-level` (`L` = `gate`). Within those, the stricter reading of
   "above `L`" is used: *any* undischarged non-floor entry at a stack layer
   above `L` fails, including `spec`, `src` and `wir` entries that the
   section 4.3, 4.4 and 4.6 examples leave remaining. `os`/`gov` entries have
   no stack position.
8. **Evidence kinds (AC-3).** `code/assumptions/evidence-kinds.json`
   (schema `code/specs/schemas/ver13-evidence-kinds-v1.schema.json`) has one
   row per kind: `kind`, `manifest` (`test`, `proof`, `bench`, or `planned`
   when no manifest produces the kind yet), `patterns`, `verdict` (`pass`),
   `per_run`, `description`. A pattern is 1..16 `/`-separated segments, each
   a literal or `*` (exactly one segment), with a literal namespace first:
   `warrant` for test/proof kinds, `bench` for bench kinds, anything else for
   planned kinds. Every kind the catalogue names is defined, every defined
   kind is named, and no two patterns can match one id. A discharge's
   evidence id must match exactly one kind, that kind must be one the entry
   accepts, and that kind's manifest must hold the id with verdict `pass`; a
   planned kind can never discharge. A catalogue entry with status
   `discharged` that names the package is checked the same way, and its
   `discharged_by.digest` must be `sha256:` of the manifest file's bytes.
9. **Pinning (AC-9).** Per-run kinds: `ir-taint-analysis`,
   `asm-taint-analysis`, `ir-volatile-cycle-check`,
   `toolchain-tuple-manifest`, `per-run-translation-validation`,
   `host-counter-run`. "The current build tuple" is the tuple the manifest
   entry recorded for the run (`pinned`: `toolchain`, `target`, `flags`,
   `artefact_sha256`). The discharge must carry an identical `pinned` object
   and `scope: "for build <artefact_sha256>"`. No manifest records a tuple
   today, so no per-run discharge can pass until AS-003a writes one.
10. **Umbrellas (AC-6).** A refinement counts as discharged when it is under
    the same claim's `discharged` or discharged in the catalogue for the
    claim's stack. `cc.caller-context` is a floor entry, so
    `compiler.preserves_timing` can never be discharged (as section 4.2 says).
11. **Floor (AC-8)** also covers `kind: physical` (as the catalogue rule
    does) and any entry whose `discharge` is null.
12. **Unused (AC-10)** judges only entries that name the package (or `*`):
    an entry scoped to another package is not this report's concern. It is a
    warning, and an error under the new `--strict` flag. ct-compare uses all
    84 entries; gf256 (no live proof claims) leaves `tcb.report.checker-correct`
    unused.
13. **Attestations (AC-11).** An `unverified::` id that is no catalogue alias
    is `UnknownAssumption`. The owner's status must be exactly `attested`. The
    catalogue loader already refuses an attestation in any other status, so
    the report-side rule is defence in depth.
14. **Non-claims (AC-12)** are data: `code/assumptions/non-claims.json`
    (schema `ver13-non-claims-v1`), rows `{package, cwe, claim, source,
    statement}`. For each row naming the report's package, no proof
    obligation of the non-claimed kind may pass with a ranked label, and the
    CWE row may be neither `discharged` nor `excluded`, and may not cite such a
    proof. The first row is gf256's CWE-208 (MA01, "Constant-time status").
15. **Derived exemptions (AC-13).** An exemption's list is the section 3.9
    selector applied to the union of the claims' listed (undischarged)
    assumptions. The `EXEMPT-CWE-208-GATE-LEVEL` selector is "layer `cc`, `mc`,
    `uarch`, `gate` or `phys`, or `wir.cost.step-model`" (the table's
    "everything below the label's layer"). `memory.unaliased` is a WIR
    assumption of the proof itself, so AS-002 acceptance 4 ("the CWE-208
    exemption's derived list loses `memory.unaliased`") does not apply; the
    table governs. The unsafe-exception row covers every memory-safety class
    the matrix can emit (`125` and `787` as well as `416/415/476/190`).
    `EXEMPT-CWE-400-NO-STEP-PROOF`, and a CWE-208 exemption on a package with
    no live proof claim, record an absence of proof and derive the empty
    list. `EXEMPT-CWE-208-NO-SECRET-OPERANDS` records that the class does not
    apply (no secret operand) and also derives the empty list. Any other
    exemption id has no rule and fails.
16. **Order and integration.** `--check` runs the rules in the order
    AC-14, 1, 11, 2, 7, 8, 6, 3, 9, 4, 5, 12, 13, 10, inside the evidence
    cross-check and before the field-by-field comparison with the derived
    report, so a broken list is named by the rule it breaks. Falsification
    tests: `ca-assurance-report/tests/test_assumptions.rs`.
17. **Build inputs.** The crate reads `code/assumptions/` at run time, not
    through `include_str!`, like `code/specs/*.warrant`; the build-tool
    source-input boundary is unchanged, and the CI pin check (item 1) is the
    guard against a stale pin.

### 5.3 Report side

Reports cite catalogue ids only:

```json
"catalogue": {"schema": "ver13-assumption-catalogue-v1",
              "sha256": "…", "path": "code/assumptions/catalogue-v1.json"},
"claims": [{ "…": "…",
  "proof": { "label": "…", "stack": "…", "assumptions": ["id", "…"],
             "discharged": [{"id": "…", "evidence": ["…"]}] } }],
"exemptions": [{"id": "EXEMPT-CWE-208-GATE-LEVEL", "…": "…",
                "assumptions": ["id", "…"]}]
```

`unverified::<x>` evidence ids are *kept for one transition period* as
aliases of `att.*` entries; once an `att.*` entry reaches `evidenced`, the
`--allow-unverified` flag no longer covers it.

### 5.4 Label vocabulary

Extends VER00 §7.6.5 and the existing checker rank
(`proved`, `bounded(K)`, `proved-to-gate-level`):

| Label | Meaning |
|---|---|
| `proved` / `bounded(k)` | WIR-level, assumptions of every layer below `src` remain (alias of `proved-to-wir`). |
| `proved-per-run` | A per-artefact check discharged assumptions up to `cc`/`mc` for one pinned build (R1). |
| `proved-to-isa` | R2: all assumptions above `mc.isa.*` discharged on the owned stack. |
| `proved-to-gate-level` | R3: all assumptions above `gate.primitives.correct` discharged. |

Rank order for the never-round-up rule: `bounded(k) < proved <
proved-per-run < proved-to-isa < proved-to-gate-level`.

---

## 6. `--check` rules

New rules for `ca-assurance-report --check`; each gets a falsification test
(positive and negative), as with VER04h §9. Rule numbers are `AC-n`.

| Rule | Fails when |
|---|---|
| AC-1 | **Unknown id.** Any id in a claim's `assumptions`, in an exemption's `assumptions`, or in `discharged[].id` is neither an `id` nor an alias in the pinned catalogue (`UnknownAssumption`). |
| AC-2 | **Every proved label lists assumptions.** A claim with label `proved`, `bounded(k)`, `proved-per-run`, `proved-to-isa` or `proved-to-gate-level` has no `assumptions` array, or an empty one. An empty list is never valid: even `proved-to-gate-level` lists the `phys.*` floor entries (`EmptyAssumptions`). |
| AC-3 | **Discharge needs evidence.** An id under `discharged` (or catalogue status `discharged`) with no `evidence` ids, an evidence id absent from every manifest, a verdict other than `pass`, or a `rung` lower than the catalogue's required rung (`UnevidencedDischarge`). Evidence digests must equal the manifest digest. |
| AC-4 | **No silent drop (completeness).** A claim's effective assumption set is the transitive closure over `depends_on` and `refines` of what it lists, plus every `assumed`/`attested` entry whose `applies_to` matches the claim's label and package. Anything in that closure that is neither listed under `assumptions` nor under `discharged` fails (`SilentlyDroppedAssumption`). This is the rule that makes the R0 list longer. |
| AC-5 | **Layer monotonicity.** A label `proved-to-L` may list no `assumed`/`attested` entry at a layer above `L`; a label may not claim a higher rank than its undischarged assumptions allow (`LabelOutranksAssumptions`). |
| AC-6 | **Umbrella consistency.** An umbrella (`compiler.preserves_timing`) is under `discharged` only if every `refined_by` child is discharged, for the same `stack` (`PartialUmbrella`). |
| AC-7 | **Stack honesty.** `discharged[].stack` must equal the claim's `stack`; evidence for `owned-rv32i` cannot discharge a `host-x86_64` claim (`StackMismatch`). |
| AC-8 | **Floor entries cannot be discharged.** `status: floor` or `kind: declared` ids may not appear under `discharged` (`FloorDischarged`). |
| AC-9 | **Pinning.** Evidence of kind `per-run` must carry the pinned tuple (toolchain, target, flags, artefact digest); if the current build tuple differs the discharge is void (`PinMismatch`). The report text says "for build P". |
| AC-10 | **Unused assumption.** A catalogue entry whose `applies_to` matches no claim of the report is listed as `unused` (warning; error under `--strict`), per VER00 §7.5.2 "an assumption no proof uses is flagged for removal". |
| AC-11 | **Attestation retirement.** An `unverified::<x>` id whose `att.<x>` entry has status above `attested` fails (`StaleAttestation`): the checked evidence must be cited instead. |
| AC-12 | **Non-claims are not proofs.** A package whose Warrant file or spec declares a non-claim (e.g. MA01 gf256 constant time) cannot have a `proved` CWE-208 row (`NonClaimReported`). |
| AC-13 | **Exemptions are derived.** Each computed exemption must list exactly the catalogue ids it derives from (§3.9 table) and the set must equal what the claims' remaining assumptions imply (`ExemptionAssumptionMismatch`). |
| AC-14 | **Catalogue integrity.** The report's `catalogue.sha256` equals the file's digest; ids are unique; `depends_on` and `refines` are acyclic and resolve; every `discharge.backlog` names an existing backlog id; the spec §3 counts and ids equal the JSON (`CatalogueDrift`). |
| AC-15 | **Hostile-input rule carried over.** No environment-kind assumption may constrain a `hostile` input (VER00 §7.5.2); a `spec.*`/`environment` entry violating that fails at catalogue load. |

Failure ids go in the VER04h error list; every rule has a sabotage test in
`tests/test_assumptions.rs` (the checker's own corpus with negatives, in the
style of TF01).

---

## 7. First three backlog items

Each item names the assumptions it changes, the evidence produced and the
acceptance criteria. They are chosen to be independently shippable and
ordered; each reduces risk before the next builds on it.

### AS-001a (O1): Land the catalogue and the checker rules

**Scope.** Create `code/assumptions/catalogue-v1.json` from §3; teach
`ca-assurance-report` to load it, pin its digest in the report, and enforce
AC-1, AC-2, AC-3, AC-4, AC-6, AC-8, AC-11, AC-14. Migrate the ct-compare
report: replace `compiler.preserves_timing` etc. by ids, add `catalogue`,
make the nine `unverified::` ids aliases of the `att.*` entries, make the
five exemptions list their assumption ids. Do **not** discharge anything yet.

**Acceptance.**
1. Every assumption string emitted by `evidence.rs`, `matrix.rs`, `lib.rs` and
   present in `rust/ct-compare/assurance_report.json` resolves to a catalogue
   id or alias (a test greps the sources for `"…preserves…"`-style literals
   and for `unverified::`/`analyzer::`/`capability-audit::` ids).
2. `--check` fails (with the named error) on: an unknown id; a proved claim
   with `assumptions: []`; a claim that omits an `assumed` entry whose
   `applies_to` matches (AC-4); a `discharged` entry with no evidence; an
   umbrella discharged while a child is not.
3. The ct-compare report regenerates, still `report-only`, with the longer R0
   list of §4.1 (it lists more than four ids), and `--check` passes.
4. The generated §3 counts equal the JSON (AC-14), and VER00 §7.5.2 rules
   (unused assumption flagged) are exercised by a test.
5. Falsification test for every rule; `cargo test` green; CHANGELOG/README
   updated; backlog row AS-001 marked done with a PR id.

**Effort.** M. **Depends on:** WD-001 (done).

### AS-002 (O2): Discharge `memory.unaliased` and shrink the unsafe exemptions

**Scope.** (a) In `warrant-ir`, prove the constant-time and functional
obligations of `eq`, `eq_fixed`, `select_bytes`, `eq_u64` under **both**
aliasing cases (`a` and `b` the same provenance; disjoint), and prove the
`select_bytes` output is a fresh provenance. If the trace is alias-insensitive
the assumption is removed; otherwise it is refined into a narrower
precondition. (b) Run Miri over `ct-compare`'s tests including every call
to `volatile_barrier`, bound to the source digest (WD-008); record the
result as a manifest entry. (c) Replace the name-matched lowering of
`volatile_barrier::accumulate/read` by a lowering of its body, or add a
validation test relating the name-matched model to the real body
(`wir.lowering.volatile-by-name`).

**Acceptance.**
1. `memory.unaliased` is `discharged` in the catalogue with evidence ids
   `warrant/ct_compare/proof/<op>/alias-split` for each affected op; the
   ct-compare report no longer lists it in `assumptions`.
2. A negative control: a sabotaged `select_bytes` that returns its input
   buffer (not fresh) fails the freshness obligation.
3. Miri evidence manifest exists, is digest-bound, and CWE-416/415/476/190
   exemptions are replaced by `discharged` rows **only if** Miri passes on
   the region and the manifest is cited; otherwise they stay exemptions and
   now cite `src.rust.unsafe-region.sound` (honest partial).
4. The CWE-208 exemption's derived list (AC-13) loses `memory.unaliased`.
5. No label rises above the evidence (never-round-up); `--check` green.

**Effort.** M. **Depends on:** AS-001a.

### AS-003a (O3): Taint-checked optimised IR and assembly (rung R1)

**Scope.** Build `ir-taint` (§4.2) and wire it into the ct-compare evidence
set; retain `tests/optimized_ir.rs` as a cross-check. Split
`compiler.preserves_timing` into its refinements in the catalogue and in the
report; record the toolchain tuple.

**Acceptance.**
1. For the pinned toolchain tuple, the checker reports zero secret-tainted
   branches and zero secret-tainted addresses in the four entry points of
   `ct-compare` in both LLVM IR and emitted x86-64 assembly.
2. Negative controls all fail: branchy `select_bytes`, early-exit `eq`,
   secret-indexed lookup; each in IR and asm form. A control that passes
   is a gate defect.
3. `cc.llvm-ir.no-secret-branch`, `cc.llvm-ir.addresses-public`,
   `cc.volatile-honoured`, `cc.version-pinned`, `mc.backend.*` are
   `discharged` **for the pinned build**; the umbrella remains
   undischarged because of `cc.caller-context` and other toolchains; the
   report says "for build P" (AC-9).
4. A change of `rustc` version, flags or target makes `--check` fail with
   `PinMismatch` until the evidence is regenerated.
5. The report label stays `bounded(128)`/`proved`; the CWE-208 row stays an
   unapproved exemption, but its derived assumption list is shorter by the
   discharged ids.

**Effort.** L. **Depends on:** AS-001a (and benefits from AS-002).

**Parallel, not counted among the three (integrity prerequisites):**
WD-009 (`wir.proof.obligation-binding`), the `unchecked` heap/depth rows
(`wir.bound.*`, one-day each), and `src.analyzer.G-1` for ct-compare
(empty dependency closure as a checked fact). They belong in O1 and should land
before any `proved-per-run` or `proved-to-isa` label is allowed.

**Next after the three:** a spec and crate for `wir-to-riscv` + symbolic RV32I
(R2); a spec for netlist extraction from `riscv-gatelevel` (R3); VER18
(hardware equivalence/translation validation) written to include
`gate.isa-equivalence` as its first obligation.

---

## 8. Relationship to other specs

| Spec | Relationship |
|---|---|
| VER00 §7.5, §7.6.5 | VER13 is the catalogue named there; the label vocabulary is extended (§5.4). |
| VER01 | The report schema gains `catalogue`, `stack`, `rung`, `discharged`, `remaining_by_layer`; claims' `assumptions` become catalogue ids. |
| VER04 | `assume platform:` cites catalogue ids (`spec.platform.declared-none`). |
| VER04h | AC rules extend §9; WD-009 and WD-008 are in this catalogue (`wir.proof.obligation-binding`, `src.rust.unsafe-region.sound`). |
| VER15 | Weakness-registry rows gain per-row assumption ids; CWE-208 floor logic changes as described in §4.6. |
| VER18 (planned) | Owns `gate.isa-equivalence` and netlist miters; VER13 only defines the ladder position and evidence artefact. |
| VER20 | `ObsEvent` extension (`spec.obs-model.complete`, `uarch.fetch.pc-trace`). |
| VER26 §8.1 | The "next pilot" (Busicom/Nib) is a separate owned-stack pilot; the constant-time ladder uses RV32I. |

---

## 9. Open questions

1. **Which stack does a guarantee describe?** `ct-compare` ships as a host
   Rust library. A `proved-to-gate-level` label for the *owned RV32I build* is
   about a different artefact. Does the CWE-208 floor retire on that, or only
   for a package whose deployment stack is the owned one? (Proposed: label per
   `stack`, and the exemption retires only for the declared deployment stack.)
2. **Netlist source.** Trace the existing engine into HNL (preferred: same
   artefact the differential tests trust) or re-synthesise from RTL via
   `hdl-ir`/`synthesis` and equivalence-check against the engine?
3. **Is an ISA with a multiplier needed?** Real CT code often needs `mul`
   (e.g. GF multiplication). RV32M's latency model would have to be proven
   data-independent per implementation; do we want a second ladder for that
   or restrict constant-time kernels to RV32I?
4. **Granularity of observations.** Cache-line vs byte address (some
   attacker models observe lines only). The stricter byte model is proposed;
   is a coarser model ever acceptable?
5. **Catalogue governance.** Who may mark an entry `discharged`? Proposed:
   only the checker, from evidence; humans may only add/modify `assumed`
   entries and need review. Does adding an assumption need the exemption
   approval flow of VER00 §9.1?
6. **Where does the catalogue live?** `code/assumptions/` (proposed) vs
   `code/specs/`; and whether VER13 should absorb the per-language
   catalogue of VER00 §7.5.2 (the `lang.*` family is reserved and empty
   today).
7. **Gate-level scale.** Is a bounded BMC of `ct_eq` at `n <= 4` on the
   step netlist worth its cost, given strategy (A) is complete? (Proposed: yes
   once, as a one-time independent cross-check.)
