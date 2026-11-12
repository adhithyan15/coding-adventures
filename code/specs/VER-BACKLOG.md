# VER delivery backlog

Implementation queue for VER00 and the Rust VER26 pilot. Reconcile fetched
`origin/main` and open PRs before selecting each item. Initial audit: main
`462710b53fc`, bootstrap PR #18807 at `4e608afe3bc9`. Local tests, an open PR
and a verified merge are different completion states.

For portable Warrant, [portable-warrant.md](VER-BACKLOG.d/portable-warrant.md)
is the current status, receipt, and discovery record. The coordinator rows and
dated snapshots below are historical overview; use the shard for current
implementation state.

## Delivery and prioritization

One bounded change per PR, spec committed before tests/code, meaningful local
validation and a security sub-agent review before pushing. Investigate actual
current-head CI failures and conflicts; enable squash auto-merge only after
terminal acceptable checks. Verify the expected head and merge on fetched main.
Prepare dependent work locally while CI waits; do not duplicate an existing PR.
After every merge, log discoveries and prioritize safety/correctness, missing
validation, critical-path prerequisites, then independent extensions.

Other-language CT01 adoption belongs to the package-parity backlog. Publishing
and approval implementation stays BLOCKED by VER00 section 12.1. The report-only
pilot does not authorize a package release.

## Landed foundations

| Item | Evidence on main | Boundary |
|---|---|---|
| VER09 CDCL SAT | #18783 | Boolean solving, not source verification |
| VER11 DRAT kernel/logging | #18787 | Propositional certificates; theory proof terms remain |
| VER10 AIG/Tseitin/bit-vectors | #18808 carried #18804; #18804 merged | First slice; EUF, general LIA and arrays remain |
| LIA false-UNSAT repair | #18808 | Incomplete searches return Unknown; not a complete decision procedure |
| D21 rand/thread/ui taxonomy | #18781 | Taxonomy/selected enforcers, not full effect proofs |
| Pilot quirks log | #18786 | Record of rough edges, not proof evidence |

## Prioritized pilot queue

`ready-local` permits preparation, not a claim that dependencies are delivered.
Each selected group is split into bounded PRs if its implementation requires it.

| Priority / ID | Work | State / prerequisite and acceptance |
|---|---|---|
| done / VER-P003 | TF01 Rust L0 bootstrap | #18807 merged as 6a09869c279; expected head 4e608afe3bc9 verified on fetched main; all checks acceptable |
| done / VER-P004 | Rust CT01 corpus through L0 | #18849 merged as 0bee67f3a81; expected head ca72cad1191 verified on fetched main; all 44 checks acceptable |
| P2 / VER-CI001 | Retain diagnostics for intermittent macOS iPad UI failure | original job125811351687 failed; bounded same-head retry125843044183 and run41602352046 attempt2 passed; root cause and missing UI-state artifacts remain unresolved; no assertions weakened |
| done / VER-I004 | Enforce TF01's closed failure vocabulary at the Rust harness boundary | #18875 merged as 089fb8e7bf7 at expected d2b5ae4db2c; verified on fetched main; all 39 checks acceptable |
| done / VER-P005 | Warrant v0 specification | #18890 merged as 655b99e5e27 at expected 5a60550f9d1; verified on fetched main; all 37 checks acceptable; no implemented compiler claimed |
| done / VER-P006a | Canonical Warrant grammars, compiled Rust grammar data and syntax corpus | #18898 merged as ce9ab9e57d48 at expected 4271246dbe7; verified on fetched main; all 40 checks acceptable; protected frontend remains P006b |
| done / VER-P006b1 | Bounded source admission and CRLF/original-byte mapping | #18913 merged as 964dc3df7bc; verified on fetched main; all checks acceptable; no public parser |
| done / VER-P006b2 | Exact bounded significant-layout capture | #18992 merged as dc360bfb789; verified on fetched main; all checks acceptable; synthetic tokens zero-width |
| done / VER-P006b3 | Guarded Warrant source lexer/parser | #18996 merged as 2f1e8361a38; [VER04c](VER04c-guarded-warrant-parser.md) specified; bounded recursive descent over layout tokens, node quotas, recursion depth guards (depth <= 64), 90%+ line/branch coverage |
| done / VER-P007 | Warrant type checker/Rust bindings | #18997 merged as f471a7195f3; [VER04d](VER04d-warrant-type-system.md) specified; scope/symbol resolution, acyclic models, static typing, contract verification and closed Rust binding schema; 90%+ line/branch cov |
| done / VER-P008 | Rust backend and ct-compare.warrant | #19002 merged as 652207f348d; [VER04e](VER04e-warrant-codegen.md) specified; bounded pure model evaluator, TFV test vector compiler, Rust L0 harness generator, checked regeneration **AUDIT (VER-P030): generated L0 harness never contained claims that execute the model; the checked-in generated artefact was only `fixed_widths.rs`.** |
| done / VER-P009 | Property spec and pbt-v1 corpus | #19004 merged as 5b1fcef8b2e at expected c6f4c29e61c; verified on fetched main; all checks acceptable |
| done / VER-P010 | Rust property engine | #19007 merged as f66f3a2e057; [VER05](VER05-property-engine.md) generator algebra, integrated shrinking, deterministic replay, pbt-v1 corpus (25 cases), zero third-party deps, 92%+ line/branch cov **AUDIT (VER-P030): the engine is real; no ct-compare property ran on it until VER-P030 (the only ct-compare PBT test compared `a == b` with `a == b`).** |
| done / VER-P011 | Warrant claims to properties | #19011 merged as 315447685ec; [VER04f](VER04f-claims-to-properties.md) specified; compiled universal claims to native Level 2 PBT suites, public API calls and broken implementations detected, 93%+ line/branch cov **AUDIT (VER-P030): the emitted claim harness hard-coded `a == b` as the oracle, was only string-matched (never compiled or run against the crate), supported at most 2 binders, and `ct-compare.warrant` held one claim for four operations; "public API calls" was not delivered. Replaced by VER-P030.** |
| P1 / VER-P012 | Stateful properties | waiting P011; select only if pilot needs them, otherwise explicitly defer |
| done / VER-P013 | Fuzz spec and fuzz-v1 corpus | #19021 merged as 97618fd2c1f; [VER06](VER06-coverage-guided-fuzz-engine.md) specified; canonical fuzz-v1 corpus (25 cases in cases.json/cases.tfv), standard-library Python reference validator, private failure-output policy, 98% line cov |
| done / VER-P014 | Rust fuzz engine | #19022 merged as cbc5befeaf9; [VER06](VER06-coverage-guided-fuzz-engine.md) mutator algebra, 65,536-bit coverage bitmap, power scheduling, FNV-1a crash deduplication, minimization, fuzz-v1 corpus test (25 cases), zero third-party deps, 95%+ line/region cov |
| done / VER-P015 | ct-compare fuzz targets | #19023 merged as e623e7bb5f9; [VER06a](VER06a-ct-compare-fuzz-targets.md) specified; ct_eq, ct_eq_fixed, ct_select_bytes, ct_eq_u64 fuzz suites, negative sabotage detection and minimization, privacy-preserving TAP reporting, 100% coverage, zero third-party deps **AUDIT (VER-P030): suites are real but hand-written (300-500 iterations), not derived from Warrant; "100% coverage" refers to the harness, not the iteration depth.** |
| done / VER-P016 | Microbenchmark spec/harness | #19026 merged as 7168cda2272; [VER07](VER07-microbenchmarks.md) specified; warmup/calibration, robust statistics (median/MAD/bootstrap CI), deterministic cost counters (steps/allocs/instructions), budgets, zero third-party deps, 93%+ cov |
| done / VER-P017 | Equal-instruction-count measurements | #19027 merged as 36c2252e98a; [VER07a](VER07a-ct-compare-microbenchmarks.md) specified; n=16/256/4096 ct_eq, ct_eq_fixed, ct_select_bytes, ct_eq_u64, zero-alloc assertions, equal instruction counts, sabotage detection **AUDIT (VER-P030): zero-alloc/step-equality results are real tests but were not recorded anywhere the report could read; hardware retired-instruction counts are only Live on some hosts.** |
| done / VER-P021 | WIR core/reference interpreter | [VER25](VER25-wir-core-interpreter.md) specified; WIR core AST, static type checker, cost semantics, provenance-tracking memory model, observable trace recorder, bounded reference interpreter, constant-time validation; 91%+ line/branch cov |
| done / VER-P022 | Rust-subset grammar | #19014 merged as 3d8a14c46d8; [VER25a](VER25a-rust-subset-grammar.md) specified; real ct-compare source, volatile/unsafe assumptions, zero third-party deps, 93%+ line/branch cov |
| done / VER-P023 | Rust-to-WIR front end | #19017 merged as b486bf3b28e; [VER25b](VER25b-rust-to-wir-frontend.md) specified; lowered parsed Rust AST to WIR modules, differential validation against ct-compare, zero third-party deps, 92%+ line/branch cov |
| done / VER-P024 | Symbolic execution | #19019 merged as b56a4b1d0fe; [VER19](VER19-symbolic-execution.md) specified; bit-precise proofs via bit-blaster and cdcl-sat, memory safety bounds, honest bounded(k) labels **AUDIT (VER-P030): symbolic executor does not execute `CallFunction`, so public exports that call helpers (`ct_eq`, `ct_eq_fixed`) could not be proved directly.** |
| done / VER-P025 | Bounds/constant-time trace proofs | #19020 merged as e2a4c386892; [VER20](VER20-resource-timing-proofs.md) specified; self-composition 2-safety hyperproperties, timing/address divergence detection, DRAT-certified UNSAT, >90% line/branch cov **AUDIT (VER-P030): `ct_eq_u64` proof is complete but its DRAT is the trivial `x and not-x` refutation (miter collapsed structurally); `ct_eq` was proved only via the internal equal-length helper on 4-byte buffers and `ct_select_bytes` on 4-byte buffers; the only `BoundProver` test ran on a synthetic 3-step function. The `bounded(4096)` / `proved` labels in the report were not backed by these runs.** |
| done / VER-P027a | Rust capability analyzer/name allowlist | #19028 merged as cbd8b93f0e7; [VER08a](VER08a-rust-capability-analyzer.md) specified; zero third-party deps, 92%+ cov, fail-closed audit |
| done / VER-P027b | Generated effect module/certificate | #19030 merged as a1a722f32c5; [VER23a](VER23a-rust-effect-modules.md) specified; VER23/24, small checker, empty effects, ct-compare certified |
| done / VER-P028 | Weakness registry/report-only assurance | #19031 merged as f53d4b45daa; [VER15](VER15-weakness-registry.md), [VER01](VER01-assurance-report.md) specified; weakness registry v1 (26 CWE classes), per-package weakness matrix, computed unapproved CWE-208 exemption, not-live rungs, ct-compare report-only assurance **AUDIT (VER-P030): `generate_ct_compare_report` hard-coded case counts (10000), fuzz iterations (100000), certificates (`drat:sha256:e2a4c386892...`) and every `PackageAuditContext` flag; `--check` validated shape only. Evidence IDs matched no real test. Superseded by manifest-derived reports.** |
| done / VER-P029 | TypeScript bootstrap harness & test runner | #19033 merged as 9c40051eff8; [TF01a](TF01a-typescript-test-bootstrap.md) specified; [TF00](TF00-portable-test-framework.md), [TF01](TF01-bootstrap-harness.md), VER00 §7.1; zero npm dependencies, test-bootstrap-v1 self-corpus (1,256 cases), pure deep-equality, SplitMix64 PRNG, TAP v14 reporting, node:test integration, vitest/jest retirement pilot |
| P0 / VER-P030 | Warrant-derived evidence for ct-compare | [VER04h](VER04h-warrant-derived-evidence.md) specified. Phase 1: Warrant claims generate compiled+executed example, bounded-exhaustive, property and fuzz suites with sabotage controls (`ct-compare/tests/generated/warrant_evidence.rs`) and a regeneration check in BUILD. Phase 2: `labels`/`bound` drive 2-safety and step-bound obligations over the real lowered source via a WIR inliner, DRAT files content-addressed with SHA-256. Phase 3: `generate_ct_compare_report` reads the manifests; `--check` fails closed on unknown evidence IDs, count/label drift, bad certificate hashes, uncovered Warrant claims. Acceptance: no literal evidence number in report code; each failure mode has a falsification test. **Status: implemented in local commits (phases 1-3), not yet merged.** Remaining: heap/depth bounds unchecked; all certificates are trivial miter-collapsed; 9 static CWE attestations have no producing tool (`unverified::`); E2 not-live; CWE-416/415/476/190 need Miri or equivalent; select_bytes mismatch shapes are counted not proved |
| done / VER-P030 | CLI Builder integration for test-bootstrap | #19035 merged as c5a32473432; [TF01b](TF01b-typescript-cli-runner.md) specified; declarative CliSpec using internal cli-builder engine, multi-target runner, file discovery, seed/sabotage/filter/reporter/bail flags, closed exit codes, zero third-party npm deps |
| done / VER-P031 | TypeScript Level 2 property-based testing engine | #19036 merged as b7f4bebaea5; [VER05a](VER05a-typescript-pbt-engine.md) specified; VER05 generator algebra, integrated shrinking, deterministic SplitMix64 replay, pbt-v1 conformance corpus (25 cases), fast-check retirement, zero third-party deps |
| done / VER-P032 | TypeScript Level 2 coverage-guided fuzzing engine | #19037 merged as 5bba76c771c; [VER06b](VER06b-typescript-fuzz-engine.md) specified; VER06 mutator algebra, coverage tracking, power scheduling, crash deduplication, fuzz-v1 corpus test (25 cases), zero third-party deps |
| done / VER-P033 | TypeScript Level 1 Warrant compiler & claim verifier | #19038 merged as 323822e0bbd; [VER04g](VER04g-typescript-warrant-compiler.md) specified; Warrant frontend, claim verification, AST evaluation, test generation, ct-compare.warrant validation, 57 syntax cases, zero third-party deps |
| done / VER-P034 | TypeScript Level 3 CDCL SAT solver & Tseitin bit-blaster | #19039 merged as 9b242cc2591; [VER09a](VER09a-typescript-sat-solver.md) specified; Boolean satisfiability, 2-watched literals, VSIDS heuristics, conflict-driven clause learning, incremental assumptions, Tseitin bit-blaster, DIMACS CNF, zero third-party deps |
| done / VER-P035 | Standalone TypeScript CDCL SAT Solver package (`@coding-adventures/sat-solver`) | #19041 merged as ebc9f01ec2d; Package extraction under `code/packages/typescript/sat-solver`; standalone zero-runtime-dep package, CDCL engine, Tseitin bit-blaster, DIMACS transport, 22/22 test suite, locked compiler, 94%+ cov |
| done / VER-P036 | Standalone TypeScript Warrant Compiler package (`@coding-adventures/warrant-compiler`) | #19042 merged as 894254ef863; Package extraction under `code/packages/typescript/warrant-compiler`; standalone zero-runtime-dep Warrant compiler, claims evaluator, test vector generator, 57 syntax cases, ct-compare validation, 97%+ cov |
| done / VER-P037 | Standalone TypeScript Property-Based Testing package (`@coding-adventures/pbt`) | #19043 merged as c46f0836e60; Package extraction under `code/packages/typescript/pbt`; standalone zero-runtime-dep PBT engine, generator algebra, shrinking, 47 tests, 98%+ cov |
| done / VER-P038 | Standalone TypeScript Coverage-Guided Fuzzing package (`@coding-adventures/fuzz`) | #19044 merged as d89423032e1; Package extraction under `code/packages/typescript/fuzz`; standalone zero-runtime-dep fuzz engine, mutator algebra, 65536-bit bitmap coverage, 40 tests, 97%+ cov |
| done / VER-P039 | Standalone TypeScript Test Matchers package (`@coding-adventures/test-matchers`) | Package extraction under `code/packages/typescript/test-matchers`; standalone zero-runtime-dep assertions and matchers, 27 tests, 99%+ cov |
| in-progress / VER-P040 | TypeScript Level 4 Bounded Model Checker & Invariant Verifier | BMC engine using Tseitin bit-blaster and sat-solver for transition system unrolling |

