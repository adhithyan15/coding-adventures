# VER delivery backlog

Implementation queue for VER00 and the Rust VER26 pilot. Reconcile fetched
`origin/main` and open PRs before selecting each item. Initial audit: main
`462710b53fc`, bootstrap PR #18807 at `4e608afe3bc9`. Local tests, an open PR
and a verified merge are different completion states.

For portable Warrant, [portable-warrant.md](VER-BACKLOG.d/portable-warrant.md)
is the current status, receipt, and discovery record. The coordinator rows and
dated snapshots below are historical overview; use the shard for current
implementation state.

For bringing the L0 harness, the property engine and the fuzz engine to every
language lane, [polyglot-testing-tooling.md](VER-BACKLOG.d/polyglot-testing-tooling.md)
holds the lane matrix, the definition of done (whole `pbt-v1`/`fuzz-v1` corpus
plus negative controls) and the ordered plan.

For verified, publishable basic logic gates in every language lane (per-lane
CWE threat lists, corpus runners, analyzers and publishing),
[logic-gates-everywhere.md](VER-BACKLOG.d/logic-gates-everywhere.md) holds
the audit, definition of done, queue and status.

## Delivery and prioritization

One bounded change per PR, spec committed before tests/code, meaningful local
validation and a security sub-agent review before pushing. Investigate actual
current-head CI failures and conflicts; enable squash auto-merge only after
terminal acceptable checks, guarded by the expected head (for example,
`gh pr merge --auto --squash --match-head-commit <sha>`). Verify the expected
head and squash payload on freshly fetched main.
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

The TypeScript `sat-solver` package (`code/packages/typescript/sat-solver`) and
Rust `cdcl-sat` package (`code/packages/rust/cdcl-sat`) have distinct canonical
names; the tracked Go tree has no `sat-solver` package. The optional bounded Go
parser miter reuses the TypeScript solver from test code only. It establishes
neither a Go solver port nor cross-language package parity; track that separate
gap in the [package parity roadmap](package-parity-roadmap.md).

## Prioritized pilot queue

`ready-local` permits preparation, not a claim that dependencies are delivered.
Each selected group is split into bounded PRs if its implementation requires it.

| Priority / ID | Work | State / prerequisite and acceptance |
|---|---|---|
| done / VER-P003 | TF01 Rust L0 bootstrap | #18807 merged as 6a09869c279; expected head 4e608afe3bc9 verified on fetched main; all checks acceptable |
| done / VER-P004 | Rust CT01 corpus through L0 | #18849 merged as 0bee67f3a81; expected head ca72cad1191 verified on fetched main; all 44 checks acceptable |
| P2 / VER-CI001 | Retain diagnostics for intermittent macOS iPad UI failure | original job125811351687 failed; bounded same-head retry125843044183 and run41602352046 attempt2 passed; root cause and missing UI-state artifacts remain unresolved; no assertions weakened |
| P2 / VER-CI002 | Diagnose recurring Windows TypeScript child-process `ETIMEDOUT` under fixture wall-time limits | Earlier jobs 130798040371, 131243804408 and PR #19529 run 44010659629 had real-CLI `spawnSync ETIMEDOUT` failures followed by successful same-head retries. PR #19644 exact head `15060c8` run 44351985766 attempt 1 failed both `tests/discovery.test.ts` duplicate-identity and `tests/resolver.test.ts` invalid-UTF-8 real CLI fixtures (`node.exe` via `tsx`, each 5,000 ms); its one authorized failed-job rerun job 135008986193 passed duplicate identity but repeated invalid-UTF-8 timeout. PR #19644's emitted-JavaScript setup compiled once per Vitest run and launched fresh Node processes, preserving fixture arguments, 5,000 ms process-tree limits and exact exit/diagnostic assertions. Registry bytes and Node dependency resolution were checked. The reviewed repair merged in #19644 squash `4ec975d`; exact-head protected checks passed. Root cause is not claimed. See lesson [Run real TypeScript CLI fixtures against emitted JavaScript when loader startup consumes their process budget](../../lessons.d/run-real-typescript-cli-fixtures-against-emitted-javascript-when-loader-startup.md) and [portable Warrant receipts](VER-BACKLOG.d/portable-warrant.md) |
| P2 / VER-CI002-KOTLIN | Keep Kotlin Gradle compiler settings effective in the Windows captured-output build | On PR #19644 head `45ee520` Windows job `135065756220`, Kotlin tests and JaCoCo completed successfully (`BUILD SUCCESSFUL`), but the Go executor hit its existing five-second post-exit pipe wait. Package recipes and CI supplied `-Dkotlin.compiler.execution.strategy=in-process -Dkotlin.incremental=false`; pinned Kotlin Gradle Plugin 2.1.20 does not read these settings from JVM system properties. Local Gradle 9.4.1 `--info` verification reproduced daemon-backed/incremental compilation with those flags. The package-local `gradle.properties` makes fresh production/test compiles report `IN_PROCESS`, including the workflow-shaped `-p` invocation with existing `-D` arguments. The actual Go executor then completed a clean Kotlin build with tests and JaCoCo, captured `BUILD SUCCESSFUL`, and returned BUILT within its unchanged five-second post-exit wait; JUnit 3/3 passed and JaCoCo line coverage was 98.98% on local JDK 21. The shared configuration repair passed exact-head Windows CI in the reviewed PR #19644 head `0d110`; Gradle reports BUILT. CI uses Gradle 9.8.2; local Gradle was 9.4.1. The pipe-owning process and root cause remain unknown; no five-second budget or executor policy changed. See [Kotlin Gradle settings](KOTLIN-GRADLE-SETTINGS.md) and lesson [Use Gradle project properties for Kotlin compiler execution policy](../../lessons.d/use-gradle-project-properties-for-kotlin-compiler-execution-policy.md). |
| P2 / VER-CI002-UBUNTU-GRADLE | Diagnose Ubuntu Gradle daemon startup failure before Kotlin package tasks | PR #19644 exact-head run `44395603937`, attempt 1 Ubuntu job `135125201658` failed while connecting to Gradle daemon before any Kotlin task receipt; daemon startup was Gradle 9.9.1. Available tail shows registry-removal/shutdown exceptions but omits the initial cause, and the artifacts contain no full daemon log. One authorized job-specific debug rerun on unchanged head passed as attempt 2 job `135150245836`; aggregate CI gate `135165050874` passed. Preserve the first failure and successful debug retry. Root cause remains unresolved; no further retry is justified without new evidence. |
| P2 / VER-CI003 | Safely release per-ref CI concurrency after a run is superseded | PR #19685 head `4e8a42e` had a completed macOS package failure and runnerless queued CI gate `135430756253`; replacement head `02cbd0e` run `44486323864` remained pending with no jobs. Ordinary cancellation of only the verified superseded run did not release the gate; force-cancelling that same old-head run did, after which replacement detect `135432213702` started. PR #19689 also had two queued Swift workflow runs (`44512509869`, `44514478240`) from superseded heads; after verifying each run's head and the current PR head, only those obsolete runs were normally cancelled, leaving the current-head workflow untouched. Preserve logs, verify the PR/run head is superseded before cancellation, never cancel a current required gate, and verify replacement scheduling. These observations do not justify skipping gates or broad workflow changes. |
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
| done / VER-P014 | Rust fuzz engine | #19022 merged as cbc5befeaf9; [VER06](VER06-coverage-guided-fuzz-engine.md) mutator algebra, 65,536-bit coverage bitmap, power scheduling, FNV-1a crash deduplication, minimization, fuzz-v1 corpus test (25 cases), zero third-party deps, 95%+ line/region cov. An adopter feedback adapter from real compiler/source branches is not yet demonstrated; semantic outcome categories must not be reported as native branch coverage. |
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
| done / VER-P041 | Native Go `sat-solver` port | [VER09b](VER09b-go-sat-solver.md); PR #19675 merged by protected auto-squash as `45ebafd2626845ea0af63f463e618f832e435b7d` on 2026-11-18 after final CI passed. Dedicated API port, shared stateful corpus, independent SAT/model/core oracles, CDCL and builder gates, bounded DIMACS, and package metadata are on main. Go source-bound proof and line/branch tooling remain separately open under VER-P042. |
| P1 / VER-P042 | Go Level 3 source-assurance lane | Define and deliver Go Warrant source binding, a reproducible proof/certificate consumer, and an actual line/branch measurement path; acceptance requires compiling and running claims against selected Go package source, mutation controls, independently checked proof artifacts, and measurable line/branch criteria. Go statement coverage does not close this item |
| done / VER-P043 | Repair dedicated TypeScript `sat-solver` correctness and contract drift | [VER09a](VER09a-typescript-sat-solver.md); PR #19679 merged by protected squash as `d90566ca5d63e12b7538f249b2556ebcfd4deaac` on 2026-11-19 after final CI and Books checks. The dedicated package identity and public API were retained; root-propagation lifecycle, public propagation contract, stale failed-core clearing, DIMACS header grammar, and inaccurate core-minimality wording were repaired. The shared Go/TypeScript corpus and independent original-CNF checks now include seeded stateful differential traces and mutation controls; emitted-JS coverage and its instrumentation boundary are recorded in VER09a. P042 remains the separate Go source-proof/Warrant lane. |
| done / VER-P044 | Bounded TypeScript source to WIR proof bridge | [VER09c](VER09c-typescript-source-wir-bridge.md); PR #19685 merged by protected squash as `9511584649b1c3294d38a9742f97eac7ddd9ae5b` on 2026-11-19 from reviewed head `02cbd0e`. After all required checks were green, the guarded `gh pr merge --auto --squash` request merged immediately; no separate fallback was used. Replacement CI `44486323864`, Books `44486323844`, and Swift `44486323707` all passed. The exact build plan selected `rust/typescript-warrant-ir` and `go/programs/build-tool` on Linux, macOS, and Windows; all six package rows were `BUILT`, including the same-path/same-length Go cache A→B→A regression. Rust 1.100 strict `cargo clippy -p typescript-warrant-ir --all-targets -- -D warnings`, debug/release tests, and 96.57% line coverage passed locally. This establishes only the bounded fixture/helper proof; no production reducer proof or TypeScript runtime parity is claimed. |
| P2 / VER-P045 | Rust TypeScript grammar must enforce ECMAScript restricted productions | A direct `ts5.8` parser probe accepts `return\n(expression)` as a return-with-expression, while Node applies automatic semicolon insertion and returns `undefined`; the same divergence can occur across CR and line breaks inside comments. Add a parser-level token/trivia rule and differential tests for LF, CR, CRLF, U+2028, U+2029, and comments, while preserving the bridge's local fail-closed refusal until the shared parser is repaired. This parser correctness work is separate from VER-P044's helper subset. |
| P2 / VER-P046 | Bind GAME10's exact validator model to the manifest-derived assurance workflow | `game-clock` currently has no `evidence.bindings` or `evidence/` manifests; its bespoke real parser/checker/evaluator tests are finite evidence but cannot be rederived by `ca-assurance-report`. The existing binding parser (`code/packages/rust/warrant-grammar/src/binding.rs::parse_host_ty_at`) rejects zero/one-field tuples and defines no host `unit`, while `code/specs/game-clock.warrant` aliases `Unit = Bool` and Rust `validate_state` returns `Result<(), ClockError>` (`code/packages/rust/game-clock/src/lib.rs`). Design and test an existing-workflow adapter path (for example, a manifest-bound bool-normalizing test wrapper) that preserves exact `ClockError` variants and record fields; generate/run the standard harness, produce its existing test/proof manifests, and rederive `assurance_report.json` with finite property/fuzz receipts while leaving universal/exhaustive/proof fields and report-only rungs unchanged. Do not hand-edit derived report numbers or create a second evidence registry. |
| P1 / VER-P048 | Represent missing step-cost coverage and CWE-407 applicability in the derived report without author-created exemptions | The fixed-point Warrant operations intentionally have no declared `steps` bounds, but the current matrix/profile path requires a CWE-407 disposition and rejects `excluded` when no all-operation step evidence exists. The same missing step coverage is also a CWE-400 gap. Extend the existing reporter with a derived `applicable` applicability only when actual nonempty operations lack step bounds; the matrix must mechanically derive missing-step-bound nonpassing dispositions for CWE-400 and CWE-407 before any CWE-400 discharge. No package-authored “approved” flag, exemption, or fabricated proof/count may authorize the disposition. Keep the existing excluded rule, cost-proof semantics, and other weakness rows unchanged. This is required for honest Q16 report closure after P047; it does not alter the arithmetic contract or source-refusal statuses. | Open; found while integrating actual `fixed-point` manifests, 2026-11-19. |
| P2 / VER-P049 | Inventory and migrate saved Rust assurance reports that predate the required VER04i lane section | A read-only CLI sweep of 34 checked-in `assurance_report.json` files found 8 current evidence-manifest reports that pass and 26 files that fail before evidence re-derivation because their JSON lacks `lanes`. The 26 are `actionscript-compiler`, `arcade-sfx`, `audio-mixer-realtime`, `avm-bytecode-lifter`, `collision2d`, `flash-display-list`, `game-breakout`, `game-clock`, `game-monster-rpg`, `game-pong`, `hamurabi`, `lunar-lander`, `paint-batch-tiles`, `paint-palette-blend`, `palette-indexed`, `rpg-battle-engine`, `rpg-grid-world`, `sim-crt-vector-beam`, `sim-discrete-arcade-logic`, `sim-discrete-audio-timer`, `sim-pdp8-ge225-io-bridge`, `sim-teletype-asr33`, `sprite-oam`, `swf-container`, `swf-shape-to-paint`, and `tilemap2d`. The exact error is `missing 'lanes' (VER04i section 8.3)`; unchanged `report.rs` on fetched `origin/main` requires this field, so this is a saved-report migration gap rather than a regression from P047/P048. Inventory current consumers, regenerate only from their real evidence inputs, and verify with the canonical CLI; do not hand-edit reports or claim those files are current canonical reports. Keep distinct from P046's whole-clock source/evidence adapter and P047/P048's fixed-point disposition work. | Open; bounded inventory, 2026-11-19. |

**Pinned pre-repair pending-root propagation control (2026-11-18):** on GoSAT
checkpoint `3ff46d5`, the unchanged dedicated TypeScript source at
`code/packages/typescript/sat-solver/src/sat.ts` had SHA-256
`0e2fdc1c000c8fc0183be2aaa7b0ca32eeb6d32f0cb615cab31f77e4f81e5c22`
(Git blob `d23aa34ad11fe6cf27c68398b24ddb6b502ce970`), under Node v25.9.0.
The six-clause case below returned UNKNOWN at `maxSteps:2`, public propagation
reported conflict index 5, then an unbounded solve returned SAT with a model
that falsified the final original clause. This is a reference-solver false
SAT, not a Go port failure; Go tests separately assert that pending root work
survives the same budget/Propagate lifecycle.

```sh
node --experimental-strip-types --input-type=module -e 'import {Solver, makeLit} from "./code/packages/typescript/sat-solver/src/sat.ts"; const s=new Solver(); for(let i=0;i<5;i++) s.newVar(); const clauses=[[1,2],[1,-2],[-1,3],[-1,4],[-4,5],[-4,-5]]; const l=n=>makeLit(Math.abs(n)-1,n<0); for(const c of clauses) s.addClause(c.map(l)); const first=s.solve({maxSteps:2}); const mid=s.propagate(); const second=s.solve(); const model=s.getModel(); const checks=clauses.map(c=>c.some(n=>n>0?model[Math.abs(n)-1]:!model[Math.abs(n)-1])); console.log(JSON.stringify({first,mid,second,model,checks,allClausesSatisfied:checks.every(Boolean),conflicts:s.conflicts,decisions:s.decisions,propagations:s.propagations}));'
```

Historical output: `{"first":"unknown","mid":5,"second":"sat","model":[true,true,true,true,true],"checks":[true,true,true,true,true,false],"allClausesSatisfied":false,"conflicts":1,"decisions":2,"propagations":5}`. The private conflict index is retained only as historical output; neither shared corpus consumer pins it. The TypeScript regression now expects UNSAT after the same public lifecycle and independently brute-force checks the original signed formula. The shared Go/TypeScript corpus pins the portable conflict outcome and validates recovery against its original CNF.

Critical path: P003 -> P004 -> P005-011 -> P021-025. Fuzzing and measurements
can progress independently once their prerequisites are available.

### Shared lexer discoveries from P006b1

