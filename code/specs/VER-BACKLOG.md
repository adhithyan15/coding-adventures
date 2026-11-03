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
| P1 / VER-P004 | Rust CT01 corpus through L0 | #18849 retargeted to main after P003 merge; local checks pass; awaiting final-head hosted CI; every case, fixed-array API, fresh output, error mapping, logical work fields, regeneration/build selection |
| P1 / VER-I004 | Enforce TF01's closed failure vocabulary at the Rust harness boundary | ready after P004; rejected invalid IDs must not escape as unregistered TAP diagnostics; spec, negative corpus and caller compatibility before code |
| P1 / VER-P005 | Warrant v0 specification | waiting P004; VER04 syntax/types/bindings and negative examples |
| P1 / VER-P006 | Warrant grammar/generated lexer/parser | waiting P005; positive/negative corpus and drift gate |
| P1 / VER-P007 | Warrant type checker/Rust bindings | waiting P006; missing exports, wrong types and vacuous bindings reject |
| P1 / VER-P008 | Rust backend and ct-compare.warrant | waiting P007; generated tests through L0, checked regeneration |
| P1 / VER-P009 | Property spec and pbt-v1 corpus | waiting P008; VER05 generation, integrated shrinking, coverage/negative cases |
| P1 / VER-P010 | Rust property engine | waiting P009; deterministic replay, shrink quality, bounded work |
| P1 / VER-P011 | Warrant claims to properties | waiting P010; public API calls and broken implementations detected |
| P1 / VER-P012 | Stateful properties | waiting P011; select only if pilot needs them, otherwise explicitly defer |
| P2 / VER-P013 | Fuzz spec and fuzz-v1 corpus | waiting P011; VER06, private failure-output policy |
| P2 / VER-P014 | Rust fuzz engine | waiting P013; bounded runs, replay, honest coverage label |
| P2 / VER-P015 | ct-compare fuzz targets | waiting P014; no-panic/agreement, private triage of new failures |
| P2 / VER-P016 | Microbenchmark spec/harness | waiting P003; VER07 counts distinct from measured timing |
| P2 / VER-P017 | Equal-instruction-count measurements | waiting P016; n=16/256/4096; absent counters report not-live |
| P1 / VER-P021 | WIR core/reference interpreter | waiting P011; VER25 semantics, types, cost/label annotations |
| P1 / VER-P022 | Rust-subset grammar | waiting P021; real ct-compare source, volatile/unsafe assumptions |
| P1 / VER-P023 | Rust-to-WIR front end | waiting P022; differential checks against real public entry points |
| P1 / VER-P024 | Symbolic execution | waiting P023; VER19, certified UNSAT, honest bounded(k) labels |
| P1 / VER-P025 | Bounds/constant-time trace proofs | waiting P024; VER20, branch/address self-composition, not counts alone |
| P2 / VER-P027a | Rust capability analyzer/name allowlist | waiting P023; VER08/23, macro policy, negative corpus |
| P2 / VER-P027b | Generated effect module/certificate | waiting P027a; VER23/24, small checker, empty effects |
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

I004 is being prepared locally on its own branch while #18849 waits for CI;
it has no PR and is not delivered. Its spec preserves the string-taking
Verdict API and validates at serialization, so direct construction cannot
bypass the guard. Publication still requires P004 merge, a fresh overlap and
priority audit, final-source validation and security review.

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
Bootstrap's exact head passed a fresh security sub-agent
review; its existing auto-merge remains waiting for CI.

P004 plan: specify deterministic CT01-to-TFV projection and fixed-width dispatch;
falsify stale transport/work/results; execute all real public operations through
L0; observe length-error panics and mutate fresh selection output; check logical
work fields; register corpus/reference build inputs and regeneration on both
platforms; update package/pilot/quirk docs; run Rust/Python/Go checks and security
review; publish a dependent draft, then retarget/mark ready after P003's merge is
verified (now done). Reprioritize again after P004's actual merge before selecting
P005. The local Warrant notes are preparation, not an accepted spec or code.