Critical path: P003 -> P004 -> P005-011 -> P021-025. Fuzzing and measurements
can progress independently once their prerequisites are available.

## Later families and explicit deferrals

| Families | Work and prerequisite |
|---|---|
| VER02 | Repository-wide test-tier enforcement after L0 evidence |
| VER03, VER17 | Simulated host/fault plans/real chaos; capability-bearing second pilot |
| VER08, VER22-24 | Other language analyzers, locked box, generated modules and information flow |
| VER10, VER11 | EUF/LIA/arrays and theory proof terms; preserve Unknown on incomplete search |
| VER12 | General verification-condition generation after Warrant/WIR settles |
| VER13, VER14 | Assumption catalogues, subset conformance and other WIR front ends |
| VER16 | Native attack harness, checked allocator/interpreter and private triage |
| VER18 | Hardware equivalence/translation validation/k-induction; Busicom/Nib pilot |
| VER21, publishing VER01 | BLOCKED: resolve VER00 section 12.1 trust-root/device/build-hook findings |
| VER26 follow-on | Busicom-to-gates, a capability-bearing pilot, then other languages |

## Discoveries and current plan

Post-P029 prioritization: VER-P029 merged cleanly in #19033 (commit 9c40051eff8) with all CI matrix checks passing. It delivered the zero-dependency TypeScript L0 bootstrap harness (`test-bootstrap`), exact deep equality, SplitMix64 PRNG, TFV v1 parser, closed failure vocabulary, matchers API, and pilot migration of `bitset` (retiring Vitest and 94 transitive packages).
Prioritize VER-P030 (CLI Builder integration for `test-bootstrap` runner): Connect `@coding-adventures/cli-builder` to power `test-bootstrap`'s CLI runner with declarative argument parsing, help formatting, test file discovery, and fail-closed exit codes. Next step: VER-P031 (TypeScript Level 2 Property-Based Testing Engine).

P009 merged in #19004 (commit 5b1fcef8b2e) with all matrix checks passing on main.
The formal property engine specification ([VER05](VER05-property-engine.md)),
canonical `pbt-v1` conformance corpus (25 cases in `cases.json` and `cases.tfv`),
standard-library Python reference validator (`code/scripts/pbt_conformance.py`),
and 40 falsification tests passed with 97% line and 93% branch coverage.
An Ubuntu 26 runner migration issue on `forme-release-platform` was identified
and resolved by runner pinning to `ubuntu-24.04`.

Post-P009 prioritization: #19004 merged cleanly at expected head. VER-P010
(Rust property engine implementation) implemented the VER05 Level 2 property-based
testing engine directly within `code/packages/rust/test-bootstrap` (`pbt.rs`), extending the
L0 harness with the full generator algebra (`Gen<T>`), integrated shrinking preserving
type refinements and invariants, deterministic `SplitMix64` PRNG replay with isolated
sub-seeds, step/shrink budgets, private failure reporting policies (§7.3), and end-to-end
execution of the `pbt-v1` conformance corpus (25 tests) with zero third-party dependencies and >=92% line and region coverage.

Post-P010 prioritization: Prioritize VER-P011 (Warrant claims to properties). VER-P011 connects
Warrant claim evaluation with the PBT engine, allowing Warrant contracts and claims to be executed
as generative property tests across arbitrary input spaces with automatic counterexample shrinking.

Post-P011 prioritization: VER-P011 delivers Warrant claim compilation ([VER04f](VER04f-claims-to-properties.md))
to native Level 2 property tests in `warrant-grammar` and `test-bootstrap::pbt`. Universal claims compile
to deterministic test suites with isolated PRNG sub-seeds, integrated multi-component shrinking, panic containment,
fault-injected sabotage detection for broken implementations, and privacy-preserving failure reporting in CI mode.
Next on the critical path is VER-P021 (WIR core and reference interpreter) to establish the semantic intermediate
representation for verified shipping.

Post-P021 prioritization: VER-P021 delivers the Warrant Intermediate Representation (WIR) core and deterministic reference
interpreter ([VER25](VER25-wir-core-interpreter.md)) in `coding_adventures_warrant_ir`. It provides the exact formal intermediate
model for proofs with mandatory types, security taint/information-flow lattice (`Public`, `Secret`, `Hostile`), operational cost
accounting, provenance-tracking memory safety with explicit `Stuck` undefined-behavior detection, and observable event trace
recording for constant-time self-composition (CWE-208).

Post-P022 prioritization: VER-P022 merged cleanly in #19014 (commit 3d8a14c46d8) with all CI matrix checks passing.
It delivers the Confined Rust Subset grammar, AST, and recursive-descent parser ([VER25a](VER25a-rust-subset-grammar.md)),
parsing real `ct-compare/src/lib.rs` (372 lines) with zero third-party dependencies, hardened resource quotas (1 MiB source cap,
32 recursion depth limit, 65,536 token cap), and 93.7% line / 91.3% region coverage.

Post-P023 prioritization: VER-P023 merged cleanly in #19017 (commit b486bf3b28e) with all CI matrix checks passing.
It delivers the Rust-to-WIR lowering front end ([VER25b](VER25b-rust-to-wir-frontend.md)), compiling Confined Rust AST into
WIR modules with linearized control flow, compiler barrier inlining, dynamic slice length queries, and differential validation
against native `ct-compare` entry points with 97.1% line / 92.9% region coverage and zero third-party dependencies.

Post-P025 prioritization: VER-P025 merged cleanly in #19020 (commit e2a4c386892) with all CI matrix checks passing.
It delivers WIR resource-bound and constant-time trace proofs ([VER20](VER20-resource-timing-proofs.md)), proving memory safety,
absence of resource exhaustion (CWE-400), and constant-time execution (CWE-208) via self-composition and DRAT-certified UNSAT.

Prioritize VER-P013 (Fuzz specification [VER06](VER06-coverage-guided-fuzz-engine.md) and canonical `fuzz-v1` conformance corpus).
VER-P013 formalizes the Level 2 coverage-guided fuzz engine architecture, SplitMix64 deterministic PRNG mutator algebra,
corpus management with 65,536-bit coverage bitmaps, crash deduplication by normalized stack signatures, and the private failure disclosure
policy (VER00 §7.3). It delivers the canonical `fuzz-v1` conformance corpus (25 cases in `cases.json` and `cases.tfv`), the standard-library
Python reference validator (`code/scripts/fuzz_conformance.py`), and 29 falsification tests with 98% line coverage.
Next step: VER-P014 (Rust fuzz engine implementation in `test-bootstrap`).

Post-P016 prioritization: VER-P016 merged cleanly in #19026 (commit 7168cda2272) with all CI matrix checks passing.
It delivers the Level 5 Microbenchmark Harness & Deterministic Cost Counters ([VER07](VER07-microbenchmarks.md))
in `test-bootstrap::bench`, including warmup/calibration, robust statistics (median, MAD, bootstrap CI),
deterministic cost counters (StepCounter, AllocCounter, HardwarePerfCounter), and CI-safe deterministic budgeting.

Post-P028 prioritization: VER-P028 merged cleanly in #19031 (commit f53d4b45daa) with all CI matrix checks passing.
It delivers the Weakness Registry v1 (26 canonical CWE classes across 6 primary security families, [VER15](VER15-weakness-registry.md)),
the machine-readable assurance report data model and fail-closed gate checker (`ca-assurance-report`, [VER01](VER01-assurance-report.md)),
and pilot integration for `rust/ct-compare` with non-live rungs and computed unapproved CWE-208 exemptions, completing the entire
VER26 pilot ladder on `rust/ct-compare`.

