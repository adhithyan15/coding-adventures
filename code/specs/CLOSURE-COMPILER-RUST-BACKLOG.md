# Closure Compiler on Rust — parity backlog

**Status:** active  
**Last reprioritized:** 2026-09-21  
**Current selection:** CCR-047, the differential complexity ladder
([#15837](https://github.com/adhithyan15/coding-adventures/issues/15837))<br>
**Current local loop base:** `coding-adventures` at
`d2610543e665da8978d45a49ea8993853810d7bb`<br>
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

**Measured agreement against the pinned oracle** (52-rung complexity ladder,
2026-09-21, [#15837](https://github.com/adhithyan15/coding-adventures/issues/15837)):

| Level | Rungs agreeing | |
|---|---:|---|
| `WHITESPACE_ONLY` | 49 / 52 | 94% |
| `SIMPLE` | 31 / 52 | 60% |
| `ADVANCED` | 13 / 52 | 25% |

Agreement falls as the amount of claimed optimization rises. The ADVANCED
figure flatters it: most of its thirteen agreements are trivial rungs where
there is nothing to optimize. Upstream ADVANCED reduces whole programs to their
observable effect (`var o={a:1,b:2};console.log(o.a)` → `console.log(1)`);
we emit approximately the input. ADVANCED is better described as **scaffolded
but not implemented** — the pass slots exist and are scheduled, but the
transformations that define the level are largely absent. A user selecting
`ADVANCED` today gets substantially less than the flag implies, and unlike a
hard failure nothing tells them so.

Note that the 462-fixture golden corpus reports 100% agreement over the same
oracle. It measures regression, not parity: its fixtures were captured from
behaviour already implemented.

The present implementation has a strong WHITESPACE_ONLY formatter, a real but
partial SIMPLE typed pipeline, a scaffolded ADVANCED level, and a large curated
golden corpus. It is **not yet a drop-in Closure replacement**: successful flag parsing often
does not imply semantics, unsupported typed syntax now fails closed rather than
silently weakening output, BUNDLE and TRANSPILE_ONLY are identity operations,
emitted source maps are placeholders, the type checker is a passthrough
scaffold, and collapse-properties does not mutate the program.

## Evidence captured by the 2026-09-19 audit

### Local implementation

- `closurec` is version `0.246.0` and declares 112 flags in `cli.spec.json`.
  Its generated audit classifies all 102 pinned upstream canonical options,
  all seven exact aliases as supported, eleven local extensions, and one
  deprecated local alias. No upstream alias remains unsupported.
- All 462 curated differential fixtures match both their checked-in expected
  bytes and the pinned Closure `v20260915` oracle refresh recorded by CCR-004.
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

**Dependency constraint (owner direction, 2026-09-21):** the Closure stack
takes **no new crates.io dependency** without an explicitly recorded decision.
It currently declares two — `serde` and `serde_json` — and reaches `regex`
transitively through `grammar-tools`. Both are tracked for removal (CCR-050,
CCR-051); zero is reachable. An external AST crate was considered and rejected
([#15840](https://github.com/adhithyan15/coding-adventures/issues/15840)): a
dependency in this position is a standing maintenance and security liability,
and ESTree is a specification rather than a library, so the shape costs nothing
to adopt ourselves.

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

Adjacent shared-library defects that do not affect `closurec` are tracked in
their owning queue rather than assigned misleading CCR IDs. The current such
finding is cli-builder local/global same-ID shadowing
[#15605](https://github.com/adhithyan15/coding-adventures/issues/15605).

## Ordered queue

### P0 — restore a trustworthy baseline

| Rank | ID | Work item | Acceptance evidence | Status |
|---:|---|---|---|---|
| 1 | CCR-001 | Make absolute and relative `--js` globs use native Windows separators and roots without regressing POSIX behavior. | The six Windows failures pass; focused mixed-separator, exclusion, `*`, and `**` tests pass; full `closurec` suite is green on Windows. | Complete — [#15529](https://github.com/adhithyan15/coding-adventures/pull/15529) |
| 2 | CCR-002 | Replace silent SIMPLE/ADVANCED typed-pipeline fallback with an explicit compatibility policy. Unsupported syntax must either be a hard error or an opt-in, diagnostic-bearing fallback. | Differential tests prove exit code, stderr, and output for parse, bridge, pass, and emit failures. | Complete — [#15544](https://github.com/adhithyan15/coding-adventures/pull/15544) |
| 3 | CCR-003 | Establish one reproducible oracle manifest pinned to upstream `v20260915` and commit `10ca677a`. Record Java version, commands, flags, hashes, licensing, and fixture provenance. | Offline manifest verifier classifies all 626 differential fixture directories exactly once; every upstream-derived golden resolves to one immutable artifact pin and normalized command. | Complete — [#15558](https://github.com/adhithyan15/coding-adventures/pull/15558) |
| 4 | CCR-004 | Re-run all 462 golden fixtures against the new oracle, classify drift, and update only reviewed deltas. | Machine-readable report records equal/changed/declined counts and every changed byte has a linked reason. | Complete — [#15583](https://github.com/adhithyan15/coding-adventures/pull/15583) |
| 5 | CCR-005 | Generate a CLI surface audit from current `CommandLineRunner.java` and `cli.spec.json`, separating upstream, generated, extension, deprecated-alias, and unsupported flags. | CI fails on unclassified flag drift; canonical `--typed_ast_output_file` is accepted with tested compatibility for the legacy typo. | Complete — [#15595](https://github.com/adhithyan15/coding-adventures/pull/15595) |
| 5.1 | CCR-043 | Add first-class long-form flag aliases to `cli-builder` and accept upstream `--D`, `--checks-only`, `--dev_mode`, and `--warnings_whitelist_file` as their canonical flags. | Alias collisions fail spec validation; parse results and explicit-flag tracking use canonical IDs; the CLI audit reports zero unsupported upstream aliases. | Complete — [#15608](https://github.com/adhithyan15/coding-adventures/pull/15608) |
| 5.2 | CCR-041 | Derive correlation-vector pass inventory from the scheduler's actual execution order instead of parallel constants. SIMPLE reported `inline` even though that pass is only registered for ADVANCED. | SIMPLE and ADVANCED provenance tests prove that every reported pass actually ran and that conditional passes appear only when scheduled. | Complete — [#15836](https://github.com/adhithyan15/coding-adventures/pull/15836) |
| 5.3 | CCR-044 | Make `closure-pass-pipeline`'s Kahn scheduler honour registration order as the global tie-breaker. Its `ready` FIFO schedules every dependency-free pass ahead of every dependent one, so `rename` (registered 8th, no deps) executes 2nd — contradicting its own documented contract. | A pipeline unit test registers independent passes out of order and gets registration order back; the 462 goldens are re-run and any delta reviewed; `cv_executed_passes.rs` order pin is updated. | Ready — [#15829](https://github.com/adhithyan15/coding-adventures/issues/15829) |
| 5.4 | CCR-047 | Build the differential complexity ladder as a committed, ordered fixture tier: simplest JS first, each rung carrying its pinned oracle command and both outputs, ramping deliberately. Work it bottom-up; a rung that disagrees is a recorded expected-divergence with a linked issue. | CI gates the ladder at all three levels; the known-gap ledger is machine-checked rather than prose; `--language_in`/`--language_out` are pinned per rung so a divergence is never ambiguous between a capability gap and a flag mismatch. | **Selected** — [#15837](https://github.com/adhithyan15/coding-adventures/issues/15837) |
| 5.5 | CCR-048 | Adopt the ESTree AST shape and retire the hand-written `GrammarASTNode` → `Program` bridge. Decision recorded in [#15840](https://github.com/adhithyan15/coding-adventures/issues/15840): **base ESTree** is the canonical serialized boundary, our own types, no external AST crate. Babel's dialect is provided as a converter over that boundary rather than as a second canonical format. | The ladder is green at every migration step; `bridge.rs`'s 117 `cv: None` sites are gone; node identity is stamped uniformly by construction; an ESTree↔Babel converter round-trips the supported node set. | Spec first — [#15840](https://github.com/adhithyan15/coding-adventures/issues/15840) |
| 6 | CCR-006 | Add an explicit capability/status command or document generated matrix so users can tell parse-only flags from implemented semantics. | Matrix is generated from the same registry used by dispatch and cannot drift manually. | Ready — unblocked by CCR-043 |

### P1 — make the front end and output contract honest

| Rank | ID | Work item | Acceptance evidence | Status |
|---:|---|---|---|---|
| 7 | CCR-007 | Wire real VLQ source maps through parser spans, transforms, emitter, wrappers, and output paths. | Upstream source-map vectors plus multi-file, Unicode, wrapper, stdin, and transformed-token end-to-end tests match decoded mappings. | Ready |
| 7.1 | CCR-046 | **EPIC** — real end-to-end provenance: trace any output byte back through every pass that touched it, including motion, inlining, renaming, and deletion. The bridge writes `cv: None` in 117 places and `cv: Some` nowhere, so no AST node carries an identity; pass contributions attach to the program root, not to nodes; only `constant-fold` records lineage; there is no edge vocabulary for moved/inlined-into/merged/renamed/deleted. | For a program exercising folding, inlining, renaming, motion, and deletion, the sidecar answers: source span of an output byte, every pass that touched it in real order, every call site an inlined body reached, a renamed binding's original name, and a tombstone for deleted code with the responsible pass. | Ready (slice P1 first) — [#15830](https://github.com/adhithyan15/coding-adventures/issues/15830) |
| 8 | CCR-008 | Define and enforce `language_in`; stop treating accepted syntax as independent of the selected input mode. | Current upstream accept/reject corpus matches diagnostics and exit status for representative ECMAScript modes. | Ready |
| 9 | CCR-009 | Implement `language_out` and TRANSPILE_ONLY as real lowering stages rather than identity. Start with optional chaining/nullish coalescing and class features. | Runtime-equivalent output and upstream differential tests across at least two output modes. | Ready |
| 10 | CCR-010 | Extend binding targets across AST, parser, typed bridge, scope analysis, passes, and emitter for array/object destructuring. | Existing declined conformance cases become value-checked; declaration, assignment, parameter, rest, default, and loop targets pass. | Ready |
| 11 | CCR-011 | Complete async/await typed bridging, including `for await` and async generators. | CLOC12 gap-165 closes and async fixtures stay on the typed pipeline. | Ready |
| 12 | CCR-012 | Replace template-literal mode heuristics with an explicit substitution stack supporting nested templates and complex expressions. | CLOC12 gap-044b closes with nested, tagged, escaped, and multiline tests. | Ready |
| 13 | CCR-013 | Survey current parser/bridge/emitter syntax against upstream language-feature enums and turn every decline into a stable capability row. | Generated syntax matrix names pass/fail stage and test for every feature. | Survey |
| 14 | CCR-014 | Integrate the type checker's first real vertical slice: primitive literals, variables, assignments, calls, returns, and JSDoc annotations. | Diagnostics have stable groups/locations; upstream TypeCheck tests for the slice pass; `checks_only` emits no JS. | Ready |
| 15 | CCR-015 | Implement diagnostics plumbing: levels, groups, warnings guards, error formatting, counts, JSON/error streams, and exit codes. | `--jscomp_*`, formatting, hide-warnings, summary detail, and output streams match selected CommandLineRunner tests. | Ready |
| 16 | CCR-042 | Match upstream `JSC_INVALID_OCTAL_LITERAL` warning count, locations, formatting, and successful exit behavior for the three legacy-octal minify fixtures. | The pinned CCR-004 stderr captures match end to end while stdout remains byte-identical. | Blocked by CCR-015 — [#15571](https://github.com/adhithyan15/coding-adventures/issues/15571) |
| 17 | CCR-016 | Replace minimal numeric printing with a shortest-round-trip algorithm and audit special numeric property keys. | Exhaustive boundary/property tests plus upstream CodePrinter numeric vectors close residual CLOC12 numeric gaps. | Ready |

### P2 — strengthen SIMPLE and ADVANCED optimization semantics

| Rank | ID | Work item | Acceptance evidence | Status |
|---:|---|---|---|---|
| 19 | CCR-017 | Implement collapse-properties mutation with namespace safety, alias, extern, getter/setter, and dynamic-access guards. | Ported `CollapsePropertiesTest` slices pass and fixed-point termination is proven. | Ready |
| 19.5 | CCR-049 | Implement object/array literal value propagation into use sites at ADVANCED — the single largest contributor to the 13/52 ADVANCED ladder result. Upstream reduces `var o={a:1,b:2};console.log(o.a)` to `console.log(1)`; we emit the input. | The ladder's `object`, `nested-obj`, `array`, `function-expr`, `iife`, and `optional-chain` rungs agree at ADVANCED. | Ready — [#15837](https://github.com/adhithyan15/coding-adventures/issues/15837) |
| 20 | CCR-018 | Audit remove-unused-vars for nested scopes, exports, destructuring, side effects, classes, and module bindings; implement missing cases. | Ported `RemoveUnusedVarsTest` slices and runtime-effect tests pass. | Ready |
| 21 | CCR-019 | Expand constant folding/peephole coverage and refresh its upstream pin. | All selected current `Peephole*` vectors are ported, passing, ignored with gap IDs, or skipped with reasons. | Survey |
| 22 | CCR-020 | Expand control-flow folding and DCE for switch/try/finally/labels/loops/throw and unreachable lexical declarations. | Ported peephole/remove-dead tests plus runtime-equivalence tests pass. | Ready |
| 23 | CCR-021 | Expand function/variable inlining with escape, recursion, evaluation-order, `this`, `arguments`, async/generator, and multi-statement safety. | Current upstream Inline test inventory is classified and selected cases pass. | Survey |
| 24 | CCR-022 | Complete variable/global/property/label renaming semantics, maps, stable names, extern protection, reserved names, and shadowing. | Rename test families pass; input/output map round trips are deterministic. | Ready |
| 25 | CCR-023 | Add missing ADVANCED passes in dependency order: normalization, call optimization, constructor optimization, dead assignments, devirtualization, property disambiguation/ambiguation, and final denormalization. | Each pass lands with a ported upstream slice and pipeline-order invariant. | Survey |
| 26 | CCR-024 | Make pass configuration reflect upstream compilation levels and relevant option interactions instead of a fixed local list. | Generated schedule snapshots and end-to-end flag interaction tests match the pinned upstream configuration. | Blocked by CCR-015 and CCR-023 |

### P3 — real build-graph and Closure workflows

| Rank | ID | Work item | Acceptance evidence | Status |
|---:|---|---|---|---|
| 27 | CCR-025 | Implement dependency modes, entry points, `goog.provide`/`goog.require`, CommonJS processing, and ES module resolution/rewriting. | Multi-file graph fixtures match order, pruning, diagnostics, and output. | Ready |
| 28 | CCR-026 | Implement chunks/modules, chunk wrappers, chunk maps, output prefixes, weak chunks, and cross-chunk motion. | DAG validation and selected upstream Chunk/Module tests pass on all hosts. | Blocked by CCR-025 |
| 29 | CCR-027 | Make BUNDLE a real bundling mode with module resolution, ordering, wrappers, source maps, and diagnostics. | A representative multi-module application bundles and runs equivalently to upstream. | Blocked by CCR-025 and CCR-007 |
| 30 | CCR-028 | Complete extern ingestion and built-in environment selection, including browser/custom extern interactions. | Extern diagnostics and property/variable protection match upstream cases. | Ready |
| 31 | CCR-029 | Implement polyfill isolation/injection/rewrite and runtime-library selection. | Output and runtime behavior match upstream across language-out modes. | Blocked by CCR-009 |
| 32 | CCR-030 | Implement manifests, dependency graphs, variable/property maps, renaming reports, exports, and name-reference reports. | Every report has deterministic golden and round-trip tests. | Blocked by CCR-022 and CCR-025 |
| 33 | CCR-031 | Implement JSON input/output streams and stdin/stdout composition without mixing diagnostics into data streams. | Pipeline and malformed-stream tests match upstream exit/status behavior. | Blocked by CCR-015 |

### P4 — checks, policy, specialized passes, and operational proof

| Rank | ID | Work item | Acceptance evidence | Status |
|---:|---|---|---|---|
| 34 | CCR-032 | Implement the JS conformance framework: config parsing, allow/deny lists, requirement matching, diagnostics, and reporting. | Selected upstream `Conformance*Test` cases pass and local five declines are reviewed. | Blocked by CCR-014 and CCR-015 |
| 35 | CCR-033 | Add remaining checks in measured slices: variables, modules, JSDoc, strict mode, control flow, access controls, suspicious code, and side effects. | Each check has a diagnostic group and current upstream test slice. | Survey |
| 36 | CCR-034 | Implement specialized passes only after core workflows: Angular/Polymer/J2CL, Closure primitives, CSS/ID replacement, define/tweak processing, message replacement, and translation bundles. | Each accepted flag gains end-to-end semantics or an explicit unsupported error. | Blocked by CCR-024 |
| 37 | CCR-035 | Implement instrumentation and coverage-array modes with source-map and optimization interaction tests. | Instrumented output executes and reports the same covered regions as upstream fixtures. | Blocked by CCR-007 and CCR-024 |
| 38 | CCR-036 | Add typed-AST serialization/deserialization using the canonical flag and versioned format; decide compatibility boundary with upstream protobuf. | Round trip preserves the supported AST and rejects incompatible versions safely. | Blocked by CCR-013 and CCR-014 |
| 39 | CCR-037 | Build a continuously generated upstream test-coverage ledger for all 395 audited test files. | Every file is ported, partially ported, blocked by a gap, or skipped with a reviewed reason and upstream hash. | Ready |
| 40 | CCR-038 | Add deterministic fuzz/property testing for lexer/parser/emitter round trips, pass idempotence, glob/resource bounds, and source maps. | Fixed seeds reproduce failures; corpora run on all supported hosts under explicit budgets. | Blocked by CCR-007 and CCR-013 |
| 41 | CCR-039 | Measure performance and memory against upstream on small, medium, and large real projects; optimize only profiled bottlenecks. | Published reproducible benchmark, peak-memory, and output-size report with regression budgets. | Blocked by CCR-024 and CCR-027 |
| 42 | CCR-040 | Run a real-world migration trial and publish the remaining incompatibility ledger. | Build, runtime, diagnostics, artifacts, maps, and performance are compared against the same upstream pin on all three hosts. | Blocked by core P0-P3 work |
| 43 | CCR-045 | TypeScript/JS authoring API so users can write passes (and checks, conformance rules, CV reporters) without building Rust. `PassRegistry` and the narrow `Pass` trait already provide the indirection; the work is a host boundary plus a typed SDK. | A TS-authored pass schedules via `depends_on`, produces byte-identical output across repeated runs, emits CV contributions indistinguishable from a Rust pass's, and fails closed on throw/hang/denied capability. | Spec first (S1) — [#15831](https://github.com/adhithyan15/coding-adventures/issues/15831) |

### P5 — dependency elimination (CI-wait filler; never ahead of Closure delivery)

Per owner direction these run **only while blocked on CI or review**. They are
self-contained, mechanical, and easy to stop and resume, which is what makes
them suitable. None of them may displace a P0-P3 item.

| Rank | ID | Work item | Acceptance evidence | Status |
|---:|---|---|---|---|
| 44 | CCR-050 | Remove `serde`/`serde_json` from the Closure stack in favour of the in-repo `json-value` / `json-parser` / `json-serializer`, which are already linked into `closurec` via `correlation-vector`. 120 derive sites are the substance; recommend generating the impls rather than hand-writing them or re-importing a proc-macro chain. | Every checked-in JSON artifact is byte-identical, including the CLI-surface audit hash and the oracle report hash; the ladder is unchanged at all three levels; a CI check prevents reintroduction. | Ready — [#15843](https://github.com/adhithyan15/coding-adventures/issues/15843) |
| 45 | CCR-051 | Adopt the in-repo `regex-engine` (Pike VM, zero deps, O(pattern × input)) in place of crates.io `regex`. For `closurec` this is one crate — `grammar-tools` — with three call sites. The validator at `token_grammar.rs:1130` and the matcher in `lexer::grammar_lexer` must move together or a pattern could validate under one engine and match under another. | No Closure-stack crate reaches crates.io `regex`; a grammar sweep proves every checked-in token pattern compiles under both engines; the differential dev-dependency test is extended with each migrated pattern. | Survey first — [#15845](https://github.com/adhithyan15/coding-adventures/issues/15845) |
| 46 | CCR-052 | Expose `regex-engine` as a native extension for Python, JS/TS, Ruby, and Java. **This, not CCR-051, is where the ReDoS exposure actually is**: those engines backtrack (215 Python and 84 JS/TS files use regex here), whereas Rust's `regex` crate is already linear-time, so CodeQL ReDoS alerts against Rust usage are false positives. | A documented, tested binding for at least one language, with a benchmark showing linear behaviour on a pattern that is catastrophic under that language's native engine. | Blocked by CCR-051 survey and a binding-mechanism decision — [#15845](https://github.com/adhithyan15/coding-adventures/issues/15845) |
| 47 | CCR-053 | Make the CLI surface audit hash a canonical projection of `cli.spec.json`'s flag surface rather than the whole file, so a version bump no longer requires the pinned upstream `CommandLineRunner.java`. | Bumping only `version` leaves `cli_surface` green with no regeneration; any real flag change still fails until regenerated; both directions are tested. | Ready — [#15832](https://github.com/adhithyan15/coding-adventures/issues/15832) |

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
| 2026-09-19 | CCR-004's real pinned-oracle run found all 462 expected stdout files byte-identical and zero upstream declines. It also preserved successful upstream warning stderr for three legacy-octal fixtures; direct local execution emits no warning, so the diagnostic gap is logged as CCR-042 ([#15571](https://github.com/adhithyan15/coding-adventures/issues/15571)). | Keep CCR-004 selected through validation and merge because its baseline remains the active dependency. Rank CCR-042 immediately after the general diagnostics plumbing it depends on; it does not outrank the in-progress oracle measurement. |
| 2026-09-19 | CCR-004 implementation and adversarial review completed. Independent real runs over all 462 fixtures reproduced report SHA-256 `e541f87f1abe4a819500e21f852e0fbe6f6a7a1d1aaf94ca265577cf75ac1ecf`; the wrong-JAR path exited 1 without touching it. The generator now hard-pins trust/argv, preflights the whole cohort, executes private immutable snapshots with sanitized JVM environment, enforces streamed and aggregate resource limits, rejects abnormal termination, rechecks Java identity, and atomically replaces only a non-symlink report. The offline verifier applies the same evidence bounds. A final independent fail-closed review reported no actionable findings; 23 focused oracle tests and the full package suite pass. | Keep CCR-004 selected through publication and merge. No new finding outranks it; CCR-042 remains dependency-ordered after CCR-015. |
| 2026-09-19 | CCR-004 merged as [#15583](https://github.com/adhithyan15/coding-adventures/pull/15583) at `fa120baa` after every attached check passed on macOS, Ubuntu, and Windows. The merge is reachable from `origin/main`; no competing Closure PR exists; upstream `master` remains `10ca677a` and the newest dated release tag remains `v20260915`. No new security, data-loss, cross-platform, or false-success defect outranks the next dependency-ordered audit. | Selected CCR-005 ([#15590](https://github.com/adhithyan15/coding-adventures/issues/15590)). It is the smallest ready slice that turns the claimed Closure CLI surface into machine-checked evidence and unblocks the generated capability matrix in CCR-006. |
| 2026-09-19 | CCR-005's generated surface inventory found seven exact upstream alias spellings. The local CLI supports `-D`, `-O`, and `-W`, but `cli-builder` cannot model the four upstream long aliases `--D`, `--checks-only`, `--dev_mode`, and `--warnings_whitelist_file`; logged as CCR-043 ([#15592](https://github.com/adhithyan15/coding-adventures/issues/15592)). | Keep CCR-005 selected because the audit must land before its findings can be removed from the unsupported ledger. Rank CCR-043 immediately after CCR-005: it closes measured surface-parity gaps and gives CCR-006 a complete alias-aware registry. |
| 2026-09-19 | CCR-005 implementation validated locally. The deterministic artifact pins upstream `v20260915` / `10ca677a`, classifies all 102 canonical options and seven exact aliases, records 100 direct upstream flags, two generated flags, 11 extensions, one deprecated alias, zero unsupported canonical flags, and four unsupported upstream aliases. Regeneration matched SHA-256 `be9a6c53c184d955b0e69edd530655075879130d238f7b810f5045a0d503a615`; ten audit tests, typed-AST compatibility tests, all-target Clippy, and the full 691-unit-test package suite plus every integration target pass. | Keep CCR-005 selected through publication and merge. CCR-043 remains the leading follow-up because it removes every measured alias gap and unlocks an alias-aware CCR-006 capability registry, subject to a fresh priority audit after merge. |
| 2026-09-19 | CCR-005 merged as [#15595](https://github.com/adhithyan15/coding-adventures/pull/15595) at `3cf21897` after every attached check passed or was intentionally skipped; Ubuntu, Windows, macOS, metadata, CodeQL detection, and aggregate gates were green. The merge is reachable from `origin/main`; no competing Closure/CLOC or `cli-builder` PR exists. Fresh upstream verification found `master` still at `10ca677a` and the newest dated release tag still `v20260915`. | Selected CCR-043 ([#15592](https://github.com/adhithyan15/coding-adventures/issues/15592)). It is the smallest measured surface-parity slice, removes every unsupported upstream alias from the audit, and gives CCR-006 one canonical alias-aware registry. No newly discovered security, data-loss, cross-platform, or false-success defect outranks it. |
| 2026-09-19 | CCR-043 implementation validated locally. `cli-builder` 1.2.0 now resolves declared long aliases to canonical IDs, rejects effective-scope and built-in collisions, exposes aliases in deterministic help, and passes 246 unit tests, 178 integration tests, nine doctests, and warnings-denied Clippy. `closurec` 0.246.0 accepts all four missing upstream spellings; its full 692-unit-test suite and every integration target pass, as does all-target Clippy apart from the repository's pre-existing obsolete-lint warning. Regenerating the pinned CLI audit twice produced SHA-256 `51cbafca461c5dcb7362b94fcc42c9a8a36b44a60635406d72218123ba20cd37` and zero unsupported upstream aliases. Review also exposed unrelated cli-builder same-ID shadowing drift, logged as [#15605](https://github.com/adhithyan15/coding-adventures/issues/15605). | Keep CCR-043 selected through publication and merge. The new shared-library defect is lower priority and does not affect closurec because it has no subcommands; CCR-006 remains the next dependency-ordered candidate, subject to a fresh audit after merge. |
| 2026-09-19 | The 350-package CI fan-out for CCR-043 exposed a deterministic Windows-only failure in an unrelated `mosaic-package-artifact-builder` stamp test: two immediate same-length writes can retain the same reported mtime. The exact failure reproduced locally and is logged as [#15621](https://github.com/adhithyan15/coding-adventures/issues/15621); the production write-record fallback already covers same-stamp rewrites. | Keep CCR-043 selected. Repair only the test's uncontrolled timestamp fixture as a CI prerequisite; this does not change Closure priority or production semantics. CCR-006 remains next after CCR-043 merges. |
| 2026-09-21 | CCR-043 merged as [#15608](https://github.com/adhithyan15/coding-adventures/pull/15608) at `d2610543` after all 61 attached checks passed or were intentionally skipped on Ubuntu, macOS, and Windows. It had been rebased five times against a fast-moving `main` before merging. CCR-006 is unblocked. | Selected CCR-041 ([#15542](https://github.com/adhithyan15/coding-adventures/issues/15542)) and promoted it from rank 18 to 5.2, ahead of CCR-006. Under policy rule 2 an emitted provenance record naming a pass that never executed is a false artifact, not merely a missing capability, and it corrupts the measurement rule 3 protects. CCR-006 is a gap in what we tell users; CCR-041 is an untruth in what we already tell them, in the feature that differentiates this compiler. |
| 2026-09-21 | CCR-041 implemented. `run_typed_pipeline` now returns `TypedPipelineRun { code, executed_passes }`, sourcing the inventory from `PipelineOutput::execution_order`; both parallel constants and the `will_rename_properties` mirror of the gating decision are deleted. Verified end to end: SIMPLE no longer reports `inline`, ADVANCED still does. Two further findings fell out. First, the constants were concealing a **second** lie: the scheduler does not execute in registration order at all, because `topo_sort`'s `ready` FIFO schedules every dependency-free pass ahead of every dependent one, so `rename` (registered 8th) runs 2nd — logged as CCR-044 ([#15829](https://github.com/adhithyan15/coding-adventures/issues/15829)). Second, removing the constants exposed that the doc comment for `transform_source_with_cv` had been orphaned onto `SIMPLE_PASS_NAMES`, leaving the public function undocumented; re-homed in the same change. | Keep CCR-041 selected through publication and merge. Rank CCR-044 at 5.3, immediately after it: it is a live ordering defect in a shared crate that the golden corpus currently masks only because the pipeline sweeps to a fixed point, and CCR-041's order pin is the test that will prove the fix. It does not outrank CCR-041, which is already implemented. |
| 2026-09-21 | User direction clarified the provenance goal: not a run-level list of pass names, but the ability to take any piece of code and see every pass that affected it and where it moved or inlined to. Audit against that bar found the blocking link — `javascript-parser/src/bridge.rs` writes `cv: None` 117 times and `cv: Some` zero times, so no AST node carries an identity to attach lineage to; pass contributions attach to the program root; only `constant-fold` records lineage; and there is no edge vocabulary for moved/inlined-into/merged/renamed/deleted. Logged as epic CCR-046 ([#15830](https://github.com/adhithyan15/coding-adventures/issues/15830)). User also requested a TypeScript/JS pass-authoring API, logged as CCR-045 ([#15831](https://github.com/adhithyan15/coding-adventures/issues/15831)). | Ranked CCR-046 at 7.1, beside CCR-007, because both need one shared span representation and building them apart would mean building it twice. Ranked CCR-045 at 43: it is capability expansion rather than parity or correctness, but its spec slice is cheap and should be designed jointly with CCR-046, since the AST transfer format and a JS pass's provenance obligations constrain each other. |
| 2026-09-21 | CCR-041 merged as [#15836](https://github.com/adhithyan15/coding-adventures/pull/15836) at `bf6db206` after all checks passed. Security review took two rounds: a MEDIUM (predictable temp-dir path in the new test, regressing the convention already documented in `tests/conformance.rs`) and a LOW (`unwrap_or_default()` on the provenance field would emit an exit-0 sidecar claiming zero passes ran) were both fixed before merge. One CI failure en route, self-inflicted: the new lesson shard used `##` sub-headings, which the render demotes to `###` and counts as lesson titles. | Selected CCR-047, the differential complexity ladder ([#15837](https://github.com/adhithyan15/coding-adventures/issues/15837)). It outranks every remaining semantic item because it is the instrument by which they are judged: the 462-fixture corpus reports 100% agreement while a 52-rung ladder finds 21 disagreements at SIMPLE and 39 at ADVANCED. Until the ladder exists, no other item's completion can be trusted. |
| 2026-09-21 | Real differential runs became possible: the pinned oracle JAR downloads from Maven Central and verifies against the manifest's SHA-256 and byte length exactly, and Java 21 is present. Measured agreement: WHITESPACE_ONLY 49/52, SIMPLE 31/52, ADVANCED 13/52. Six hard failures at SIMPLE where upstream compiles and we exit 1 — two of them (a class with a method after its constructor, an object getter/setter) were untracked by any CCR item. Logged the ADVANCED literal-propagation gap as CCR-049. | Recorded the measured gradient in the truthfulness section, replacing the prose estimate. Ranked CCR-049 at 19.5 as the largest single ADVANCED contributor. Deliberately did **not** raise CCR-010 (destructuring) or CCR-011 (async): both die in `bridge.rs`, which the ESTree migration eliminates, so implementing them now means doing the work twice. |
| 2026-09-21 | Owner set direction on architecture: adopt ESTree — **base ESTree as the canonical serialized boundary**, with Babel's dialect supplied as a converter over it rather than as a second canonical format — our own types, no external AST crate; match Closure on **observable output**, not on internal pass structure or ordering; eliminate `serde` and adopt the in-repo regex engine. Audit found `javascript-ast` has already independently made the Babel-shaped choices for literals and optional chaining, and that `regex` reaches `closurec` through exactly one crate with three call sites. | Added CCR-048 (ESTree migration) at 5.5 and a new P5 dependency-elimination track at 44-47. The dialect choice settled on base ESTree after an intermediate position favouring Babel was withdrawn: the argument for Babel (Rust cannot express ESTree's polymorphic `Literal.value`, so the split is forced) is a fact about the *internal* representation, not the boundary, which serializes to JSON where a polymorphic value is natural. Since either dialect is one mechanical converter from the other, the canonical format should be the standard rather than a single project's dialect. explicitly ranked below all Closure delivery work and designated CI-wait filler. Downgraded [#15842](https://github.com/adhithyan15/coding-adventures/issues/15842) (normalization) from prerequisite to optional technique: it was argued from Closure's internal construction, which the byte-identical-output bar does not oblige us to reproduce. |

## Parallelization

The loop is serialized on CI, not on thinking, so a second item can proceed
while a PR is in flight. Two constraints bound what may run concurrently:

1. **One active shared-crate PR.** Prioritization rule 6. `cli-builder`,
   `closure-pass-pipeline`, `javascript-ast`, and `javascript-parser` are on
   every pass's critical path; two concurrent PRs touching one of them will
   conflict semantically even when git merges them cleanly.
2. **This file is a conflict hotspot.** Every item updates the queue and the
   loop record, so two concurrent Closure PRs both edit
   `CLOSURE-COMPILER-RUST-BACKLOG.md` and collide — the same additive-merge
   failure that forced `lessons.md` to shard into `lessons.d/`. Until it is
   sharded, keep the durable state in the linked GitHub issues, which are the
   source other agents should resume from, and treat the table row as a
   pointer rather than the record.

Currently safe to run alongside an in-flight `closurec` PR, because they touch
disjoint trees:

| Item | Touches | Conflicts with |
|---|---|---|
| CCR-053 (audit hash projection) | `tests/cli_surface_support/` | a `cli.spec.json` change |
| CCR-050 step 1 (JSON byte-compat harness) | new test files only | nothing in flight |
| CCR-051 survey (grammar pattern sweep) | read-only measurement | nothing |
| CCR-048 spec (ESTree dialect) | `code/specs/` only | nothing in flight |
| CCR-037 (upstream coverage ledger) | new files under `tests/` | nothing in flight |

Not safe to run alongside a `closurec` PR: CCR-006 (same CLI registry and
`run.rs`), CCR-044 (shared `closure-pass-pipeline`, and CCR-041's order pin
moves with it), and any two items that both edit this file — which remains the
hotspot described above until it is sharded.

