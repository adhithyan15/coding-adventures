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
| P1 / VER-P006a | Canonical Warrant grammars, compiled Rust grammar data and syntax corpus | selected after P005 merge and 7-open-PR overlap audit; spec staging/transport committed before implementation; exact subtree witnesses, negative controls and regeneration |
| P1 / VER-P006b | Guarded Warrant source lexer/parser | after P006a; original-byte spans, admission/work quotas and measured multi-shape native-stack guard; no unguarded public parser in P006a |
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