Prioritize VER-P029 (TypeScript Bootstrap Harness & Zero-Dependency Test Runner, [TF01a](TF01a-typescript-test-bootstrap.md)).
Per VER00 §7.1 and TF00 §Sequencing, third-party testing dependencies (`vitest`, `@vitest/coverage-v8`, and their ~94 transitive
dependencies per package) account for >90% of repository security alerts and update churn across ~450 TypeScript packages.
VER-P029 delivers the reference TypeScript lane for Level 0 and Level 1 testing (`@coding-adventures/test-bootstrap`):
zero npm runtime dependencies, strict TFV v1 parser, pure deep equality (§3.1), SplitMix64 PRNG and FNV-1a 64 test-id hashing,
full validation of the `test-bootstrap-v1` self-corpus (1,256 cases), TAP v14 reporting, failure-detecting sabotage mode,
drop-in matchers (`describe`, `it`, `expect`), and pilot conversion of a pure TypeScript package (`typescript/bitset`) to retire Vitest.


P006b1 merged in #18913 (commit 964dc3df7bc) with all checks passing on main.
The source admission contract is active in `rust/warrant-grammar`. Eleven source
tests assert CRLF and Unicode coordinates, raw scalar ranges, invalid encoding
rejection, and budget limits.

P006b2 specification ([VER04b](VER04b-warrant-layout-capture.md)) defines the
significant-layout tokenization contract directly over `SourceDocument`. Synthetic
tokens (`INDENT`, `DEDENT`, trailing `NEWLINE`) carry exact zero-width origin boundaries
in normalized source coordinates that map faithfully to zero-width raw byte spans.
No fictitious positive-width spans are assigned to indentation whitespace or transitions.

Post-P006a prioritization: #18898's expected head 4271246dbe7 merged as
ce9ab9e57d48 and is reachable on fetched main. Eight live open PRs had no
overlapping Warrant/source/range item. Log the source/layout gaps before
selecting the next slice: the actual shared range API accepts a standard NAME
grammar but returns typed Unsupported("indentation/layout synthesis") for
valid Warrant indentation source. This is its documented CV04a boundary,
not a defect or completed frontend in the P006a artifacts.

Split P006b into source admission/normalization (b1), significant-layout origin
capture (b2), then the protected parser (b3). Select the source foundation before
type checking/backends/property engines; b1 is confined to warrant-grammar and
can preserve the original source-domain relation without changing shared lexer
consumers. No lexer range may be inferred from transformed Token.value lengths.
Synthetic INDENT/DEDENT/NEWLINE need a separate origin contract, not fictitious
positive-width raw lexemes. CRLF normalization must retain a distinct buffer
and original-byte mapping; no relabelled transformed offsets.

CV04a landed as #17803. CV04b/compiler ingestion and F12 snapshot/relex behavior
belong to separate Closure work; F12 phase1 still awaits its owner's decision.
Refresh ownership before b2 and audit every affected consumer, including
committed spreadsheet-wasm/bundle regeneration if shared lexer/parser inputs
change. Do not resume the Closure loop or claim provenance/Miri completion.
Publishing/approval and package-parity ownership boundaries remain unchanged.

P006a local proof: both dynamic and compiled grammars agree on the authored
syntax corpus, including exact precedence witnesses. Bool/Int mismatches,
duplicate names and vacuous refinements deliberately parse; later checkers
must reject them. Actual L0 sabotage exits1 with exactly three registered
corpus-mismatch failures. The regeneration CLI rejects an added blank line in
a generated Rust module, then passes after exact restoration. An always-valid
Python validator mutant is detected by assertion failures, not loader errors.
LLVM measured 880/880 executable artifact-constructor source lines; Python
coverage measured 127/127 validator statements. These figures cover data
construction/transport, not a protected public parser or semantic verification.
Full Go tests and vet passed, including new raw-grammar/corpus/reference input
selection and lookalike/missing-consumer controls. Publication and hosted CI
are still separate evidence states.

Post-P005 prioritization: expected head 5a60550f9d1 merged as 655b99e5e27,
reachable on fetched main d81b3c5a977. No competing Warrant or parser-stack PR
was found among seven open PRs. The local probe established generated grammar
embedding compilation and small positive/negative syntax behavior. It did not
establish full source limits or stack safety: grammar rule depth differs from
semantic AST nesting, and one 2,000-parenthesis refusal at rule cap64 on a
2-MiB worker covers only one shape. Log that gap before selecting P006a, then
deliver grammar/corpus artifacts before P006b's guarded public source API.
The canonical grammar also needs explicit grouping for nested match arms and
compound-expression operands; VER04 now clarifies those rules before code.

Post-I004 prioritization: #18875's expected head d2b5ae4db2c merged as
089fb8e7bf7 and is reachable on fetched main. No overlapping open Warrant/TF01
PR was found. Select P005, then P006 grammar/corpus, before backend/property
implementation. The Warrant draft now makes two parent constraints explicit:
CT01 secrets split contents/choice from public lengths/widths, and L1 must
emit input/bound checks rather than merely reserve their keywords. Missing
analyzers, observers, solver or proof layers block the corresponding evidence;
sample tests and logical input-shape counts cannot fill those rows.

GitHub rejected explicit squash auto-merge for the already-clean P004 and I004
PRs. Their actual expected-head merges were observed externally and verified;
this agent performed no direct merge. The direct-squash fallback authorization
question remains pending, and heartbeats are not an answer. Keep that workflow
limitation separate from language semantics and publishing/approval blockers.

Post-P004 prioritization: expected head ca72cad1191 merged as 0bee67f3a81,
reachable on fetched main. No overlapping open TF01/bootstrap/Warrant PR was
found in the live open-PR audit. Select I004's demonstrated validation gap
before Warrant's new design work. Replay only the six private I004 commits onto
main so the next PR does not duplicate P004. Preserve the CI diagnostic and
local checkpoint-ref discoveries below as follow-ups, not new feature scope.

- Local tooling follow-up: a general fetch failed on an unrelated broken
  `refs/codex/turn-diffs/checkpoints/...` object. A main-only fetch with an
  explicit origin/main negotiation tip succeeded; live main matched the fetched
  ref. Preserve those checkpoint refs and diagnose their owner separately;
  no repository-wide integrity repair is claimed. This does not outrank I004.

P004 CI triage: macOS SwiftUI step failed TrestleUiTests.swift:132 after typing
and Add, with exit65 and one real failed test. The same UI test passed on
bootstrap head4e608afe3bc9; its test/layout/SwiftUI emitter/bindings/task-core are
unchanged, and task-mosaic-app's diff adds only cfg(test) cases. This supports
one bounded diagnostic retry, not a claim that the root cause is fixed. GitHub
rejects a job rerun while sibling jobs keep the run active; wait without
cancelling them, refresh the expected head, then retry only the failed job.
No SwiftUI UI state/xcresult artifacts were retained. If the failure repeats,
capture the actual input values/view hierarchy and retained diagnostics before
choosing a behavioral fix. The unresolved diagnostic gap stays recorded even
if the retry passes; do not turn it into an unrelated TaskApp feature tranche.

The bounded retry did pass: the actual Trestle UI test passed at
2026-11-03T10:38:20Z (111.675 seconds); the SwiftUI step, Flutter iOS builds,
affected package tests and the full workflow attempt2 completed successfully.
This is native execution evidence, not a root-cause repair. CI001 is no longer
a P004 merge blocker, but its diagnostic follow-up remains open.

I004 was prepared locally while #18849 waited for CI and is now selected;
it has no PR and is not delivered. Its spec preserves the string-taking
Verdict API and validates at serialization, so direct construction cannot
bypass the guard. Publication still requires final-source validation and
security review after the completed merge, overlap audit and prioritization.

Local preparation evidence for I004: the new runtime tests first reproduced
three failures on the old runner. With the guard, 31 Rust harness unit tests,
85 self-corpus records, the CT01 18 unit + 26 L0 + 6 IR checks, and Clippy pass.
The independent Python harness suite passes 25 tests and the CT01 suites 16.
Actual sabotage runs retain 18 registered harness failures and three registered
CT01 failures, with nonzero exits. Instrumented tests pass; the direct LLVM
report is 92.53% overall harness source-line coverage (100% in verdict.rs).
An early read-only security review passed. This remains unpublished local
evidence and must be reconciled/validated on the final publication head.

The I004 regeneration path also exposed a Windows byte-transport gap: the
bootstrap generator's default text writer could translate LF to CRLF while its
text check normalized that difference away. I004 writes LF explicitly and pins
the actual generated bytes, without weakening the strict TFV parser.

- During pending CI for P004, its two negative-check failure paths were found
  to invent adapter-specific IDs outside TF01's closed vocabulary. Correct them
  to corpus-mismatch and validate the actual failing TAP with sabotage mode.
  Follow-up I004 hardens the bootstrap API/output boundary so this class cannot
  recur silently. Prioritize that concrete validation gap before selecting
  Warrant's next design tranche after P004 merges.

- VER00's original "nothing implemented" banner is stale; distinguish landed
  foundations from full-program completion.
- CT01 expected_work is logical public input shape, not measured instructions
  or a constant-time proof. Preserve the separate optimized equality CFG gate;
  full trace proofs, selection and addresses remain VER20 work.
- #18807 merged at its expected head. Reprioritization keeps P004 first: no new
  safety blocker was found, and its completed local evidence unlocks Warrant.
  Merging fetched main into P004 required retaining three reviewed additive
  hunks (CT01 CI commands, Go input predicate and its tests) against empty sides
  from the bootstrap squash; unrelated main changes were retained.
- #18808 records a pre-existing LOW: LIA search budget is not scaled by constraint
  size. Reproduce before choosing a repair; do not weaken soundness or bundle
  speculative performance work into corpus adoption.
- Windows coverage reporting can exceed the command-line limit when
  cargo-llvm-cov expands exclusions for the whole workspace. Direct llvm-cov
  reporting over the same instrumented artifacts works; a reusable wrapper or
  upstream response-file fix is lower priority than the pilot's critical path.

- A dependent-branch replay printed partial-clone fetch errors but completed;
  the requested blob's local bytes hashed to the expected SHA. Diagnose repeat
  occurrences separately; preserve committed work and validate the replay.
- CI filters PRs to main. #18849 initially targeted #18807's branch as a draft
  without hosted CI; GitHub retargeted it to main when P003 merged. Reconcile
  main, check the exact diff/head, mark ready and require hosted CI before
  auto-merge. No dependent change was merged into the bootstrap feature branch.
- Low-priority tooling follow-up: investigate a Go cached test invocation that
  remained active after package PASS output. A fresh `go test -count=1
  -timeout=2m ./...` on the final branch terminated successfully in about 31 s
  for the main package, with all internal packages passing. Disabling test
  caching is a local validation workaround; the root cause is not established.

P004 local evidence: all 21 CT01 records and five negative adapter checks pass;
18 existing unit tests and six optimized-IR tests pass; the CT01 Python suites
pass 16 tests; Clippy and Ruff pass; focused Go repository-input tests pass.
Direct LLVM reporting over the instrumented run covers 145/145 ct-compare source
lines. The full Go suite passed on the initial integration tree and on the
focused dependent branch with `-count=1 -timeout=2m`. Hosted CI remains separate.
Bootstrap and P004 passed exact-head security reviews and hosted CI, and their
expected-head merges have been verified on fetched main.

P004 plan: specify deterministic CT01-to-TFV projection and fixed-width dispatch;
falsify stale transport/work/results; execute all real public operations through
L0; observe length-error panics and mutate fresh selection output; check logical
work fields; register corpus/reference build inputs and regeneration on both
platforms; update package/pilot/quirk docs; run Rust/Python/Go checks and security
review; publish a dependent draft, then retarget/mark ready after P003's merge is
verified (now done). Reprioritize again after P004's actual merge before selecting
P005. The local Warrant notes are preparation, not an accepted spec or code.

