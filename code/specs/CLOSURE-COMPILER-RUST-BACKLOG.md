# Closure Compiler on Rust — parity backlog

**Status:** active  
**Last reprioritized:** 2026-09-19  
**Current selection:** CCR-004, refresh the 462-fixture golden cohort against
the pinned upstream oracle
([#15569](https://github.com/adhithyan15/coding-adventures/issues/15569))<br>
**Current local loop base:** `coding-adventures` at
`31de98aecb5d260e5a5b84328f4038dff179c954`<br>
**Local audit base:** `coding-adventures` at `06fc0524051a397ccc53c628b08c019b2bbf75ba`  
**Upstream audit base:** `google/closure-compiler` at
`10ca677aff381d2c2e6e1b254ba32861e503173d` (2026-09-17), current release
`v20260915`

## Purpose and truthfulness boundary

This is the live, evidence-backed queue for turning `closurec` and its Rust
libraries into a practical replacement for the Google Closure Compiler. It
supersedes status prose in older CLOC specs when the two disagree; those specs
remain the design and historical slicing record.

"Parity" is not one boolean. We track five independently verifiable levels:

1. **Surface parity:** a Closure command line parses with compatible defaults,
   validation, diagnostics, and exit status.
2. **Language parity:** supported JavaScript parses, lowers, prints, and maps
   back to sources without silently changing compilation mode.
3. **Pass parity:** checks and optimizations have the same observable behavior
   as the pinned upstream compiler for the covered corpus.
4. **Build parity:** dependency management, modules, chunks, externs, source
   maps, manifests, JSON streams, and reports compose as they do upstream.
5. **Operational parity:** the same invocation is deterministic and supported
   on Linux, macOS, and Windows, with useful failures and bounded resource use.

The present implementation has a strong WHITESPACE_ONLY formatter, a real but
partial SIMPLE/ADVANCED typed pipeline, and a large curated golden corpus. It
is **not yet a drop-in Closure replacement**: successful flag parsing often
does not imply semantics, unsupported typed syntax now fails closed rather than
silently weakening output, BUNDLE and TRANSPILE_ONLY are identity operations,
emitted source maps are placeholders, the type checker is a passthrough
scaffold, and collapse-properties does not mutate the program.

## Evidence captured by the 2026-09-19 audit

### Local implementation

- `closurec` is version `0.242.0` and declares 111 flags in `cli.spec.json`.
- All 462 curated differential fixtures match their checked-in expected bytes.
  Most WHITESPACE_ONLY fixtures cite Closure `v20240317`; newer SIMPLE fixtures
  cite `v20260712`, so the corpus does not yet share one current oracle pin.
- The conformance suite has 24 declared cases: 19 compare values and five are
  explicit unsupported-syntax declines. All three harness tests pass.
- Both correlation-vector provenance tests pass.
- CCR-001 fixed Windows-native absolute, drive-rooted, UNC, and
  mixed-separator glob expansion in [#15529](https://github.com/adhithyan15/coding-adventures/pull/15529).
  Its full Windows validation passed all 688 unit tests and every integration
  target before CI passed on Windows, macOS, and Ubuntu.
- A fresh `v20260915` oracle probe found a false-success compilation defect:
  upstream rejects malformed `var = ;` with exit 1, `JSC_PARSE_ERROR`, and no
  output, while `closurec --compilation_level SIMPLE` exits 0 and emits
  `var=;`. For destructuring that upstream compiles successfully, `closurec`
  silently emits whitespace-only output after the typed bridge declines it.
  The exact oracle hash and reproduction are tracked in
  [#15534](https://github.com/adhithyan15/coding-adventures/issues/15534).
- CCR-002 fixed that defect in
  [#15544](https://github.com/adhithyan15/coding-adventures/pull/15544):
  parse, typed-bridge, pass, and emit failures now surface as structured
  diagnostics and a failed process without JavaScript, output files, or
  correlation-vector sidecars.
- `tests/diff/` contains 626 fixture directories, not one homogeneous corpus:
  462 are `minify_*` stdout goldens targeted by CCR-004, while 164 exercise
  CLI, diagnostic, typed-pipeline, source-map, and local extension contracts.
  Of 546 fixture READMEs, 466 contain a version token and only 22 contain a
  reproducible `java -jar` command. Eight minify READMEs have no version token.
  Existing version mentions are dominated by `v20240317`, so README prose
  cannot serve as the oracle registry.
- The AST covers the main statement and expression families, including
  classes, modules, optional chains, generators, templates, and async nodes.
  Binding targets remain identifier-only, so destructuring and several
  parameter forms cannot cross the typed bridge.
- `closure-typechecker` documents and implements a passthrough v1 with no
  inference diagnostics. `closure-pass-collapse-properties` discovers
  candidates but intentionally reports `changed = false` and applies none.
- The command runner consumes only a subset of the parsed configuration. Major
  families still lacking end-to-end semantics include diagnostics, dependency
  and chunk management, polyfills, conformance, instrumentation, translations,
  JSON streams, renaming map inputs/reports, and several special passes.
- `--create_source_map` currently calls a minimal v3 serializer with empty
  sources and mappings. The repository already has a real VLQ source-map crate,
  but the typed emitter path does not feed it spans.

### Current upstream surface

- The current Maven release is `v20260915`; the audited upstream checkout is
  two days newer at commit `10ca677a`.
- The release tag is annotated object `72421c28d352e5dda9a111bec39c3d41af46f3a3`
  and dereferences to commit `56007b2869ef6ce70b659b033459b8d8113101de`.
  The Maven artifact
  `com.google.javascript:closure-compiler:v20260915` is 14,976,538 bytes with
  SHA-256
  `9C8AF06056AA06F968B5A457540A85869C7BA2861C211C56D8D4EF6C35DDF36D`;
  its manifest records JDK 21 and it embeds Apache-2.0 license and notice files.
- Current `CommandLineRunner.java` exposes 102 explicit option declarations.
  After accounting for cli-builder-provided help/version and the eleven local
  correlation-vector extensions, the actionable naming drift is the upstream
  canonical `--typed_ast_output_file` versus the local legacy misspelling
  `--typed_ast_output_file__INTENRNAL_USE_ONLY`.
- The audited upstream test directory contains 395 `*Test.java` files. Signal
  clusters include 70 `Check*`, 28 `Es6*`, 19 `*Module*`, 13 `TypeCheck*`,
  12 `Inline*`, ten transpilation, eight peephole, six chunk, five collapse,
  five rename, four remove-unused, three source-map, three conformance, two
  code-printer, and one command-line test file.
- Upstream's `DefaultPassConfig` composes many checks and optimizations beyond
  the local scheduler: module/JSDoc/variable/strict/type/control-flow/access/
  conformance checks; normalization; property disambiguation and collapse;
  function, call, constructor, and variable inlining; cross-chunk movement;
  dead-assignment and unused-code removal; polyfill/transpilation passes; and
  final property, variable, and label renaming.

Primary upstream references:

- <https://github.com/google/closure-compiler>
- <https://github.com/google/closure-compiler/blob/master/src/com/google/javascript/jscomp/CommandLineRunner.java>
- <https://github.com/google/closure-compiler/blob/master/src/com/google/javascript/jscomp/DefaultPassConfig.java>
- <https://github.com/google/closure-compiler/wiki/Design-Documents>
- <https://github.com/google/closure-compiler/wiki/JS-Conformance-Framework>

## Completion contract

An item is complete only when its behavior is specified, covered by focused
tests and at least one end-to-end test where applicable, documented in package
README/changelog files, formatted and lint-clean with warnings denied, and
verified through every affected Cargo workspace with `--no-fail-fast`. Parity
items additionally require a reproducible upstream oracle command or a ported
upstream test with an exact source tag/commit.

The overall project is complete only when:

- supported and intentionally unsupported upstream flags are machine-audited;
- no successful compilation silently substitutes a weaker compilation level;
- the supported JavaScript/language-mode matrix is explicit and tested;
- all selected upstream test families are ported or listed with a reason;
- source maps, diagnostics, modules/chunks, externs, and reports work together;
- Linux, macOS, and Windows run the same conformance gate; and
- a representative real-world Closure build passes a differential migration
  trial with documented output, diagnostics, source-map, and runtime results.

## Prioritization policy

Re-run priority after every merged PR and immediately after discovering work.
Order by:

1. security, data loss, broken main, nondeterminism, and cross-platform test
   failures;
2. silent semantic corruption or successful-but-weaker fallback;
3. oracle quality and measurements needed to evaluate later work;
4. end-to-end correctness before adding isolated pass breadth;
5. broadly used Closure workflows before niche flags;
6. smallest dependency-unblocking slice, with only one active shared-crate PR.

New findings are added before selecting the next item. IDs are stable; priority
order may change. A row marked `Blocked` names its prerequisite. `Survey` means
the implementation must begin with a measured inventory rather than assumed
scope.

## Ordered queue

### P0 — restore a trustworthy baseline

| Rank | ID | Work item | Acceptance evidence | Status |
|---:|---|---|---|---|
| 1 | CCR-001 | Make absolute and relative `--js` globs use native Windows separators and roots without regressing POSIX behavior. | The six Windows failures pass; focused mixed-separator, exclusion, `*`, and `**` tests pass; full `closurec` suite is green on Windows. | Complete — [#15529](https://github.com/adhithyan15/coding-adventures/pull/15529) |
| 2 | CCR-002 | Replace silent SIMPLE/ADVANCED typed-pipeline fallback with an explicit compatibility policy. Unsupported syntax must either be a hard error or an opt-in, diagnostic-bearing fallback. | Differential tests prove exit code, stderr, and output for parse, bridge, pass, and emit failures. | Complete — [#15544](https://github.com/adhithyan15/coding-adventures/pull/15544) |
| 3 | CCR-003 | Establish one reproducible oracle manifest pinned to upstream `v20260915` and commit `10ca677a`. Record Java version, commands, flags, hashes, licensing, and fixture provenance. | Offline manifest verifier classifies all 626 differential fixture directories exactly once; every upstream-derived golden resolves to one immutable artifact pin and normalized command. | Complete — [#15558](https://github.com/adhithyan15/coding-adventures/pull/15558) |
| 4 | CCR-004 | Re-run all 462 golden fixtures against the new oracle, classify drift, and update only reviewed deltas. | Machine-readable report records equal/changed/declined counts and every changed byte has a linked reason. | In progress — [#15569](https://github.com/adhithyan15/coding-adventures/issues/15569) |
| 5 | CCR-005 | Generate a CLI surface audit from current `CommandLineRunner.java` and `cli.spec.json`, separating upstream, generated, extension, deprecated-alias, and unsupported flags. | CI fails on unclassified flag drift; canonical `--typed_ast_output_file` is accepted with tested compatibility for the legacy typo. | Ready |
| 6 | CCR-006 | Add an explicit capability/status command or document generated matrix so users can tell parse-only flags from implemented semantics. | Matrix is generated from the same registry used by dispatch and cannot drift manually. | Blocked by CCR-005 |

### P1 — make the front end and output contract honest

| Rank | ID | Work item | Acceptance evidence | Status |
|---:|---|---|---|---|
| 7 | CCR-007 | Wire real VLQ source maps through parser spans, transforms, emitter, wrappers, and output paths. | Upstream source-map vectors plus multi-file, Unicode, wrapper, stdin, and transformed-token end-to-end tests match decoded mappings. | Ready |
| 8 | CCR-008 | Define and enforce `language_in`; stop treating accepted syntax as independent of the selected input mode. | Current upstream accept/reject corpus matches diagnostics and exit status for representative ECMAScript modes. | Ready |
| 9 | CCR-009 | Implement `language_out` and TRANSPILE_ONLY as real lowering stages rather than identity. Start with optional chaining/nullish coalescing and class features. | Runtime-equivalent output and upstream differential tests across at least two output modes. | Ready |
| 10 | CCR-010 | Extend binding targets across AST, parser, typed bridge, scope analysis, passes, and emitter for array/object destructuring. | Existing declined conformance cases become value-checked; declaration, assignment, parameter, rest, default, and loop targets pass. | Ready |
| 11 | CCR-011 | Complete async/await typed bridging, including `for await` and async generators. | CLOC12 gap-165 closes and async fixtures stay on the typed pipeline. | Ready |
| 12 | CCR-012 | Replace template-literal mode heuristics with an explicit substitution stack supporting nested templates and complex expressions. | CLOC12 gap-044b closes with nested, tagged, escaped, and multiline tests. | Ready |
| 13 | CCR-013 | Survey current parser/bridge/emitter syntax against upstream language-feature enums and turn every decline into a stable capability row. | Generated syntax matrix names pass/fail stage and test for every feature. | Survey |
| 14 | CCR-014 | Integrate the type checker's first real vertical slice: primitive literals, variables, assignments, calls, returns, and JSDoc annotations. | Diagnostics have stable groups/locations; upstream TypeCheck tests for the slice pass; `checks_only` emits no JS. | Ready |
| 15 | CCR-015 | Implement diagnostics plumbing: levels, groups, warnings guards, error formatting, counts, JSON/error streams, and exit codes. | `--jscomp_*`, formatting, hide-warnings, summary detail, and output streams match selected CommandLineRunner tests. | Ready |
| 16 | CCR-016 | Replace minimal numeric printing with a shortest-round-trip algorithm and audit special numeric property keys. | Exhaustive boundary/property tests plus upstream CodePrinter numeric vectors close residual CLOC12 numeric gaps. | Ready |
| 17 | CCR-041 | Derive correlation-vector pass inventory from the scheduler's actual execution order instead of parallel constants. SIMPLE currently reports `inline` even though that pass is only registered for ADVANCED. | SIMPLE and ADVANCED provenance tests prove that every reported pass actually ran and that conditional passes appear only when scheduled. | Ready — [#15542](https://github.com/adhithyan15/coding-adventures/issues/15542) |

### P2 — strengthen SIMPLE and ADVANCED optimization semantics

| Rank | ID | Work item | Acceptance evidence | Status |
|---:|---|---|---|---|
| 18 | CCR-017 | Implement collapse-properties mutation with namespace safety, alias, extern, getter/setter, and dynamic-access guards. | Ported `CollapsePropertiesTest` slices pass and fixed-point termination is proven. | Ready |
| 19 | CCR-018 | Audit remove-unused-vars for nested scopes, exports, destructuring, side effects, classes, and module bindings; implement missing cases. | Ported `RemoveUnusedVarsTest` slices and runtime-effect tests pass. | Ready |
| 20 | CCR-019 | Expand constant folding/peephole coverage and refresh its upstream pin. | All selected current `Peephole*` vectors are ported, passing, ignored with gap IDs, or skipped with reasons. | Survey |
| 21 | CCR-020 | Expand control-flow folding and DCE for switch/try/finally/labels/loops/throw and unreachable lexical declarations. | Ported peephole/remove-dead tests plus runtime-equivalence tests pass. | Ready |
| 22 | CCR-021 | Expand function/variable inlining with escape, recursion, evaluation-order, `this`, `arguments`, async/generator, and multi-statement safety. | Current upstream Inline test inventory is classified and selected cases pass. | Survey |
| 23 | CCR-022 | Complete variable/global/property/label renaming semantics, maps, stable names, extern protection, reserved names, and shadowing. | Rename test families pass; input/output map round trips are deterministic. | Ready |
| 24 | CCR-023 | Add missing ADVANCED passes in dependency order: normalization, call optimization, constructor optimization, dead assignments, devirtualization, property disambiguation/ambiguation, and final denormalization. | Each pass lands with a ported upstream slice and pipeline-order invariant. | Survey |
| 25 | CCR-024 | Make pass configuration reflect upstream compilation levels and relevant option interactions instead of a fixed local list. | Generated schedule snapshots and end-to-end flag interaction tests match the pinned upstream configuration. | Blocked by CCR-015 and CCR-023 |

### P3 — real build-graph and Closure workflows

| Rank | ID | Work item | Acceptance evidence | Status |
|---:|---|---|---|---|
| 26 | CCR-025 | Implement dependency modes, entry points, `goog.provide`/`goog.require`, CommonJS processing, and ES module resolution/rewriting. | Multi-file graph fixtures match order, pruning, diagnostics, and output. | Ready |
| 27 | CCR-026 | Implement chunks/modules, chunk wrappers, chunk maps, output prefixes, weak chunks, and cross-chunk motion. | DAG validation and selected upstream Chunk/Module tests pass on all hosts. | Blocked by CCR-025 |
| 28 | CCR-027 | Make BUNDLE a real bundling mode with module resolution, ordering, wrappers, source maps, and diagnostics. | A representative multi-module application bundles and runs equivalently to upstream. | Blocked by CCR-025 and CCR-007 |
| 29 | CCR-028 | Complete extern ingestion and built-in environment selection, including browser/custom extern interactions. | Extern diagnostics and property/variable protection match upstream cases. | Ready |
| 30 | CCR-029 | Implement polyfill isolation/injection/rewrite and runtime-library selection. | Output and runtime behavior match upstream across language-out modes. | Blocked by CCR-009 |
| 31 | CCR-030 | Implement manifests, dependency graphs, variable/property maps, renaming reports, exports, and name-reference reports. | Every report has deterministic golden and round-trip tests. | Blocked by CCR-022 and CCR-025 |
| 32 | CCR-031 | Implement JSON input/output streams and stdin/stdout composition without mixing diagnostics into data streams. | Pipeline and malformed-stream tests match upstream exit/status behavior. | Blocked by CCR-015 |

### P4 — checks, policy, specialized passes, and operational proof

| Rank | ID | Work item | Acceptance evidence | Status |
|---:|---|---|---|---|
| 33 | CCR-032 | Implement the JS conformance framework: config parsing, allow/deny lists, requirement matching, diagnostics, and reporting. | Selected upstream `Conformance*Test` cases pass and local five declines are reviewed. | Blocked by CCR-014 and CCR-015 |
| 34 | CCR-033 | Add remaining checks in measured slices: variables, modules, JSDoc, strict mode, control flow, access controls, suspicious code, and side effects. | Each check has a diagnostic group and current upstream test slice. | Survey |
| 35 | CCR-034 | Implement specialized passes only after core workflows: Angular/Polymer/J2CL, Closure primitives, CSS/ID replacement, define/tweak processing, message replacement, and translation bundles. | Each accepted flag gains end-to-end semantics or an explicit unsupported error. | Blocked by CCR-024 |
| 36 | CCR-035 | Implement instrumentation and coverage-array modes with source-map and optimization interaction tests. | Instrumented output executes and reports the same covered regions as upstream fixtures. | Blocked by CCR-007 and CCR-024 |
| 37 | CCR-036 | Add typed-AST serialization/deserialization using the canonical flag and versioned format; decide compatibility boundary with upstream protobuf. | Round trip preserves the supported AST and rejects incompatible versions safely. | Blocked by CCR-013 and CCR-014 |
| 38 | CCR-037 | Build a continuously generated upstream test-coverage ledger for all 395 audited test files. | Every file is ported, partially ported, blocked by a gap, or skipped with a reviewed reason and upstream hash. | Ready |
| 39 | CCR-038 | Add deterministic fuzz/property testing for lexer/parser/emitter round trips, pass idempotence, glob/resource bounds, and source maps. | Fixed seeds reproduce failures; corpora run on all supported hosts under explicit budgets. | Blocked by CCR-007 and CCR-013 |
| 40 | CCR-039 | Measure performance and memory against upstream on small, medium, and large real projects; optimize only profiled bottlenecks. | Published reproducible benchmark, peak-memory, and output-size report with regression budgets. | Blocked by CCR-024 and CCR-027 |
| 41 | CCR-040 | Run a real-world migration trial and publish the remaining incompatibility ledger. | Build, runtime, diagnostics, artifacts, maps, and performance are compared against the same upstream pin on all three hosts. | Blocked by core P0-P3 work |

## Loop record

Update this section at every selection and merge so the backlog explains why
the loop moved.

| Date | Event | Prioritization result |
|---|---|---|
| 2026-09-19 | Initial code/upstream audit. Full Windows test run found six failures sharing one root cause; no open Closure/CLOC PRs were found. | Selected CCR-001 because broken cross-platform tests and unusable native absolute globs outrank semantic expansion. |
| 2026-09-19 | CCR-001 implementation verified locally: all 688 unit tests and every integration target pass under `--no-fail-fast` on Windows. | Keep CCR-001 selected until its PR is green and merged; then re-fetch, record completion, and reprioritize the full queue. |
| 2026-09-19 | CCR-001 merged as [#15529](https://github.com/adhithyan15/coding-adventures/pull/15529) at `a31b2ccc`; upstream remained at commit `10ca677a` and release `v20260915`. Direct oracle probing then proved malformed input exits 1 upstream but exits 0 with emitted JS locally, and proved typed-bridge declines silently weaken SIMPLE output. | Selected CCR-002 ([#15534](https://github.com/adhithyan15/coding-adventures/issues/15534)). A false-success compiler result outranks oracle infrastructure and all semantic expansion work under priority rule 2. |
| 2026-09-19 | While implementing CCR-002, inspection found that SIMPLE correlation-vector provenance reports `inline` although the scheduler registers that pass only for ADVANCED; logged as CCR-041 ([#15542](https://github.com/adhithyan15/coding-adventures/issues/15542)). CCR-002's complete package suite passed after implementing explicit typed-pipeline errors. | Kept CCR-002 selected because false-success output remains the highest-priority open defect until merged. Ranked CCR-041 after front-end/output correctness work and before broader optimization expansion because inaccurate provenance impairs later parity measurement but does not change emitted JavaScript. |
| 2026-09-19 | CCR-002 merged as [#15544](https://github.com/adhithyan15/coding-adventures/pull/15544) at `608963c3`; all required checks passed and no competing Closure PR remained. Fresh upstream verification found no release or `master` drift. Corpus inventory found 626 differential fixture directories, fragmented provenance, and only 22 README capture commands. | Selected CCR-003 ([#15549](https://github.com/adhithyan15/coding-adventures/issues/15549)). With false-success compilation repaired, oracle quality is the highest-priority dependency: it makes the 462-golden refresh reviewable and separates upstream evidence from local-only extension contracts before more semantic work lands. |
| 2026-09-19 | CCR-003 implementation validated locally. The strict offline verifier classifies all 626 fixtures exactly once, preserves the 462-fixture minify cohort, verifies immutable release/artifact/capture pins and evidence paths, and keeps local contracts out of upstream provenance. The full `closurec` suite, all-target Clippy, and all 567 lesson shards pass; Clippy emits only the repository's pre-existing obsolete-lint warning. | Keep CCR-003 selected through publication and merge. CCR-004 remains the next dependency-ordered candidate, subject to a fresh upstream/repository audit and reprioritization after merge. |
| 2026-09-19 | CCR-003 merged as [#15558](https://github.com/adhithyan15/coding-adventures/pull/15558) at `31de98ae` after all 46 checks passed. The merge commit is reachable from `origin/main`; no competing Closure/CLOC PR exists; upstream `master` remains `10ca677a` and the latest release remains `v20260915`. No newly discovered security, data-loss, cross-platform, or false-success defect outranks the now-unblocked oracle refresh. | Selected CCR-004 ([#15569](https://github.com/adhithyan15/coding-adventures/issues/15569)). It is the smallest dependency-unblocking measurement slice and must settle the current 462-golden baseline before CLI-surface or semantic expansion. |