- **P1 / VER-LEX001** — Rust shared indentation lexer can stall on a leading
  hash when the grammar does not consume it as trivia. The original canonical
  Warrant probe timed out after one second while its valid control returned in
  0.187 seconds. The unchanged-cursor path is still present on fetched main
  `57bfc809e3eb`. See [the lexer progress contract](02-lexer.md#leading-hash-in-grammar-driven-indentation-mode).
  Validate unsupported input, an explicit hash token and legacy hash comments.
  BUILD-001 landed in #19086, so validate a fresh spreadsheet WASM and VisiCalc
  bundle with the build-at-use scripts. The generated outputs are ignored.
- **P006b2/P006b3 reconciliation** — #18992 and #18996 landed with a dedicated
  bounded Warrant layout scanner. It classifies `--` comment-only lines before
  comparing indentation. The shared lexer's treatment of other comment
  markers is a separate concern for grammars that use that API. Warrant's
  guarded parser does not depend on changing it.

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
| P1 / LV-002 | CI re-selects every adopter when its `.warrant`, a pinned fixture, `warrant-grammar`, `warrant-ir` or `ca-assurance-report` changes (build-tool consumer rule) | proposed; can ship inside the first leaf conversion's enablement PR Sequenced by [VER30](VER30-warrant-expansion-roadmap.md), Track A3. |
| P3 / LV-003 | Per-lane assumption-conformance tests (VER00 §7.5.3) written as small Warrant files and run through the VER04i corpus, moving `src.<lane>.<construct>` entries to `evidenced` | proposed; after the lane's WD-1xx runner exists |
| P3 / LV-004 | Write VER14 (per-lane WIR front ends over LANG84 versioned grammars, with the LP-001a differential method as the template) | proposed; after LP-001a has run once on gf256 |
| P1 / LV-005 | Layer contract linking (owner, 2026-11-12; VER27 §4.13, §4.14). A package's proved Warrant claims are published as catalogue-style ids (for example `contract.logic-gates.and`); an upper layer's `assume platform:` names them; the assurance report counts such an assumption as discharged only if the lower package's evidence actually proves that claim at the pinned revision, else it stays open. Needs its own specification before any code; the spec also settles VER27 conflict C14 (VER04 §3 and the closed VER13 family set). First Warrant feature with a concrete adopter (LV-006), so it complies with Q7 | spec drafted in [VER29](VER29-layer-contract-linking.md) (it settles C14 as a generated `contract` family); implementation is its five slices (section 13: id parsing, resolution and discharge, CI re-selection, reach-around check, amendments); adopter is LV-006 Sequenced by [VER30](VER30-warrant-expansion-roadmap.md), Track A9. Slice 7, "interface reach" (LV-008), is the VOS02 amendment: only the reach rules of section 8 and the feature rule of section 5.1.1 change; no id shape or section 3, 5.1(a)-(e), 6 or 9 text. |
| P2 / LV-006 | The Intel 4004 chain, a second independent stack and bottom-up adoption pilot (owner, 2026-11-12; VER27 §4.15), complementing the RV32I-first route of AS-004. Layers, in order: gates (`logic-gates` on `transistors`; truth tables), combinational (mux, decoder), arithmetic (`arithmetic`; 4-bit result and carry = `(a+b+c) mod 16`, 512 cases), sequential (flip-flops, registers), CPU step (`intel4004-gatelevel`; each instruction against the datasheet ISA model), programs (Nib to 4004, Busicom). Each layer proves its own contract and assumes the ones below by id (LV-005); contracts cover time where a constant-time claim needs it. One `.warrant` ISA model tests the Rust, C, C++, Go, Python, Elixir, Lua and Perl 4004 simulators. `bits.rs` in `intel4004-gatelevel` is outside the gate contract and needs its own claim or removal (no reaching around). A `U4` or fixed-width bits type is requested only if the chain needs it (P7). Proves nothing about real silicon | in progress: layer 1 (gates and combinational, LG01) done on branch claude/warrant-adopt-logic-gates (see LP-009); layer 2 (`arithmetic`, AR01) done on branch claude/warrant-adopt-arithmetic (see LP-011); next: sequential, then the CPU step; needs the LV-005 spec for upper layers to link by id, and the CPU-SIMULATOR-ROADMAP gate-integrity audits |
| P1 / LV-007 | Release standard (owner, 2026-11-12; VER27 §8.1). **Replaced 2026-11-23 by [VER31](VER31-tiered-release-standard.md):** a release tier is computed from the generated report and the assumption buckets (`code/assumptions/buckets-v1.json`), never typed; the minimum publishable tier is `Tested` (0.x); the original conditions (Warrant spec, verdict `pass` as "meets the bar", live jail evidence) gate any higher tier. Release authorization: hardware-key-signed tag plus the owner's approval of the protected `release` environment | spec: VER31, approved by the owner 2026-11-23; implementation: LV-007a..d |
| P1 / LV-007a | The VER31 gate: `code/scripts/release_gate.py` (tier, floor checks F1–F12, generated notes and `ASSURANCE.md`) with falsification tests; bucket-file coverage check in `assumption_catalogue.py`; `publish.yml` gains crates.io and npm OIDC jobs, a signed-tag check against `.github/release-signers`, the `release` environment, a `workflow_dispatch` dry run, and the gate in front of every language's publish job | in review: branch `claude/lv007a-publish-gate`; adds `.github/CODEOWNERS` and a placeholder `.github/release-signers` |
| P1 / LV-007b | First release candidate: `rust/ct-compare` (crates.io `coding_adventures_ct_compare`) and `typescript/ct-compare` (npm `@coding-adventures/ct-compare`) brought to the `Tested` floor: MIT licence and `LICENSE`, report and `ASSURANCE.md` shipped in the artifact, `security-review.json`, TypeScript property/fuzz suites and bench manifest, `dist/` build output; gate passing in a dry run; tag plan and generated notes handed to the owner. The owner publishes; nothing is published by an agent | proposed; after LV-007a |
| P2 / LV-007c | Python and Ruby under VER31: their publish jobs refuse until the package has a report; move RubyGems from `RUBYGEMS_API_KEY` to trusted publishing | proposed; after LV-007a |
| P3 / LV-007d | Report fields the VER31 ladder needs: `exhaustive.domain` (`"full"` makes the `exhaustive` rung reachable) and, once the buckets settle, buckets emitted by `ca-assurance-report` itself | proposed |
| P1 / LV-008 | VER29 slice 7, "interface reach" (owner, 2026-11-16; [VOS02](VOS02-interface-contracts-and-profile-linking.md) section 9.1, normative text AM1 to AM5). AM1: a path `<P::A as T>::f` whose trait `T` resolves by defining path to a trait of a direct dependency is an interface use `(crate, T, f)`, not `opaque` (section 8.2). AM2: in a package with exactly one `impl <ProfileTrait>` per profile type, each `type A = <path>` composes the use to the implementation item, matched through a new binding key `realises=<crate>::<T>::<f>` and then classified as usual (section 8.3). AM3: generics count as resolved for this one shape only (VQ13). AM4: manifests record the evidence build's features and `cfg`; a declared evidence-only feature that gates exactly one module item (plus `pub use` items from it) is not `build-config-unpinned` (section 5.1.1, VQ14). AM5: negative controls (two impl blocks for one profile type, a missing `realises`, a wrapper that does not forward). Requests no id shape, no VQ12 domain check and no section 9.1 change; VOS keeps interface conformance as a VOS-validator digest check | proposed; after VER29 slice 4 (the reach pass) and slice 2; slice 7 is added to VER29 section 13 after slice 4 (VER04n's slice 6 unchanged); estimate about 6 selected packages (the analyzer, `ca-assurance-report`, `warrant-grammar` bindings and the adopters); VOS-033 and VOS-034 do not wait for it |

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
| done / WD-003 | Rust Warrant compiler emits the language-neutral corpus for ct-compare Warrant v0; independent stdlib-only Python semantic/digest validator; byte-stable regeneration and falsification checks | PR #19177 merged as `d746f3b16bbbc78781ade0987abd88bc5a953e37`; explicitly unsharded ct_compare-v0 with no Python lane execution. [Current queue, receipts, and discoveries](VER-BACKLOG.d/portable-warrant.md) |
| done / WD-908 | Route Linux Lua prerequisites through bounded apt install retries after repeated CI DNS failures | PR #19177 merged; retry wrapper preserves both package lists and step timeouts, and hosted validation passed. See [portable Warrant receipts](VER-BACKLOG.d/portable-warrant.md) |
| P1 / WD-004 | Generalize ca-assurance-report beyond `rust/` (per-lane manifests, honest not-live rungs) | WD-004a merged in PR #19236; WD-004b1 lane artifact loader delivered in PR #19492; WD-004b2 report/CLI integration delivered on `claude/wd-004b2-report-integration` (not yet merged). See [portable Warrant queue](VER-BACKLOG.d/portable-warrant.md) Sequenced by [VER30](VER30-warrant-expansion-roadmap.md), Track D1/D2. |
| P2 / WD-1xx | One PR per lane: L0 corpus runner + sabotage + manifest + report for ct-compare (python, typescript, go, ruby, c, cpp, java, kotlin, csharp, fsharp, elixir) | Python (WD-101, PR #19354) and TypeScript (WD-102, PR #19368) runners and manifests are merged (report integration, sandbox and layer B still open; TypeScript also lacks a registry row); Go (WD-103, PR #19484) is merged (same open items, plus a registry row); the Go lane passes the WD-100 conformance kit (PR #19493); every lane must also pass the shared runner conformance fixture WD-100 (PR #19489; the Go lane adopts it in a follow-up); other lanes after WD-004b1/WD-004b2. Owner 2026-11-16: priority is evidence in every lane language (about 18); every new lane passes WD-100 and runs on corpus v2 once CV2-S6/S7 land; Go finishes on v1 then ports to v2; order after Go: ruby, java, csharp, kotlin, fsharp, c, cpp, elixir, lua, perl, haskell, swift, dart, ocaml, two at a time. See [portable Warrant queue](VER-BACKLOG.d/portable-warrant.md) Sequenced by [VER30](VER30-warrant-expansion-roadmap.md), Track D3 (Python, TypeScript, Go first, per owner 2026-11-14). |
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
| P1 / LP-002 | x25519: RFC 7748 known answers, DH commutativity, secret labels; CT proof needs VER20 lowering | after LP-001; **generator-path pilot done on `claude/warrant-bytes-n`** (`x25519.warrant`: RFC 7748 5.2/6.1 vectors, the clamp as a `Bytes<32> -> Bytes<32>` claim, low-order refusal; 3 operations, 8 claims, 9 examples, `Bytes<32>` domains sampled and labelled so; no secret labels, no report, no CT proof, so the row stays open); DH commutativity needs `Result` unwrapping in a claim body and is not written |
| P1 / LP-003 | lzw, then lzss, lz77, lz78: round trip + hostile decompress, exhaustive to length 3; needs an error channel (API change across ports) and `max_output` bound | after SEC-001; **lzw done as LP-010 without the API change** (it already returned `Result<_, String>`; an absolute 16 MiB cap stands in for `max_output`); lzss, lz77, lz78 and the other ports' caps remain Sequenced by [VER30](VER30-warrant-expansion-roadmap.md), Track B wave 1. |
| P1 / LP-004 | sha512, blake2b, sha1, md5 shared template (known answers, block-boundary sweep, bounds); thin without extension E | after LP-001 |
| P2 / LP-005 | des; then Tier 2 (wasm-leb128, der-tlv, protobuf varint, zeroize, bitset) as Warrant extensions land | after WX items |
| P2 / LP-006 | Add missing `required_capabilities.json` (sha512 found without one); audit all leaves | independent, small (haiku) |
| P2 / LP-007 | Negative/malformed-input tests absent in lzss, lz77, lz78, sha1, md5, x25519, range-coder, caesar/atbash, irc-framing | folded into each LP item |
| P1 / LP-008 | wasm-leb128 (first Warrant v1 adopter, with the PR-0 enablement fixes: `gen_evidence` passes host lines; report CWE-208 "not applicable" branch and `varint-codec` profile; `warrant-ir` `wasm_leb128_proofs`): LEB01 + `wasm-leb128.warrant`, every input of length <= 3 exhaustive per decoder, report-only (proofs not-live) | local branch `claude/warrant-adopt-wasm-leb128`; CWE-327/409 report branches left to the sha1/md5/lzw PRs |
| P1 / LP-009 | logic-gates stateless slice (second Warrant v1 adopter; layer 1 of the 4004 chain, LV-006): LG01 + `logic-gates.warrant`, 22 operations (11 scalar gates, 3 n-input gates, 8 combinational blocks), 88 claims, 3,225,774 exhaustive + 544,349 bounded-set (3,770,123 total) cases, bit domain and shapes rejected by panic in every profile, runtime jail LIVE; new audit profile `boolean-logic` (the `varint-codec` sentences name LEB128); findings SEC-007, SEC-008; `sequential.rs` deliberately out | local branch `claude/warrant-adopt-logic-gates`; next slice: sequential (LG01 "Next slice": needs WX-G `inout` and WX-F drivers with trace enumeration) |
| P1 / LP-010 | lzw (third Warrant v1 adopter and the first decompressor; LP-003 slice 1): LZ01 + `lzw.warrant`, 2 operations, every `decompress` input of length <= 3 exhaustive (16,843,009), `compress` wire image of every 1- and 2-byte input, round trip over the `Bytes` domain, runs and byte pairs, a real decompression bomb built in the document from a 279-byte unit, `output`/`heap` bound rows with every instance observed (first document to use WX-H), runtime jail LIVE; new audit profile `lossless-compression` and **CWE-409 derived from the bound rows instead of excluded for every package** (`--check` re-derives it, both directions); findings SEC-002 (bomb, contained by a 16 MiB cap and a linear dictionary, no API change) and SEC-002a (quadratic encoder, wrapping header); the `max_output`/error-channel API change stays open (LP-003 for lz77/lz78/lzss and the other ports) | local branch `claude/warrant-adopt-lzw`; next: lzss, lz77, lz78 on the same template |
| P2 / LP-010a | CWE-409 `output` magnitude: `--check` proves that every `output-bound` suite ran (`cases + iterations >= 1`) and carries both a peak and a bound, but not that the bound the manifest cites (`bound_at_max`, the evaluated `heap` row) is derived from the `output` expression the Warrant document declares (for lzw `imin(16777216, 32768 * len(data))`): the test manifest records only the evaluated heap bound at the peaking instance, not the `output` expression or its value, so the cross-check needs the manifest writer (`gen_evidence`) to record the evaluated `output` bound per entry before `decompression_bounds` can compare it with the document | after LP-010; found by the CWE-409 security review (low) |
| P2 / LP-010b (narrowed) | CWE-409 profile pin: `verify_profile` runs only when `--check` is given `--profile`, and library callers of `verify_evidence` never run it, so a future decompressor (lzss, lz77, lz78) whose BUILD omits `--profile`, and whose Warrant document has no `output` row, verifies an `Excluded` CWE-409 row under a recorded `comparison` profile. The recorded profile also feeds only the CWE-409 check: the static CWE-326/327/407 sentences are not re-derived from it. Fix: require `--profile` with `--check` (or make `verify_evidence` take the expected profile), or derive "decompresses" from the Warrant document (an op that expands, or an explicit marker) so the profile cannot be chosen by the BUILD author | after LP-010; found by the CWE-409 fix re-review (low) | **Narrowed by VER04h section 9 item 22**: the profile is now a mandatory hash-bound file with no default and no `--profile` flag to omit, and `verify_evidence` always checks its kind; still open because "decompresses" cannot be derived from a document that omitted its `output` row, so a decompressor may still declare `kind: "transform"` (review-visible only). Sequenced by [VER30](VER30-warrant-expansion-roadmap.md), Track A5; the BUILD pin now binds the kind too (`--profile-kind`). |
| P1 / LP-011 | arithmetic adders and ALU (third Warrant v1 adopter; layer 2 of the 4004 chain, LV-006): AR01 + `arithmetic.warrant`, 5 operations (half adder, full adder, two ripple-carry adders, the ALU with six operations and four flags), 63 claims, 2,655,437 enumerated cases = 859,848 exhaustive + 1,279,161 sampled + 516,428 bounded-set (exhaustive: every input at 1 to 4 and 8 bits; 16 bits SAMPLED and named so), bit domain and shapes rejected by panic in every profile (inherited from the logic-gates guard, pinned in release), runtime jail LIVE; new audit profile `bit-arithmetic` (`boolean-logic` says "single-bit values only", false for a 16-bit adder); domain checks found NO defect in this crate (no SEC row); the layer would assume the logic-gates truth-table and rejects claims by id, pending LV-005 | local branch `claude/warrant-adopt-arithmetic`; next: the CPU step (`intel4004-gatelevel`) consumes it; needs LV-005 to link by id |
| P1 / LP-012 | Warrant case counts: the generated harness, the test manifests and `ca-assurance-report` count every case run in "exhaustive" mode as exhaustive, even when the enumerated domain is a sample (`edge(N)`, e.g. the `*_16_bit_sampled` claims) or a bounded set (`Bytes set(347)`). The headline figure cannot be trusted without a per-claim audit: arithmetic's 2,655,437 is 859,848 exhaustive + 1,279,161 sample + 516,428 bounded-set; the lzw and logic-gates figures need the same split. Design only, not implemented: the binding or claim declares its domain kind (`full`, `sample` or `bounded-set`); the manifest records counts per kind; the report shows only `full` as exhaustive and the others under their own names ; **partly landed (WX-N, `claude/warrant-bytes-n`)**: the v1 manifest `domain` text of a `Bytes<N>` binder now says `full` (N <= 1) or `sample` (N >= 2) instead of `set`, gated so no existing harness changes; the per-kind counts and the report change are still open; until they land, suite-level "exhaustive" totals (and the negative-control kind `exhaustive`) include `sample` `Bytes<N>` cases, and the per-claim `domain` field is the authoritative label Sequenced by [VER30](VER30-warrant-expansion-roadmap.md), Track A2. |
| P3 / LG-001 | logic-gates: replace `(n as f64).log2()` (mux_n, demux, encoder, priority_encoder) with `trailing_zeros()`, and the closures/`reduce` idioms the proof front end refuses, before a steps-bound proof is attempted; found as proof refusals (VER27 section 7.4, P11), not defects | open |
| P3 / ARITH-001 | arithmetic: the proof front end refuses `adders.rs` (the `(u8, u8)` tuple return type and `let (s, c) = ...` of `half_adder`, `full_adder` and the ripple-carry loops) and `alu.rs` (the `enum AluOp` item), so every steps-bound obligation is `not-live`; found as proof refusals (VER27 section 7.4, P11), not defects. Reshape (a small `Sum { sum, carry }` struct) or request the feature before a steps-bound proof is attempted | open Sequenced by [VER30](VER30-warrant-expansion-roadmap.md), Track E. |

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
| P1 / WX-D1 | Implement VER04k §2.1 sequence constructors `take`, `drop`, `slice`, `replicate`, `range`, `to_list`, `to_bytes`; unlocks a faithful `gf256` multiply model, general `wasm-leb128` model, and sequence pre/postconditions for streaming hashes and writers | WX-H/WX-E merged #19121; alongside WX-F/G baseline work; compiler/checker/evaluator/corpus update with boundary and negative tests per VER04k §2.15 Sequenced by [VER30](VER30-warrant-expansion-roadmap.md), Track C3. |
| P1 / WX-F | Implement VER04k §3 binding-level constructor/step/finish drivers; enables real streaming SHA-family, range-coder and rANS sequences | alongside WX-D1 and WX-G; split-point sweeps that use `slice` follow WX-D1; each driver is binding-validated, runs subject operations across every sequence element, and kills missing/reordered/duplicated-step controls Sequenced by [VER30](VER30-warrant-expansion-roadmap.md), Track C2. |
| P1 / WX-G | Implement VER04k §4 `inout` and `ensures`; enables zeroize post-state evidence and output-buffer writers such as sqlite-file varint and TLS ALPN | alongside WX-D1 and WX-F; sequence postconditions that use list/byte built-ins follow WX-D1; checker rejects inadmissible/hidden post-state, generator binds mutable adapter slots, and both true and false postconditions are falsified **First slice landed on `claude/warrant-wx-g` (VER04k 4.13):** `inout` and `ensures` in the parser, checker (W0713) and generator; host forms `slice vec array int bool struct`, five faults, monitored calls and `<op>/post` suites; pilot `code/specs/zeroize.warrant` (4 operations, 9 claims, 5 examples; labelled `full`/`sample`/`set`/`edge` domains; stale-post, skip-last-write and cursor-off-by-one caught; no secret labels, report or jail; CWE-14, dead-store elimination of a buffer that is never read, is invisible to this evidence, and `zeroizes_every_byte_of_the_347_buffer_set` is a bounded set of 347 buffers, not an exhaustive claim). Still open: host forms `returned`, `slice-of:struct:`, `vec-of:struct:`, `codec:` (VER04m S1), WX-D1 forms in generated rows (the pilot uses a hand-built zeros model), the v1 corpus emission (`corpus::inout_descriptor` is ready), the sqlite-file varint and tls13-alpn pilots. Sequenced by [VER30](VER30-warrant-expansion-roadmap.md), Track C1. |
| P2 / WX-D2 | Implement VER04k §2.2 first-order bounded `fold`, `map`, `count`, `all`, `any`; enables reusable list/byte reference models for hashes, codecs and bitset | after WX-D1, WX-F and WX-G; validate typing, model purity, and evaluator budget exhaustion as specified Sequenced by [VER30](VER30-warrant-expansion-roadmap.md), Track C3. |
| P2 / WX-D3 | Implement VER04k §2.3 bounded symbolic unrolling for sequence forms; enables selected WIR proofs over loop-based `gf256` and codec implementations | after WX-D2 and the corresponding WIR lowering; proof rows remain `unchecked` until source lowering and certificates are independently verified |
| P1 / VER30-A1 | Data-driven assurance-report profiles: a package adopts a report profile by adding a data file beside its `.warrant`, so an adoption no longer edits `ca-assurance-report` code (shared-crate conflicts; every edit rebuilds 554 packages). Lands alone; fold in AR-001 | none; first item of VER30 track A, see [VER30](VER30-warrant-expansion-roadmap.md) |
| P2 / VER30-INV | Land the Warrant inventory scanner (extract, classify, analyze, summarise) as a tested, ruff-clean `code/scripts/warrant_inventory.py` that regenerates `code/specs/data/warrant-inventory/inventory-YYYY-MM-DD.csv` and the summary, carrying forward the CSV hand-overrides | after VER30; scripts are not committed with it, see [VER30](VER30-warrant-expansion-roadmap.md) section 6 |
| P1 / WX-B2 | `Bytes<N>` (`[u8; N]`) and fixed-array (`[T; N]`) parameters and results in the evidence generator; 128 and 116 crates have them as a primary blocker, 17 unblocked by `Bytes<N>` alone (crypto: x25519, ed25519, chacha20-poly1305, hmac) | none; VER30 track A6, see [VER30](VER30-warrant-expansion-roadmap.md) |
| P3 / WX-I128 | `u128`/`i128` in the Warrant type system, evaluator and generator; primary blocker for 72 crates, unblocks none alone | after WX-B2; VER30 track A7 |
| P1 / WX-T0 | Warrant TEXT specification: [VER04l](VER04l-warrant-text.md) defines `Text`/`Char` as Unicode scalar sequences (no normalisation, case mapping, graphemes or locale), `len` as a type error without a written unit, per-lane `ill_formed = unreachable \| rejects \| unspecified` (`unspecified` always printed as an open exemption), the header `v2`, TFV `S<hex>` plus two stream tags, boundary rows, five text faults, a 14-lane hazard table and the pilots | spec drafted (PR pending); owner decisions of 2026-11-14 recorded in VER04l section 1; docs only |
| P1 / WX-T1 | Implement VER04l slice T1: `v2` header, `Text`, `Char`, literals, `utf8 from_utf8 chars text code char_of`, domains `char_full`, `char_edge`, `text_d1`, TFV and stream encodings, boundary rows, five faults, `ill_formed` policies, Rust and Python lanes with the Python oracle; proofs `not-live`; pilots `caesar-cipher` + `atbash-cipher`, `url-parser`, `http1` (sub-slices T1-S0..S4 in VER04l section 12.3) | after WX-G, WX-D2, WX-F, LP-012 and stateful S1 (owner order, 2026-11-14); WX-AD ships in the same slice; measure `rcg-baseline-1` on the first pilot |
| P2 / WX-AD | Alphabet-domain built-in `within(xs, alphabet, k)` for `Text`, `Bytes` and `List` (the "WX-I" of the design memo; renamed because WX-I already means recursive sums): a top-level conjunct with a ground alphabet and literal `K` gives the binder the domain `A^{<=K}`, labelled `bounded-set`, never `exhaustive` | specified in VER04l section 4; shared with the stateful-object work; implemented with WX-T1 |
| P1 / WX-S0 | Spec stateful objects (`&mut self` method protocols): extends the WX-F driver, bounded call sequences, a reference state model, fault behaviour; the 4004 and RV32I CPUs are the acceptance test. Requirements in VER30 section 4.3; design from the separate memo **Spec drafted (PR pending):** [VER04m](VER04m-warrant-stateful-objects.md) (owner decisions of 2026-11-14 recorded in its section 1); it amends VER29 H3 (sections 6.1, 6.2, 14, D26) and VER04k 4.3/4.6/4.12 in the same series. Open: re-measure the inventory with explicit-state, protocol and text-only tags before scheduling adopters (VER04m O-8) | owner 2026-11-14 lifted the freeze; 268 crates once text exists; owner order 2026-11-14: WX-G, WX-D2, WX-F and LP-012's count split, then S1, then text T1, then S2, then S3 |
| P1 / WX-S1 | Implement slice S1 of VER04m: explicit state through WX-G `inout` (`state=explicit`, `inv=` and `states=` reach classes, the `codec` line, `omit=`, `strmap`, `inout List<record>` host forms, the four state-completeness obligations, `frame-leak` faults, manifest and report lines, the VER29 H3 classifier rule). Pilots in order: `logic-gates::sequential` first (the 4004 chain's layer 2: `d_flip_flop` 64 cases, `register` width 4 8,192, `shift_register`, `counter`; the first `exhaustive` label on a stateful layer), then `pixel-container` (variable-size state, `bounded-set`), then a first group of 5 to 10 `intel4004-simulator` instructions (codec, footprint and frame labelling, error leaves state unchanged), with `riscv-simulator` `decode` as a fourth | after WX-S0 approval; technical prerequisite is WX-G only, no text; owner order puts WX-D2, WX-F and LP-012's count split first, and LP-012's split is a hard prerequisite of shipping (an R3 family would otherwise count as exhaustive); `shift_register`'s rejection of other direction strings stays `uncovered` until WX-T1 |
| P1 / WX-SV | By-value explicit state (owner request through [VOS02](VOS02-interface-contracts-and-profile-linking.md) section 9.3, 2026-11-16): a carve-out of WX-S1 for steps of the form `op(State, In) -> (Out, State)` with no `inout`, so it must **not** depend on WX-G (WX-S1 is queued behind WX-G, WX-D2, WX-F and LP-012). Items: (1) the VER04m E11 `codec` line (`build=fn:`, `observe=`) for by-value argument and result positions, with E15's `codec-roundtrip` and `rebuild-equivalence` required for the H3 rung once it exists [P2]; (2) VER04j section 4.8 faults applied **inside tuple components** (VER04j 14.1 item 23 reaches only a top-level result or the inside of `Ok`/`Some`; the acceptance spike found no fault applies to `(Variant, Record)` and that every `op` needs one), plus `drop-state-update` (the by-value twin of `stale-post`) and a fault that changes a record field whose value is already zero (`field-zero` is inert on a zero initial state); `const-variant:<Ctor>` only if a component-level `wrong-ctor` does not exist [P1: the negative controls of refinement and of `-any` coverage]; (3) VER04m E8 `closure:` and `init:` lines for `state=none` steps and the `lemma` entry kind (E18, V9), which covers VOS-D005 [P2]; (4) a manifest `config` object fed by a `config=<NAME>` key on nullary ops, printed in labels [P2] | proposed; spec first (an amendment to VER04m and VER04j section 4.8); item 2 is the first to schedule; the VOS ghost face (VOS02 section 4) is the workaround until it lands, and its interim projection vocabulary sunsets with item 2; **VOS need (owner, 2026-11-25, VOS-D122):** VOS Priority-order lines 16b (item 2) and 16c (item 3) schedule items 2 and 3 of this row before VOS item 17 (`thread-table`), as VER-lane work; the ratings above are unchanged |
| P1 / WX-SV2 | A fault for a record result that is never inert (found at VOS-011, VOS-D065): the fault set of VER04j 4.8 has exactly one fault for a single-constructor record, `field-zero`, which changes nothing when the field is already 0, so a by-value `init() -> Impl` over a zero state can never have a negative control that passes (3 of 64 tests fail in `vos-sched-fp-bitmap`; every `op` needs at least one fault). Add `field-flip`: the lowest bit of the first numeric field is flipped (applicable exactly where `field-zero` is), emitted from a prelude region enabled only when a binding names it so no existing harness changes byte for byte; spec amendment to VER04j 4.8 first. Subsumes the second half of WX-SV item 2 | VOS-D065; `warrant/check_evidence.sh` of `vos-sched-fp-bitmap` fails the day it lands, so the exception is removed with it | **done (VOS-038a):** `field-flip` in VER04j 4.8; gated prelude region (`fieldflip`), no existing harness changed (13 Rust packages swept with `gen_evidence --check`); also `WARRANT_EVIDENCE_ATTACK=<op>`; bound in the three VOS schedulers, fp-bitmap script deleted. `vos-sched-rr`'s script was narrowed, not deleted: its `init` control had only `rep_init` as a member until VOS-D098 closed it (every calling claim attacks the control) |
| P1 / WX-S2 | Implement slice S2 of VER04m: protocols through a WX-F `dispatch` driver on a `Call` variant (`abs` observer, `reset=`, trace domains with `within`, `observer-pure`, `instance-isolation`, `reset-equals-new`, `drop-runs`, `skip-call` faults, the `retained` bound metric). Pilots: `irc-framing` (smallest; the `retained <= 1024` row is expected to fail on the current crate, SEC-011), `block-ram` `SinglePortRAM` (`dump` as `abs`), `lz78` `TrieCursor`; `der-tlv` `DerCursor` is plain WX-F and goes first | after WX-S1, WX-F, WX-D2 and the alphabet-domain predicate of VER04l (WX-T1); trace claims are `bounded-set`, never `exhaustive` |
| P2 / WX-S3 | Slice S3 of VER04m, deferred and not yet specified in detail: the `object` block as checked sugar, the derived inductive row (`exhaustive (by induction over: ...)`, owner-enabled, in `tcb`), published `contract.<pkg>.state.<op>` facts | after two S2 adopters have written the long form and WX-SP1 exists to calibrate the derived row |
| P1 / WX-SP1 | Mode E functional-equivalence obligation (a new VER04h section 7 kind, `functional-equivalence`; VER04m 7.3 item 1): a bit-blasted miter between the lowered Rust step and the lowered Warrant model for `d_flip_flop` and a 4-bit `register`, with a negative control (a sabotaged step yields a counterexample); lifts `proofs_not_live=state` for the operations it covers and gives the first `proved` stateful layer, calibrating VER29 VQ4 | funded next after WX-S1 (owner 2026-11-14); needs its own spec slice first |
| P1 / VER-SAT1 | Streaming DRAT proof sink for `cdcl-sat` (from [VOS30](VOS30-proof-route-and-capacity.md) section 10, VOS-D044): today `Solver::enable_proof` keeps every lemma and every deleted clause in memory (`solver.rs:375`), about 350 MB of proof and 1.8 GB of resident memory per million conflicts in the advisor calibration; add a sink that writes DRAT to a caller-supplied `Write` as it is produced, keeping the in-memory form for tests. Rules: the digest of the stream is computed over the bytes **as written** (a hash of a different buffer than the one on disk proves nothing); a **short or failed write fails closed** (the solve returns an error, never an `Unsat` with a truncated proof); and the checker's own memory on a large DRAT is accounted for (`drat-check` reads the whole proof, so a streaming writer without a streaming reader only moves the problem; either bound the proof or add a streaming read, reviewed as a kernel change). Does not change what `drat-check` accepts | queued; a likely prerequisite of VOS-029 |
| P1 / VER-SAT2 | Certified assumption-based UNSAT (from VOS30 section 6.1, VOS-D045): after `solve_with_assumptions` returns UNSAT, log the clause of the negated failed assumptions as a DRAT lemma, and let `drat-check` accept a proof that derives a stated clause (VER11 section 4 reserves this); needed to certify cut-point composition, BMC and k-induction incrementally. Soundness rules: the certificate binds `(original CNF, stated clause)`; segment hypotheses are **assumptions, never permanent clauses**; and the check is implemented **without a new accept mode** by the caller appending the negated stated clause as unit clauses to the original formula and running the existing `drat-check` for the empty clause, so the trusted kernel does not grow (any genuinely new accept mode needs its own kernel review). Default limits stay as they are (VOS-D042) | queued; VOS-029 can start without it by using one CNF per cut |
| P1 / WX-O0 | Spec opaque types: VER04n (nominal `opaque T` as a construction recipe, makers and observers, adequacy, immutability and unforgeability obligations), with VER29 (`contract.<pkg>.opaque.<T>` ids) and VER04m amendments; foreign handles only through VER29 published lower contracts. Written on another branch; adversarial review before implementation | none; owner decision 5 (2026-11-16), VER30 track C8, see [VER30](VER30-warrant-expansion-roadmap.md) |
| P1 / WX-FL0 | Spec floats: VER04p (integer-only softfloat model in every lane, never host libm; lanes failing the environment probe are `not-live`; values travel as bit patterns; float `==` is a type error, use `bits_eq`, `same` or `ieee_eq`). Written 2026-11-16; the 2026-11-18 domain/dependency errata clarifies representable edge encodings versus conversion/midpoint witnesses. The first combined adversarial review remains historical in VER04p §20. Scoped role findings on candidate `c69bbb0` are recorded in §18; final acknowledgements of the revised candidate remain required before FL1-S0. The bounded rational diagnostic is not native/compiler evidence. | no dependency; **errata in progress**, see VER04p §§11.1.1, 17–20 and AF-05a in [AUDIO-WORKSTATION-BACKLOG](AUDIO-WORKSTATION-BACKLOG.md); VER30 track C9 |
| P1 / WX-M0 | Spec collections plus the first generics slice: VER04o (`Set`/`Map` as mathematical values, one canonical `ord` order, an unspecified-order host cannot bind to a `List` result; generic claims never `exhaustive` except `all-instances(n/n)` over a closed universe, coverage per lane and instance). Another branch writes it; adversarial review | after WX-O0 (VER04n); VER30 track C10, see [VER30](VER30-warrant-expansion-roadmap.md) |
| P1 / WX-O1 | Implement VER04n opaque types; pilot `rsa-public-operation` (stage P-a local first; stage P-b waits for WX-O4a, WX-O4b and the `x509-rsa-public-key` adoption, owner decision 17, 2026-11-16) | after WX-O0, WX-G, WX-D2, WX-F, LP-012, WX-S1 and WX-T1; first of the four new features in the `warrant-grammar` queue; VER30 track C11 |
| P2 / WX-M1 | Implement VER04o collections; pilot `csv-parser` | after WX-O1 and WX-M0; VER30 track C11 |
| P1 / WX-FAULT1 | Negative control for a fixed-width integer result of a composite-adapter operation (`Bytes -> U32/U64`): accept `flip-result-bit` for a `Fx` result in `composite_adapter` and add `V::Fixed(f, x) => V::Fixed(f, x ^ 1)` to `fault_flip_result_bit`; add a generator test; found by WX-P1a, blocks it and any checksum or non-cryptographic hash adoption | **Done; merged as #19508.** The arm sits in a new `fixedfault` prelude region emitted only when an operation with that control has a fixed-width result, so all eight existing adopters and the WX-N/WX-V fixtures regenerate byte-identically; tests in `warrant-grammar/tests/wxe_edges.rs`; VER04j 14.1 item 46 updated. `warrant-grammar` queue; WX-P1a may resume |
| P1 / WX-FAULT2 | Add a narrowly scoped `bool-field-flip` negative control for composite record results with a direct Boolean field, discovered by the real `fixed-point` nested `SaturatingResult { value: Q16, overflowed: bool }` admission attempt. The existing `field-zero`/`field-flip` faults intentionally cover numeric fields only, and direct-field traversal cannot reach the nested raw field. Define one opt-in attack on the first direct Boolean field in declaration order; accept a declared record directly or inside exactly one `Result`/`Option` wrapper, rejecting nested/multi-wrapper shapes with W0701. Preserve constructor, all nested fields and all other flags. Add true/false non-vacuity, nested-only/non-Boolean/variant rejection, `Ok`/`Err`/`Some`/`None` preservation, generator/prelude gating, operation-target applicability controls, scoped negative-manifest controls, and byte-identical regeneration for every existing adopter. Do not widen existing fault semantics. | GBL-020a1 prerequisite; actual `gen_evidence` diagnostic on the real nested saturation export: W0701 fault applicability, after the generator's every-operation-fault requirement. |
| P2 / WX-FAULT3 | Audit whether legacy composite faults that use the one-layer `map_payload` helper are inert or misapplied for nested wrappers such as `Result<Option<Record>, E>` and `Option<Result<Record, E>>`. Produce a bounded synthetic repro with an observable second-layer payload, compare W0701 admission against actual fault replay, and identify affected public adopters before changing behavior. Keep this separate from WX-FAULT2; no public adopter bug is confirmed yet and no legacy fault behavior changes until the audit is complete. | Open; raised during GBL-020a1 admission review. |
| P2 / WX-SEED1 | Audit whether the shared generated `fuzz_claim` initial corpus gives meaningful coverage of wrapper-result branches before mutation. The fixed eight seeds are `[m, 4, ...]`; `decode` reads its stream field at position 1 and the first Boolean becomes `4 % 2 == false` for every seed, so early `Ok`/`Some` branches can depend on later mutator changes. A bounded WX-FAULT2 fixture repro surfaced `negctl/fuzz/wrapped` at a 32-mutation budget; this does not establish universal wrapper or adopter failure. Record seed callbacks separately from mutation callbacks, and report whether the targeted success/error branches are reached. Keep full claims and do not alter corpus behavior globally without broader evidence. | Open; observed while validating WX-FAULT2, source `warrant-grammar/src/evidence_prelude.rs.txt::fuzz_claim` and fixture `tests/fixtures/wxfault2.warrant`. |
| P1 / WX-P1a | Verify `hash-functions`: a Warrant document and evidence of its own, adopting its T0 functions first. Its primary blocker `Bytes<N>` has landed; per the 2026-11-16 inventory it is T1 with secondary blockers floats, generics and text. The only path dependency of `hash-map` | owner decision, 2026-11-16 (VER04o OWN-6), consistent with decision 17; before the WX-P1 pilot; VER30 track C11. **Status 2026-11-17: complete (WX-P1a and WX-P1a2).** Contract [DT17a](DT17a-hash-functions-warrant.md), `hash-functions.warrant` (8 byte-string hashes; hash_str_*, the trait and structs listed as not covered; `steps`/`depth` bound rows on every operation and `heap <= 4096` on the five ops without 2^24-case claims; `fnv1a_32`, `fnv1a_64` and `djb2` have no `heap` row because the allocation observer would add about half an hour), `evidence.bindings`, the generated harness (277 tests, all pass; first user of WX-FAULT1's `V::Fixed` arm), 9 sabotage lines in BUILD/BUILD_linux/BUILD_windows, the VER08b jail test, the proof manifest via `warrant-ir/tests/hash_functions_proofs.rs`, and (WX-P1a2) the capability analyzer certificate, `assurance-profile.json` and `assurance_report.json` with their BUILD lines. Closed by splitting the OS-randomness `avalanche_score` and `distribution_test` out of the crate into the new `rust/hash-analysis` (owner decision 2026-11-16): `hash-functions` 0.2.0 declares `capabilities: []` truthfully, the jail's rand-syscall control stays detected, and the report shows CWE-330 excluded, CWE-400 and CWE-208 as unapproved exemptions (no live step-bound or constant-time proof), verdict report-only. Follow-ups outside this row: a live step proof (front end refuses `algorithms.rs`), a SipHash constant-time proof, `hash_str_*` (WX-T), a `capabilities.rs` witness so `--verify-cert` can run (LP-001b) |
| P2 / WX-P1 | Implement the first generics slice of VER04o; pilot **`hash-map`** (needs collections, generics and stateful objects; only path dependency `hash-functions`) | after WX-M1, WX-S1 and WX-P1a; VER30 track C11 |
| P2 / WX-P1b | Second generics adopter: `hash-set` (the VER04o section 15 design), on fully verified dependencies | after WX-P1 and WX-P1a; VER30 track C11 |
| P2 / WX-FL1 | Implement VER04p floats (softfloat model per lane, environment probe, bit-pattern corpus encoding); `within_ulps` strict at infinity, Go float evidence amd64-only, `f32_all` on compiled lanes only (decision 18) | after the WX-FL0 §18 role-specific review and §17 FL1-S0 errata/compatibility gate; preserve VER30 C11 owner order WX-O1, WX-M1, WX-P1, then WX-FL1 and its scheduled prerequisites WX-G, WX-D2, WX-F, LP-012, WX-S1, WX-T1. These are roadmap queue requirements, not an assertion that every scalar float operation requires every feature. TRIG-CONV gates the `trig` pilot in FL1-S6 only. The independent audio filter normalization fix AF-01a is not a prerequisite. `fclass` and `f_to_int` claims remain `not-live` until FL-17 class-preserving boundary evidence and FL-18 exact wide-integer result support exist for the affected lane. VER30 track C11 |
| P2 / WX-FL2 | Restore the parked floating-point documents `code/specs/collision2d.warrant.draft` and `code/specs/paint-batch-tiles.warrant.draft`, `code/specs/game-breakout.warrant.draft` and `code/specs/game-pong.warrant.draft` and `code/specs/sim-crt-vector-beam.warrant.draft` to `.warrant` and make them pass `parse_document` and `check_warrant` (they use `F64`) | after WX-FL1 |
| P1 / VER30-INV2 | **Done 2026-11-16.** Re-ran the Warrant inventory with a `recursive-types` tag: the four unfrozen features alone add 432 crates beyond a baseline of 492 (down from 566, because 74 crates need recursion); with recursive types 775 crates (2,028 ports); recursive types alone 74; 38% of the 701 crates behind the four also need recursive types. No real cryptography becomes newly reachable; the gain is parsers and codecs. Data: `data/warrant-inventory/summary-2026-11-16.md`, `inventory-2026-11-16.csv`, `method-delta-2026-11-16.md` | done; owner decisions 6 and 14 (2026-11-16), see [VER30](VER30-warrant-expansion-roadmap.md) section 3.4 |
| P2 / WX-R0 | Spec recursive algebraic types: **VER04s**. **Written, PR #19501** (owner decision 15, 2026-11-16); spec first, adversarial review done (see WX-R-REVIEW). Takes no stream tag; diagnostics `W0770` to `W0779` | written after WX-M0 (VER04o); VER30 track C14, [VER30](VER30-warrant-expansion-roadmap.md) section 3.4 |
| P2 / WX-R-REVIEW | Adversarial review of VER04s: **done** (2026-11-16, section 21 of VER04s). **Still owed before WX-R2a:** the lane-specialist read of VER04s section 8.3 (per-lane table) and RL1a, against a running host | before WX-R2a |
| P3 / WX-R1 | Implement VER04s recursive types (slices R1-S0 to R1-S5); pilot `canonical-cbor`, a **partial** adoption (3 of 8 entries: `encode`, `try_encode`, `decode`; the other five tabled as not covered). Unlocks the parsers and codecs of VER30 section 3.4. **Owner decision OWN-R1 (2026-11-16): keep the VER30 order**; only groundwork slice **R1-S0** (errata pass, fuel-total measurement) may start early | after WX-R0 and after the four features (WX-O1, WX-M1, WX-P1, WX-FL1) except R1-S0; also T1, WX-D2, the `ir.seq` amendment (VER04r 4.1), VER04o C1-S1; last item in the `warrant-grammar` queue; VER30 track C15 and decision 25 |
| P3 / WX-R2a, WX-R2b, WX-R2c | VER04s lane rollout, one recursive fixture-port package per lane (BUILD, README, CHANGELOG, `required_capabilities.json`): R2a typescript, go; R2b ruby, c, cpp, java, kotlin, csharp, fsharp, elixir; R2c lua, perl, haskell, swift, dart, ocaml | R2a after R1-S5 and the lane's v2 runner (VER04r S6, S7); R2b after R2a; R2c after R2b; all after WX-R-REVIEW's lane read |
| P3 / WX-R3 | VER04s slice R3: generic recursive types (`Tree<T>`; regular recursion accepted, polymorphic refused) | after WX-R1 and WX-P1 (VER04o G1-S2) |
| P3 / WX-R-PILOT2 | Second recursive-types adopter: `bounded-json` (OWN-R2, 2026-11-16), then `json-serializer` and `json-value` | after WX-R1 and FL2 |
| P3 / WX-R-AMEND | VER04s amendments, each a PR of its own (VER04s section 17, 19.2): VER04j, VER04k, VER04o, VER04r (the `ir.seq` meter); VER04i (registry rows and lane table); VER13 assumption `A-REC-HOST`; VER29 published-claim field (scope and canonical type digest, RT9). Details in [portable Warrant queue](VER-BACKLOG.d/portable-warrant.md) | before the slice that relies on each; R1-S0 re-reads them |
| done / TRIG-CONV | Converge the `trig` ports (seventeen on `origin/main`, not sixteen: the fifteen established lanes plus C and C++) on one reference algorithm (term count, range reduction with a stated result for NaN, infinities and out-of-range quotients, operand order, `radians`/`degrees` grouping, sign of `sin(-0)`): **a `trig` specification first**, then **every port updated in its own PR**. This is VER04p FL-15. **Spec written 2026-11-16: [PHY00a](PHY00a-trig-convergence.md)** (survey, the TRIG-REF-1 algorithm, accuracy, per-language bindings, the bit-exact conformance corpus `fixtures/trig-reference-v1`, migration slices TC-0 to TC-18, open questions OQ-1 to OQ-14; adversarially reviewed once: arguments beyond 2^29 return NaN instead of a folded value, and four questions are left for the owner); **TC-1 landed 2026-11-16:** the bit-exact fixture `fixtures/trig-reference-v1` (62,154 vectors; generator `code/scripts/generate_trig_reference_fixture.py`, soft-float oracle, exact-rational `cr`, C and Rust cross-checks, `bc` check, regeneration `--check`; measured worst case 2.38 ulp `sin` / 2.37 `cos`, gate 3) (fixture-only: no port changed; Python, Rust and TypeScript converge together in TC-2 because they share the `music-machine` golden). The review of the first draft's gates found one typo in the spec's `PIO2_3` hex float (fixed) and an over-long "cannot be seen" list, which a later review showed false: eleven of those constants and sites are now directed vectors and 10 constants and 15 sites remain masked after a 1.2e9-input search per item (PHY00a 6.4); `tan` rows carry a correctly rounded column, Part II rows do not (TRIG-CONV-CR2). **TC-2 (music trio) done in its own PR:** Python, Rust and TypeScript `trig` converged together with `oscillator`/`musical-instruments`/`percussion-kit`, and the shared `music-machine` golden changed once (`15c793ee...` -> `25bc4eb1...`). **TC-1b landed 2026-11-16 (spec only, by reading `origin/main` a4723983852, nothing run but one Python `atan` probe; PHY00a 8.1):** the survey of the fourteen unconverged ports (kernels, reduction, Part II, versions, callers, goldens), a bit-for-bit estimate with the language obstacles, and eight corrections to the spec (Dart and Haskell `atan2` of NaN is `+-HALF_PI`, not `0.0`; F# and Haskell `atan` have no early exit but are bit-equal; Haskell dependents pin `trig <0.2`; `go/oscillator` is an unreduced-angle caller; Swift `sqrt` traps). **No shared bit-exact golden exists among the fourteen** (only `music-machine`, done in TC-2), so no further cross-language PR is forced; every port moves with its own in-language dependents. **Complete 2026-11-16: all seventeen ports converged and merged**, each bit-exact against `trig-reference-v1`: Python, Rust and TypeScript in TC-2 (#19506); C #19510, C++ #19511, Ruby #19512, Java #19514, Haskell #19515, Swift #19517, C# #19518, F# #19519, Kotlin #19520, Lua #19521, Elixir #19522 (raises `ArithmeticError`, 85 unrepresentable rows), Perl #19523, Go #19525 (FMA guards in BUILD) and Dart #19526 (VM-only, generated vector source). The build tool selects every port on a corpus change (TC-18; PHY00a 8). Follow-ups stay open as TRIG-CONV-CR2 and TRIG-CONV-TAN | done; unblocks the floats pilot (VER04p, WX-FL1); owner decision 16 (2026-11-16); VER30 track C16 |
| P3 / TRIG-CONV-CR2 | Add a correctly rounded `cr` column for the Part II rows of `trig-reference-v1` (`sqrt`: integer square root; `atan`, `atan2`: an exact arctangent series with a remainder bound and its own `bc` cross-check, as `trig_ref_exact.py` has for `sin`/`cos`/`tan`), regenerate, and extend the generator's gates; TC-1 left it out because Part II is pinned legacy behaviour with no accuracy claim (PHY00a 3.9.1, 6.3 "The Part II gap") | TC-1; owed before any Part II accuracy claim, and before the floats pilot states one for `atan` |
| P3 / TRIG-CONV-TAN | Add a `tan` accuracy gate: `tan` rows of `trig-reference-v1` already carry a correctly rounded `cr` column, but `tan` is `sin / cos` with the pinned pole sentinel and only `sin`, `cos`, `radians` and `degrees` have a stated ulp bound (PHY00a 3.8, 4). Measure `tan`'s worst distance from `cr` across the generated rows away from the poles, state a bound, and have the generator's gates enforce it | TC-1; owed before any `tan` accuracy claim, and before the floats pilot states one for `tan` |
| P1 / WX-O4a | Verify `x509-subject-public-key-info` (the SPKI view `X509SubjectPublicKeyInfo`): a Warrant document and evidence of its own. First of the two lower crates of the RSA pilot | owner decision 17 (2026-11-16), reversing the VER04n review's recommendation (OWN-4); before pilot stage P-b of VER04n; VER30 track C17 |
| P1 / WX-O4b | Verify `der-asn1` (the ASN.1 limits `Asn1Limits`): a Warrant document and evidence of its own. Second of the two lower crates | as WX-O4a; then the `x509-rsa-public-key` adoption, then P-b |
| P1 / WD-100b | Conformance-kit follow-up (VER04i section 9.1, **done on branch `claude/warrant-conformance-reserved-names`, PR pending**): pin two cross-lane rules in `warrant-corpus-conformance-v1` and apply them to Python, TypeScript, Go and the Rust loader: (1) Windows reserved device names (`CON`, `PRN`, `AUX`, `NUL`, `COM1` to `COM9`, `LPT1` to `LPT9`, with or without extensions, plus trailing dots and spaces) are refused (the Rust loader in PR #19492 does; the lanes do not); (2) `win32` case-insensitive comparison folds ASCII only and compares non-ASCII exactly, failing closed (Go and Python differ today on `ſ`, `Σ`, `İ`). New cases are added only (64 added: containment 77 to 141). Implemented in all four places; the every-host artifact-path acceptance has no lane consumer yet | owner decision 20 (2026-11-16); after PR #19489 and PR #19492; gates every new lane with WD-100; VER30 track D6 |
| P1 / CV2-S1..S13 | `warrant-corpus-v2` (in-lane enumeration plus a model IR the lane re-evaluates), slices per the corpus design memo: S1 spec amendment of VER04i (v2 profile, binding grammar, arithmetic erratum, manifest path), S2 `warrant-grammar` export of the resolved-domain enumerator and post-slot resolver, S3 leaf crate `rust/warrant-corpus-gen`, S4 FULL independent `.warrant` source reader oracle gating each adopter, S5 conformance fixture, S6 Python and S7 TypeScript runners read v2, S8 gf256 first adopter after ct-compare, S9 generic v2 lane validator in `ca-assurance-report`, S10 `ir.fixed` then wasm-leb128, lzw, x25519, S11 `ir.data` then logic-gates, arithmetic, S12 `inout`/`ensures` then zeroize, S13 delete v1. PR CI runs a corpus-chosen `slim` chunk subset, `full` nightly on compiled lanes, depth shown in labels. **Progress: S1 done (#19488, cf556af70ad); S2 done (#19563, 3fc5071561f); S3-0 done (#19565, c9253018ccc); S3a done (#19568, a3bf446c0b8); S3b done (#19575, e395894625e); S3c done (#19577, 8792de62177); S4 part 1 done (#19499, 095e187f808); S4 part 2 done (#19581, e813859f3e6); S3d in progress (PR pending).** **Later progress:** S3d done (#19586) and S3e done (#19590): D16–D19, admit check D19, v0 wraps in examples D18; generator 0.5.0, oracle 0.4.0. S10-0 spec erratum done (#19592): D20–D35, `ir.tuple` with `match` on Result in S10, restricted `omit_example` (D34). S10a oracle `ir.fixed` lowering done (#19595). S10b generator lowering, PR #19598 (open, auto-merge). S10-1 (`omit_example`) in progress. Lane runners S10d/e/f wait on S5–S7. S10-1 done (oracle #19603, generator #19610); S10t-1 done (#19612); S10t-2 done (#19617, generator tuples and `match` on Result, census 89 → 105 emittable). S10c-1, oracle `ir.fixed` evaluation, done (#19624); with it, lzw evaluates fully and needs `max_ir_cost 16384`. S10c-2 done (#19631, generator `ir.fixed` evaluation). Pre-flip P1 done (#19635, oracle); pre-flip P2 (test-only generator parity tests pinned to an explicit closed gate) in #19647. S10c flip done (#19647): `warrant-corpus-gen` 0.6.0, `warrant-oracle` 0.5.0, `ir.fixed`/`ir.tuple` gates open, ct-compare `eq_u64` control live. **2026-11-18:** S10 is complete: the wasm-leb128 adopter merged as #19663 (974bf91). Flip #19647 merged. Adopters: x25519 #19653 (fixture `code/specs/fixtures/x25519-warrant-v2/`, D32 table-driven build-tool adopter rows) and lzw #19655 (`lzw-warrant-v2`, six omitted bomb examples, `max_ir_cost 16384`, generator `--check` in `--release`). The `off-by-one-consumed` form: S10o-0 spec #19656 (owner decisions D36–D38), S10o-1 oracle #19658, S10o-2 generator #19661, follow-up #19666 (Option caveat, Ok-form refusal contract row). wasm-leb128 #19663: D28 split 42→44 claims, 20 faults including `off-by-one-consumed`, 46,566,930 cases, one CI line per tool per D38. Next: S10d–f (behind S5–S7), then S11 (`ir.data`). | S1 first and must settle model arithmetic (WD-013) before S8; S2 is third in the `warrant-grammar` queue and S13 last; see [portable Warrant queue](VER-BACKLOG.d/portable-warrant.md), [VER30](VER30-warrant-expansion-roadmap.md) track D7 **2026-11-20:** S5-0 erratum merged (#19719; [D40]–[D45], VER04r §16.13) and S5-1 merged (#19724; v1 lanes accept digest-checked `v2-*` rows; Go lane selected by the kit); S5a oracle derivation in progress. S11-0 erratum in PR #19723 ([D46]–[D57], §16.14, Appendix A.2; lists move to the reserved level `ir.list`); S11f-1 (oracle scalar faults `saturate-add`, `saturate-sub`, `shift-mask`, scalar `missing-error`) ready behind it. Next: S5b, S11a, S11f-2. |
| P2 / CV2-F1 | The oracle should refuse a VER04r §4.4 impure guard at lowering, not only at evaluation. Then reshape the eight `c_not_*` claims in `python/warrant-oracle/tests/fixtures/fixed-forms-v1/`, and the generator drops its dump-only `skip_pure_guards` path. | Done (oracle #19616, generator #19623): lowering-time impure-guard, no-op-claim, model-body and v0-rule-3 refusals; the 4,096-byte example cap at emission; an admit size error refuses; `skip_pure_guards` removed. Source: CV2-S10b review. |
| P2 / CV2-F2 | The generator CLI exits 1 for a refusal, but VER04r says refusals exit 2. Fix: refusal → 2, drift → 1, with the `check_cli` tests updated. | Done (#19607): refusal exits 2, drift exits 1; `proc:signal` declared, owner-approved. Source: CV2-S10b review. |
| P2 / CV2-F3 | hash-functions has six W0701 "matches both models" claims (fnv1a/djb2) that need a D28-style split when hash-functions adopts v2. This is an owner decision, to be recorded beside D28 in VER04r §16.10. | Decided 2026-11-20: [D39] (VER04r §16.12) — split each into a step-model and an exact-model claim, both renamed; made in the hash-functions adopter PR with the Rust evidence regenerated. |
| P2 / CV2-F4 | Both lowerers refuse a `Bytes<N>` model result whose body is `concat(t, b"..")`, which the checker accepts. Candidate erratum. | Open. Source: CV2-S10a review. |
| P2 / CV2-F5 | Census gap, partly closed. The tuple gap closed with S10t-2, the guard-timing gap closed with CV2-F1, and wasm-leb128's 3 signed-`fx_edge` claims closed with S10c-1 (#19624) / S10c-2 (#19631). Remaining gaps, listed in VER04r §16.10 R13(d): hash-functions: 6 claims named `ir.fixed` by the oracle and `W0701` by the generator, refused by both (unchanged); wasm-leb128's two W0701 claims are resolved by the D28 split in #19663 (merged); arithmetic: records and variants at S11; zeroize: `inout`. `ORACLE_PINS` in `tests/census.rs` keeps it visible. | Partly closed (S10t-2, CV2-F1, S10c-1/S10c-2); open for the rest, as listed. |
| P2 / CV2-F6 | Add a generator `--check` for the synth-v2 fixture, with a build-tool row. | Open. Source: CV2-S3e review. |
| P2 / CV2-F7 | The self-grading ct-compare lanes (go, python, typescript) use an in-harness `sabotage` subcommand with no outside check. Bring them under `warrant_negctl.py` or document why not. | Open. Source: WD-HARN1c design note. |
| P2 / CV2-F8 | The oracle should refuse a `match` pattern `bind` named like a model, op, error type, constructor or reserved built-in. The checker refuses these and the oracle currently accepts them. | Done (#19720): the oracle refuses a `match` bind named like a model, op, error type, constructor (records included) or a version-scoped reserved built-in; parity with the checker's lists is pinned by a test. Spec sync of VER04r §16.10 R2 pending (after #19723, which edits §16.10). |
| P2 / CV2-F9 | The generator names `W0701` and the oracle names `ir.fixed` for the 6 hash-functions claims that have both an impure guard and a fixed built-in. Both refuse; align which refusal each tool names. | Open. Source: CV2-F1 generator. |
| P2 / CV2-F10 | Decide at S11 whether the oracle should refuse record and variant types until `ir.data` is specified. Today it counts them in `requires`, while the generator refuses them. | Decided 2026-11-20: closed by S11a/S11b ([D53], PR #19723); anything still unimplemented after S11 is refused, not counted. |
| P2 / CV2-F12 | Extend the stand-in rule (VER04r 16.6 item 5) to pure computed arguments, literals included: for a `post` or `replace` fault the stand-in needs only the argument values, which are pure when no argument holds a subject call, so a `reference` claim whose subject side is `op(e1..ek)` with every `ei` pure has a stand-in; a `pre` fault keeps the plain-binder rule, because it must re-evaluate `expected` at one rewritten binder. Lands wholly before S11c-1 or wholly after the arithmetic adopter ([D57]). (Wording quoted from VER04r §16.14 R36 on `claude/cv2-s11-0-erratum`; it replaces the earlier "literal arguments" scope.) Needs both tools (oracle first), then fixture regeneration. Today the v2 tools prove the observability of none of x25519's controls; only the v1 Rust harness does. | Open. Source: x25519 adopter review (#19653). **2026-11-20:** reworded scope per VER04r R36 (#19723); sequencing ([D57]): lands wholly before S11c-1 or wholly after the arithmetic adopter. |
| P2 / CV2-F13 | A spec-only edit to an adopter's `.warrant` does not select that adopter's own Rust crate in the build tool, although the crate's BUILD runs `gen_evidence --check` against the spec. Pre-existing; affects ct-compare, x25519, lzw and wasm-leb128. | Done (#19716): every Rust crate whose BUILD runs `gen_evidence --check <crate>` is selected by an edit to `code/specs/<crate>.warrant` (nine crates plus the four VOS crates via vosImplementationRow); a coverage test pins rows to BUILD/evidence.bindings and runs when an adopter's `evidence.bindings` changes. |
| P3 / CV2-F14 | The generator reports a stale `omit_example` line only after the full enumeration; check it before enumerating. | Open. Low. Source: lzw adopter review (#19655). |
| P2 / CV2-F15 | The oracle does not refuse a parameter or forall-binder name that the checker refuses (`check_binder_name`, checker.rs ~1177): a constructor name in every version, a reserved built-in only in v1 (a v0 parameter named `len` is legal). The rule differs from the match-bind rule of CV2-F8, so do not reuse `_declare`'s set (it would over-refuse v0). | Open. Source: CV2-F8 review (#19720). |
| P3 / CV2-F16 | An edit to `code/packages/rust/warrant-grammar/src/checker.rs` does not select `python/warrant-oracle`, although the oracle's bind-name parity test reads it; drift surfaces in the next unrelated oracle PR. Add the exact file to `warrantOraclePaths` with a build-tool test (cost: the oracle BUILD, ~10 min, on every checker edit). | Open. Source: CV2-F8 review (#19720). |
| P3 / WG-V0-WILDCARD | The v0 checker path inserts "_" into the match-arm environment (checker.rs ~3652-3709), so a nested `Ok(_)` inside an `Ok(_)` arm may raise W0203 there while the oracle accepts it. Investigate; fix in the checker if confirmed. | Open. Source: CV2-F8 review (#19720). |
| P3 / KILN-STRACE-ORDER | Kiln sandbox `TestSandboxStraceSignals` (daemon-left F2: a survivor's exit line ordered before init's exit_group) and the #19671 cross-check ("stricter-only divergences") failed once on a PR that does not touch Kiln (#19718, ubuntu-26.04), and passed 15 minutes earlier on #19716. The strace report order is not deterministic; owned by the Kiln/GBL owners. | Open; watching for recurrence. |
| P2 / BT-BUILDLOCAL-1 | Build tool follow-ups of #19669 (BUILD-front edits in Rust packages are package-local, an owner decision; the gate is Rust-only because Lua and TypeScript dependents run or consume their prerequisites' BUILD). (1) Move `TestEveryBuildFileUsingTheNegctlCheckerDeclaresPython`, a `go/programs/build-tool` test that PRs touching only package BUILDs never select, into `-validate-build-files`. **Part (1) done: PR #19718 (`-validate-build-files` enforces negctl users declare Python, all-platform discovery, read errors fail closed).** (2) Accepted residual risk: a dependent that relies on a toolchain only its prerequisite declares via `# needs-toolchain:` is not re-run when that declaration is removed. | Open. Source: #19669 design. |
| P3 / KILN-G1-XREF | Cross-reference only, owner GBL (`GBL-063` in [GAME-BACKLOG](GAME-BACKLOG.md)): the Kiln G1 14 MB/s synthetic-throughput flake failed PRs that edit `code/programs/go/build-tool/repository_inputs.go` several times on 2026-11-18 (S10c-2, lzw, oracle S10o-1). Not a VER item; no action here. | Tracked by GBL-063. |
| P2 / WD-HARN1d | The `go/ct-compare` Go runner must print a TAP `1..N` plan and numbered points, so its 8 sabotage controls can move to `warrant_negctl.py`. | Open. Source: WD-HARN1c PR 3. |
| OPEN / CV2-D19 | Owner question (VER04r §16.8): should subject-dependent admit-read arguments be exempt from the D19 static refusal (`W0701`, VER04r section 4.5)? Recommendation: no. Details in the [portable Warrant queue](VER-BACKLOG.d/portable-warrant.md) | owner decision; no implementation until answered |
| P1 / WD-100 | Lane conformance kit `warrant-corpus-conformance-v1` (VER04i section 9.1): every new lane must pass it | delivered in PR #19489; the Go lane adopts it in a follow-up; gates every WD-1xx lane; VER30 track D6 |
| P0 / WD-HARN1 | Warrant harness supervisor hardening: wait timeout, never block exit, poison recovery (from the 2026-11-14 Windows `rust/lzw` hang). **Done, merged as #19530 (0c9f75a9e92)** (VER04j 14.2 item 53; every adopter regenerated) | after WX-G (PR #19369); second in the `warrant-grammar` queue, before CV2-S2; VER30 section 5.3, track D9 |
| P2 / WD-HARN1b | LOW follow-ups of the WD-HARN1 review. **Done, merged as #19550 (50b6c679762)**: obs adopters' sabotage lines pass only with a `not ok ` line and no `Bail out!` line (a hang after an intended failure was hidden by the pipe status); `obs_bail_out` writes stderr with `writeln!` (no panic on a closed pipe); stale `OBS_CALL_TIMEOUT_MS` comment and one dead `latched = true` fixed; obs adopters regenerated; hash-functions wording on the zero-modulus panic allocation refined | after WD-HARN1; edits confined to the prelude `obs` region so only obs adopters regenerate |
| P2 / WD-HARN1c | Remaining LOW findings from the WD-HARN1b review of the Windows sabotage lines: (1) the bail-out branch of the Windows `findstr` lines has never executed on any machine; add one synthetic check on a Windows runner or move the check into a shared tested script; (2) a crash with no `Bail out!` after an earlier `not ok` still passes; also require the TAP plan line; (3) a failing Windows control prints nothing; `type` the capture on the failure path; (4) a stale `%TEMP%` capture could satisfy the check if the redirect cannot open; `del` it first | after WD-HARN1b (PR #19550). **Status:** PR 1 done (#19594): shared tested checker `code/scripts/warrant_negctl.py`, the test-bootstrap `/C:` vacuous-control fix, TF01 §6 normative form. PR 2 done (#19599): lzw, wasm-leb128 and hash-functions migrated (51 controls). PR 3, the legacy batch, is #19621 (open): 10 packages, 263 lines. It is blocked on the build-tool change #19669 (in review): its Windows build hit the 300-minute step timeout because its BUILD edits in hub packages rebuilt about 803 packages (replica: 803 → 35 after #19669). #19621 will be retried unchanged after #19669 merges. `go/ct-compare` remains on the old form, because its Go runner prints no TAP plan and unnumbered points; it needs a plan and numbered points first (see WD-HARN1d). VOS migrates `vos-sched-*` itself, using the recipe in the script's docstring. Owner decisions (2026-11-17): per-adopter `# needs-toolchain: python`; tests in warrant-grammar BUILD; VOS migrates its own; legacy in one batch. |
| P2 / OBS-PERF | Allocation observer cost: the obs observer hands each observed call to a worker thread, about 35 to 50 microseconds a call, which is why `fnv1a_32`, `fnv1a_64` and `djb2` (2^24-case suites) carry no `heap` row. Proposal: run a whole claim body on the worker instead of one hand-off per call. **Needs a design note first** | after WD-HARN1b; hash-functions DT17a section 7 |
| P2 / BT-TESTONLY | Build tool: a change confined to an adopter's generated test harness (`tests/generated/`) or evidence files does not change the library its dependents link, so the build tool should not cascade it to dependents (WD-HARN1 rebuilt about 428 packages for about 2.5 h per platform because of this). **Proposed, needs design review.** Risks: dev-dependency edges, and anything that compiles tests across crates, must still be honoured | independent; companion to BT-TIMEOUT |
| P1 / BT-TIMEOUT | Build tool: per-command timeout so a hung command can no longer consume a CI job silently | PR #19483; independent; companion to WD-HARN1, VER30 section 5.3 |

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
| P2 / AS-008 | VER13 catalogue entry for Rust `core` (owner request through [VOS02](VOS02-interface-contracts-and-profile-linking.md) section 8.1, 2026-11-16): VER29 section 7.2.1 withholds resource, timing and capability strengths for any used dependency with no contract of that kind, `std`, `core` and external crates included, and every class K crate (VOS01 section 1.4, SH1) depends on `core`; so none of those strengths can print for a VOS component until a catalogue entry (a family and id per AC-1, with a discharge plan) covers `core`. Functional strengths are unaffected (they print with `conditional_on`) | proposed; through the catalogue's own process (AS-001a rules AC-14); the VOS checks (empty effect set, ceilings) do not wait for it |
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
| P1 / CAP-RT-002 | Feed runtime evidence into the evidence-derived assurance report (new rung; `--check` requires it on Linux) | after WD-001 + CAP-RT-001 (mandatory: owner 2026-11-12, VER27 §6.1) Sequenced by [VER30](VER30-warrant-expansion-roadmap.md), Track A4. |
| P1 / CAP-RT-003 | Make CAP-RT a required step of every leaf-library conversion (LP-0xx) | after CAP-RT-001 (mandatory: owner 2026-11-12, VER27 §6.1) |
| P2 / CAP-RT-1xx | Per-language lanes: arm the filter after interpreter/VM startup (Python, TypeScript/Node, Go, JVM, .NET, ...) | after CAP-RT-001. Python, TypeScript/Node and Go: not started; the WD-101 Python, WD-102 TypeScript and WD-103 Go Warrant lanes run unsandboxed, so it records no `sandbox` field and cannot be `release-ready` (see [portable Warrant queue](VER-BACKLOG.d/portable-warrant.md)) Sequenced by [VER30](VER30-warrant-expansion-roadmap.md), Track D4. |

### Follow-ups logged 2026-11-09 from agent reports

| Priority / ID | Work |
|---|---|
| P1 / SEC-002 | Decompression-bomb surface remains: declared lengths up to 4 GiB satisfiable by real tokens; LZ78 chains quadratic; LZW ~2700x amplification. Needs `max_output`/error channel (LP-003, WX-H). **rust/lzw: found again by the `lzw.warrant` bound rows, and contained on branch `claude/warrant-adopt-lzw` without an API change**: `decompress` stored every dictionary entry as its own `Vec<u8>` (entry = previous entry + 1 byte, so a chain of k entries holds k^2/2 bytes: a 13 KB stream held 32 MiB of dictionary beside its 32 MiB output, a 128 KB one two gigabytes of each) and delivered whatever the header declared (up to 4 GiB). Fix: the dictionary is `(prefix, byte, length)` (linear, under 1 MiB) and delivered output is capped at `MAX_OUTPUT_LEN` = 16 MiB, failing closed with `Err`. The cap is a constant, not a `max_output` parameter, so the other ports and the lz77/lz78/lzss rows stay open until LP-003 / owner question 6 |
| P2 / SEC-002a | rust lzw `compress` hashed the whole current prefix for every input byte (`HashMap<Vec<u8>, u16>`), so redundant input cost the sum of the prefix lengths: 8 MiB of zeros took about 35 s in a debug build, and 16 MiB minutes, with no error channel to bound it. It also wrote `data.len() as u32` into the header, so an input over 4 GiB wrapped to a small length and `decompress` returned a silent prefix. Fix: the dictionary is keyed by `(prefix code, byte)` (one constant-size lookup per byte, output byte for byte unchanged, pinned against the old algorithm kept as a test reference) and the header saturates at `u32::MAX`, which `decompress` then rejects at the cap |
| P2 / SEC-003 | huffman-compression: crafted code-length table overflows `code <<= len - prev_len` |
| P2 / SEC-004 | elixir lz77 decode O(n^2) list appends |
| P1 / SEC-009 | grammar-tools lexer: an F10 group push has no depth budget, so nesting is unbounded. Found by RS01 slice A4 (RS01 §4.3 "Verified by slice A4"): a 1,000,000-deep `/*` comment lexes in about 1.0 s and 419 MB. This affects every grammar that uses F10 groups, not only Rust. Fix: a `MAX_GROUP_DEPTH` check at push that fails closed with a lex error, plus a hostile-input test at the cap and cap+1; until then the Rust `rust_block_comments` hook leaves an over-deep run undeleted so the grammar refuses it. Related engine gaps from the same slice: `GrammarLexer` ignores a `.tokens` `errors:` section, and a post-tokenize hook cannot return an error (an unterminated comment is a parse refusal, not a lex one); fix with a checked `add_post_tokenize` variant. | open (before RS-A5 registers the hook) |
| P2 / SEC-006 | **FIXED on branch claude/warrant-adopt-wasm-leb128**: rust wasm-leb128 `decode_*_bounded` now reject `max_bits > 64` before decoding (previously 66..=70 silently dropped data bits above 64 and >= 71 with an 11-byte input shifted a u64 by 70). `wasm-leb128.warrant` has the `rejects` row and width claims (LEB01 "Findings") |
| P2 / SEC-007 | rust logic-gates accepted a value that is not a bit, silently, in a release build. `validate_bit` was a `debug_assert!`, so in release `tri_state(2, 1)` returned `Some(2)`, `priority_encoder` read a 2 as an inactive input, and `encoder` counted active inputs in a `u8` (257 active inputs wrapped to 1 and passed the one-hot check); the gates and the mux/decoder/demux panicked only because the transistor model underneath rejects the value. **FIXED on branch claude/warrant-adopt-logic-gates**: `assert!` in every profile, `usize` count, the three CMOS gates validate; `tests/bit_domain.rs` (failed under `cargo test --release` first), `*_rejects_non_bit*` claims of `logic-gates.warrant` incl. `tri_state_rejects_non_bit_data`; lesson written. `sequential.rs` not examined (next slice) |
| P2 / SEC-008 | rust logic-gates `decoder` (and `demux`, built on it) sized its output as `1usize << n` for any input width n, so n is an amplification factor on a hostile caller: 17..=40 inputs mean a 128 KiB to 1 TiB allocation and 2^n loop turns, and 64 inputs wrapped the shift to `1 << 0` in a release build and returned one output line silently (a debug build panics on the overflow). Fix: a width cap `MAX_DECODER_WIDTH` = 16, rejected by a panic in every profile. **FIXED on branch claude/warrant-adopt-logic-gates** (`tests/decoder_width.rs` failed first; examples `decoder_seventeen_inputs_rejected`, `decoder_sixty_four_inputs_rejected`, `demux_seventeen_select_lines_rejected`; lesson written) |
| P2 / SEC-010 | First reported, unconfirmed, by the VER04m design survey. rust caesar-cipher `decrypt` computed `encrypt(text, -shift)`, and `-i32::MIN` overflows: a panic in a debug build, a silent wrap back to `i32::MIN` in a release build, which then shifts by +2 instead of -2 (`(-2^31) mod 26 = 2`) and so returns the WRONG plaintext with no error (the two profiles disagreed: debug panics, release is silently wrong). Found by reading the code. Fix: reduce the shift with `rem_euclid(26)` first, then negate the small value. | **confirmed, fixed in Rust on branch claude/sec-010-011** (`caesar-cipher/tests/cipher_tests.rs` `decrypt_i32_min_does_not_panic_and_is_correct` failed first; lesson written). Ports: c and cpp already avoid the negation; python, ruby, elixir, perl, typescript cannot overflow (arbitrary-precision or float); **ports fixed on branch claude/sec-010-ports** (each gained a test that failed first; decrypt now negates the shift reduced to 0..25, never the raw shift): csharp, fsharp, java, kotlin (all `Int` wraps; their `encrypt` was already safe because `% 26` runs first), dart (VM 64-bit int wraps, `encrypt` already safe, native semantics only), haskell (`negate minBound == minBound`, `encrypt` already safe), lua (5.3+ integer wraps, `encrypt` already safe), go (`Decrypt` fixed, and `Encrypt` was ALSO unsafe: `pos+shift` overflowed near `math.MaxInt`, now reduced first), swift (`-Int.min` traps; `encrypt` ALSO trapped on `offset + shift` near `Int.max`, both fixed). Verified locally with the real toolchains: csharp, fsharp, java, kotlin, go, dart, haskell, swift, lua. Already safe, no change: c (`normalise_shift`, decrypt uses `26 - normalise_shift`), cpp (read; reduces first). Not affected (read, not run): python, ruby, elixir (bignum ints), typescript (IEEE doubles, no integer overflow), perl (IV overflow promotes to NV; `%` then works on the promoted value) Row added here because PR #19353 had not landed on main |
| P1 / SEC-011 | First reported, unconfirmed, by the VER04m design survey. rust irc-framing `Framer::feed` appended every byte to its buffer and only looked for a line feed (LF) in `frames()`, and the 510-byte limit was applied only to lines that already ended in LF. A peer that never sends LF therefore grows the buffer without bound (CWE-400, CWE-770). Fix: a pending-line cap of 511 bytes (510 content plus a possible CR, RFC 1459 section 2.3 and RFC 2812 section 2.3: 512 including CRLF); past it the partial line is dropped and the framer discards input up to the next LF (fail closed, consistent with the existing silent discard of overlong lines, so no API change). | **confirmed and fixed in every affected port.** Rust: PR #19359 (`test_no_lf_in_chunks_is_bounded` and `test_no_lf_in_one_feed_is_bounded` failed first with a 16 MiB buffer). Ports, branch `claude/sec-011-ports`, tests committed first and red (buffer 16,777,216 or the 512/2000-byte boundary cases), then the fix with the same semantics as Rust (511-byte pending cap, drop the partial line and skip to the next LF, decision before any copy, `reset` clears the discard state, no signature change): **c** fixed (gcc and clang via `tools/run.sh`, 82 checks green; the header had documented the unbounded buffer as "by design", corrected; adds `IRC_MAX_PENDING_BYTES`), **cpp** fixed (g++ and clang++, 49 checks; same header correction; adds `Framer::MAX_PENDING_BYTES`), **csharp** fixed (dotnet test, 18 tests, adds `Framer.MaxPendingBytes`), **fsharp** fixed (dotnet test, 18 tests, adds `IrcFraming.MAX_PENDING_BYTES`), **elixir** fixed (mix test, 32 tests; the moduledoc claim that the 510-byte rule prevents exhaustion was wrong and is corrected; the struct gains `:tail_len` and `:discarding`, `:buf` still the only enforced key), **go** fixed (go test, 100% coverage), **python** fixed (pytest, 100% coverage, ruff check clean), **ruby** fixed (minitest run directly; `bundle exec rake` could not find rake in this sandbox), **typescript** fixed (`npm test` and `test:coverage`, 28 tests). Not applicable, no `irc-framing` package exists on main: java, kotlin, swift, dart, haskell, lua, perl, ocaml, wasm, starlark, twig, mosaic. The consumers checked (go, python, ruby, elixir ircd and the elixir/python net layers) call `frames()` after every `feed()`, so the complete-line backlog stays bounded by one read. Spec `irc-framing.md` updated to the precise rule. Row added here because PR #19353 had not landed on main |
| P1 / SEC-012 | **FIXED on branch claude/lv007b-ct-compare-rc** (found by the LV-007b pre-release security review): typescript ct-compare `toBytes` wrapped an `ArrayBuffer` in a view instead of copying it. `ctEq(left, right)` converts `left` first, and converting a plain-array or Proxy `right` runs caller code (a `length` trap, an element getter) that could detach, shrink or rewrite `left`'s buffer afterwards, so two different inputs compared equal and `ctSelectBytes` could return bytes the buffer no longer held. Fix: `new Uint8Array(new Uint8Array(value))` (copy at conversion; a buffer detached before the call still throws). Pinned by `tests/snapshot.test.ts` (detach via `structuredClone` transfer, resizable shrink, rewrite, select), which failed 4/5 before the fix | fixed; lane evidence regenerated; lesson recorded |
| P1 / SEC-013 | **FIXED on branch claude/lv007b-ct-compare-rc** (found by the LV-007b pre-release security review): rust ct-compare `ct_select_bytes` compiled at `-C opt-level=3` to `select i1 %choice, ptr %a, ptr %b` + `memcpy`, so it read only the chosen buffer and the addresses touched depended on the secret `choice` (cache side channel), contradicting `cc.llvm-ir.addresses-public` / `mc.backend.addresses-public` that ASSURANCE.md lists. The optimizer could do this because it knew the mask was 0x00 or 0xFF. Fix: the mask passes through `core::hint::black_box` (a hint, which the WIR proof pipeline lowers to identity), so every output byte is computed from both inputs; the IR test makes a compiler that sees through the hint a CI failure. Pinned by `tests/optimized_ir.rs::optimized_select_reads_both_inputs_at_public_addresses` (no pointer select, no memcpy/memmove, no branch on `choice` in the optimized IR), which failed before the fix | fixed; evidence, effect certificate and report regenerated; lesson recorded |
| P1 / SEC-005 | Run the C#, F#, Dart, Swift, Haskell, Elixir, Lua lz* fixes in CI or a local toolchain; perl Test2::V0 suites; ruby standardrb |
| P1 / CAP-RT-004 | CI wiring: set RCG_STRICT=1 and RCG_REQUIRE_DOCKER=1 on Linux runners once strace/Docker availability is confirmed. Partly done on branch claude/warrant-adopt-wasm-leb128: ci.yml installs strace on Linux and wasm-leb128's `BUILD_linux` runs the gate with RCG_STRICT=1 (macOS `BUILD` runs it plain). RCG_REQUIRE_DOCKER stays unset: the container ring has not yet been seen live on a CI runner Sequenced by [VER30](VER30-warrant-expansion-roadmap.md), Track A4. |
| P2 / CAP-RT-005 | Coverage of sandboxed child code (flush profile before arming or via pre-opened fd); crate at 72.7% < 80% |
| P2 / CAP-RT-006 | vDSO-backed time reads are invisible to both rings; document as residual or detect via vDSO unmapping |
| P2 / CAP-RT-007 | Stateful-object jail items (VER04m section 9): measure `mremap` and `madvise` on the first stateful pilot and bump the armed baseline to `rcg-baseline-2` only on that measurement (VER08b section 4); a per-lane phase-A warm-up drive; the `drop-runs` obligation inside the armed phase; with the WX-S1 pilot, measurement first, never widened on expectation |
| done / AR-001 | `ca-assurance-report --check` now re-derives the weakness matrix (including the profile-supplied `reason` text) from the evidence and the hash-bound `assurance-profile.json`; edited text, another profile or an edited profile file is `EvidenceMismatch` / `StaleEvidence` (VER04h section 9 item 22) |
| P2 / WD-005 | Non-trivial 2-safety miters (current DRAT certificates are the fixed x and not-x refutation); prover support for traps as path outcomes; heap/depth bound proofs |
| P2 / WD-006 | Single binding table: op->export map duplicated in gen_evidence.rs and warrant_proofs.rs Sequenced by [VER30](VER30-warrant-expansion-roadmap.md), Track A8. |
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
| P1 / WD-013 | (2026-11-16: model arithmetic has three semantics today, spec wraps, runners refuse, oracle unbounded; resolve to VER04 §3 wrapping in CV2-S1 before gf256 v2, see VER30 risk 12) BUG: the Rust evidence generator evaluates U8 arithmetic as non-wrapping, contradicting VER04 §3 (U8/U64 wrap). Claims using U8 arithmetic get a wrong oracle; gf256 worked around it. Fix with differential tests in the first WX slice | done-local for v1 documents (WX-B, `warrant-grammar` 0.2.0): in a v1 document `U8`/`U64` are typed `V::Fixed` and wrap, with native-Rust differential tests and a killed non-wrapping-U8 mutant. The **v0** generated harness still evaluates `U8` as `V::Nat`: it is frozen byte-identical so `ct-compare`/`gf256` evidence does not change; the gf256 conversion to a v1 document (backlog) retires it. Also on this branch: VER04j section 14 lists every deviation |; **re-checked on `claude/warrant-bytes-n` (WX-N)**: still true and still deliberate for v0, closed for every v1 document, which is what each crypto oracle is (`Bytes<N>`/`U32` need v1). New test `tests/wxn_evidence.rs::u8_arithmetic_wraps_in_the_generated_model_code` pins `200u8 + 100u8 == 44u8` through the generated model code of a `Bytes<1>` operation, all 65,536 cases against a wrapping subject. The v0 `V::Nat` evaluator cannot be fixed without a type on the value (a `U8` and a `Nat` are the same `V::Nat`) and that changes every v0 harness byte, so it is retired with the gf256 conversion to v1 (VER27 section 6 step 6), not patched; gf256's models are written so no `U8` expression wraps. Status: done for v1, v0 residual accepted

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
| P2 / WD-015 | Fixed arrays of fixed-width integers (`[u32; 8]`, `[[u8; 4]; 4]`: sha256 state, AES round keys, ChaCha20 state) are not expressible: `Bytes<N>` is done (VER04j section 14.4, WX-N) but there is no length-refined `List<T>`/array type, so these cross the host boundary only as `Bytes` of their serialisation (`be_bytes`/`le_bytes` per word) or as a `List<U32>` of unchecked length. A sound design needs a checker type (`Array<T, N>` or `List<T>` with a literal length), a domain (sampled, labelled `sample`), a VER04i `L` encoding with a length check, a harness value and `[T; N]` host conversion with a fail-closed length check, i.e. a language change, not a binding change; do it when the first hash/cipher adopter needs state or key-schedule claims (sha256 and aes are the likely first) | proposed; found by the WX-N design |

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
| spec / WX-001 | VER04k (`code/specs/VER04k-warrant-v1-wave4.md`): normative WX-D (bounded sequence combinators, in slices D1/D2/D3), WX-F (binding-level sequence drivers) and WX-G (in-out postconditions); new diagnostics W0711-W0713 | spec merged #19115; WX-H/WX-E prerequisites merged #19121; WX-D1 S0 errata merged #19204 at `d214abc36793908274a21a15282d39fb0694dd16`; S1 checker/ground evaluator merged #19209 at `df621fd51d3633a0cf9fff353372374efd617d5d`; evidence-generator/corpus translation S2a remains outstanding. Keep the existing D1/F/G/D2/D3 sequence and resolve VER04k §9 questions during implementation. See [portable Warrant queue](VER-BACKLOG.d/portable-warrant.md) |
| P1 / WX-RD1 | Security (WX-D1 S1 review): read operations deep-clone `EvalValue`; a 48-reference list or model call peaked at 2.4 GB, despite 16 MB charged, with about 33 ms per clone. First bounded repair selected: API-preserving 64 MiB read-copy footprint guard before each deep copy, W0703 with existing error shape. This is not RSS/total-memory boundedness: the portable data meter still charges 32 bytes per slot versus a 96-byte `EvalValue`, and allocator overhead/non-copy allocations remain outside the guard. Keep the residual open; do not claim a complete memory cap. See [portable Warrant queue](VER-BACKLOG.d/portable-warrant.md) |

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
| P1 / RS-A3 | Evidence tooling `rust_release_pairs.py` (`reviewed`, `copy:`, `semantic-copy:`; `check` mode); first `pair-evidence.csv` rows | **Status: merged (#19239); the first rows came with the first pairs in RS-A4** |
| P1 / RS-A4 | Modern lexical pairs `rust1.97.0-e{2015,2018,2021,2024}` (subset `rs-token-trees`); generated raw-string arms and per-Unicode XID classes; verify the section 4.3 comment design | **Status: implemented in local commits (generator `code/scripts/rust_lexical_pairs.py` with `write`, `check` and `import-xid` and 41 tests; the four 1.97.0 pairs; the `rs-token-trees` corpus of 56 cases; four `pair-evidence.csv` rows; the 1.97.0 row at stage G; RS01 4.3 verified against the real engine and `rustc` 1.97.0, with two open engine items; A4 decisions in RS01 section 10), not yet pushed; RS-A3 is merged.** The depth budget and typed hook errors need the small engine changes named in RS01 4.3 (D1, or stopgaps in A5) |
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

## Lean 4 proof kernel (VER11a) and the Lean programme (LEAN00)

[VER11a](VER11a-lean4-proof-kernel.md) designs the trusted kernel: Lean 4's
type theory (owner decision 2026-11-23), checking lean4export files. The same
kernel checks Lean libraries and the repository's own claims, so ct-compare's
claims can become `proved` for all lengths with no open own-tool-gap
(bucket (c)) ids. [LEAN00](LEAN00-lean-on-the-generic-pipeline.md) is the
umbrella for Lean as a language on the generic pipeline (L1 kernel, L2
grammar, L3 core elaborator, L4 command execution, L5 full compatibility).

Milestones: K-M0 (= L1-a) the kernel checks Lean's core library; K-M1 all
four ct-compare claims kernel-checked at WIR level for every length (VER31
rung `checked`; open (c) ids from 20 to 11); K-M2 the Rust-to-WIR step
validated (six more close). `proved` also needs the owner's VER11a Q5–Q7
decisions on the kernel's own ids and two model ids, and VER31 closing the
report checker. Buckets are VER31's `a|b|c`.
Order below is dependency order; one bounded change per PR, spec before code.
Nothing starts before the owner approves VER11a and LEAN00.

| Priority / ID | Work | State / prerequisite and acceptance |
|---|---|---|
| done / KRN-001 | VER11a and LEAN00 specs; Fable adversarial review (two rounds); owner decisions recorded; VER00 §7.5 amended (kernel size promise replaced by VER11a §4.2 and §12) | merged in #19880; revised with the KRN-S0 findings in #19890 |
| P1 / KRN-002 | `lean-kernel` (Rust): expressions, levels, inference, reduction and definitional equality, inductive types (incl. K-like reduction, structure η at recursor majors, large-elimination rules), literals with primitive-definition validation; `forbid(unsafe_code)`, no dependencies; CI budget gate (≤ 8,500 lines incl. reader); literate; written from the type theory, studying but not copying nanoda_lib and lean4lean; budget revisited once after the reading, by owner decision | after KRN-001. Slice 1 (term layer: names, levels, hash-consed expressions, VER11a §4.2 row 1) merged in #19889; slice 2 (environment, local context, `infer`/`check`, core `whnf` and definitional equality) in progress; then inductives, full reduction, literals |
| done / KRN-002a | `bignum-core`: bitwise operations, shifts and `log2` on `BigInteger`, tested against a naive reference (the kernel's trusted surface) | merged in #19886 (bignum-core 0.9.0) |
| P1 / KRN-003 | **Re-scoped 2026-11-24 by [VER32](VER32-proof-interchange-format.md)** (owner decision: our own Proof Interchange Format; VER11a Q9 resolved). The kernel reads **PIF**, not lean4export: the trusted reader is PIF-002 and the `pif-check` binary is PIF-003; the pin file records the PIF format and kernel-logic versions plus the Lean toolchain and lean4export versions used by the *untrusted* Lean→PIF converter (PIF-008). Reading lean4export, dropping recursors and listing `unsafe` declarations move into that converter | with or after KRN-002; superseded in detail by PIF-002, PIF-003, PIF-008 |
| P1 / KRN-004 | Corpora and oracle: Lean's kernel-rejection tests and independent checkers' adversarial corpora as data; own negative corpus; mutation testing; CI differential against the official Lean kernel at the pin through a small Lean replay harness (`addDecl`), with per-export agreement recorded as evidence (oracle, never a gate dependency) | after KRN-003 |
| P1 / KRN-005 | **K-M0 / L1-a**: Lean core library (`Init`, then needed `Std`) accepted, same verdicts as the oracle | after KRN-004 |
| done / KRN-S0 | Feasibility spike in official Lean 4.35.0: constant time of `ct_eq_equal_length` for every length, axioms `[propext, Quot.sound]`; verdict feasible with changes (`code/learning/lean/krn-s0-spike/REPORT.md`) | merged in #19890; findings folded into VER11a §8.3, §9.2, §14.0 |
| done / KRN-006 | VER25 amendment (spec), per KRN-S0's findings: settle the cost schedule (VER25 §5.1 and `cost.rs` disagree on return, alloc and free); out-of-bounds is `stuck`, not a branch; total variables and operators with the type checker ruling out ill-typed cases; the Lean semantics of WIR is normative; instruction-level small-step machine with frame stack and `ip`; structural step-count fuel with accumulated cost; `Alloc` kind (stack/heap, heap counted in bytes) and secret/site tag; deterministic allocation ids; numbered functions and variables | after KRN-001. **Merged in #19892**: [VER25 §8](VER25-wir-core-interpreter.md#8-normative-semantics-krn-006), implementation in KRN-006a. It settles the cost schedule (return 1, alloc 3, free 2, trap 1; one cost per completed step) and lists the missing checker rules (`UseBeforeDefinition`, `NoEntryBlock`, `BlockIdsNotDense`) and the interpreter changes for KRN-006a and KRN-010 |
| P1 / KRN-006a | `warrant-ir` code: `Alloc` kind and tag in the AST, lowering and interpreter; heap accounting in bytes | after KRN-006 |
| P1 / KRN-006b | Finding (VER25 §8.5, KRN-006): `check.rs` threads one environment through `func.blocks` in vector order, not control-flow order, so a use reached by a path that never binds the variable passes (block 0 → 2, block 1 binds `x` → 2, block 2 reads `x`). Add a dominance-based `UseBeforeDefinition` rule, plus `NoEntryBlock` and `BlockIdsNotDense`; failing tests first (VER27 §7.4 protocol); lesson | fix in #19894 (awaiting CI); before KRN-007 relies on the totality guarantee |
| done / KRN-006c | Finding (VER25 §8.10): the interpreter checks its step budget only inside the instruction loop and `call_fn`, so a cycle of instruction-less blocks never checks and spins forever. Check before every step, including terminators; failing test first; lesson | merged in #19895 (budget checked before every block and terminator) |
| P2 / KRN-006d | Findings (VER25 §8.2–8.9): trap-mode overflow returns `Stuck` but VER25 §2.1 says trap (settled: trap); `decode_value` for a pointer type invents provenance 1 and a pointer store writes only the offset (settled: pointer load/store is `stuck`); `Free` ignores the offset (settled: offset must be 0); `lower_block` makes `acc` a 1-byte slot by name (settled: stack slots marked by construct). One failing test and fix per item | proposed; with KRN-006a |
| P2 / KRN-006e | New catalogue id `wir.typing.checker-guarantees` (bucket (c)): the WIR type checker rules out ill-typed operations and unbound reads (no preservation theorem yet); VER11a §13 gains the row | proposed; with KRN-011 |
| P1 / KRN-006f | Finding (found while fixing KRN-006b): the full WIR type checker `check_function` already rejects the Rust front end's lowered output for ct-compare (`ct_eq`, `ct_eq_u64` and `read` return `unit` where `Bool`/`U8` is declared; `ct_eq_equal_length` mixes `U8` and `U64`; `ct_select_bytes` returns `Ptr(U8)` vs `unit`), because the lowering emits unit returns for tail expressions. Nothing in the repository runs the checker on lowered code, only on hand-built ASTs, so VER11a's totality guarantee currently has no checked basis. Fix the lowering's return typing, then make every lowering run the full checker (fail closed); failing test first; lesson | proposed; before KRN-007 relies on the checker |
| P1 / KRN-007 | `warrant-lib` (Lean, inside LEAN00 L3's tactic subset): port the KRN-S0 relational core (semantics, step lemmas, relation, obligation lemmas, `lockstep`, `lockstep_obs`) to full width-exact WIR; prove `call` as a lockstep step with callee label tables; `invariant_run`, `terminates_by`; elaborated by the pinned official toolchain until LEAN-L3a (owner Q2; if "no", only the syntax and semantics are written, as raw terms, and the rest waits for L3) | after KRN-S0, KRN-005, KRN-006 |
| P1 / KRN-008 | Write VER12 (verification-condition generation) from VER11a §7 to §9 | after KRN-001 |
| P1 / KRN-009 | `cdcl-sat` emits LRAT (a hint chain for every learnt clause). **Raised to P1 (2026-11-24, VER34 13.1):** VER34's CVM-152 (`lrat` reconstructor), CVM-160 and CVM-246d need it, so it is on the critical path of VER34 M1, though not of K-M1 (no bit-level facts needed there). **The verified-checker half ("LRAT turned into proof terms, or a verified LRAT checker in `warrant-lib` run by kernel reduction; measure both") is superseded by VER34 CVM-150/151** (`Pif.Lrat`, `checkLRAT_sound`, on the proof side of VER32 E7); whether reflection fits the kernel is measured by CVM-S1, and CVM-153 is the term-route fallback | `cdcl-sat` only: the KRN-007 prerequisite belonged to the superseded checker half |
| P1 / KRN-010 | Interpreter alignment: trace and steps on every outcome, per-block fuel; differential test of the Lean semantics against the reference interpreter | after KRN-007 |
| P1 / KRN-011 | Claim elaborator (≤ 400 lines) and WIR-to-Lean embedding (≤ 300 lines), literate, mutation corpora; release-checker path: allow-listed environment, no claim-declared constants in the goal, program constant by value, no-`stuck` companion and witnesses, axiom footprint and buckets; report fields; VER13 slice with `kernel.checker.correct`, `kernel.logic.consistent`, `spec.goal.faithful`, `wir.semantics.lean-faithful` and bucket entries in VER31's `buckets` file (signed by the owner); VER31 amendments of VER11a §12.4 (`goals_sha256`, `lean-export:` certificates) | after KRN-005, KRN-007 |
| P1 / KRN-012a | Linear-arithmetic certificate procedure over `Nat` producing Lean proofs (bounds such as `c0 + c1*n ≤ 64*n + 512`). **It is a `Nat` procedure and is not the `Int`/`Pif.Rat` Farkas engine of VER34 (CVM-217)**; neither reuses the other (VER34 13.1) | after KRN-007 |
| P1 / KRN-012 | VC builder emitting **data** for the relational case (program constant plus per-point label table with public variables and public/secret pointer facts, checked by `decide`); test the unary claims in the same obligation-lemma-plus-decidable-check shape before generating any Lean proofs; covers input-contract invariants (exact rejection, `NoEffect`, `NoRead`), label-guided relational invariant with the tag-based secret set, template invariants with `WfState` and stack-length framing, step-lemma application per program point; VER19 predictor gains symbolic allocation sizes | after KRN-008, KRN-011, KRN-012a |
| P1 / KRN-013a | **K-M1a**: input contracts and `no_panic`: `wasm-leb128` `rejects max_bits > 64` and `rejects offset >= len(data)` (exact, effect-free, `max_bits` before any read), `no_panic` and termination on every byte string (needs the front end to lower `wasm-leb128`; otherwise ct-compare `select_bytes` first); ct-compare `select_bytes` rejection and `no_panic`/termination for all four operations | after KRN-010, KRN-012 |
| P1 / KRN-013 | **K-M1b**: ct-compare constant time (with no-`stuck` companion and witnesses differing in every secret) and step, heap, depth bounds for every length; sabotage controls fail; refuted bounds go through VER27's finding protocol; report regenerated; claims move from `bounded(128)` to VER31 rung `checked`; open (c) from 20 to 11 | after KRN-013a |
| P2 / KRN-014 | K-M1c: functional claims (`eq_agrees`, `eq_fixed_agrees_*`, `select_picks`, `select_rejects_mismatch`, `eq_u64_agrees`) as Lean proofs through the escape hatch | after KRN-013 |
| P1 / KRN-015 | Write VER25c: Lean semantics of the Rust subset ct-compare uses, including its events and costs; choose per-function translation validation or per-construct simulation proofs | after KRN-013 |
| P1 / KRN-016 | Rust-subset semantics in `warrant-lib`; differential tests against rustc-built code | after KRN-015 |
| P1 / KRN-017 | **K-M2**: Rust-to-WIR validation (per VER25c); volatile bodies lowered for real; no-`stuck` for the unsafe region; goal restated over the source text and the Rust semantics; the six Rust-to-WIR ids close; `proved` once the owner's VER11a Q5–Q7 decisions and VER31's report-checker closure are in place | after KRN-016; over whatever lowering RS-B4 leaves |
| P2 / KRN-018 | `lean-kernel-ffi`: C-ABI and WASM wrapper (separate crate, `unsafe` for raw pointers, ≤ 200 lines, trusted and counted separately), one entry point `check` | after KRN-005 |
| P1 / KRN-019 | Independent second checker (TypeScript), written without reading `lean-kernel`; agreement gate; evidence kind `independent-kernel-agreement` for `kernel.checker.correct` (route R1) | after K-M1; before any `proved` (owner Q6) |
| P3 / KRN-020 | Deferred: modular relational call contracts; VOS30 adoption of `lockstep_obs`; verifying the kernel in itself (lean4lean direction) | after K-M2 |
| P2 / LEAN-L1b | Mathlib export checked at the pin; time measured | after KRN-005 |
| P2 / LEAN-L2-spec | Write LEAN01 (Lean 4 grammar and extensible syntax): bootstrap grammar, `ParserDescr` translation, engine extensions | after KRN-001 |
| P2 / LEAN-L2-engine | `grammar-tools` engine batch (`ci-full-build`): parser-driven lexing against a changing token trie, Pratt categories, runtime rule and token insertion, scoped activation, position predicates, quotations with antiquotations, `choice` nodes; shares RS01 X1 and X4 | after LEAN-L2-spec |
| P2 / LEAN-L2a | `lean4.tokens`/`lean4.grammar`; `Init` parses and matches the oracle's syntax trees; then core, `Std` (L2-b), Mathlib minus code-defined parsers (L2-c) | after LEAN-L2-engine |
| P2 / LEAN-L4a | `lean-cli`: `#check`, `#print axioms`, `#reduce` on checked exports; REPL | after KRN-005 |
| P2 / LEAN-L3-spec | Write LEAN02 (core elaborator) | after LEAN-L2a |
| P2 / LEAN-L3a | Core elaborator reading the pinned environment-extension snapshot; `warrant-lib` and generated proofs elaborated in-house (the toolchain still supplies the snapshot, LEAN00 QA) | after LEAN-L3-spec |
| P3 / LEAN-L4b | `#eval`: Lean to InterpreterIR on vm-core; Lean runtime primitives; programs built from definitions L3 elaborated. Calls into core-library code follow once LEAN-L5's IR loader lands | after LEAN-L3a |
| P2 / LEAN-L4c | `bv_decide`-like tactic on the Constraint VM with LRAT proof terms | after KRN-009 and LEAN-L3a (tactic framework) |
| P3 / LEAN-L5 | Lean IR and `.olean` loaders, module initialisers, native replacements for inline C, full runtime with kernel externs bound to L1; Lean's frontend on our VM, output re-checked by L1 (L5-a, L5-b, L5-c) | after LEAN-L4b's runtime and a JIT (LANG03); owner QA |

## Proof interchange format (VER32) and first proof target (logic gates)

[VER32](VER32-proof-interchange-format.md) records the owner decisions of
2026-11-24: the kernel keeps Lean 4's type theory (VER11a §3.3, no logical
framework); every prover and our own tools reach it through an untrusted
converter into our own **Proof Interchange Format (PIF)**; and the **logic
gates** are the first proof target, ahead of ct-compare, with the gate model
**Warrant-informed** (becoming from-Warrant when VER33's `logic-gates-model.warrant`
exists) in a separate reviewed model file (Warrant growing into the algorithm
definition language is VER33).
VER11a Q9 (how the kernel reads lean4export's JSON) is **resolved** by this
decision: the kernel reads PIF, and the lean4export reading lives in an untrusted
converter. How the PIF reader is built is **decided** (VER32 Q1, 2026-11-24: a hand-written reader of at most 600 lines, differentially tested, with an approved exemption); the proof side for proof libraries (E7) and option D for G-M2 (Q2) are decided too; Q9 is the one open VER32 question. The spec
was revised after two adversarial reviews (VER32 section 15). Buckets are
VER31's `a|b|c`. Order below is dependency order; one bounded change per PR, spec
before code. Kernel slices 3a and 3b of KRN-002 are defined in the rows below and
slice 3a is being implemented in parallel.

| Priority / ID | Work | Prerequisite and acceptance |
|---|---|---|
| P1 / PIF-000 | VER32 spec, adversarial review (round 1 answered), owner answers to VER32 section 12 | owner answers recorded 2026-11-24 (VER32 12.1); new question Q9 (role words for the WIR semantics file and the program constant) open |
| P1 / KRN-002 slice 3a (defined by VER32) | **Single (non-mutual, non-nested) inductive types, including indexed families**: positivity, universe constraints, derivation of constructors' use and recursors, **reflexive inductives** (`Acc`-style, hypothesis `(xs) → motive js (b xs)`), **ι-reduction**, the **large-elimination rule** (VER32 3.4 table, including the possibly-Prop `Sort u` case), **K-like reduction** for `Eq`-like types, **proof irrelevance and function η** (implemented here), name-collision checks (VER32 3.8). The derived recursor's layout is **normative** (VER32 3.4: the motive level first, named `u` or `u_1`, … if a block parameter is `u`, block parameters never renamed, and absent when Prop-only; params, motive, minors in constructor order with fields then induction hypotheses, indices, major; motive type). **Acceptance:** the recursors of the prelude types equal a committed Lean-exported fixture, and `Or.rec`/`Exists.rec` into `Type` and a level-count mismatch on `Or.rec` are rejected. Delivers everything G-M1 and G-M3a need | in progress in parallel; slice 2 merged |
| P1 / KRN-002 slice 3b (defined by VER32) | **Mutual and nested** inductives, structure detection, **structure η** at recursor majors, **projection reduction**, unit-like types, `mutual`/`nested` features. Not needed by G-M1 or G-M3a (`Prod` is used through `Prod.rec`) | after slice 3a |
| P1 / KRN-002 slice 4 (defined by VER32) | **Literals** (`lits`, `Nat` acceleration, `String` expansion), the **quotient package** (`quotient`, `quot`) and `prim` validation, including the `iff`, `nonempty` and `axiom.*` roles. Not needed by G-M1 or G-M3a | after slice 3b; needed by PIF-009 |
| P1 / PIF-001 | `code/grammars/pif/pif.tokens` and `pif.grammar` (VER32 4.2, 4.3); tokens carry their spacing, so there is no pre-pass; both parse the worked example and the lexical corpus; ci-full-build wiring. **Merged; grammar differences from the first sketch are recorded in 4.2 and 4.3** (never-matching `skip:`, first-match `FIDENT_LONG`, `estr` takes either string token, `trailer` names `EOF`) | after PIF-000 |
| P1 / PIF-002 | **Trusted reader** in `lean-kernel` (≤ 600 lines, literate, `forbid(unsafe_code)`): exactly the rules of VER32 5.3, limits (7), `prim` single assignment, goal checks; counted in the 8,500. SHA-256, import resolver and `nat` parsing counted separately in the trusted base. **Merged (585 counted lines); Q1 (b) and the exemption decided 2026-11-24** | after PIF-001 and KRN-002 slice 2; inductive blocks after slice 3a, mutual/nested/reflexive after 3b, `quot`, `lits` and `prim` after slice 4 |
| P1 / PIF-002b | **Reader file class `ProofSide`** (5.3): R-AX-2 and R-GOL-4 on every proof-side file, the library's name freedom; corpus cases (a proof-side file declaring an axiom, declaring a goal constant, a non-ASCII proof-side name accepted); differential row for PIF-007. **Needed before any VER34 claim file** | after PIF-002 |
| P1 / PIF-003 | `pif-check` binary (replaces `lean-check`): resolver with SHA-256 verification, the gate-fixed ordered environment, accepted declarations, axiom footprint (a constant's footprint includes itself), goal bindings; `pif-fmt` (lint C1 to C3, untrusted); `pif-pp` (the printer, `tcb.pif.printer`). Formats are VER32 3.9. **`pif-eval` is dropped** (VER33 13.1 request 1: the differential states vector theorems and proves them by `Eq.refl`, so no normaliser is needed; VER34's Q13 and 13.1 raise nothing that needs it). PIF-003a to d are implemented | after PIF-002 |
| P1 / PIF-003e | `pif-check`: the **`proof` role and the proof side** (E7, E8): role order `prelude, adl, library, model, axioms, proof, claim`, `ProofSide` wiring, proof-side digests in the report, sign-off and allow-list refuse `proof`; the implemented order has `library` before `model` and no `proof` role, so this changes the chain file's accepted language (a compatible extension) | after PIF-002b, PIF-003d |
| P1 / PIF-004 | **PIF prelude** generator: the inductive types `Bool`, `Nat`, `Int`, `Eq`, `And`, `Or`, `False`, `True`, `Exists`, `Option`, `List`, `Prod` under Lean's names with declarations identical to `Init`'s; native functions and lemmas in `Pif.*` (including `Pif.Not`, `Pif.Result`; **no `Iff`, and `Pif.GateError` is not here: it is declared by the gate model file, ADL-002**); the native prelude and `Init` are **not interchangeable** (Init's function values are elaborator output, `Prod.fst` is a projection); logic-axiom names reserved; digest allow-listed; `pif.prelude.faithful` signed. Recursor binder infos are derived by the kernel as default where Lean writes implicit: metadata only, no effect on digests of binding CTEs | after PIF-002 and KRN-002 slice 3a |
| P1 / PIF-005 | PIF corpus: positive (the VER32 4.6 example first), one file per rule of 5.3, hostile (limits, the 2^1000-node DAG), kernel-negative (including `axiom propext : False`, a registered `Src` name with a different type, a `goal` on an axiom, duplicate claim ids, derived-name collisions, `DuplicateRole`); reader mutation runner; CI job | after PIF-003 |
| P1 / PIF-007 | Differential harness: the reader against the grammar-driven parser **with an independently written semantic layer**, comparing the sequence of kernel calls and the resulting environments, on every corpus file plus grammar-generated and mutated files (evidence for Q1) | after PIF-001, PIF-002 |
| P1 / PIF-017 (= VER33 ADL-003) | **Warrant model elaborator** (untrusted): elaborates Warrant documents into the reviewed **model PIF file (definitions only)** and a separate **goals list** (one line per claim: claim id, a space, the display CTE); today `logic-gates-basic.warrant` has no gate definitions (only `b2u` is a model), so the model is Warrant-informed, and becomes from-Warrant when `code/specs/logic-gates-model.warrant` (VER33 ADL-002) exists. Chain: PIF prelude, ADL prelude, model, claim. Model and goals digests signed under `gates.model.faithful` | after PIF-004, ADL-002 |
| P1 / PIF-006 | **Case-split producer** (untrusted): proofs by exhaustive case analysis over finite inductives **and** bounded-depth splits on `Nat`/`Int` with a symbolic remainder, `False.rec` on contradictory hypotheses, `Option.rec`/`Bool.rec` trees, the T1 elimination principle; may declare no axiom and no constant occurring in a goal; fails closed | after PIF-004, PIF-005, PIF-017 |
| P1 / PIF-020 | VER31 amendment, **superseding** the `goals_sha256` amendment listed under KRN-011: the **goal-signing aggregate** is the SHA-256 of the sorted lines `claim-id goal-digest base-environment-digest` (bytes in 3.9); the signature covers the **base** environment (not the proof side or the claim file) **and the whole sign-off, roles included**, and the sign-off records each claim's base chain with roles (**the `pif-signoff 1` format of 3.9**); the **model-theorem category** with a link status (model-only or code-linked, hand-written or generated, **and the domain actually quantified**; VER32 Q8, VER33 1.5); the `pif:sha256:` certificate kind with kernel crate version and commit | after PIF-000; before G-M1 can be signed **Drafted 2026-11-25, pending owner signature.** Written as proposed text in [VER31](VER31-tiered-release-standard.md) section 13.1 to 13.3 (sign-off gate rules and worked aggregate, `pif:sha256:` certificate kind, model-theorem category with link status and domain quantified). Not in force until the owner signs; the gate half (code) waits for that. |
| P1 / G-M1 | **Gates, model level**: T1 to T5 for the eleven gates over `Int`, the model and goals list from the Warrant-informed model file (PIF-017), the proofs from PIF-006, goal bindings (CTE digests), report fields; ids opened as in VER32 9.2; registered by PIF-016a. Needs KRN-002 slice 3a | after PIF-004, PIF-006, PIF-017 (ADL-002/003), PIF-020, KRN-002 slice 3a **G-M1 status 2026-11-25: implemented, PRs pending (not on `main` yet).** The gate model file, the 89-line goals list and the claim file are produced. All 89 goals are kernel-checked with empty footprints, end to end through `pif-check`, **with a test-fixture sign-off, not a real signature**. The vectors are 1021/1021. Remaining: the owner's signatures (the `pif-signoff 1` file and `buckets-v1.json` revision 2; VER31 13.8), ADL-005b/c, the ADL-008a/b gate conditions, and the push queue (the implementation branches). |
| P1 / PIF-016a | VER13 slice for G-M1: `pif.reader.correct`, `tcb.pif.gate-binding`, `tcb.pif.printer`, `pif.prelude.faithful`, `gates.model.faithful`, `gates.model.code-link`, **and the assumption ids of VER33's ADL items** (VER33 9.1, including `adl.prelude.faithful`, `warrant.model.elaboration-faithful`, `adl.wir-abi.faithful`), with bucket entries (owner signs the bucket file, as for KRN-011) | with G-M1 **Drafted 2026-11-25, pending owner signature.** Registration proposed as data: `code/assumptions/catalogue-v2.proposed.json` and `buckets-v2.proposed.json` (13 ids, no bucket moved), diff summary and signing steps in `code/assumptions/PROPOSED-gm1-registration.md`; the human table is VER31 13.7 and VER13 section 10. **The owner signs `buckets-v1.json` revision 2.** Blocked on PIF-016e (the catalogue family list) before it can be applied. |
| P1 / PIF-016e | **Apply the G-M1 registration** (follows PIF-016a's proposal): extend the catalogue family list with `kernel`, `pif`, `gates`, `warrant` and `adl` in the three places that must agree (`FAMILIES` in `assumption_catalogue.py`, the list in `ca-assurance-report/src/catalogue.rs`, the `family` enum of the JSON schema), add the 13 rows to VER13 section 3 and the generator overlay, regenerate `catalogue-v1.json` and every report's catalogue pin; then the owner signs the bucket file. Small data and checker change; not made in the drafting PR | after PIF-016a is approved and VER31 section 13 is signed |
| P1 / PIF-016d | VER13 slice for **VER34's ids** (parallel to PIF-016a to c; VER34 CVM-025): the ids VER34 opens, **including `tcb.vcgen.claim-faithful`, registered with VER33 (ADL-013, ADL-006a)**, `tcb.drat.checker-correct`, `tcb.bitblast.encoding-correct` and `tcb.symexec.engine-correct`, with bucket entries; spec-only apart from the bucket file | with CVM-025 |
| P1 / PIF-008 | Lean→PIF converter (untrusted crate): reads the pinned lean4export output, drops recursors, emits `prim` (including the `axiom.*` roles), `quot`, features, skips and lists `unsafe`; **replaces KRN-003's lean4export reader**. P1 because PIF-009 depends on it | after PIF-002; pin file |
| P1 / PIF-009 | K-M0 through PIF: Lean's `Init` converted and checked; agrees with the official kernel on every safe declaration; kernel-derived recursors equal the exported ones (replaces KRN-005's input) | after PIF-008, KRN-002 (full) |
| P2 / PIF-010 | PIF→Lean replay harness (oracle only, reads PIF with its **own reader**, calls `addDecl`); recorded agreement as evidence (part of the route that closes `pif.reader.correct`); replaces the harness of KRN-004 | after PIF-003 |
| P1 / PIF-019 | **Straight-line WIR semantics as native PIF** (a generator, not Lean source), so G-M2 does not depend on K-M0, KRN-005 or KRN-007. **It states the word representation for widths above 8** (VER34 13.1 item 4; CVM-155a depends on it: a word is a fixed-length bit list, or a `Nat` below `2^w` through `lits`, decided with the `cvm-smt-v1` spike CVM-S1) | after PIF-004, KRN-006 |
| P1 / G-M2 | **Gates, code level**: the Rust gates lowered to WIR; the G-M1 theorems and a link theorem over the straight-line semantics (PIF-019); scope per VER32 Q2, **decided: option D** (the integer switch-level path covers all 11 gates; option A, **4 of 11 gates, conditional on `gates.nand.primitive-contract`; 7 primitives not covered**, remains only as a relabelled first slice). Opens the `wir.*` ids, closes `gates.model.code-link`; registered by PIF-016b | after G-M1, PIF-019, KRN-006a/b/f; for all 11 gates also G-M2-D |
| P1 / PIF-016b | VER13 slice for G-M2: `gates.nand.primitive-contract` and the `wir.*` bucket entries it relies on | with G-M2 |
| P1 / G-M2-D | `transistors`: an **integer switch-level `evaluate_digital` path** (no `f64`), the voltage model kept as a differential check; extend G-M2 to the seven primitives (VER32 Q2 **option D, decided 2026-11-24**, so all 11 gates are provable at code level). Raised from P2 because the owner's decision makes it the route, not an option | after G-M1 (it does not need G-M2's proofs, only G-M2's lowering target); before G-M2 can claim all 11 |
| P1 / PIF-018 | **Native induction and rewriting term builder** (untrusted producer for G-M3a, so P1 with it): motives, `Nat.rec`/`List.rec` induction, `Eq.rec`/`congrArg` rewriting. The alternative is to admit the Lean toolchain through PIF-008 as the producer | after PIF-006 |
| P1 / G-M3a | **Compositions, model level**: half adder, full adder, n-bit ripple-carry adder correct for every `n ≥ 1` by induction, with the model rejecting unequal lengths, empty vectors and non-bits; native `Pif.Nat` lemma set, no literals; opens `gates.compose.faithful` and `gates.compose.code-link`; registered by PIF-016c | after G-M1, PIF-018 |
| P1 / PIF-016c | VER13 slice for G-M3: `gates.compose.faithful`, `gates.compose.code-link` | with G-M3a |
| P2 / G-M3b | Adder at code level (allocation and memory semantics, VER11a §6.3; the loop's block invariant) and a link theorem, for every `n ≥ 1`; closes `gates.compose.code-link` | after G-M2, KRN-012, KRN-013a |
| P2 / PIF-011a | **Rocq feasibility spike** (time-boxed): translate a small development (inductives, structural `Fixpoint`, an `SProp` use and a cumulativity case); decide whether the `SProp` to `Prop` conjecture and the cumulativity lifts hold | after PIF-009 |
| P2 / PIF-011 | Rocq→PIF converter (untrusted; VER32 6.2; refuses impredicative Set, primitive ints/floats/arrays, coinduction, non-structural fixpoints, `Src` axioms of unestablished consistency) | after PIF-011a |
| P2 / PIF-012 | HOL-family converters with relativisation (OpenTheory first, then HOL Light and HOL4; Isabelle/HOL later; VER32 6.3) | after PIF-009 |
| P3 / PIF-013 | Reader option (a): a `grammar-tools`-generated reader behind a flag, compared with the hand-written one; adoption decided on size and audit | after PIF-007; Q1 |
| P3 / PIF-014 | Compact binary form (a mechanical bijection of the text records; digest over the text) | after PIF-009 measures `Init` |
| P2 / PIF-015 | ct-compare on PIF: KRN-013a/013 emit PIF through the claim elaborator and the generic producers; the Lean toolchain optional | after G-M2 |

## Warrant algorithm definition language (VER33)

[VER33](VER33-warrant-algorithm-definition-language.md) records the owner decisions of
2026-11-24: Warrant, extended with an opt-in algorithm profile, is the repository's algorithm
definition language, with **partial adoption first-class** (rungs A0 tests/PBT/fuzz from a
Warrant file for existing code, A1 solver-backed testing, A2 kernel proofs of the model, A3
code-linked proofs by lifting, A4 generated idiomatic code). The queue is the shard
[VER-BACKLOG.d/warrant-adl.md](VER-BACKLOG.d/warrant-adl.md): **every row is one PR of about
100 to 400 lines**, per-lane lifter and backend rows included. The VER32 section (`PIF-*`,
`G-M*`) lands via #20009; this section goes after it when both are on the base. Follow-up
specifications named, not written: **VER33a** (lifters, ADL-030) and **VER33b** (ABI,
`Refines`, certificate, ADL-020). Nothing starts before the owner approves VER33 and a second
adversarial review has been answered.

Milestones (rows in the shard):

- **M0** Spec and decisions: ADL-000, ADL-020, ADL-030
- **M1** Profile and the gate models: ADL-001a to c, ADL-002, ADL-002b, ADL-004a/b (unblocks G-M1)
- **M2** Elaborator and evidence for its meaning: ADL-003a to c, ADL-005a to e, ADL-008a/b, G-M1
- **M3** Registration, gate and the package-claim rule: ADL-006a, ADL-006b, ADL-006c1 to c3, ADL-007, ADL-009
  - **2026-11-25:** ADL-006a and ADL-006b are **drafted, pending owner signature** (VER31 section 13, `code/assumptions/PROPOSED-gm1-registration.md`); G-M1 is **implemented, PRs pending**.
- **M4** Solver-backed testing (A1): ADL-010, ADL-011, ADL-012a/b, ADL-013 to ADL-015
- **M5** ABI, refinement and the A3-WIR link for Rust: ADL-021 to ADL-023, ADL-032, ADL-033, G-M2
- **M6** Generated code (A4): ADL-040 to ADL-047, G-M4 (per-lane rows)
- **M7** Recursive algorithms: ADL-050, ADL-051, G-M3a, G-M3b
- **M8** A3-source and per-lane lifters (all depend on K1, which is now native words and `lits` with **no `Init`**; PIF-009 stays listed until ADL-030 decides whether the lane semantics need `Init`; the `Init` tier is ADL-060b): ADL-031, ADL-035, ADL-L01 to L09, ADL-060a, ADL-060b, ADL-061

## Constraint VM as Warrant's SMT solver (VER34)

[VER34](VER34-constraint-vm-smt-solver.md) (owner direction 2026-11-24; adversarial
review round 1 answered) grows the Constraint VM into the SMT solver Warrant uses,
assembled from small units that fit the LANG24 layers (an opcode, a tactic, a pass,
a certificate emitter, a reconstructor, a combinator, a harness). The solver is
untrusted: UNSAT counts toward `proved` only as a PIF proof the kernel checks. The
rows, one PR each, with a certificate column, are the decomposition tree and live
in the shard [constraint-vm-smt](VER-BACKLOG.d/constraint-vm-smt.md) (262 rows).
Milestones:

| Milestone | Rows | Gate (VER34 section 10) |
|---|---|---|
| CVM-M0 foundations | 50 | Baseline vs Z3/cvc5 recorded; combinators, push/pop, unsat cores, two evaluators, trace and tactic grammars, boundary points; unlocks VER33 ADL-007 on Bool/BV |
| CVM-M1 certified QF_BV (P1 operators) | 68 | After the spikes CVM-S1 (kernel fit) and CVM-S2 (authoring), owner Q7, and Q13 (**decided 2026-11-24: Lean as an untrusted producer, conditional on CVM-S2**): gate and ct-compare obligations kernel-checked under `cvm-smt-v1`; QF_BV `R` gates; `c_kernel` 100% on the Warrant corpus; unlocks ADL-008 and ADL-018 for fixed-width obligations |
| CVM-M1b BV widening | 22 | Shifts, multiplication, division with their bridges and lemmas |
| CVM-M2 EUF, arrays, LRA, LIA, combination | 62 | Per-logic `R` gates for QF_UF, QF_LRA, QF_LIA, QF_AX, QF_UFBV; `c_trace` at least 95%, `c_kernel` 100% on the Warrant corpus |
| CVM-M3 datatypes, quantifiers, nonlinear, classical | 36 | `R` gates for QF_DT, quantified UF/LIA, QF_NIA; classical claims reach `checked` until VER11a Q5 |
| CVM-M4 strings, sequences, floating point | 20 | Per-logic `R >= 0.2`; FP waits for a kernel IEEE model |
| CVM-M5 hardening and speed | 4 | Re-baselined `R`; rows added from the CVM-500 profile |
