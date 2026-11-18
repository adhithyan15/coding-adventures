# Game Platform Backlog

This file tracks delivery order across the historical game roadmap. `GAME00` remains the era and
system catalog; this backlog records what is implemented, where it runs, and what evidence exists.
Game work should grow reusable simulation and device primitives that support multiple titles.

## How to read status

Capability, language support, and verification are separate dimensions. A package directory or a
roadmap tier is not proof of a complete capability or cross-language parity. “Present” means a
source package exists; “incomplete” means the stated contract or host path has known gaps; “planned”
means no implementation has been verified here. Language lanes must be audited against the
repository's supported build lanes before claiming parity. Verification names identify evidence
that must be produced, not a promise that the evidence exists already.

| Workstream | Capability status | Language status | Verification status / limit |
| --- | --- | --- | --- |
| `GAME10` fixed-step clock | Rust core and v1 integer contract present; host pacing/render integration is separate and incomplete | Rust implementation; Python is an independent fixture validator, not a port; TS, Go, and other supported lanes remain | Durable consumer merged in [PR #19602](https://github.com/adhithyan15/coding-adventures/pull/19602), squash `72778f4f553cf35fd361e2688acaeb6201414ea1`; 23 vectors (9 success, 14 exact errors) pass package conformance. The constructor-only Warrant model landed in merged [PR #19618](https://github.com/adhithyan15/coding-adventures/pull/19618), squash `a2163fc64c74c10f3bb0e62cd268363cbb560d21`; local finite execution covers 648 boundary cases and 1,024 stratified full-width inputs, while aggregate CI `BUILT` receipts do not establish Warrant execution. `validate_state`/`advance_frame`, universal proof, and native source proof remain unproved; no assurance proof slot is upgraded. |
| `GAME11`–`GAME15c` early arcade | Rust packages exist for collision, arcade SFX, Pong, Breakout, discrete logic, CRT beam, and timer | Rust source present; other supported language lanes need inventory | No unified portable-evidence audit yet; collision, Pong, Breakout, and CRT Warrant drafts are parked. |
| `GAME20`–`GAME25` tile/sprite/RPG | Rust packages exist for indexed palette, tilemap, sprite OAM, RPG grid/battle, and Monster RPG | Rust source present; verify required lanes rather than inferring from GAME00's portable label | Shared independent corpus and cross-language execution audit remain open. |
| `GAME26`–`GAME29b` console devices | GAME26 GBA bridge, GAME27 NES, GAME28 Game Boy, and GAME29 GBA devices are missing from the Rust package tree | No verified portable console-device lane | Start with a narrow independently specified NES register/timing slice. Do not claim a complete console. |
| `SWF01`–`SWF05` Flash foundations | Rust packages exist for SWF container, shape lowering, ActionScript compiler, AVM lifter, and display list | Rust source present; LangVM/host integration is incomplete | Existing packages lack an audited shared corpus across public APIs. SWF06 audio bridge, SWF07 player host, and SWF08 conformance corpus are missing. |
| `GAME30`–`GAME35` 2.5D/3D | No Rust packages found for math3d, raycast, BSP, rasterizer, dungeon, or Quake bridge | No verified lane matrix | Capability is missing; fixed-point/F64 contracts and independent rendering/geometry oracles must precede claims. |
| `GAME40`–`GAME45` flight | No Rust packages found for rigidbody, aero, terrain, HUD, GPU bridge, or flight sim | No verified lane matrix | Capability is missing; host/device integration and deterministic physics evidence remain unbuilt. |
| Networking and tooling | Not decomposed into owned game contracts | No verified lane matrix | Define explicit protocol and tool inputs/outputs only after deterministic simulation foundations. |
| `GAME01` Hamurabi | Deterministic simulation core present; console and host integration are separate concerns | Canonical report snapshot at base `7bec66bbd9059ab11be24920bf876162df8da302`: present in all 15 established lanes and all 3 emerging lanes; package presence is not parity | Rust's canonical 31-case public-API consumer merged in [PR #19609](https://github.com/adhithyan15/coding-adventures/pull/19609), squash `794c5795da8c76072f988cce02a267b1a314b4a7`. Local debug/release and strict Clippy pass; capability/certificate checks and Go fixture selection pass. macOS reports Linux-only runtime gate NOT-LIVE. Exact-head CI's Ubuntu/macOS build-table recipes selected affected-package jobs and reported BUILT; aggregate logs suppress command counts/runtime verdicts. The Windows affected-package test step was SKIPPED, so native Windows execution is not claimed. The 2026-11-17 static 26-row audit remains historical, not live parity: Go ran the corpus; Python/TypeScript routes were sampled; OCaml embeds literals despite its README; five lanes read a text projection without freshness checking. Rust assurance remains report-only with empty evidence and null proof/fuzz entries; `GBL-009` tracks substantiation. |
| `GAME02` Lunar Lander | Fixed-point contract and several cores present; host/presentation integration is incomplete | Rust, Python, TypeScript, Go, Ruby, Perl, Lua, Elixir packages exist; audit them against supported lanes | Verify each language against shared vectors; do not infer parity from directory presence. Expand generated/property coverage after corpus consumers. |
| Parked F64 specifications | Five game Warrant drafts depend on float support | Warrant does not yet implement F64 | WX-FL1 in `VER-BACKLOG.md` gates restoration; no proof/evidence claim until then. |

## Dated package and corpus audit (2026-11-17)

This is a static source and BUILD-entry-point audit of 26 package/language lanes: 18 Hamurabi
lanes and 8 Lunar Lander lanes. The initial inspection read fetched `origin/main`
`ce2b783909ab99cdd185f6ab8cfc9c63cac69058`; the package source and BUILD paths in scope were
then diff-checked through the requested PR base `77e830cac55e57b588a66efffa6c437aba2ccc25`
and were unchanged. The canonical package report was run from a full checkout. This records
what checked-in conformance tests appear to read and whether normal `BUILD` points at those
tests. No package tests were rerun. **No cross-language parity is verified by this inventory.**
A test that names a corpus, a package directory, or a BUILD command is not a live pass receipt.
The Hamurabi JSON corpus has 31 cases: 17 trajectories with 53 turns/reports and 14 refusals;
the Lunar Lander JSON corpus has 39 cases.

| Game / lane | Fixture path actually read (static source evidence) | Normal BUILD selects test? | Copy, freshness, or transport note |
| --- | --- | --- | --- |
| GAME01 C / `hamurabi` | `code/specs/fixtures/hamurabi-v1/cases.txt`; `code/packages/c/hamurabi/tests/conformance_test.c:15-17,148` | Yes, through `code/packages/c/hamurabi/BUILD` -> `tools/run.sh:31-32` | Derived text fixture; freshness not checked. |
| GAME01 C++ / `hamurabi` | `code/specs/fixtures/hamurabi-v1/cases.txt`; `code/packages/cpp/hamurabi/tests/conformance_test.cpp:14-16,117` | Yes, through `code/packages/cpp/hamurabi/BUILD` -> `tools/run.sh:33-34` | Derived text fixture; freshness not checked. |
| GAME01 Haskell / `hamurabi` | `code/specs/fixtures/hamurabi-v1/cases.txt`; `code/packages/haskell/hamurabi/test/ConformanceSpec.hs:167-172` | Yes, `code/packages/haskell/hamurabi/BUILD:3` | Derived text fixture; freshness not checked. |
| GAME01 Java / `hamurabi` | `code/specs/fixtures/hamurabi-v1/cases.txt`; `code/packages/java/hamurabi/src/test/java/com/codingadventures/hamurabi/ConformanceTest.java:15,21-24` | Yes, `code/packages/java/hamurabi/BUILD:1` | Derived text fixture; freshness not checked. |
| GAME01 Kotlin / `hamurabi` | `code/specs/fixtures/hamurabi-v1/cases.txt`; `code/packages/kotlin/hamurabi/src/test/kotlin/com/codingadventures/hamurabi/ConformanceTest.kt:12-15` | Yes, `code/packages/kotlin/hamurabi/BUILD:1` | Derived text fixture; freshness not checked. |
| GAME01 C# / `hamurabi` | `code/specs/fixtures/hamurabi-v1/cases.json`; `code/packages/csharp/hamurabi/tests/CodingAdventures.Hamurabi.Tests/ConformanceTests.cs:10-20` | Yes, `code/packages/csharp/hamurabi/BUILD:1` | Reads canonical JSON directly. |
| GAME01 Dart / `hamurabi` | `code/specs/fixtures/hamurabi-v1/cases.json`; `code/packages/dart/hamurabi/test/conformance_test.dart:10-18` | Yes, `code/packages/dart/hamurabi/BUILD:2` | Reads canonical JSON directly. |
| GAME01 Elixir / `hamurabi` | `code/specs/fixtures/hamurabi-v1/cases.json`; `code/packages/elixir/hamurabi/test/conformance_test.exs:7,121-122` | Yes, `code/packages/elixir/hamurabi/BUILD:2-3` | Reads canonical JSON directly. |
| GAME01 F# / `hamurabi` | `code/specs/fixtures/hamurabi-v1/cases.json`; `code/packages/fsharp/hamurabi/tests/CodingAdventures.Hamurabi.Tests/ConformanceTests.fs:16-17,102` | Yes, `code/packages/fsharp/hamurabi/BUILD:1` | Reads canonical JSON directly. |
| GAME01 Go / `hamurabi` | `code/specs/fixtures/hamurabi-v1/cases.json`; `code/packages/go/hamurabi/conformance_test.go:33` | Yes, `code/packages/go/hamurabi/BUILD:2` | Reads canonical JSON directly. |
| GAME01 Lua / `hamurabi` | `code/specs/fixtures/hamurabi-v1/cases.json`; `code/packages/lua/hamurabi/tests/test_conformance.lua:69-72` | Yes, `code/packages/lua/hamurabi/BUILD:1` | Reads canonical JSON directly. |
| GAME01 Perl / `hamurabi` | `code/specs/fixtures/hamurabi-v1/cases.json`; `code/packages/perl/hamurabi/t/02-conformance.t:15-24` | Yes, `code/packages/perl/hamurabi/BUILD:1-2` | Reads canonical JSON directly. |
| GAME01 Python / `hamurabi` | `code/specs/fixtures/hamurabi-v1/cases.json`; `code/packages/python/hamurabi/tests/test_conformance.py:26` | Yes, `code/packages/python/hamurabi/BUILD:3` | Reads canonical JSON directly. Separate randomized invariant test at `code/packages/python/hamurabi/tests/test_hamurabi.py:518-557` uses a fixed seed and 1,000 iterations. |
| GAME01 Ruby / `hamurabi` | `code/specs/fixtures/hamurabi-v1/cases.json`; `code/packages/ruby/hamurabi/test/test_conformance.rb:7,17-19` | Yes, `code/packages/ruby/hamurabi/BUILD:2` | Reads canonical JSON directly. |
| GAME01 Swift / `hamurabi` | `code/specs/fixtures/hamurabi-v1/cases.json`; `code/packages/swift/hamurabi/Tests/HamurabiTests/ConformanceTests.swift:33-69` | Yes, `code/packages/swift/hamurabi/BUILD:1` | Reads canonical JSON directly. |
| GAME01 TypeScript / `hamurabi` | `code/specs/fixtures/hamurabi-v1/cases.json`; `code/packages/typescript/hamurabi/tests/hamurabi.test.ts:41-52,97-106` | Yes, `code/packages/typescript/hamurabi/BUILD:3` | Test-only parser quotes `rng_state`, `rng_inc`, and `seed` before `JSON.parse` to preserve 64-bit values. |
| GAME01 OCaml / `hamurabi` | No shared path read; `code/packages/ocaml/hamurabi/test/test_hamurabi.ml:140-148,575-620` embeds and parses a flat corpus literal | Yes, `code/packages/ocaml/hamurabi/BUILD:3` | Embedded duplicate. README claims shared `cases.txt` input (`code/packages/ocaml/hamurabi/README.md:18`), unlike this test source. |
| GAME01 Rust / `hamurabi` | No shared path read; `code/packages/rust/hamurabi/tests/conformance.rs:223-294,358+` contains local deterministic gameplay examples | Yes, `code/packages/rust/hamurabi/BUILD:1-2` | Registered conformance test is not a canonical corpus consumer. `code/packages/rust/hamurabi/assurance_report.json:29-41` lists 1,000 property cases but empty evidence and null fuzz. |
| GAME02 Elixir / `lunar_lander` | `code/specs/fixtures/lunar-lander-v1/cases.json`; `code/packages/elixir/lunar_lander/test/conformance_test.exs:7,32-33` | Yes, `code/packages/elixir/lunar_lander/BUILD:2-3` | Reads canonical JSON directly. |
| GAME02 Go / `lunar-lander` | `code/specs/fixtures/lunar-lander-v1/cases.json`; `code/packages/go/lunar-lander/conformance_test.go:10` | Yes, `code/packages/go/lunar-lander/BUILD:1` | Reads canonical JSON directly. |
| GAME02 Lua / `lunar-lander` | `code/specs/fixtures/lunar-lander-v1/cases.json`; `code/packages/lua/lunar-lander/tests/test_conformance.lua:54-60` | Yes, `code/packages/lua/lunar-lander/BUILD:1` | Reads canonical JSON directly. |
| GAME02 Perl / `lunar-lander` | `code/specs/fixtures/lunar-lander-v1/cases.json`; `code/packages/perl/lunar-lander/t/02-conformance.t:15-24` | Yes, `code/packages/perl/lunar-lander/BUILD:1-2` | Reads canonical JSON directly. |
| GAME02 Python / `lunar-lander` | `code/specs/fixtures/lunar-lander-v1/cases.json`; `code/packages/python/lunar-lander/tests/test_conformance.py:22-33` | Yes, `code/packages/python/lunar-lander/BUILD:3` | Reads canonical JSON directly. |
| GAME02 Ruby / `lunar-lander` | `code/specs/fixtures/lunar-lander-v1/cases.json`; `code/packages/ruby/lunar-lander/test/test_conformance.rb:7,14-19` | Yes, `code/packages/ruby/lunar-lander/BUILD:2` | Reads canonical JSON directly. |
| GAME02 Rust / `lunar-lander` | `code/specs/fixtures/lunar-lander-v1/cases.json`; `code/packages/rust/lunar-lander/tests/conformance.rs:20` (`include_str!`) | Yes, `code/packages/rust/lunar-lander/BUILD:1-2` | Includes canonical JSON at compile time. |
| GAME02 TypeScript / `lunar-lander` | `code/specs/fixtures/lunar-lander-v1/cases.json`; `code/packages/typescript/lunar-lander/tests/lunar_lander.test.ts:72-87,222+` | Yes, `code/packages/typescript/lunar-lander/BUILD:3` | Reads canonical JSON directly. |

The canonical report has 15 established implementation lanes and 3 emerging lanes (`c`,
`cpp`, `ocaml`); keep those buckets distinct. Use `code/scripts/package_parity_report.py`
and its CSV/JSON output as the one package-presence inventory. Do not create a private
all-repository duplicate language matrix, and do not treat that structural report as
behavioral evidence. At this base, Hamurabi is present in 15/15 established and all 3
emerging lanes. Lunar Lander is present in 8/15 established lanes; the missing established
lanes are C#, Dart, F#, Haskell, Java, Kotlin, and Swift. Its emerging C, C++, and OCaml
lanes are also absent and remain tracked separately. Clock, arcade, and RPG package
coverage is predominantly Rust-only (1/15 established, with emerging lanes separately
tracked); use the canonical report to refresh each inventory before execution.

For a portable game contract, a missing established package/language lane is authorized
port work, not an indefinite audit placeholder. A port includes the relevant public
contract, shared fixture consumer, BUILD and fixture-input wiring, package metadata,
changelog, tests, and generated/property, fuzz, or Warrant evidence where applicable.
Refresh the canonical package report and game-specific evidence when a port lands. Track
emerging lanes separately from the established 15 and do not imply that one bucket's
completion satisfies the other.

## Prioritized delivery

Priority reflects dependencies visible on 2026-11-18: GAME10's exact v1 corpus consumer, the
GAME01 Rust corpus consumer, and the separate Kiln allocation-reuse repair have merged. Reusable
deterministic contracts precede new device, graphics, and host stacks.
This order avoids duplicating repository-wide `VER-*` and `MUS00`–`MUS08` work.

As of 2026-11-18, `GBL-001`, `GBL-002`, `GBL-003a`, and `GBL-061` are merged. `GBL-010` is under
review in open PR #19644; keep it open until its exact-head checks and merge are verified. Before
that PR can clear, `GBL-063` is the blocking Kiln Ubuntu 24.04 G1 performance follow-up: diagnose
the unchanged 14 MB/s gate miss on the exact 66,600-byte / 936-line workload, then make the smallest
reusable hot-path repair supported by native Linux measurements. The current evidence correlates
two misses with an Azure runner image/kernel and one pass with AWS, but the CPU model is absent, so
it does not establish a runner cause. Keep the workload, floor, and failure receipts intact; do not
retry unchanged code or infer a same-host speedup from separate CI jobs. After `GBL-010` lands,
resume `GBL-050` Rust input action frames and `GBL-050a` TypeScript input action frames. A fresh
full-checkout canonical package-presence report must be recorded before the next new language-port
selection; a sparse-worktree report is not valid inventory evidence. This sequence is coordination
state, not implementation authorization for unrelated primitives.

For each serial delivery, use a fresh worktree from the latest fetched `origin/main`. After a PR
merges, verify the published head and merged tree, confirm the worktree is clean, published, and
idle, then remove it with normal Git worktree removal and prune registrations. Only then create a
new worktree for the next selected item. Never force-remove a dirty, unpublished, or active
worktree; report and preserve it if those checks do not pass.

### P0 — `GBL-001`: Make GAME10 v1 a durable conformance boundary (merged; [PR #19602](https://github.com/adhithyan15/coding-adventures/pull/19602), squash `72778f4f553cf35fd361e2688acaeb6201414ea1`)

- [x] Consume all 23 `game-clock-v1` cases from a Rust package test through public APIs. Dispatch all
  five operation tags; compare complete configs, complete states, all step-result fields, and exact
  error variants. Unknown operations and skipped cases fail closed.
- [x] For every rejected frame advance, verify the supplied state remains unchanged. Exercise the
  public mutating wrapper against an untouched clone as well as the immutable API.
- [x] Add negative controls proving the conformance checker rejects an always-`Err` adapter, a
  changed expected success field, and a changed expected error code.
- [x] Register the exact fixture path as an external build input for every required consumer. Test
  exact v1 selection and reject a v10 lookalike. Keep the Rust package's debug/release and capability
  gates intact.
- [x] Classify the result as finite package-corpus conformance only. Do not upgrade Warrant or
  assurance claims without separate evidence.

  **Completion evidence (2026-11-18):** exact-head required CI passed on `77e830cac55e57b588a66efffa6c437aba2ccc25` (Ubuntu, macOS, Windows builds; D18F/P/Q/T; both Kiln sandbox jobs; metadata, CI-gate, and Human Languages Books gates). The squash commit is present on freshly fetched `origin/main`. Rust runtime capability execution was NOT-LIVE on the local macOS host; the Linux build lane is covered by CI. This completion records the clock corpus consumer and selection wiring only, not host pacing/render integration or formal proof.

### P1 — Close existing evidence gaps and Warrant limits

- [x] `GBL-002` (merged in [PR #19609](https://github.com/adhithyan15/coding-adventures/pull/19609), squash `794c5795da8c76072f988cce02a267b1a314b4a7`) Make Rust Hamurabi consume the shared 31-vector corpus; audit every registered language
  consumer before describing parity. Scope: consume canonical `code/specs/fixtures/hamurabi-v1/cases.json`
  and compare all 31 cases, 53 trajectory turns and byte-exact reports (including final newlines),
  and 14 refusal cases across all 8 error variants. Preserve full-width `u64` fixture numbers from
  the JSON parser's original numeric token text; compare all 15 state fields and enum payloads; chain
  the actual returned state into the next turn. Exercise `validate_order` and `step_year` for every
  refusal. Pin the four contractual fresh-start cases against `new_game(seed)`; the fixture's
  initial states remain authoritative for transitions, including two cases intentionally different
  from `new_game(seed)`. Fail on duplicate/missing cases and unknown operation tags. Same-harness
  negative controls must reject a one-bit change to a >2^53 RNG word, a removed report newline, a
  different valid error variant, and an always-error adapter. This proves finite Rust package-corpus
  conformance only; do not label it cross-language parity or formal proof. Warrant evidence and the
  report-only assurance limitations remain open under `GBL-005`/`GBL-009`.

  **Selection rationale (2026-11-18):** the GAME10 consumer is merged, Rust Hamurabi already has a
  registered conformance test but hardcodes its transcript, and the shared 31-vector fixture is an
  existing canonical input. A 2026-11-17 static 26-row audit found 11 JSON consumers, five `cases.txt`
  consumers, one OCaml test embedding literals despite its README, and this Rust hardcoded transcript;
  that inventory is not live parity evidence. The JSON-to-text freshness gap is tracked separately.

  **Implementation validation (2026-11-18, macOS):** the Rust integration test consumes all 31 cases
  and passes same-harness controls for always-error, a +1 mutation to the >2^53 initial RNG word,
  removal of a report's final newline, a substituted valid error, duplicate/missing case IDs,
  duplicate keys including an escaped alias, and unknown/missing operation fields. `BUILD` and
  `BUILD_windows` command sets both pass the Rust debug and release package suites, capability check,
  and effect-certificate verification; strict package Clippy passes. The runtime capability test
  correctly reports Linux-only NOT-LIVE on macOS. A focused Go build-tool test confirms the exact
  fixture path selects all 18 registered Hamurabi lanes and rejects version lookalikes. This is Rust
  package conformance plus build selection only; the static audit remains distinct from live
  cross-language parity, and Hamurabi assurance evidence remains open under `GBL-009`. Exact-head
  CI build-table recipes selected Ubuntu and macOS affected-package jobs and produced BUILT package
  receipts, but aggregate outputs suppress individual command counts and runtime verdicts. The
  Windows affected-package test step was SKIPPED; no native Windows Hamurabi execution is claimed.
- [x] `GBL-003a` (merged in [PR #19618](https://github.com/adhithyan15/coding-adventures/pull/19618), squash `a2163fc64c74c10f3bb0e62cd268363cbb560d21`) Separately execute and assess the GAME10 constructor Warrant model. Local finite execution covered the 648-case boundary (105 `Ok`, 72 `ZeroFixedTimestep`, 162 `InvalidMaxFrameSteps`, 309 `InvalidMaxFrameDelta`) and 1,024 stratified full-width inputs; aggregate CI `BUILT` receipts are not Warrant execution evidence. Constructor-only execution does not prove `validate`, `advance`, or the shared `WX-SP1` proof; those source and universal proof claims remain open under `GBL-003`. The portable corpus/WX-SP1 route also depends on Warrant `ir.data` records and variants (CV2-S11; tracked gap CV2-F10); current fixed built-ins and tuple `Result` support do not encode `GameClockConfig`/`GameClockState` records. Do not call this model execution a Rust-to-Warrant source proof or a full-domain universal proof.
- [x] `GBL-061` (merged in [PR #19630](https://github.com/adhithyan15/coding-adventures/pull/19630), squash `1de3c7f6e9b74765f48cce277187f1920a487a81`) Add caller-owned token-vector reuse to the Go lexer and strace parser after measuring the per-line allocation hotspot. The 14 MB/s Kiln Ubuntu 26.04 floor was unchanged. On exact-head CI run `44247169191`, the 66,600-byte / 936-line synthetic reported 19.9 MB/s on Ubuntu 24.04 and 22.1 MB/s on Ubuntu 26.04 (`gated=1`, seven repeats); the separate 4,195,800-byte census reported 18.8 and 20.3 MB/s (`gated=0`). These are candidate gate receipts, not a native same-host Linux A/B or proof of a Linux speedup. Full affected-package CI built Go lexer, strace lexer/parser, and Kiln; no native Windows execution was selected. Local macOS allocation and timing evidence, sparse full-suite limits, and review scope are recorded in the PR.
- [ ] `GBL-059` Preserve useful per-package CI receipts: commands/recipe selection, stdout summary, seed, source revision, platform, and runtime verdict. Audit existing `VER`/build-tool receipt and KILN facilities before extending them; do not create a competing evidence framework. `GBL-061` exact-head artifacts exposed only build-plan in the downloaded artifact set and aggregate logs suppressed command/test details, so package-level CI claims remain limited to explicit `BUILT` and gate receipts.
- [ ] `GBL-060` Execute portable game-core functional tests natively on Windows. Current affected-package Windows steps can skip Rust packages; compile-only cross checks and Windows job presence are not native game-core execution evidence. Keep `NOT-LIVE` platform containment separate from test coverage.
- [ ] `GBL-003` Repair GAME10's three Warrant claims and assurance report. Add meaningful positive and negative
  evidence for `validate` and `advance`, shared proof through `WXSP1`, generated/property and fuzz
  coverage, and Warrant evidence only through a separately reviewed design. A passing finite corpus
  or constructor-only model does not repair a claim that accepts every error.
- [ ] `GBL-004` Add generated/property tests for arithmetic invariants, validation precedence,
  conservation, and atomic failure. Record generators and bounds.
- [ ] `GBL-007` Add bounded fuzzing where package tooling supports it, and record corpus, runtime,
  and configuration limits.
- [ ] `GBL-009` Substantiate Rust Hamurabi assurance claims: map each checked-in property-count claim
  to a live, reproducible property, fuzz, or Warrant evidence artifact, or correct the report's
  status. The current report has 1,000 property counts per claim but empty evidence and null
  exhaustive/fuzz/proof entries, plus two unapproved exemptions. A finite corpus pass under
  `GBL-002` does not close this item; required per-lane CWE dispositions and Warrant proof targets
  remain open until separately discharged.
- [ ] `GBL-005` Complete live evidence for the 26 package/language rows in the dated audit: reuse and
  record existing fresh receipts, run missing normal `BUILD` checks, and identify any host boundary
  separately. Mark each lane pass, fail, or unverified; do not claim parity until every required lane
  has evidence against its shared canonical fixture.
- [ ] `GBL-006` Repair OCaml Hamurabi fixture wiring: replace the embedded copy with a read of the
  shared Hamurabi fixture and align README/test claims with the actual transport. Keep its ordinary
  BUILD test entry point exercising that consumer.
- [ ] `GBL-008` Generate Hamurabi `cases.txt` from canonical `cases.json` and add a freshness check
  to fixture validation/BUILD. The five current text consumers are C, C++, Haskell, Java, and Kotlin;
  require drift detection before relying on text-derived results. Migrate those consumers to JSON
  only as a separately reviewable follow-up.

### P2 — Port the clock contract

- [ ] `GBL-010` Implement GAME10 public APIs and exact v1 corpus consumption in TypeScript, preserving
  canonical `u64` values as `BigInt` across fixture parsing and public APIs.
- [ ] `GBL-011` Add fixture external-input/build-selection edges and negative v10 selection tests in each
  affected build lane. Preserve canonical decimal `u64`, bounded numeric `u32`, strict booleans, and
  exact error behavior across runtimes.
- [ ] `GBL-013` Implement GAME10 public APIs and exact v1 corpus consumption in Go.
- [ ] `GBL-014` Implement GAME10 public APIs and exact v1 corpus consumption in Python.
- [ ] `GBL-012` Add the remaining established GAME10 package ports with exact shared corpus consumers,
  BUILD fixture edges, package metadata, changelog, tests, and applicable generated, fuzz, or Warrant
  evidence. Implement each lane as a separate subitem:
  - [ ] `GBL-012a` C#.
  - [ ] `GBL-012b` Dart.
  - [ ] `GBL-012c` Elixir.
  - [ ] `GBL-012d` F#.
  - [ ] `GBL-012e` Haskell.
  - [ ] `GBL-012f` Java.
  - [ ] `GBL-012g` Kotlin.
  - [ ] `GBL-012h` Lua.
  - [ ] `GBL-012i` Perl.
  - [ ] `GBL-012j` Ruby.
  - [ ] `GBL-012k` Swift.
- [ ] `GBL-015` Stage emerging GAME10 ports separately from the established 15-lane count; each port needs
  its own contract, corpus consumer, BUILD, metadata, changelog, and tests:
  - [ ] `GBL-015a` C.
  - [ ] `GBL-015b` C++.
  - [ ] `GBL-015c` OCaml.

### P3 — Complete Lunar Lander language coverage

- [ ] `GBL-020` Verify current Rust, Python, TypeScript, Go, Ruby, Perl, Lua, and Elixir packages against the
  normative fixed-point contract and shared vectors.
- [ ] `GBL-021` Add the missing established C# Lunar Lander port, consuming the shared fixed-point
  corpus and satisfying package/build metadata requirements.
- [ ] `GBL-023` Add the missing established F# Lunar Lander port with shared corpus and package/build evidence.
- [ ] `GBL-024` Add the missing established Haskell Lunar Lander port with shared corpus and package/build evidence.
- [ ] `GBL-025` Add the missing established Java Lunar Lander port with shared corpus and package/build evidence.
- [ ] `GBL-026` Add the missing established Kotlin Lunar Lander port with shared corpus and package/build evidence.
- [ ] `GBL-027` Add the missing established Swift Lunar Lander port with shared corpus and package/build evidence.
- [ ] `GBL-028` Stage emerging Lunar Lander ports separately from the established 15-lane count; each port
  needs its own fixed-point contract, shared fixture consumer, BUILD wiring, metadata, changelog, and tests:
  - [ ] `GBL-028a` C.
  - [ ] `GBL-028b` C++.
  - [ ] `GBL-028c` OCaml.
- [ ] `GBL-022` Specify and verify parked F64 contracts separately; do not describe fixed-point coverage as F64
  coverage.
- [ ] `GBL-029` Add the missing established Dart Lunar Lander port, consuming the shared fixed-point corpus
  and satisfying package/build metadata requirements.

### P4 — Build reusable 2D and console foundations; finish Flash in slices

- [ ] `GBL-030` Audit public API and shared-corpus evidence for existing GAME11–GAME25 packages;
  record language support separately from package presence.
- [ ] `GBL-031` Define an independent oracle and portable contract for indexed palette primitives.
- [ ] `GBL-032` Define an independent oracle and portable contract for tilemap primitives.
- [ ] `GBL-038` Define an independent oracle and portable contract for sprite/OAM primitives.
- [ ] `GBL-039` Specify and implement a narrow NES register/timing slice with deterministic state transitions,
  then grow device coverage by explicit module boundaries. Full console/PPU/APU/cartridge claims
  require corresponding independent tests.
- [ ] `GBL-033` Implement the missing SWF06 audio bridge.
- [ ] `GBL-044` Implement the missing SWF07 player host.
- [ ] `GBL-045` Implement the missing SWF08 conformance corpus.
- [ ] `GBL-046` Audit SWF01–SWF05 public APIs and LangVM host integration separately.
- [ ] `GBL-034` Audit the five parked Warrant drafts: `code/specs/collision2d.warrant.draft`,
  `code/specs/game-breakout.warrant.draft`, `code/specs/game-pong.warrant.draft`,
  `code/specs/paint-batch-tiles.warrant.draft`, and
  `code/specs/sim-crt-vector-beam.warrant.draft`. Their F64 dependency waits on `VER04p` floats
  (`WX-FL1`; see `VER-BACKLOG.md`); keep this dependency explicit and do not claim proof meanwhile.
- [ ] `GBL-035` Decompose Flash completion into SWF parsing, VM/runtime, display list, and host integration
  slices, each with format or behavior evidence.

### P5 — Decompose 3D, physics, networking, and tooling

- [ ] `GBL-040` Before GAME30–GAME35 implementation, define deterministic fixed-point contracts and tests for
  foundational vector/matrix/quaternion, broadphase collision, and physics capabilities.
- [ ] `GBL-043` Decide and specify F64 behavior separately where needed, tracking its dependency on
  VER04p / WX-FL1 without treating that Warrant work as game-platform proof.
- [ ] `GBL-041` Add networking protocols and tooling as explicit contracts with independent vectors, then
  build on verified simulation foundations. Replay and checkpoints are tracked in `GBL-051`.
- [ ] `GBL-042` Reconcile GAME00's GAME30–GAME35 and GAME40–GAME45 rows with actual packages, language lanes, device backends,
  and host integration. Roadmap presence alone is not completion evidence.
- [ ] `GBL-050` Define portable input action frames, beginning with the Rust contract and canonical vectors.
- [ ] `GBL-050a` Port the input action-frame contract and vectors to TypeScript after the Rust boundary is stable.
- [ ] `GBL-051` Define deterministic replay and checkpoint contracts over verified action frames.
- [ ] `GBL-052` Define bounded rollback behavior and resource limits over replay/checkpoint state.
- [ ] `GBL-053` Define generational handles and deterministic scheduling for reusable game entities.
- [ ] `GBL-054` Define grid navigation inputs, costs, tie-breaking, and deterministic path results.
- [ ] `GBL-055` Define portable asset identity, loading, and invalid-input behavior.
- [ ] `GBL-056` Define tick-based animation and skinning contracts with deterministic interpolation/state.
- [ ] `GBL-057` Define a renderer contract that separates deterministic simulation output from host rendering.
- [ ] `GBL-062` Add a canonical package-inventory `--git-ref` mode using the existing classifier and
  regular-blob path inventory. The default working-tree report must fail closed when sparse-checkout
  paths are omitted, while preserving untracked overlays in ordinary non-sparse working mode. Define
  symlink and gitlink handling and test committed-ref parity, untracked-only changes, collisions, and
  unknown classifications. This P2 tooling follow-up does not block `GBL-010`: refresh the inventory
  from refreshed `origin/main` using the existing classifier and tracked-path audit; `GBL-003a` is
  already merged.
- [ ] `GBL-063` Repair the repeated Ubuntu 24.04 Kiln G1 synthetic throughput miss with the smallest
  reusable optimization justified by native Linux evidence. On PR #19644 head
  `d563077857e66f3917220611e5df91fafa7a0348`, run `44316496407` job `134844872264` measured
  66,600 bytes / 936 lines at 12.7 MB/s (5.246689 ms) against the unchanged 14 MB/s floor, which
  requires about 9.3% less elapsed time; census throughput was 12.0 MB/s and `gated=0`. The
  concurrent Ubuntu 26 lane passed at 19.7 MB/s. The earlier unchanged-source Ubuntu 24 run
  `134824627592` measured 12.8 MB/s (5.220751 ms) and also failed; Ubuntu 24 run `134816988790`
  measured 19.5 MB/s and passed. All used Go 1.26.1 on linux/amd64. The pass was on AWS image
  `20261111.127.1` / kernel `7.0.0-1014-aws`; both misses were on Azure image `20261111.370.1` /
  kernel `7.0.0-1017-azure` (westcentralus and eastus). CPU model is not recorded, so provider/image
  correlation is not proof of cause. A later limited manual diagnostic dispatch `44321293161`
  measured 12.6 MB/s (5.297186 ms) for the same gated slice on Ubuntu 26 Azure eastus, image
  `20261111.196.1`, kernel `7.0.0-1019-azure`; its separate census was 11.6 MB/s (`gated=0`).
  The corresponding Ubuntu 24 AWS leg passed at 19.4 MB/s (census 18.0 MB/s). Both used Go 1.26.1.
  This was a diagnostic/manual run, not full CI; it was cancelled after preserving the requested
  artifacts. Preserve all receipts and gate semantics; make no blind retry of unchanged code.
  The failed PR job checked out synthetic merge `99856c512` (PR head `d563077857e66f3917220611e5df91fafa7a0348`
  into base `73fa1d9c446...`); the diagnostic dispatch checked out branch commit
  `8638751e91d813a41767684c7af7f5bb39be85df` on current main with D33. Its Kiln trace/traced trees
  were `de74a093b405c88126635bd33bb42d05383523e2` and
  `e37d3956972662d7e4c5a243e15c934787746fe6`; lexer, strace-lexer, and strace-parser trees were
  `8262cdf49df65e485f7f96d556565edc45cb7a7c`, `209ad45586ec07be748945f013a68bf99546d45d`, and
  `92cb784a49174228d01ff853af57e5a708df3cef`. The exact diagnostic fixture and repeated-slice
  SHA-256 values were `dc8acd0de1d9b235c3924d0d71e2fcd565274e09ded9975570f429b548525aae` and
  `6cd61f63fdec55ce37294ad061705394b1babda80e5368b17ecb5bd38e2207af`.
  The bounded native CPU profile attributed 33.77% cumulative CPU to `parseState.match`, 18.66% to
  `matcher.match`, and 40.86% to `GrammarLexer.tokenizeStandardInto`; `newLexer` was 1.87% cumulative
  CPU despite 49.45% of allocation space. This supports testing cursor reuse as a falsifiable
  allocation hypothesis only; it does not establish the roughly 10.2% elapsed reduction needed by
  the lower-rate probe, nor the >10.2% needed for useful margin. If same-host admission ABBA shows
  only marginal wall gain, inspect the measured parser/matcher CPU paths next. Native summaries and
  profiles are preserved under `/tmp/kiln-g1-profile-44321293161/`.
  First run a native Linux amd64 diagnostic dispatch from `.github/workflows/ci.yml`
  with `full_build=false`, `cross_os_full=false`, and `kiln_g1_profile=true`; it records runner/source
  identity and runs profiles only on Ubuntu 24.04. Keep the existing uninstrumented G1 test as the
  admission result. A separate `TestG1DiagnosticSyntheticSlice` constructs the same repeated
  `build.strace` slice and calls `trace.BestPass1` with `HelperRoot=true`, excluding the 4 MiB census
  and other sandbox tests from both profiles. Assert 66,600 bytes / 936 lines and record the fixture
  SHA-256. Run 20 `BestPass1` calls with `-cpuprofile` (at least 5 seconds, up to 4,000 timed passes)
  and two calls in a separate `-memprofile -memprofilerate=1` run (at least 500 ms, up to 400 timed
  passes), each with a 30-second timeout; instrumented timing is diagnostic only. Record image,
  kernel, provider/region, CPU model when available, toolchain, actual checkout SHA/runtime tree
  IDs, and workload identity. Inspect the actual hot path before choosing a repair;
  compare pre/post revisions on the same host and toolchain in A-B-B-A order on that identical
  slice. Keep the 14 MB/s floor, input, and failure receipts unchanged. G1 uses at least seven timed
  passes spanning at least 250 ms (up to 200); the logged `repeats=7` is the minimum requested count,
  not the actual adaptive pass count. Keep the separate ungated census. Add focused correctness
  controls for public/owning and alias paths,
  atomic errors, and relevant Unicode/property/fuzz behavior without mirroring the implementation.
  A bounded opaque zero-value-ready single-owner lexer cursor was prototyped and reviewed, with one
  cursor per independent
  hook-free `LineParser`; it binds the immutable `Compiled` grammar for each call, with no cached
  grammar pointer, token storage, hook, reset API, or global pool. Both fresh-owned and cursor
  initialization must share setup while initializing self-referential stacks at the final address;
  clear the cursor's full lexer state after success and error. Indentation/layout grammars and
  hook-bearing calls remain on the owning path. Compare fresh-owned and reused sequences including
  retained outputs, error/refusal recovery, spans, mode changes, Unicode normalization, and
  independent cursors under `-race`; reuse the existing bounded lexer/strace-parser fuzzers. These
  correctness controls passed locally (full affected-package race suites and 30-second lexer,
  parser, and trace fuzz campaigns); they establish neither a performance repair nor universal Go
  source/lifetime equivalence.
  The reviewed candidate `639a7129ddf9d72995df1b6c73c112db8aab2c44` was measured against pinned
  baseline `10eeb7add42d2e7af504eef2f5e3707a61971cc9` in workflow run
  [44334489030](https://github.com/adhithyan15/coding-adventures/actions/runs/44334489030), with
  the corrected workflow checkout at `abb2b3217ac975226870bab17d7f54c65a203c1d`. The four cells
  were A1 baseline, B1/B2 candidate, A2 baseline, on each of Ubuntu 24.04 and 26.04; both used Go
  1.26.1 linux/amd64 and identical fixture (`dc8acd0d…525aae`) / repeated-slice
  (`6cd61f63…2207af`) hashes. Ubuntu 24 AWS (image `20261111.127.1`, kernel `7.0.0-1014-aws`,
  Xeon 6975P-C) passed every official gate cell: A1 19.4, B1 19.7, B2 19.9, A2 19.4 MB/s.
  Its separate precise helper medians were 3,616,626 ns for A and 3,463,755 ns for B (about 4.2%
  faster). Ubuntu 26 Azure (image `20261111.196.1`, kernel `7.0.0-1019-azure`, Xeon Platinum
  8370C) failed the unchanged official 14 MB/s gate in all four cells: A1 13.7, B1 13.9, B2
  displayed 14.0 but failed below the floor, and A2 13.8 MB/s. The helper medians were 4,937,395
  ns for A and 4,936,559 ns for B (about 0.02% faster), so cursor reuse did not improve the slow
  host meaningfully and is not the G1 floor repair. The run preserved all admission failures and
  precision observations; no retry was made. Keep this prototype recoverable for isolated
  correctness/allocation work, but do not publish it as the performance repair. Await a new scope
  based on the measured parser/matcher CPU path. The missing ownership/GC source bridge remains
  under existing VER/Warrant debt; do not add an unrelated SAT model.
  The selected parser-only repair optimizes a leading contiguous run of direct basic
  `kTok` alternatives in `parseState.match(kAlt)`. Native pprof attributed 330 ms flat to the
  `switch n.kind`, 70 ms to failed-choice stack restoration, and 1.68 s cumulative to recursive
  `kAlt` calls (overlapping costs); the strace grammar repeats token alternatives in `arg`, `inner`,
  and `tail`. For a leading run of at least two `kTok` nodes sharing `skipNL`, inspect normalized
  token name and legacy enum once while preserving choice order, the ordered name/enum predicate,
  failure recording and expected-prefix order, post-skip failure position, and rollback to the
  pre-skip position if the entire run misses. Continue with ordinary sequential matching after the
  run. Generic matching stops when `s.exceeded` is set, so retain depth/memo/resource refusals and
  never record later expected IDs after exhaustion. Keep later mixed alternatives sequential; do
  not change whole rules, memoization/depth/curMax, or cursor ownership in this candidate. Old/new
  behavior was compared across exact tokens, name/enum ambiguity, enum-only tokens, EOF,
  newline-skip rollback, expected-prefix order, compact output, and resource-bound refusals; existing
  differential/sequence fuzzers and focused race checks passed. The reviewed candidate
  `242c6943acd9b168740796f11e603f1316b81e61` was compared with pinned baseline
  `10eeb7add42d2e7af504eef2f5e3707a61971cc9` in narrow workflow run
  [44343414219](https://github.com/adhithyan15/coding-adventures/actions/runs/44343414219).
  All four official admission cells passed on both hosts without changing the 14 MB/s floor or
  4 MiB census. Both used Go 1.26.1 linux/amd64, fixture SHA-256
  `dc8acd0de1d9b235c3924d0d71e2fcd565274e09ded9975570f429b548525aae`, and repeated-slice SHA-256
  `6cd61f63fdec55ce37294ad061705394b1babda80e5368b17ecb5bd38e2207af`. Only the parser tree
  differed (`17c0d769dd57a57d6677d1702e77173c85edff4c` baseline,
  `c84ff3d25b4cd8a56a1db012ca3c45ec360e9479` candidate); Kiln, trace, traced, lexer, strace-lexer,
  strace-parser, gate, and measurement trees matched.
  Ubuntu 24.04 used Azure image `20261111.370.1`, kernel `7.0.0-1017-azure`, Xeon Platinum 8573C:
  official A1/B1/B2/A2 rates 14.7/15.2/15.3/14.5 MB/s, precise BestPass1 medians 4,715,529 ns
  baseline versus 4,509,879.5 ns candidate (4.36% lower elapsed time), and ungated census rates
  13.5/13.5/13.9/13.3 MB/s. Ubuntu 26.04 used Azure image `20261111.196.1`, kernel
  `7.0.0-1019-azure`, AMD EPYC 9V74: official rates 16.7/17.4/17.3/16.7 MB/s, precise medians
  4,006,460.5 ns baseline versus 3,864,630 ns candidate (3.54% lower elapsed time), and census
  rates 15.7/16.0/16.2/15.4 MB/s. These show useful within-host gains and modest U24 admission
  margin. They do not prove the historically slower CPU class is fixed: its CPU model was not
  recorded, and no host-cause inference is claimed. Preserve all cells and artifacts under
  `/tmp/kiln-g1-parser-abba-44343414219/`. The SAT/VER bridge covers a bounded 32-row observation
  domain; it does not prove Go source behavior or lifetime semantics. GBL-063 remains pending until
  ordinary protected PR checks pass and the repair is merged. Keep the cursor experiment separate
  as a rejected floor repair, preserve the 14 MB/s gate and prior failure receipts. No open PR
  in the current list owns parser fast-path work; draft PR #19633 concerns gate calibration and
  cannot change or weaken this unchanged acceptance criterion.
  A later focused lexer candidate `eedb854e2a5e6436be8ce6dc55295c902710d9e5`
  avoids copying each immutable `ModeTransition` value in the per-token loop
  and checks nonempty mode guards before scanning token triggers. The bounded
  frozen-oracle controls and 2,048 seeded cases passed locally; the Go fuzz
  campaign ran 5,998,782 executions, and the negative guard mutation was
  detected. These are finite tests, not source/Warrant/SAT proof. In paired
  workflow run [44370322596](https://github.com/adhithyan15/coding-adventures/actions/runs/44370322596),
  pinned baseline `adadc8fc0b9783b3a829fbb4309182f558d4561b` and candidate
  `eedb854e2a5e6436be8ce6dc55295c902710d9e5` used Go 1.26.1, the same unchanged
  G1 gate (`f834ff07849b8c1dd8b412d99e4e95e119176614`), BestPass1 measurement
  source (`c04b3a093c81262201ecb2f43d48f9a162312eff`), fixture SHA-256
  `dc8acd0de1d9b235c3924d0d71e2fcd565274e09ded9975570f429b548525aae`, and
  66,600-byte / 936-line slice SHA-256
  `6cd61f63fdec55ce37294ad061705394b1babda80e5368b17ecb5bd38e2207af`. All
  eight official cells and eight separate precision cells completed in
  A1/B1/B2/A2 order, and every official cell passed with the original floor
  and separate census intact. Ubuntu 24.04 Azure image `20261111.370.1`, Intel
  Xeon Platinum 8573C: official slice rates 18.4/19.3/19.5/18.6 MB/s; helper
  times 3,871,256/3,484,245/3,569,618/3,627,890 ns, paired mean 5.94% lower
  elapsed time for B. Ubuntu 26.04 image `20261116.202.1`, AMD EPYC 9V45:
  official rates 22.1/22.9/22.7/22.2 MB/s; helper times
  3,043,198/3,163,751/2,998,331/3,142,800 ns, paired mean 0.39% lower for B,
  within the observed spread. Both A hosts passed, so the experiment does not
  show rescue of the historical Ubuntu 26.04 EPYC 7763 miss at 13.2 MB/s
  (run `44362035252`, job `135005747661`); that host has no paired B result.
  It does not establish pooled or stable 6% gain. Exact per-cell source trees,
  official and helper binary SHA-256 values, host details, and artifacts
  `14458713072` / `14457444089` are recorded in
  `code/programs/go/kiln/ci/G1_PROFILE_RECEIPT.md`. A narrowly titled lexer
  optimization PR is being opened with unchanged protected CI and gate
  semantics. Keep this broader G1 host-robustness item open until the PR passes
  ordinary protected checks and any slow-host rescue claim has a predeclared
  matched-host cohort independent of candidate results; retain every run and
  make no result-selected retries.
- [ ] `GBL-058` Repair the conflict between the actual `P2D09`/`P2D10` specification filenames/scopes
  and GAME00 labels. Keep this as a separate documentation repair; do not mint or reuse colliding game
  identifiers to paper over it.

## Evidence rules

- Unit tests show the tested examples pass; generated/property tests cover their stated input
  generator and bounds; fuzz runs cover their recorded corpus, duration, and configuration; SAT or
  model checking covers only the encoded model. Report each type separately.
- Differential tests require an independent oracle or implementation and an explicit comparison
  surface. Reusing the implementation's own expected-value function is not differential evidence.
- Warrant evidence must satisfy the actual claim, including positive cases where the claim describes
  success. A universal `Err` path cannot establish successful behavior.
- Never claim exhaustive coverage, formal proof, cross-language parity, or complete host integration
  from package existence or finite examples. State missing domains and unverified lanes.

## Duplicate-work boundary

Reuse the existing `VER-*` assurance and shared test infrastructure; this backlog does not create a
second generic verification roadmap. Reuse `MUS00`–`MUS08` audio/synthesis and transcription work;
game-specific audio adapters belong here only when a concrete integration gap exists. Prioritize
game-owned contracts, portable primitives, and evidence that those capabilities are actually
consumed.