## Layered validation roadmap (VER27)

[VER27](VER27-layered-validation-roadmap.md) records the owner's vision for
Warrant (one `.warrant` file, evidence in all eighteen lanes, explicit
assumptions discharged down or up the stack, self-hosting with an independent
oracle, adoption before features, every language version) and maps the
programmes below onto a ten-layer ladder from gates to system integration.
Its section 8 is the recommended order across them; its section 10 lists
conflicts with rows here (notably AS-004's CPU target and the WX slices that
landed before any adopter). The rows below are the new work it proposes; every
other id it orders already exists in this file.

| Priority / ID | Work | State / acceptance |
|---|---|---|
| P2 / LV-001 | VER13 slice: per-lane language assumptions (`src.<lane>.semantics-as-specified`, `cc.<lane>.implementation-conforms`, `cc.<lane>.version-pinned`) and widening the `cc` layer text to cover interpreters and VMs (VER27 §5.2). Rust umbrella first; each other lane's entries land with its first manifest (pin) or first proof label (semantics) | proposed; regenerates the ct-compare and gf256 reports when the Rust umbrella lands |
| P1 / LV-002 | CI re-selects every adopter when its `.warrant`, a pinned fixture, `warrant-grammar`, `warrant-ir` or `ca-assurance-report` changes (build-tool consumer rule) | proposed; can ship inside the first leaf conversion's enablement PR |
| P3 / LV-003 | Per-lane assumption-conformance tests (VER00 §7.5.3) written as small Warrant files and run through the VER04i corpus, moving `src.<lane>.<construct>` entries to `evidenced` | proposed; after the lane's WD-1xx runner exists |
| P3 / LV-004 | Write VER14 (per-lane WIR front ends over LANG84 versioned grammars, with the LP-001a differential method as the template) | proposed; after LP-001a has run once on gf256 |
| P1 / LV-005 | Layer contract linking (owner, 2026-11-12; VER27 §4.13, §4.14). A package's proved Warrant claims are published as catalogue-style ids (for example `contract.logic-gates.and`); an upper layer's `assume platform:` names them; the assurance report counts such an assumption as discharged only if the lower package's evidence actually proves that claim at the pinned revision, else it stays open. Needs its own specification before any code; the spec also settles VER27 conflict C14 (VER04 §3 and the closed VER13 family set). First Warrant feature with a concrete adopter (LV-006), so it complies with Q7 | proposed; spec first; adopter is LV-006 |
| P2 / LV-006 | The Intel 4004 chain, a second independent stack and bottom-up adoption pilot (owner, 2026-11-12; VER27 §4.15), complementing the RV32I-first route of AS-004. Layers, in order: gates (`logic-gates` on `transistors`; truth tables), combinational (mux, decoder), arithmetic (`arithmetic`; 4-bit result and carry = `(a+b+c) mod 16`, 512 cases), sequential (flip-flops, registers), CPU step (`intel4004-gatelevel`; each instruction against the datasheet ISA model), programs (Nib to 4004, Busicom). Each layer proves its own contract and assumes the ones below by id (LV-005); contracts cover time where a constant-time claim needs it. One `.warrant` ISA model tests the Rust, C, C++, Go, Python, Elixir, Lua and Perl 4004 simulators. `bits.rs` in `intel4004-gatelevel` is outside the gate contract and needs its own claim or removal (no reaching around). A `U4` or fixed-width bits type is requested only if the chain needs it (P7). Proves nothing about real silicon | proposed; after the LV-005 spec; needs the CPU-SIMULATOR-ROADMAP gate-integrity audits and a `logic-gates` Warrant file (SH thread) |
| P1 / LV-007 | Publish gate (owner, 2026-11-12; VER27 §8.1). Today `.github/workflows/publish.yml` checks only the tag format and that the commit is on `main`. Before the first release, build a gate there that requires the package's Warrant spec, a generated assurance report that meets the bar, and live runtime-jail evidence. Nothing has been published; the owner holds all publishing until packages are proven | proposed; after CAP-RT-002 and CAP-RT-003 give live jail evidence; before the first release |

## Polyglot Warrant-derived evidence (coordinator queue)

Goal: a Warrant spec is the single source that is mechanically translated into
example, bounded-exhaustive, property, fuzz and proof evidence, in every
language lane the repo supports, and every assurance report is computed from
the evidence those runs actually wrote. Started 2026-11-09 after an audit found
`rust/ct-compare`'s report was a hand-written literal (see WD-001).

Architecture (three layers, so every lane gets real Warrant-derived tests early):

1. Portable layer (all lanes): the canonical Rust Warrant compiler expands a
   spec into a language-neutral corpus (examples, bounded-exhaustive cases,
   seeded biased property cases, negative controls). Each lane executes it
   through its own L0 runner against its own implementation, runs the sabotage
   mode, and writes an evidence manifest. Modelled on the der-tlv-v1
   cases.json/consumers.json pattern (TF00 "Portable architecture").
2. Native layer: per-lane Warrant backends emit native L2 property and fuzz
   suites over that lane's engine (gated by the pbt-v1 / fuzz-v1 corpora).
3. Proof layer: per-lane WIR front ends (VER14); labelled not-live until real.

Lanes, in order: rust, python, typescript, go, ruby, c, cpp, java, kotlin,
csharp, fsharp, elixir (have ct-compare today); then lua, perl, haskell,
swift, dart, ocaml (need a CT01 ct-compare port first).

| Priority / ID | Work | State / acceptance |
|---|---|---|
| done / WD-001 | Rust ct-compare: Warrant -> executed example/exhaustive/PBT/fuzz suites, Warrant-driven CT and bound proofs, evidence-derived report with fail-closed `--check`; VER04h spec (VER-P030) | merged #19054 (`2ff2db8cb02`), verified on main; 9 claims, 71 generated tests, exhaustive up to 240,818 cases/op, proofs bounded(128) except ct_eq_u64 proved; certificates are trivial miter refutations (recorded honestly) |
| done / WD-002 | Spec VER04i: polyglot evidence corpus format, consumers registry, per-lane manifest schema, report generalization | merged #19078; corrected contract is tracked in VER04i |
| P1 / WD-003 | Rust Warrant compiler emits the language-neutral corpus for ct-compare Warrant v0; independent stdlib-only Python semantic/digest validator; byte-stable regeneration and falsification checks | PR #19177: WD-003 plus the #19209 Warrant WX-D1 S1 main integration; after the merge, local corpus regeneration/oracle, routing, and Rust package checks pass; hosted checks for the integrated head are pending. Explicitly unsharded. [Current queue, receipts, and discoveries](VER-BACKLOG.d/portable-warrant.md) |
| P1 / WD-908 | Route Linux Lua prerequisites through bounded apt install retries after repeated CI DNS failures | CI runs #65010 and #65018 failed the Lua prerequisite step resolving Ubuntu archive hosts; both Linux Lua setup paths bypassed the existing retrying `code/scripts/ci/apt-install.sh`. PR #19177 attempt 5 passed at its prior exact head; merged-head checks remain pending. Retain `libreadline-dev`/`libncurses-dev`; regression checks both paths and timeouts. See [portable Warrant receipts](VER-BACKLOG.d/portable-warrant.md) |
| P1 / WD-004 | Generalize ca-assurance-report beyond `rust/` (per-lane manifests, honest not-live rungs) | after WD-003: WD-004a defines strict lane-manifest schema and consumer registry; WD-004b integrates report generalization; then Python L0 runner. See [portable Warrant queue](VER-BACKLOG.d/portable-warrant.md) |
| P2 / WD-1xx | One PR per lane: L0 corpus runner + sabotage + manifest + report for ct-compare (python, typescript, go, ruby, c, cpp, java, kotlin, csharp, fsharp, elixir) | after WD-003 and WD-004a/004b; Python runner follows report/manifest contract. See [portable Warrant queue](VER-BACKLOG.d/portable-warrant.md) |
| P3 / WD-2xx | Native Warrant backends emitting L2 PBT/fuzz suites per lane (TypeScript first: engines exist) | waiting WD-1xx per lane |
| P3 / WD-3xx | CT01 ct-compare ports for lua, perl, haskell, swift, dart, ocaml, then their WD-1xx lanes | independent |
| P4 / WD-4xx | Proof lanes via WIR front ends (VER14), C first | waiting VER14 spec |
| P2 / WD-901 | Deduplicate TypeScript engines: test-bootstrap/src copies vs standalone sat-solver/pbt/fuzz/warrant-compiler/test-matchers (already diverged) | independent |
| watch / WD-902 | Main full build red: repository-source-input-boundary missing six TS packages; fixed by open #19050 (not ours) | watch only |
| P2 / WD-903 | PR CI does not run repo-wide contract tests when a new package directory appears | independent |
| P2 / WD-904 | VER-P040 (TS bounded model checker) marked in-progress with no branch or PR anywhere; confirm owner or reset to ready | independent |
| P2 / WD-905 | TypeScript Vitest retirement stalled: only `bitset` uses test-bootstrap; 436/482 packages still depend on vitest | continue TF01 conversion in batches after WD-901 |
| P3 / WD-906 | `typescript/bitset` imports `../../test-bootstrap/dist/index.js` by relative path; dependency undeclared in package.json (only BUILD wires it) | fold into WD-905 conversion pattern |
| P2 / WD-907 | VER02 repository-wide test-tier enforcement has no spec; it is the repo-wide validation gate | spec after WD-004 |

### Leaf-library conversion program (user direction 2026-11-09)

"Leaf" means a real library package with no internal dependencies, like
`ct-compare`. 280 Rust crates have zero `[dependencies]`; many exist in 17-18
languages (e.g. gf256, wasm-leb128, sha1, md5, lzss, caesar/vigenere ciphers,
trie, heap, bitset). Each is converted one at a time:

1. Careful spec (CT01-style) + Warrant spec as the single source of truth.
2. Warrant generates every test kind (WD-001 pipeline); exhaustive where the
   domain is enumerable; proofs where WIR can lower the code.
3. Evidence-derived assurance report; release-ready verdict.
4. Port the same Warrant-derived evidence to every language lane (WD-1xx).

"Release-ready" here means the evidence gate passes. Actual publishing stays
BLOCKED by VER00 section 12.1 (approval clients and the maintainer's computer
in the trust base, malicious build hooks) until the owner resolves it.

| Priority / ID | Work | State / acceptance |
|---|---|---|
| done / LP-000 | Survey + rank pure leaf libraries (scratchpad leaf-survey.md; to be committed as code/specs/VER-LEAF-SURVEY.md) | done 2026-11-09, read-only; timings are estimates |
| P0 / SEC-001 | lz77/lz78/lzss/lzw (+ huffman-compression, brotli, huffman-tree) untrusted-header allocation/loop/OOB fixes across 18 lanes | local, branch worktree-agent-ada735047ffb1aece (19 commits); real bugs in rust/go (OOM), c/cpp (OOB read), fsharp (4e9 loop), swift (traps), quadratic decode in several; C#, F#, Dart, Swift, Haskell, Elixir, Lua not run locally; queued for PR after WD-001 |
| P1 / LP-001 | gf256: field laws, all 65,536 pairs (2^24 triples) exhaustive, cross-language table fixture | after WD-001 template |
| P1 / LP-002 | x25519: RFC 7748 known answers, DH commutativity, secret labels; CT proof needs VER20 lowering | after LP-001 |
| P1 / LP-003 | lzw, then lzss, lz77, lz78: round trip + hostile decompress, exhaustive to length 3; needs an error channel (API change across ports) and `max_output` bound | after SEC-001 |
| P1 / LP-004 | sha512, blake2b, sha1, md5 shared template (known answers, block-boundary sweep, bounds); thin without extension E | after LP-001 |
| P2 / LP-005 | des; then Tier 2 (wasm-leb128, der-tlv, protobuf varint, zeroize, bitset) as Warrant extensions land | after WX items |
| P2 / LP-006 | Add missing `required_capabilities.json` (sha512 found without one); audit all leaves | independent, small (haiku) |
| P2 / LP-007 | Negative/malformed-input tests absent in lzss, lz77, lz78, sha1, md5, x25519, range-coder, caesar/atbash, irc-framing | folded into each LP item |

