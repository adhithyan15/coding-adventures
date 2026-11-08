# VER delivery backlog

Implementation queue for VER00 and the Rust VER26 pilot. Reconcile fetched
`origin/main` and open PRs before selecting each item. Initial audit: main
`462710b53fc`, bootstrap PR #18807 at `4e608afe3bc9`. Local tests, an open PR
and a verified merge are different completion states.

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
| done / VER-P008 | Rust backend and ct-compare.warrant | #19002 merged as 652207f348d; [VER04e](VER04e-warrant-codegen.md) specified; bounded pure model evaluator, TFV test vector compiler, Rust L0 harness generator, checked regeneration |
| done / VER-P009 | Property spec and pbt-v1 corpus | #19004 merged as 5b1fcef8b2e at expected c6f4c29e61c; verified on fetched main; all checks acceptable |
| done / VER-P010 | Rust property engine | #19007 merged as f66f3a2e057; [VER05](VER05-property-engine.md) generator algebra, integrated shrinking, deterministic replay, pbt-v1 corpus (25 cases), zero third-party deps, 92%+ line/branch cov |
| done / VER-P011 | Warrant claims to properties | #19011 merged as 315447685ec; [VER04f](VER04f-claims-to-properties.md) specified; compiled universal claims to native Level 2 PBT suites, public API calls and broken implementations detected, 93%+ line/branch cov |
| P1 / VER-P012 | Stateful properties | waiting P011; select only if pilot needs them, otherwise explicitly defer |
| done / VER-P013 | Fuzz spec and fuzz-v1 corpus | #19021 merged as 97618fd2c1f; [VER06](VER06-coverage-guided-fuzz-engine.md) specified; canonical fuzz-v1 corpus (25 cases in cases.json/cases.tfv), standard-library Python reference validator, private failure-output policy, 98% line cov |
| done / VER-P014 | Rust fuzz engine | #19022 merged as cbc5befeaf9; [VER06](VER06-coverage-guided-fuzz-engine.md) mutator algebra, 65,536-bit coverage bitmap, power scheduling, FNV-1a crash deduplication, minimization, fuzz-v1 corpus test (25 cases), zero third-party deps, 95%+ line/region cov |
| done / VER-P015 | ct-compare fuzz targets | #19023 merged as e623e7bb5f9; [VER06a](VER06a-ct-compare-fuzz-targets.md) specified; ct_eq, ct_eq_fixed, ct_select_bytes, ct_eq_u64 fuzz suites, negative sabotage detection and minimization, privacy-preserving TAP reporting, 100% coverage, zero third-party deps |
| done / VER-P016 | Microbenchmark spec/harness | #19026 merged as 7168cda2272; [VER07](VER07-microbenchmarks.md) specified; warmup/calibration, robust statistics (median/MAD/bootstrap CI), deterministic cost counters (steps/allocs/instructions), budgets, zero third-party deps, 93%+ cov |
| ready-pr / VER-P017 | Equal-instruction-count measurements | [VER07a](VER07a-ct-compare-microbenchmarks.md) specified; n=16/256/4096 ct_eq, ct_eq_fixed, ct_select_bytes, ct_eq_u64, zero-alloc assertions, equal instruction counts, sabotage detection |
| done / VER-P021 | WIR core/reference interpreter | [VER25](VER25-wir-core-interpreter.md) specified; WIR core AST, static type checker, cost semantics, provenance-tracking memory model, observable trace recorder, bounded reference interpreter, constant-time validation; 91%+ line/branch cov |
| done / VER-P022 | Rust-subset grammar | #19014 merged as 3d8a14c46d8; [VER25a](VER25a-rust-subset-grammar.md) specified; real ct-compare source, volatile/unsafe assumptions, zero third-party deps, 93%+ line/branch cov |
| done / VER-P023 | Rust-to-WIR front end | #19017 merged as b486bf3b28e; [VER25b](VER25b-rust-to-wir-frontend.md) specified; lowered parsed Rust AST to WIR modules, differential validation against ct-compare, zero third-party deps, 92%+ line/branch cov |
| done / VER-P024 | Symbolic execution | #19019 merged as b56a4b1d0fe; [VER19](VER19-symbolic-execution.md) specified; bit-precise proofs via bit-blaster and cdcl-sat, memory safety bounds, honest bounded(k) labels |
| done / VER-P025 | Bounds/constant-time trace proofs | #19020 merged as e2a4c386892; [VER20](VER20-resource-timing-proofs.md) specified; self-composition 2-safety hyperproperties, timing/address divergence detection, DRAT-certified UNSAT, >90% line/branch cov |
| done / VER-P027a | Rust capability analyzer/name allowlist | #19028 merged as cbd8b93f0e7; [VER08a](VER08a-rust-capability-analyzer.md) specified; zero third-party deps, 92%+ cov, fail-closed audit |
| in-progress / VER-P027b | Generated effect module/certificate | [VER23a](VER23a-rust-effect-modules.md) specified; VER23/24, small checker, empty effects, ct-compare certified |
| P2 / VER-P028 | Weakness registry/report-only assurance | waiting proof/effect evidence; VER15/01, not-live rows, unapproved CWE-208 exemption |

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

Prioritize VER-P017 (Constant-Time Equal-Instruction-Count Microbenchmark Suite for ct-compare [VER07a](VER07a-ct-compare-microbenchmarks.md)).
VER-P017 verifies equal instruction counts across equivalence classes (identical, diff first, diff last, diff middle),
multi-scale input sweeps (n=16, 256, 4096), compile-time fixed widths, branch-free selection, and negative sabotage detection.
Next step: VER-P027a (Rust capability analyzer / name allowlist).

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