Warrant extensions, cheapest unlock-per-cost first (from survey; active ordering and status are maintained in the [portable Warrant queue](VER-BACKLOG.d/portable-warrant.md)): WX-B fixed-width
ints U16/U32/I64; WX-C bitwise ops; WX-A tuples/records/non-recursive sums;
WX-E external oracle / vector-table construct + per-language fixture generator;
WX-F binding-level sequence drivers for stateful objects; WX-D bounded
fold/count/map; WX-G in-out postconditions; WX-H `max_output` bounds;
WX-I recursive sums; WX-J strings/floats/maps/stateful models (Tier 3).

### Missing Warrant v1 implementation features (user direction 2026-11-11)

VER04k already specifies the next language features; this queue records
implementation work against those existing WX IDs rather than opening duplicate
specs. WX-B/C/A/E/H are implemented in `warrant-grammar`; WX-D/F/G remain
spec-only. Keep the corpus profile honest: Warrant v1-only constructs are not
accepted by WD-003's Warrant v0 corpus until their compiler and evaluator work
lands.

| Priority / ID | Work and affected evidence | Dependency and acceptance |
|---|---|---|
| P1 / WX-D1 | Implement VER04k §2.1 sequence constructors `take`, `drop`, `slice`, `replicate`, `range`, `to_list`, `to_bytes`; unlocks a faithful `gf256` multiply model, general `wasm-leb128` model, and sequence pre/postconditions for streaming hashes and writers | WX-H/WX-E merged #19121; alongside WX-F/G baseline work; compiler/checker/evaluator/corpus update with boundary and negative tests per VER04k §2.15 |
| P1 / WX-F | Implement VER04k §3 binding-level constructor/step/finish drivers; enables real streaming SHA-family, range-coder and rANS sequences | alongside WX-D1 and WX-G; split-point sweeps that use `slice` follow WX-D1; each driver is binding-validated, runs subject operations across every sequence element, and kills missing/reordered/duplicated-step controls |
| P1 / WX-G | Implement VER04k §4 `inout` and `ensures`; enables zeroize post-state evidence and output-buffer writers such as sqlite-file varint and TLS ALPN | alongside WX-D1 and WX-F; sequence postconditions that use list/byte built-ins follow WX-D1; checker rejects inadmissible/hidden post-state, generator binds mutable adapter slots, and both true and false postconditions are falsified |
| P2 / WX-D2 | Implement VER04k §2.2 first-order bounded `fold`, `map`, `count`, `all`, `any`; enables reusable list/byte reference models for hashes, codecs and bitset | after WX-D1, WX-F and WX-G; validate typing, model purity, and evaluator budget exhaustion as specified |
| P2 / WX-D3 | Implement VER04k §2.3 bounded symbolic unrolling for sequence forms; enables selected WIR proofs over loop-based `gf256` and codec implementations | after WX-D2 and the corresponding WIR lowering; proof rows remain `unchecked` until source lowering and certificates are independently verified |

### Bottom-up hardening and self-hosting (user direction 2026-11-09)

The three "LH" audits below cover the bootstrap tooling itself (the test
harness, proof checker and capability analyzer), which must be READY before
Warrant and test-bootstrap can be trusted to test themselves and the leaves.

Order: harden leaf packages first (careful spec, exhaustive tests, honest
READY / READY-WITH-CAVEATS / NOT-READY verdict), then climb the dependency
graph. Once the bootstrap is ready, the stack tests itself: each package gets a
Warrant spec, and its tests are generated by Warrant and executed on
test-bootstrap.

Rust dependency graph (verification stack): leaves `drat-check`,
`test-bootstrap`, `ca-capability-analyzer`, `ct-compare`; then `cdcl-sat`
(drat-check) -> `bit-blaster` (cdcl-sat, drat-check) -> `warrant-grammar`
(test-bootstrap + shared grammar-tools/lexer/parser) and `warrant-ir`
(bit-blaster, cdcl-sat, drat-check, ct-compare, test-bootstrap) ->
`ca-assurance-report`.

Trust anchor for self-hosting: a harness cannot be the only judge of itself.
Self-generated suites for test-bootstrap must also be cross-checked by the
independent standard-library Python reference validators (tfv.py,
bootstrap_harness_conformance.py, pbt_/fuzz_conformance.py), and the L0 core
stays small enough to review by hand.

| Priority / ID | Work | State / acceptance |
|---|---|---|
| P0 / LH-001 | Leaf audit `drat-check` (VER11): soundness spec, bounded-exhaustive SAT/UNSAT proofs, mutants, coverage | in progress (sonnet, worktree) |
| P0 / LH-002 | Leaf audit `ca-capability-analyzer` (VER08a/23a) | local, branch worktree-agent-a274a2c279c1b0d70 (5 commits); READY-WITH-CAVEATS; 141 tests (was 45), 348+7 evasion cases with no misses, 20/20 mutants killed, 96.6% lines measured; fixed brace-group `use std::{fs, process}` evasion, unscanned capabilities.rs, nested target/ skip, unknown-std-is-pure, ABI list, include!/#[path]/proc-macro now fail closed, `--emit-cert` writing `proved` for violators, truncated ct-compare cert signature; excepted unsafe now visible (`SUMMARY unsafe_code=present-excepted(2)`). Queued for PR |
| P1 / LH-002a | Assurance report must read the analyzer's SUMMARY line: memory-safety CWE exclusions require `unsafe_code=absent`; no injection/FFI exclusions when `unanalyzed_dependencies>0` or `ffi=present` | after LH-002 + WD-001 merge |
| P2 / LH-002b | Analyzer caveats G-1..G-9 (deps assumed pure, tests/benches not analysed, macros unexpanded, cfg ignored, cert digest omits Cargo.toml/build.rs); runtime CAP-RT covers part | backlog |
| P2 / OPS-001 | Agents must use unique scratch directories: one agent's mutation run was clobbered by another in the shared scratchpad | apply in every brief |
| P0 / LH-003 | Leaf audit `test-bootstrap` (TF01, VER05-07): false-pass tests, TFV cross-check with Python oracle, mutants | in progress (sonnet, worktree) |
| P0 / LH-004 | Leaf audit `ct-compare` (CT01) | after WD-001 lands (WD-001 rewrites its tests) |
| P1 / LH-005 | `cdcl-sat` (VER09) audit | after LH-001 |
| P1 / LH-006 | `bit-blaster` (VER10) audit | after LH-005 |
| P1 / LH-007 | `warrant-grammar` (VER04a-g) and `warrant-ir` (VER25, 19, 20) audits | after LH-003/006 |
| P1 / LH-008 | TypeScript leaves: test-bootstrap + standalone engines (after WD-901 dedupe) | after WD-901 |
| P1 / SH-001 | Self-hosting gap analysis: can Warrant v0 express each leaf's contracts (needs sequences/lists, records, error results, parser-shaped ops?) -> Warrant language extensions spec | after LH-001..003 verdicts |
| P1 / SH-002 | Warrant specs for leaves (`ct-compare` done by WD-001; then prng, tfv, verdict, drat-check, sha256) with generated suites run on test-bootstrap + Python oracle cross-check | after SH-001 |
| P2 / SH-003 | Self-host up the graph: cdcl-sat, bit-blaster, warrant-grammar (Warrant compiler tests itself), warrant-ir, ca-assurance-report | after SH-002 |

### Assumption reduction, programs to logic gates (user direction 2026-11-09)

Every proof label carries assumptions (today: `compiler.preserves_timing`,
`memory.unaliased`, and the unapproved CWE-208 gate-level exemption). The long
arc removes them one at a time, from source down to a simulated netlist, so a
claim such as "constant time" is eventually proved on gates, not assumed below
WIR. Work starts at the leaves and climbs.

| Priority / ID | Work | State / acceptance |
|---|---|---|
| P2 / AS-001 | Assumption catalogue (VER13): every assumption used by any report, its meaning, what would discharge it, which packages depend on it | after WD-001 |
| P2 / AS-002 | Discharge `memory.unaliased` for ct-compare via WIR provenance proof or Miri evidence on the unsafe region | after AS-001 |
| P3 / AS-003 | `compiler.preserves_timing`: translation validation of the optimized IR / machine code against WIR for ct-compare (existing optimized_ir tests are a start) | after AS-001 |
| P3 / AS-004 | Gate-level constant time: compile ct-compare's kernel to RV32I and prove trace equivalence on a simulated netlist of `riscv-gatelevel` (VER13 §4.4, VER18); retire EXEMPT-CWE-208-GATE-LEVEL for the stack proved. RV32I on `riscv-gatelevel` is the first route (owner, 2026-11-12); the Busicom/Intel 4004 track is a second, independent stack (LV-006, VER27 §4.15) | long-term |
| P3 / AS-005 | Repeat AS-002..004 per leaf as each converts | long-term |

### Audit findings (2026-11-09) on the delivered implementation

Each finding is tracked to the item that closes it. A finding is closed only
when the fix is merged and verified on fetched main.

| ID | Finding (verified by reading code and running `cargo test`) | Closed by |
|---|---|---|
| AF-01 | `ca-assurance-report::generate_ct_compare_report` builds the whole ct-compare report from literals; no number is measured | WD-001 phase 3 |
| AF-02 | Report claims 10,000 property cases per op; no property test ever calls ct-compare | WD-001 phase 1 |
| AF-03 | `warrant-grammar/tests/codegen.rs::test_claim_execution_and_sabotage_detection` compares `a == b` with `a == b`; it never calls `ct_eq` | WD-001 phase 1 |
| AF-04 | `compile_rust_l0_harness` output for ct-compare is only string-matched (`suite.add(...)`), never compiled or executed | WD-001 phase 1 |
| AF-05 | Bytes-pair generators draw 0..32 random bytes, so equal pairs almost never occur; the equal branch is barely exercised | WD-001 phase 1 (biased generators) |
| AF-06 | Report claims 100,000 fuzz iterations per op; the campaigns run 300-500, and are hand-written rather than Warrant-derived | WD-001 phase 1 |
| AF-07 | No bounded-exhaustive unit tests over any small domain | WD-001 phase 1 |
| AF-08 | `ct_eq` is "proved" only via internal `ct_eq_equal_length` on 4-byte buffers; length-mismatch path unproved | WD-001 phase 2 |
| AF-09 | `ct_select_bytes` proof runs on 4-byte buffers while the report says `bounded(4096)` | WD-001 phase 2 |
| AF-10 | Certificate `drat:sha256:e2a4c386892...` is PR #19020's commit hash plus literal `...`, pasted into all four claims | WD-001 phase 2/3 |
| AF-11 | CWE-400 "discharged" by a linear step bound; the only `BoundProver` test targets a synthetic 3-step function | WD-001 phase 2 |
| AF-12 | Evidence IDs (e.g. `bench_instruction_counts::equal-retired-instructions`, `warrant-ir::linear-step-bound`) match no real test | WD-001 phase 3 |
| AF-13 | `--check` validates shape/consistency only; never cross-checks evidence or certificate files | WD-001 phase 3 |
| AF-14 | `--check` hard-requires a `rust/` package prefix, blocking other lanes | WD-004 |
| AF-15 | `ct-compare.warrant` has one claim (`eq_agrees`); `eq_fixed`, `select_bytes`, `eq_u64` have no claims | WD-001 phase 0 |
| AF-16 | VER-BACKLOG entries P010-P028 restate the inflated evidence; Discoveries stop at P029; P021 and P039 lack merge evidence (#19013, #19045) | WD-001 phase 0 |
| AF-17 | TS engines exist twice (test-bootstrap/src vs standalone packages) and already differ by 60-560 lines each | WD-901 |
| AF-18 | Six new TS packages never added to repository-source-input-boundary.json; main full build red | WD-902 (#19050) |
| AF-19 | PR CI diff detection skipped the repo-wide contract test that would have caught AF-18 | WD-903 |
| AF-20 | VER-P040 recorded as in-progress without any branch or PR | WD-904 |
| AF-21 | Vitest retirement covers 1 of ~450 packages; bitset's test-bootstrap dependency is undeclared | WD-905, WD-906 |
| AF-22 | ct-compare has two `unsafe` blocks (volatile barrier) declared as a `banned_construct_exceptions` entry, yet the report excludes CWE-416/787/125/415/476/190 because "analyzer proved no unsafe blocks" | WD-001 phase 3 + LH-002 |
| AF-23 | Capability conformance is only checked statically; no runtime evidence that a package uses nothing beyond `required_capabilities.json` | CAP-RT |
| AF-24 | lz77 (and by reading, lz78/lzss) preallocate from untrusted headers: a 4-byte input aborts the process with a 16 GiB allocation | SEC-001 |
| AF-25 | lz* `decompress` has no error channel, so malformed input cannot be rejected or specified | LP-003 |

### Runtime capability conformance (user direction 2026-11-09)

| Priority / ID | Work | State / acceptance |
|---|---|---|
| P0 / CAP-RT-001 | Spec VER08b + new crate capability-runtime-conformance; ct-compare pilot | local, branch worktree-agent-a9f1811c0377a9acf (6 commits); READY-WITH-CAVEATS: 6-syscall armed baseline, 18/18 negative controls caught by both rings, Docker ring live; coverage 72.7% (sandboxed child cannot flush profiles); vDSO time reads invisible; CI runner strace/Docker unverified |
| P1 / CAP-RT-002 | Feed runtime evidence into the evidence-derived assurance report (new rung; `--check` requires it on Linux) | after WD-001 + CAP-RT-001 (mandatory: owner 2026-11-12, VER27 §6.1) |
| P1 / CAP-RT-003 | Make CAP-RT a required step of every leaf-library conversion (LP-0xx) | after CAP-RT-001 (mandatory: owner 2026-11-12, VER27 §6.1) |
| P2 / CAP-RT-1xx | Per-language lanes: arm the filter after interpreter/VM startup (Python, TypeScript/Node, Go, JVM, .NET, ...) | after CAP-RT-001 |

### Follow-ups logged 2026-11-09 from agent reports

| Priority / ID | Work |
|---|---|
| P1 / SEC-002 | Decompression-bomb surface remains: declared lengths up to 4 GiB satisfiable by real tokens; LZ78 chains quadratic; LZW ~2700x amplification. Needs `max_output`/error channel (LP-003, WX-H) |
| P2 / SEC-003 | huffman-compression: crafted code-length table overflows `code <<= len - prev_len` |
| P2 / SEC-004 | elixir lz77 decode O(n^2) list appends |
| P1 / SEC-005 | Run the C#, F#, Dart, Swift, Haskell, Elixir, Lua lz* fixes in CI or a local toolchain; perl Test2::V0 suites; ruby standardrb |
| P1 / CAP-RT-004 | CI wiring: set RCG_STRICT=1 and RCG_REQUIRE_DOCKER=1 on Linux runners once strace/Docker availability is confirmed |
| P2 / CAP-RT-005 | Coverage of sandboxed child code (flush profile before arming or via pre-opened fd); crate at 72.7% < 80% |
| P2 / CAP-RT-006 | vDSO-backed time reads are invisible to both rings; document as residual or detect via vDSO unmapping |
| P2 / WD-005 | Non-trivial 2-safety miters (current DRAT certificates are the fixed x and not-x refutation); prover support for traps as path outcomes; heap/depth bound proofs |
| P2 / WD-006 | Single binding table: op->export map duplicated in gen_evidence.rs and warrant_proofs.rs |
| P2 / WD-007 | Retire or derive remaining hand-written ct-compare tests (conformance, fuzz_targets, optimized_ir) |
| P2 / WD-008 | Miri evidence for CWE-416/415/476/190; tools behind the nine `unverified::` attestations |
| P2 / OPS-002 | `cargo build --workspace` cannot run in this sandbox (skia-bindings download 403); rely on CI for workspace builds |

### Coordinator round 2 status (2026-11-09)

| Priority / ID | Work | State |
|---|---|---|
| done / WD-001 |  | Merged as #19054 (2ff2db8cb02), verified on main |
| in PR / SEC-001 |  | #19059 |

### Status deltas after 2026-11-09 coordinator round 1 (append to VER-BACKLOG polyglot section)

| Priority / ID | Work | State |
|---|---|---|
| done-local / LH-003 | test-bootstrap audit: branch worktree-agent-aca74c66a7307fa91 (7 commits). READY-WITH-CAVEATS overall (tfv/verdict/prng READY; run/pbt/fuzz/bench with caveats). 7 false-pass bugs fixed in pbt/fuzz/bench (zero-case property passed; trusted shrinker; fuzz dropped Timeout/Oom; unchecked hard bench budgets; dropped Verdict; crash dedup/minimization), i128 range bias, hangs, TAP injection. 22/22 mutants killed; 95.7% lines measured; Rust/Python TFV differential over 364,413 documents | queued for PR (conflicts likely with WD-001's test-bootstrap 0.3.0 changes; rebase after WD-001 merges) |
| P1 / LH-003a | ct-compare zero-allocation evidence is vacuous: AllocCounter is cooperative and the closures never record; replace with a counting `#[global_allocator]` and re-derive CWE-400 evidence | done-local: counting allocator in test-bootstrap 0.5.0; ct-compare re-derived (ct_eq/ct_eq_fixed/ct_eq_u64: 0 allocations; ct_select_bytes: 1 allocation of a.len() bytes, removed from the zero-alloc claim) |
| P1 / LH-003b | Add `gen_bytes_pair(min,max,p_equal,p_near)` (VER05 §7.2); independent 16-byte draws never collide | after LH-003 |
| P2 / LH-003c | L0 supervisor must check `1..N` against results (process::exit(0) truncation); VER05 CI digest is 64-bit FNV not SHA-256; gen_nat/gen_ascii unimplemented; VER06 engine uses a subset of the mutator algebra, 32-bit signature with unescaped ':' joins | backlog |
| P1 / WD-009 | Bind proof certificates to obligations by recomputing CNF/lowered-function hash in `--check` (security review B4 follow-up) | after WD-001 |
| done-local / LH-001 | drat-check audit: branch worktree-agent-adde9212bf3b22b8f (4 commits). READY-WITH-CAVEATS; no soundness bug; 5,003,728 candidate proofs over all 15,180 width-2 formulas (none accepted for SAT), all 1,686 UNSAT accepted, 1.65M-proof differential vs independent checker, width-3 domain; 18/18 mutants killed; 99.83% lines. Fixed `+N`/`-0` token acceptance (`-0` acted as a terminator) | queued for PR |
| P2 / LH-001a | drat-check: `check_with_limit(.., usize::MAX)` allows 2^30 vars (memory DoS footgun); binary DRAT unsupported; encoding faithfulness of bit-blaster and cdcl-sat proof emission unverified (LH-005/006) | backlog |
| done-local / LH-005 | cdcl-sat audit: branch worktree-agent-a111d37eee462ba9e (5 commits). READY; no bugs; ~290k exhaustive/assumption solves all brute-force checked with every UNSAT proof DRAT-certified; PHP/XOR/Tseitin families; 28/29 mutants killed (1 equivalent); 99.93% lines; VER09 reconciled (DRAT emission implemented, budgets, cores) | queued for PR |
| P0 / LH-005a | INTEGRATION: drat-check audit (LH-001) now rejects `+N`/`-0`; cdcl-sat still accepts them and its new solver/kernel DIMACS agreement test will fail once both land. Make cdcl-sat's DIMACS reader strict in the same PR that lands LH-001+LH-005 | must ship together |
| P2 / WD-010 | Security review round 3 INFO: unsafe code outside `src/**/*.rs` text (include!, #[path] outside src, build.rs/OUT_DIR, proc-macros, tests/benches) is neither source-bound nor scanned; reject include!/#[path]/build.rs in the checker or require them in the certificate | backlog |
| P3 / WD-011 | `ca-assurance-report/src/report.rs` ~395/410/428 `as_i64().unwrap_or(0) as u64` wraps negative counts (pre-existing); use u64::try_from | backlog |
| P3 / WD-012 | `ct-compare/tests/sabotage_guard.rs` uses predictable temp dir in /tmp with remove_dir_all+create_dir_all; move under CARGO_TARGET_TMPDIR | backlog |
| done-local / LP-001 | gf256 Warrant conversion: branch worktree-agent-a88d5407e27ca898b (5 commits on 21d86a2323, pre-security-fix base; rebase onto merged #19054). READY-WITH-CAVEATS: 38 claims, 51,472,354 exhaustive cases incl. all 2^24 triples, 7 scoped sabotage faults caught, model oracles pin polynomial 0x11D; data-driven bindings (ct-compare output byte-identical); package-agnostic ca-assurance-report. No gf256 bug; spec/README corrected (AES values, 0^0). Proofs not-live: WIR front end rejects `&'static`, struct-field index, `return if..else` | queued for PR after #19054 |
| P1 / LP-001a | Rust-subset front end (VER25a/b): lifetimes, struct field + index, `return if/else` expressions, OnceLock tables, so gf256 proofs go live. **Now delivered as [RS01](RS01-rust-grammars.md) phase B (RS-B1..RS-B5, below); `OnceLock` replaced by compile-time tables (LP-001c), by owner decision 2026-11-12** | after LP-001 |
| P2 / LP-001b | `Field::new` accepts any u16 polynomial without validation; capabilities.rs witness + `--verify-cert` for gf256 (E0 currently rests on an unverified cert) | backlog |
| P2 / OPS-003 | Agent worktrees are created from origin/main, not the current branch; briefs that need unmerged work must say so (lesson recorded by LP-001) | apply in briefs |
| merged / WD-002 | VER04i polyglot spec: merged in #19078. `warrant-corpus-v1`: exhaustive suites enumerated in-lane from domain descriptors with inputs/expected SHA-256 digests cross-checked against the Rust compiler and stdlib Python oracle; per-lane rollout and manifest/report contract remain queued in WD-1xx/004 |

Provisional decisions on VER04i §14 (coordinator defaults, pending owner review):
1. Runner lives in each lane's test-bootstrap package (create one per lane as needed), not a separate warrant-corpus package.
2. In-lane model evaluator cross-checked by digest: yes (no WASM toolchain dependency).
3. Keep hand-written ct-compare-v1 as the independent oracle for work-accounting fields.
4. Shard by default when a lane exceeds 120 s; ceiling stays.
5. Native-layer generators emitted by the Rust compiler (single source of truth).
6. A fault no claim catches always forces a new claim (fail closed).
7. No release-ready without an armed VER08b sandbox (matches the user's sandbox requirement).
8. gf256.warrant exists on the LP-001 branch; examples become real once it merges.
| done-local / LH-006 | bit-blaster audit: branch worktree-agent-afc4e1727d486b34a (3 commits). READY: no unfaithful encoding, no false UNSAT (bit-blaster or constraint-engine LIA); exhaustive widths 1-6 through AIG->Tseitin->cdcl-sat vs reference with drat-certified uniqueness; 3.5M depth-2 assignments; certified identities; 24/24 mutants killed; 100% lines. Added udiv/urem/sdiv/srem/extract/concat/zext/sext circuits; width-0 panic made explicit | queued for PR |
| done-local / LH-006a | constraint-engine::bv lowering audit (constraint-engine 0.5.0): no unfaithful lowering; exhaustive w1-8 vs SMT-LIB reference, random+corners 16/32/64, e2e certified; bvsmod + udiv/urem/sdiv/srem/concat/extract/extend added; wide-miter limits documented (CDCL budget from w10 for a*b=b*a; drat 2^32 at w16 for x*-1, 2^34 per-test budget); wide-divider pre-size refusal | queued for PR |
| done-local / LP-006 | required_capabilities.json audit: branch worktree-agent-aa95d6cf90c4675a6 (3 commits). 280 leaf crates: 153 valid, 60 added (57 pure; repl, fpga-bitstream, wall-clock with declared capabilities), 53 still missing, 14 invalid | queued for PR |
| P1 / LP-006a | 14 invalid existing manifests (legacy shape: atbash-cipher, caesar-cipher, irc-framing, irc-proto, layout-ir, sql-backend, text-interfaces, write-ahead-log; array: bounded-json; non-object entries: csprng, vault-pm-storage-removable; "abilities" typo: resp-protocol, skip-list, tcp-server which likely needs real net capabilities). INTEGRATION RISK: the hardened analyzer (LH-002) rejects lenient/unknown-field manifests; fix these before or with LH-002 | before LH-002 PR |
| P2 / LP-006b | 53 leaf crates still without a manifest (mostly simulators/protocols; protocol, os-job-core, jit-loader-macos, cuda-compute, jni-bridge likely need real capabilities); rng CHANGELOG.d shard | backlog |
| done-local / LP-006a | 14 invalid manifests fixed: branch worktree-agent-a04ef1c6a7559cf4a (commit 2c2469ea85); all pass the hardened analyzer; tcp-server now declares net/time; csprng rand/fs/ffi with unsafe+foreign_linkage exceptions; write-ahead-log fs; vault-pm-storage-removable fs/ffi | ship with or before LH-002 |
| P2 / LH-002c | Analyzer over-declaration: `#[cfg(test)]` code counted (forces test-only time/env/proc declarations), bare `use std::thread` counted as thread:spawn; target charset stricter than the schema | backlog |
| done-local / WD-3xx | ct-compare ports for lua, perl, haskell, ocaml, dart, swift: branch worktree-agent-a867a8cd1052bd444 (7 commits). lua/perl/haskell/ocaml/dart run locally and pass (100% measured where tooling existed); swift never compiled (toolchain download blocked) — first CI run decides; build-tool registers the six lanes as CT01 consumers. Satisfies package-parity owners ct-compare-<lang>-lane-parity (state file left to the parity loop controller) | queued for PR; check no parity PR is active first |
| P2 / OPS-004 | The package-parity loop (.claude/package-parity-loop-state.json) owns per-lane ct-compare items; coordinate so the two loops never pick the same item concurrently | ongoing |
| P1 / OPS-005 | CI lesson: changing a crate's source invalidates its committed effect_certificate.json (`--verify-cert` source_digest mismatch); regenerate certificates for every changed crate before pushing | lesson to record with the #19054 fix |
| done-local / WX-000 | VER04j Warrant v1 extensions spec: branch worktree-agent-ad69283a1344b3c11 (commit a809a22a915). `warrant x v1` header; WX-B fixed-width ints (wrapping), WX-C bitwise (named functions; shift >= width gives 0), WX-E pinned vector tables, WX-A tuples/records/variants, WX-H output bounds with allocation-capped observer; WX-F/WX-D sketched. Order B+C (6-9d), E (6-8d), A (10-15d), H (5-7d), F, D | queued for PR; reconcile its provisional corpus encodings with VER04i (it could not see the VER04i branch) |
| P1 / WD-013 | BUG: the Rust evidence generator evaluates U8 arithmetic as non-wrapping, contradicting VER04 §3 (U8/U64 wrap). Claims using U8 arithmetic get a wrong oracle; gf256 worked around it. Fix with differential tests in the first WX slice | done-local for v1 documents (WX-B, `warrant-grammar` 0.2.0): in a v1 document `U8`/`U64` are typed `V::Fixed` and wrap, with native-Rust differential tests and a killed non-wrapping-U8 mutant. The **v0** generated harness still evaluates `U8` as `V::Nat`: it is frozen byte-identical so `ct-compare`/`gf256` evidence does not change; the gf256 conversion to a v1 document (backlog) retires it. Also on this branch: VER04j section 14 lists every deviation |

Provisional decisions on VER04j §12 (coordinator defaults, pending owner review):
1. `warrant x v1` header (not per-feature `use`). 2. Fixed-width `+ - *` wrap by default (keeps v0 meaning). 3. Signed div/rem rejected; `div_trunc`/`rem_trunc`; MIN / -1 -> None. 4. Shift counts >= width give 0 (mathematical), not hardware masking. 5. DER-TLV offset via a second observer op (no payload-carrying error types yet). 7. VER04i owns corpus encodings; VER04j defers to it. 8. Exhaustive budget stays a per-package option.
NEEDS USER DECISION: (6) WX-H lz* `max_output` + error channel is a breaking API change across ~17 ports — Rust-only adapter shim for the pilot vs cross-port API change; (9) who vets NIST/RFC vector data before a pin is accepted.
| done-local / AS-000 | VER13 assumption catalogue + constant-time discharge ladder: branch worktree-agent-a51d8746a7f0723d2 (commit c1db6260647). 84 assumptions across spec/src/wir/tcb/compiler/machine-code/uarch/gate/phys/os/governance (only 4 emitted today). Ladder R0 WIR bounded(128) -> R1 IR/asm taint translation validation (pinned toolchain) -> R2 owned wir-to-riscv + symbolic RV32I `proved-to-isa` -> R3 `riscv-gatelevel` netlist `proved-to-gate-level`. Catalogue JSON + `--check` rules AC-1..AC-15 | queued for PR |
| P1 / AS-001a | Land catalogue JSON + AC checks; migrate ct-compare report (assumption list grows before it shrinks) | after AS-000 merges |
| P1 / AS-002 | Discharge `memory.unaliased`; Miri on volatile_barrier; replace name-matched volatile lowering (`wir.lowering.volatile-by-name`) | after AS-001a |
| P2 / AS-003a | R1 IR/asm taint checker for ct-compare with pinned toolchain tuple | after AS-001a |
| P2 / AS-007 | Spec inconsistency (renumbered from a second AS-004, VER27 §10 C1): ct-compare.warrant says `assume platform: none` while proofs rely on platform assumptions (`spec.platform.declared-none`) | with AS-001a |

Provisional decisions on VER13 open questions (coordinator defaults, pending owner review):
1. Labels are per `stack`; CWE-208 retires only for the stack actually proved (owned RV32I), not for host rustc builds. 2. Netlist by tracing the existing riscv-gatelevel engine into HNL (re-synthesis later as a cross-check). 3. Constant-time kernels restricted to RV32I first; RV32M ladder later. 4. Byte-address observation model. 5. Only the checker marks entries discharged, from evidence. 6. Catalogue in code/assumptions/, VER13 absorbs VER00 §7.5.2. 7. Bounded BMC cross-check once at R3.
| ready / LH-003 | test-bootstrap audit integrated onto main: branch worktree-agent-a411ad7edc430baa3 (0.4.0; no consumer changes; all BUILDs pass; security review PASSED) | next PR after SEC-001 |
| P2 / LH-003d | Security LOW: bench hard budgets that cannot be checked still yield BudgetVerdict::Pass (listed only in output) — add strict mode / `is_fully_checked()`; nothing consumes FuzzReport::is_clean() yet; tfv_differential.py f-string backslash needs Python >= 3.12 (CI uses 3.13) | backlog |
| P2 / OPS-006 | A worktree-isolated agent switched the MAIN checkout's branch (to its own worktree branch); no work lost. Briefs must say never run `git checkout`/`switch` outside the agent's own worktree | apply in briefs |
| ready / LP-001 | gf256 + package-agnostic report: branch worktree-agent-a6e1ded123339efdd (integration on main + security fixes M1/L1/L3/L4/L5; round 2 PASSED) | queued for PR |
| P2 / WD-014 | Security INFO: `--check` step 6 doesn't re-derive CWE-125/787 discharge; proof `kind` not whitelisted; evidence.bindings not hash-bound; gen_evidence numeric options uncapped and file I/O uncapped/symlink-following | backlog |

### Coordinator status, 2026-11-10 (parallel PR wave)

Every queued branch above is now an open PR with auto-merge armed; the rows
below map items to PRs and log what the wave discovered. Status `PR` means
"open, auto-merge armed, babysat"; `open` means not started.

| Priority / id | Item | Status |
|---|---|---|
| PR / SEC-001 | lz77/lz78/lzss/lzw decompression bounds in all lanes | PR #19059 |
| PR / LH-003 | test-bootstrap audit (macOS: /dev/full test now Linux-only, see LH-003c) | PR #19075 |
| PR / LH-001, LH-005 | drat-check + cdcl-sat soundness audits, strict DIMACS | PR #19076 |
| PR / LP-001 | gf256 Warrant evidence + package-agnostic report | PR #19077 |
| merged / WD-002, AS-000 | VER04i, VER04j, VER13 specs + backlog | PR #19078 (merged) |
| PR / CAP-RT-001 | runtime capability conformance gate | PR #19079 |
| PR / LH-002, LP-006 | analyzer hardening + 74 manifests (14 removed from the v1 invalid ledger) | PR #19080 |
| PR / LH-006 | bit-blaster faithfulness audit | PR #19081 |
| PR / CT01 ports | ct-compare in lua, perl, haskell, ocaml, dart, swift (OCaml corpus now embedded: tests need no fs capability) | PR #19082 |
| PR / SEC-003, SEC-004 | Rust huffman table validation; Elixir lz77 linear decode | PR #19084 |
| PR / OPS-012..014 | BUILD_windows parity (vitest, PERL5LIB, EndOfLine) + CI audit script | PR #19085 |
| PR / BUILD-001 | stop committing generated wasm; build where used | PR #19086 |
| PR / SEC-003b | huffman decoder hardening in 16 more lanes | PR #19087 |
| PR / AS-001a(1) | VER13 catalogue data (84 entries) + AC-14 checker in CI | PR #19089 |
| P1 / AS-001a(2) | ca-assurance-report side: pin `catalogue.sha256`; enforce AC-1..AC-13 report rules; migrate ct-compare report to catalogue ids; derived exemptions; `evidence-kinds.json` | after #19077 and #19089; done-local on branch `claude/as-001a2-report-catalogue` (ca-assurance-report 0.2.0; VER13 clarifications part 2) |
| P3 / AS-001b | AC-12 matches a non-claim to proofs by `p.kind == nc.claim`, but the `CLAIM_KINDS` vocabulary (`bound`, ...) does not map to proof kinds (`steps-bound`, `heap-bound`, ...): a `bound` non-claim would never match a proof. Add an explicit claim-kind -> proof-kind mapping, or restrict the non-claim vocabulary to kinds with a 1:1 proof kind | open (AS-001a security review) |
| P3 / AS-001c | AC-4 checks the report-level `assumptions` only as a superset of the derived list, while per-claim lists must also equal the derivation (step-5 field comparison). Decide whether the report-scope list must be exactly equal | open (AS-001a security review) |
| P0 / CI-RED-001 | main fails `test_repository_source_input_boundary_is_closed_and_canonical` (six TypeScript packages missing from the TS workspace boundary); fix is open PR #19050 | watch; blocks #19086 |
| P2 / LH-003c | Rust std treats EBADF on stdout as a successful write, so a harness whose fd 1 is closed or read-only prints nothing and exits 0. Detect at start (fstat fd 1) or document in TF01 §8 known limits; supervisors must require the plan line | done-local: stdout checked at start (Linux closed fd via .init_array; read-only fd; Windows handle), limits in TF01 §8.3 item 9 |
| P2 / CI-001 | Playwright 1.58 cannot install Chromium on ubuntu-latest (26.04); `release-visicalc.yml` verify pinned to ubuntu-24.04 in #19086. Bump release-tests Playwright, then unpin | open |
| P3 / SEC-003c | C and C++ huffman encoders hold codes in 32-bit integers; an extremely skewed distribution needs longer codes | open |
| P3 / OPS-015 | Windows leftovers from the parity audit: lua nib_* BUILD uses `>/dev/null \|\|`; matrix-rust-napi has no Windows addon copy; four advisory runner-set differences (go/barcode-1d, swift/Barcode1D, swift/PaintCodecPNGNative, typescript/forme-shell-desktop) | open |
| P3 / AS-006 | Backlog hygiene: AS-004 is used for two rows; VER13 references WD-009 and CAP-RT ids that need their own rows | partly done 2026-11-12: the second AS-004 row is renumbered AS-007 (VER27 §10 C1); the WD-009 and CAP-RT rows remain open |
| spec / WX-001 | VER04k (`code/specs/VER04k-warrant-v1-wave4.md`): normative WX-D (bounded sequence combinators, in slices D1/D2/D3), WX-F (binding-level sequence drivers) and WX-G (in-out postconditions); new diagnostics W0711-W0713 | spec merged #19115; WX-H/WX-E prerequisites merged #19121; WX-D1 S1 checker and ground evaluator merged #19209; WX-D1 S0 errata PR #19204 remains open; evidence-generator/corpus translation S2a is outstanding. Keep the existing D1/F/G/D2/D3 sequence and resolve the VER04k §9 questions during implementation. See [portable Warrant queue](VER-BACKLOG.d/portable-warrant.md) |
| P1 / WX-RD1 | Security (WX-D1 S1 review): the Warrant ground evaluator's reads deep-clone `EvalValue` and charge no data (VER04k §9.1, note on E4), so the 64 MiB data counter bounds each v1 value, not an evaluation's memory. Measured: a list literal of 48 references to a value charged about 16 MB peaked at 2.4 GB before W0703; a model call with 48 such arguments peaked at 2.4 GB and succeeded with `data_used` 16 MB; about 33 ms of CPU per clone at about 3 work units; worst case hundreds of GB. Fix: shared storage (`Rc`/`Arc`) in `EvalValue`, or a host-side read guard; whichever lands, generated code and the lanes keep the same meter charges. Also (S1 security review, round 2): even a charged value costs more than its charge. The meter counts 32 bytes per value slot (`32 + dsize`), but an `EvalValue` is 96 bytes, and every `String` it owns (bytes, constructor and error names) is a separate heap allocation with its own malloc chunk overhead. So the real memory of a charged list or composite is about 3-5 times its `data_used`, still bounded, because the charge is, by the 64 MiB budget (a few hundred MB per value). The `Rc`/`Arc` fix should re-measure the slot size, or the slot charge should rise to match | open |

### Rust capability analyzer: open gaps after the third security review (2026-11-11)

The branch `claude/ver-cap-split-a-analyzer` closes the cheap third-review
cases (VER08a D43-D49, including review G-A, associated-type projections) and
the four shapes a follow-up review found those fixes missed (D50-D53).
These false negatives stay open; each has an example, root cause and intended
fix in [VER08a §8.1](VER08a-rust-capability-analyzer.md#81-known-open-gaps-security-review-round-3).

| Priority / id | Item | Status |
|---|---|---|
| P1 / CA-OG1 | Review G-C: a type alias's generic parameter default is never bound, so `type F<P: ?Sized = std::path::Path> = P; <F>::exists(p)` is pure (case `alias-default-param`). Fix: bind the alias to every path inside a parameter default in `type_alias_leaves`, as for the right-hand side; a default that is a metavariable or a macro invocation (`type F<P = $t> = P;`, `P = ty!()`) binds the alias to the unknown type; an undelimitable default is CAP003 | open |
| P1 / CA-OG2 | Review G-E: the local-receiver waiver (L4) is bypassed when the local type derefs to `Path` (cases `waiver-cfg-out`, `waiver-macro-body`, `waiver-generic-instance`, `waiver-ctor-wrong-type`, `waiver-same-name-module`, `waiver-arbitrary-self`, `og2_private2`); `collect_local_types` keys by bare name and ignores cfg, macro bodies, generic arguments, modules, arbitrary self types and visibility (a private inherent method in another module, skipped by method lookup at the call site). Fix: refuse the waiver when any `Deref`/`DerefMut` impl for `<Name>` exists, including ones not spelled `impl Deref for Name` (derive macros such as `derive_more::Deref` and `shrinkwraprs`, `macro_rules!`-generated impls, a renamed trait `use std::ops::Deref as D; impl D for L`, an impl through a type alias), failing closed when the type has any non-std derive or macro-generated impl; otherwise require one unique definition, a non-generic inherent impl outside macro bodies and cfg, a method visible at the call site, and a plain receiver | open |
| P3 / CA-IPC | The Spec 13 capability taxonomy has no IPC category, so `std::io::pipe` and `PipeReader`/`PipeWriter` are classified as `ffi:call` (VER08a D47, the row for raw OS handles) as a stand-in. Add an IPC category (pipes, Unix-domain socket pairs, shared memory) to the taxonomy and move them there | open |

## Versioned Rust grammars (RS01)

[RS01](RS01-rust-grammars.md) (slice A1, spec only) defines a `.tokens`/`.grammar`
pair for every published Rust release, 0.1 to the newest tag, per edition from
1.0. Order is value first by owner decision (2026-11-12): phases A and B are
milestone 1, need no engine change, and deliver LP-001a; then the full modern
grammar (C), the engine batch (D), macro expansion (E), 1.x newest to oldest (F)
and pre-1.0 oldest to newest (G). One slice per PR.

| Priority / id | Work | State / prerequisite |
|---|---|---|
| P1 / LP-001c | gf256: build the log/antilog tables at compile time (`const fn`, `static TABLES: Tables = build_tables();`) instead of `OnceLock`; regenerate its Warrant evidence, assurance report and effect certificate (OPS-005); about 95 dependents. Owner decision 2026-11-12; `OnceLock` and lazy statics stay unsupported in `rs-confined` | separate PR, before RS-B4 |
| P1 / RS-A2 | Inventory: `code/grammars/rust/releases.csv` (158 rows; 1.100.0 when tagged), README, `editions` column, `subsets.csv` with `allowlist`; `release_inventory.py` edition stems, uniform claims, `.reject@<release>-e<edition>` | **Status: implemented in local commits (158 rows, 347 pairs; checker, tests, README, A2 decisions in RS01 section 10), not yet merged.** `rs-confined.allowlist` is deferred to RS-B1 |
| P1 / RS-A3 | Evidence tooling `rust_release_pairs.py` (`reviewed`, `copy:`, `semantic-copy:`; `check` mode); first `pair-evidence.csv` rows | after RS-A2 |
| P1 / RS-A4 | Modern lexical pairs `rust1.97.0-e{2015,2018,2021,2024}` (subset `rs-token-trees`); generated raw-string arms and per-Unicode XID classes; verify the section 4.3 comment design | after RS-A3 |
| P1 / RS-A5 | Crates `rustlang-lexer`/`rustlang-parser`, the two Rust C3 hooks, `release_probes`, hostile probes, host-gated token oracle; `ci-gates.json` edit (all gates run) | after RS-A4 |
| P1 / RS-B1 | `rs-confined` grammar in the 1.97.0 pairs, `rs-confined.allowlist` subset gate, corpus with rustc-run `.expect` (LP-001a) | after RS-A5 |
| P1 / RS-B2 | `warrant-ir` grammar path; differential against the VER25a parser (byte-identical WIR, rejection parity, mutation fuzz) (LP-001a) | after RS-B1 |
| P1 / RS-B3 | gf256 grammar and gate coverage (RS01 section 7.2) (LP-001a) | after RS-B2 |
| P1 / RS-B4 | VER25b lowering over CST views; `const fn` table initialization; `gf256_proofs` (38 claims) live (LP-001a) | after RS-B3 and LP-001c |
| P1 / RS-B5 | Retire the hand-written VER25a parser after the parity window (LP-001a) | after RS-B4 + parity window |
| P2 / RS-C1 | Full grammar for the newest release in four editions; `rs-syntax` corpus; repository self-corpus | after phase B |
| P2 / RS-C2 | Oracle harness `rust_oracle.py`: pinned toolchains (SHA-256), `cfg(FALSE)` + token dumper, scheduled (daily) non-blocking sweep | after RS-A5 |
| P2 / RS-C3 | `rustlang-cfg`: the L1 `cfg` evaluator with per-release `CfgSet` rules | after RS-C1 |
| P2 / RS-C4 | Capability analyzer step 1: shared lexer, dual-lex for unknown editions, parity suite; own security review (changes a CAP003 class) | after RS-A5 |
| P2 / RS-D1 | Engine batch, lexer side (`ci-full-build`, ~5,300 packages): X1 nested scanner, X3 shebang guard, optional X2; delete the Rust C3 hooks | after phase C |
| P2 / RS-D2 | Engine batch, parser side (`ci-full-build`): X4 parse-rule/prefix API, X5 synthetic tokens, optional X6 | after RS-D1 |
| P2 / RS-E1 | `rustlang-macros`: `macro_rules!` definition checker (per-release fragments, follow sets, 1.89 rule) | after RS-D2 |
| P2 / RS-E2 | Matcher, transcriber, invisible-delimiter interpolation, expansion budgets | after RS-E1 |
| P2 / RS-E3 | Hygiene side table; textual and path scoping; `$crate`; `#[macro_use]`/`#[macro_export]` | after RS-E2 |
| P2 / RS-E4 | Builtins per release; `std` macro definitions extracted per release as data; `include*`/`env!` opaque | after RS-E3 |
| P2 / RS-E5 | Optional builtin derives; the `Unexpanded` fail-closed contract | after RS-E4 |
| P2 / RS-E6 | Consumers adopt expansion (proof front end: local `macro_rules!`; analyzer: fewer CAP003) | after RS-E5 |
| P3 / RS-F1 | 1.x back-fill: 1.85–1.99 (2024 era) | after phase E |
| P3 / RS-F2 | 1.x back-fill: 1.56–1.84 (2021 era) | after RS-F1 |
| P3 / RS-F3 | 1.x back-fill: 1.31–1.55 (2018 era) | after RS-F2 |
| P3 / RS-F4 | 1.x back-fill: 1.13–1.30 | after RS-F3 |
| P3 / RS-F5 | 1.x back-fill: 1.0.0–1.12 | after RS-F4 |
| P3 / RS-F6 | 1.x back-fill: 1.0.0-alpha, 1.0.0-alpha.2, 1.0.0-beta | after RS-F5 |
| P3 / RS-G1 | Pre-1.0: 0.1 (`#name(exprs)` extensions; `.rc` files via X4; optional ast-mbe `#macro`) | after phase F |
| P3 / RS-G2 | Pre-1.0: 0.2 | after RS-G1 |
| P3 / RS-G3 | Pre-1.0: 0.3 and 0.3.1 (`copy:0.3`) | after RS-G2 |
| P3 / RS-G4 | Pre-1.0: 0.4 | after RS-G3 |
| P3 / RS-G5 | Pre-1.0: 0.5 | after RS-G4 |
| P3 / RS-G6 | Pre-1.0: 0.6 | after RS-G5 |
| P3 / RS-G7 | Pre-1.0: 0.7 | after RS-G6 |
| P3 / RS-G8 | Pre-1.0: 0.8 | after RS-G7 |
| P3 / RS-G9 | Pre-1.0: 0.9 | after RS-G8 |
| P3 / RS-G10 | Pre-1.0: 0.10 | after RS-G9 |
| P3 / RS-G11 | Pre-1.0: 0.11.0 | after RS-G10 |
| P3 / RS-G12 | Pre-1.0: 0.12.0 (joins 1.0.0-alpha) | after RS-G11 |
