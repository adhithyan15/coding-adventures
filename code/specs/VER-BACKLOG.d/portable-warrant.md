# Portable Warrant implementation queue

**Authority:** current queue, status, receipts, and discoveries for the
portable Warrant work described by [VER04i](../VER04i-polyglot-warrant-evidence.md).
The main [VER-BACKLOG](../VER-BACKLOG.md) keeps the compact program index;
this shard owns operational detail. Historical rollout snapshots in VER04i
remain historical and must not be read as the current queue.

## Current queue

| Order | Work | State / dependency |
|---|---|---|
| WD-003 | Rust Warrant v0 corpus producer, independent Python semantic and digest oracle, byte-stable regeneration and falsification, consumer/build routing | Merged as PR #19177 at `d746f3b16bbbc78781ade0987abd88bc5a953e37` (head `7d2c92409b15fb19ea82a6cb892eaed44067975c`). Attempt 1 run #65115: all 31 jobs terminal, 30 substantive jobs plus CI gate passed; 40 exact-head checks terminal (35 success, 5 skipped). Swift Apps #596 and Books #13690 passed. Local Rust suite: 422 tests/44 targets; generator `--check`; Python oracle 20 tests and 572,109 cases/9 suites; build-tool corpus validation 204 cases/19 boundaries/25 source inputs. Sparse checkout prevented the broad local boundary suite from reading 15 Haskell BUILD files; hosted full-tree contracts passed. The corpus remains explicitly unsharded ct_compare-v0 and does not execute a Python lane binding. |
| WD-004a | Define the strict per-lane manifest schema, planned consumer registry, and corpus structural validator | Merged as PR #19236 at `28572c410df730098f7bf92c390c07237aa80591` (head `0c3676504666241a2815b0ecff4b6fbc924cd316`). CI attempt 2: 14 jobs passed, 13 expected skips, explicit gate success; Swift Apps attempt 2: 2 passed; Books: 2 passed, 5 skipped. Structural parsing only: no real lane execution, report output, or proof credit. |
| WX-RD1 | Bound the ground evaluator's deep read-copy amplification while preserving public `EvalValue` and error shapes | Existing P1 under WX-001, selected as the next bounded item after PR #19236. Spec-first guard contract; the 32-byte portable slot-charge under-accounting and total-memory/RSS claims remain open after the first guard slice. |
| WD-004b1 | Add the safe lane artifact loader, registry join, raw digest checks, and static readiness checks | Delivered on branch `claude/wd-004b1-lane-loader` (not yet merged); specified in VER04i 8.2. `ca-assurance-report::lane_loader` (library only): bounded, contained, symlink-refusing reads; raw digests of spec, corpus, TFV and binding; the labels sidecar and its canonical digest (`toolchain.version` blanked, name kept; reproduces the Python, TypeScript and Go runners' values); the WD-004a join; eight named static readiness checks; output ends with the unclaimed list and `release_ready: false` (sandbox not armed). Registry: `go/ct-compare`, `python/ct-compare`, `typescript/ct-compare`, all `active`, count 3; the Python, TypeScript and Go runner tests that pinned the old registry now use synthetic ones, and the TypeScript and Go BUILD lines pass `--registry`. Legacy Rust loader untouched; adopters' `assurance_report.json` unchanged. Security review (two MEDIUM, fixed: sidecar overflow and strict integer tokens; plus Windows reserved names, a spec-pinned readiness list and the shared containment fixture) deferred to WD-004b2: the check-to-open TOCTOU (documented), BUILD lines passing `--require-active`, and junction or reparse-point tests on a Windows runner. Not done: `runner.src_sha256` and `source_sha256` are format-checked only, not recomputed; no authenticity claim about source, build or toolchain. |
| WD-004b2 | Integrate lane-aware package reports, CLI/check behavior, and honest absent rungs | Delivered on branch `claude/wd-004b2-report-integration` (not yet merged); specified in VER04i 8.2 and 8.3. The report gains a required `lanes` array (a rung per registry lane: `live` only when `load_lane` passes every named check, `planned`, or `absent`; `release_ready` false and `sandbox` `absent` always); derived from the loader only, isolated from the Rust proof, effect and verdict evidence; `--check` re-derives it (`LaneMismatch`, `LabelOverclaim`, `StaleEvidence`). `source_sha256` and `runner.src_sha256` are recomputed by a contained, link-refusing walk with per-lane rules (two new readiness checks, ten in all). CLI: `--check-lane <lane>/<name> [--repo-root D] [--require-active]`; the Python, TypeScript and Go ct-compare BUILD and BUILD_windows end with it. Every Rust adopter's `assurance_report.json` regenerated (only `lanes` added). Still open: the check-to-open race (documented), junction and reparse-point tests on a Windows runner, the sandbox (CAP-RT-1xx), authenticity and build execution; a loader change does not re-run the lane BUILDs. |
| WD-004b3 | Python lane hardening left by the WD-004b2 review (low) | Done on branch `claude/wd-004b3-python-entry`. (1) Stdlib shadowing: `tests/warrant_evidence.py` removed its own directory from `sys.path` before its first non-built-in import and re-runs the child (and the runner's sabotage children) with `python -P`, so no file beside the entry (`shutil.py`, `shutil.cpython-*.so`, `.abi3.so`, `.pyd`) can be imported before isolation; new `ct-compare/tests/test_entry_shadowing.py` plants one beside a copy of the entry and asserts it never runs (with a control). (2) Unscanned tests folder: `lane_loader` scans the runner's package (`python/warrant-corpus`, whole, `tests/` included) for stray `*.pyc`/`*.pyo` with the same rules as the consumer package. Python manifest regenerated (`runner.src_sha256` moved); the three lanes report LIVE. |
| WD-101 (Python L0 runner) | First non-Rust lane: `python/warrant-corpus` runner, `python/ct-compare` binding and committed lane manifest, BUILD drift/sabotage/validate lines, repository-input registration | Delivered and merged (PR #19354). 12 examples; 9 exhaustive suites, 572,109 cases enumerated in Python over finite domains (`Bytes[347]` x `Bytes[347]`, `Bytes<0/1/2/4/16>`, `U64[377]` x `U64[377]`), every `inputs_sha256`/`expected_sha256`/call count equal to the Rust-emitted corpus; 8 negative controls, each caught by a required claim; each `sabotage` child exits 1 with `not ok`. The consumer row stays `planned`: `ca-assurance-report`'s own test pins the checked-in registry to `planned`, and flipping it belongs with WD-004b1. Not delivered: report integration, sandbox arming (CAP-RT-1xx Python), native (layer B) suites, the shared `warrant-corpus-conformance-v1` fixture of VER04i section 9. |
| WD-100 (shared runner conformance fixture) | `warrant-corpus-conformance-v1` (VER04i 9.1): 397 language-neutral cases (json 231, tagged 76, containment 77, label-digest 13) every lane runner must pass, an independent stdlib checker (`code/scripts/warrant_corpus_conformance.py`), Python and TypeScript lane wiring, build-tool consumer registration | Delivered on branch `claude/warrant-lane-conformance` (not yet merged). Fixes it forced: Python accepted unpaired surrogate escapes, floats and unlimited nesting; Python containment mishandled mixed-separator device paths and posix `//`, and production called it without a flavour; TypeScript sorted keys by UTF-16 order, rendered non-integers verbatim, enumerated integer-like keys numerically and left DEL raw, and counted scalars against the 256 nesting limit. DEL is pinned as `\u007f` and a posix path with exactly two leading slashes is refused. **The Go lane (WD-103) now passes the kit** (branch `claude/go-lane-conformance`): all four categories run in `conformance_test.go` with per-category counts compared against `index.json`; the build-tool consumer list already covers `go/warrant-corpus`. Fixes it forced in Go: DEL was written raw (now `\u007f`), the reader admitted 257 nested containers (now at most 256, scalars not counted), and containment was `filepath.Rel`-based with no flavour (now lexical, with a `posix`/`win32` parameter and the host flavour in production). Every later lane does the same. Not yet in the fixture: TFV parser cases, model-IR evaluator cases and the deliberately wrong subject result of VER04i 2.5 (each lane's own falsification tests cover them until a category lands). |
| WD-102 (TypeScript L0 runner) | Second non-Rust lane: `typescript/warrant-corpus` runner (`@coding-adventures/warrant-corpus`, zero runtime dependencies, runs as TypeScript by Node type stripping), `typescript/ct-compare` binding and committed lane manifest and label sidecar, BUILD and BUILD_windows `check`/`sabotage`/`validate-manifest` lines, repository-input registration | Delivered and merged (PR #19368). Same corpus as WD-101: 12 examples; 9 exhaustive suites, 572,109 cases enumerated in TypeScript (65,537 `full`, 506,572 `bounded-set`), every `inputs_sha256`/`expected_sha256`/call count equal to the Rust-emitted corpus and every manifest row equal to the Python lane's; 8 negative controls, each caught by a required claim; each `sabotage` child exits 1 with `not ok`. `U64`/`Nat` are `bigint`, `Bytes` results must be an exact `Uint8Array` and a `fresh` result may not alias an argument (VER04i 5.2); the corpus is read by a strict JSON reader. 183 tests, line coverage 99.7%. Same open items as WD-101: the consumer registry has no `typescript/ct-compare` row (the checked-in registry lists only `python/ct-compare` and `ca-assurance-report` pins it), so BUILD validates the manifest without `--registry`; no sandbox arming (CAP-RT-1xx TypeScript/Node); no layer B (native) suites; no `warrant-corpus-conformance-v1` fixture. |
| WD-103 (Go L0 runner) | Third non-Rust lane: `go/warrant-corpus` runner (standard library only; a Go library plus the leaf package's `cmd/warrant-evidence` entry), `go/ct-compare` binding, adapters and committed lane manifest and label sidecar, BUILD and BUILD_windows `check`/`sabotage`/`validate-manifest` lines plus one explicit `not ok` line per fault, repository-input registration | Delivered on branch `claude/warrant-go-lane` (not yet merged). Same corpus as WD-101 and WD-102: 12 examples; 9 exhaustive suites, 572,109 cases enumerated in Go (65,537 `full`, 506,572 `bounded-set`), every `inputs_sha256`/`expected_sha256`/call count equal to the Rust-emitted corpus and every manifest row equal to the Python and TypeScript lanes'; 8 negative controls, each caught by a required claim; each `sabotage` child exits 1 with `not ok`. Go cannot import by name, so a compiled adapter registry is verified by runtime symbol name and source file (`source-mismatch`); errors map by sentinel identity; `U64` is a native `uint64`; a `fresh` result is checked by mutation; the corpus is read by a strict JSON reader (duplicate keys, invalid UTF-8, lone surrogates, integers above 2^53) and its paths are contained with a cross-drive-safe check (VER04i 5.3). Statement coverage 99%. Same open items as WD-101 and WD-102: the consumer registry has no `go/ct-compare` row, so BUILD validates the manifest without `--registry`; no sandbox arming (CAP-RT-1xx Go); no layer B (native) suites; report integration. (The `warrant-corpus-conformance-v1` fixture is now passed: see WD-100.) Low findings from the Go lane security review are fixed on `claude/go-lane-conformance`: `check` refuses a non-text committed `toolchain.version`, and `sabotage` children run under a 10-minute timeout. An unknown or inapplicable op in `<fault>:<op>` still exits 0 in all three lanes (an unknown fault name is a usage error everywhere). |
| WD-100 | Lane conformance kit `warrant-corpus-conformance-v1` (VER04i section 9.1) | Delivered in PR #19489. Every new lane must pass it; the Go lane passes it (PR #19493). Gates the lane order below. |
| CV2-S1 to CV2-S13 | `warrant-corpus-v2`: S1 spec, S2 `warrant-grammar` export (**done, PR #19563**: public `domain` module, `warrant-grammar` 0.8.0, `gen_evidence --check` byte-identical for every adopter; it records five generator-vs-VER04r 5.1 differences for S3/S1 erratum to settle: byte-string order of the harness prelude, its case-set consequence for `List` of byte strings, the 73-value `NatCount`, sorted signed edge sets, and `fx_edge` at 16 bits and below; VER04r 16.2), S3 leaf crate `warrant-corpus-gen` (**S3-0, the VER04r 16.3 erratum with the owner decisions of 2026-11-17 [D8] to [D11], PR pending**; then **S3a skeleton, lowering and cost: done on branch `claude/warrant-cv2-s3a`, PR pending** (new crate `rust/warrant-corpus-gen` 0.1.0: canonical JSON writer, `corpus.profile` reader, `ir.core` lowering with level refusals and `census`, worst-path `ir_cost`, wrap arithmetic; lowering byte-identical to the oracle on ct-compare, gf256 and two synthetic documents; `generate` is a fail-closed stub; VER04r 16.4), S3b evaluator, enumeration, the 18 v1 digests and the 16.2 census, S3c faults with blind claims, emission and `--check`, S3d the committed ct-compare v2 fixture with or after S4 part 2 plus the build-tool input row; no shared-crate change), S4 FULL independent `.warrant` source oracle, S5 conformance fixture, S6 Python and S7 TypeScript v2 runners, S8 gf256 (first adopter after ct-compare), S9 v2 lane validator in `ca-assurance-report`, S10 `ir.fixed`, S11 `ir.data`, S12 `inout`/`ensures`, S13 delete v1 | Owner decision 7 (2026-11-16). PR CI runs a corpus-chosen `slim` chunk subset; `full` depth runs nightly on compiled lanes; depth is shown in labels. Go finishes on v1 and is ported to v2 with S6/S7. S1 must resolve model arithmetic to the VER04 section 3 wrapping semantics (WD-013) before S8. |
| WD-HARN1 | Harness supervisor hardening: wait timeout, never block exit, poison recovery (**done, merged as #19530 (0c9f75a9e92)**) | From the 2026-11-14 Windows `rust/lzw` hang. After WX-G (PR #19369), before CV2-S2 in the `warrant-grammar` queue. Companion: BT-TIMEOUT, PR #19483 (build-tool per-command timeout). |
| WD-HARN1b | LOW follow-ups of the WD-HARN1 review (**done on branch `claude/warrant-harness-followups`, PR pending**): obs adopters' sabotage lines pass only with a `not ok ` line and no `Bail out!` line; `obs_bail_out` writes stderr with `writeln!`; stale `OBS_CALL_TIMEOUT_MS` doc and a dead `latched = true` fixed; obs adopters regenerated; hash-functions panic-allocation wording refined | Prelude edits confined to the `obs` region, so only the obs adopters regenerate. |
| OBS-PERF | P2. The obs observer hands each observed call to a worker thread, about 35 to 50 microseconds a call, which is why `fnv1a_32`, `fnv1a_64` and `djb2` carry no `heap` row. Proposal: run a whole claim body on the worker instead of one hand-off per call | Needs a design note first. |
| BT-TESTONLY | P2, proposed, needs design review. A change confined to an adopter's generated test harness (`tests/generated/`) or evidence files does not change the library its dependents link, so the build tool should not cascade it to dependents (WD-HARN1 rebuilt about 428 packages for about 2.5 h per platform) | Risks: dev-dependency edges and anything that compiles tests across crates must still be honoured. |
| Lane order after Go | ruby, java, csharp, kotlin, fsharp, c, cpp, elixir, lua, perl, haskell, swift, dart, ocaml, two at a time | Each on corpus v2 once CV2-S6 and CV2-S7 land, each passing WD-100. |
| WX-O0 / WX-FL0 / WX-M0 | Specs VER04n (opaque, with VER29 and VER04m amendments), VER04p (floats), VER04o (collections plus first generics) | Written on other branches; VER04n and VER04p in parallel, VER04o after VER04n. Implementation WX-O1, WX-M1, WX-P1, WX-FL1 follows WX-G, WX-D2, WX-F, LP-012, WX-S1, WX-T1. |
| VER30-INV2 | Re-run the inventory with a `recursive-types` tag | **Done 2026-11-16** (data in `data/warrant-inventory/*-2026-11-16.*`; four features alone +432 crates over a baseline of 492, with recursive types +775 crates / 2,028 ports, recursive types alone +74). Recursive types are lifted (VER04s, WX-R0 after VER04o; WX-R1 after the four). |
| WX-R0 | Spec VER04s (recursive algebraic types) | **Written, PR #19501.** Owner decision 15 (2026-11-16). No stream tag; diagnostics `W0770` to `W0779`. |
| WX-R-REVIEW | Adversarial review of VER04s | **Adversarial review done** (2026-11-16, VER04s section 21). **Owed before WX-R2a:** the lane-specialist read of VER04s section 8.3 and RL1a against a running host. |
| WX-R1 | Implement VER04s (R1-S0 to R1-S5); pilot `canonical-cbor`, partial (3 of 8: `encode`, `try_encode`, `decode`; five tabled as not covered) | OWN-R1 and OWN-R2 resolved 2026-11-16 (VER30 decision 25). Keep the VER30 order: after WX-O1, WX-M1, WX-P1, WX-FL1; **R1-S0 (errata pass, fuel-total measurement) allowed early**. Also needs T1, WX-D2, the `ir.seq` amendment, VER04o C1-S1. |
| WX-R2a / WX-R2b / WX-R2c | Lane rollout: R2a typescript, go; R2b ruby, c, cpp, java, kotlin, csharp, fsharp, elixir; R2c lua, perl, haskell, swift, dart, ocaml | After R1-S5 and the lane's v2 runner (VER04r S6, S7); sequential a, b, c; after the WX-R-REVIEW lane read. Each lane's fixture port is its own package. |
| WX-R3 | Generic recursive types (`Tree<T>`) | After WX-R1 and WX-P1 (VER04o G1-S2). |
| WX-R-PILOT2 | `bounded-json` as second adopter, then `json-serializer`, `json-value` | OWN-R2 (2026-11-16). After WX-R1 and FL2. |
| WX-R-AMEND-j/k/o/r | Amendments to VER04j, VER04k, VER04o, VER04r (VER04s section 17.1, 17.2), each a PR of its own | VER04r carries the `ir.seq` meter (4.1) the pilot needs. R1-S0 re-reads each before code. |
| WX-R-AMEND-i | VER04i amendment: allocation-registry rows (landed with VER04s) and the lane table (VER04s 17.3) | Before WX-R2a. |
| WX-R-AMEND-13 | VER13 assumption `A-REC-HOST`: "the lane's adapter converts the recursive host forms it binds faithfully and within its caps", discharged per lane by the `v2-recursive` kit (RO1, RO2) | With R1-S3. |
| WX-R-AMEND-29 | VER29: the published form of a recursive claim carries its scope and the canonical type digest (RT9) | Before any recursive claim is published. |
| TRIG-CONV | One reference algorithm for the seventeen `trig` ports: spec first, then one PR per port | Owner decision 16. **Done 2026-11-16:** all seventeen ports converged on TRIG-REF-1 (PHY00a; per-port PRs in its section 8). The VER04p floats pilot (WX-FL1) is no longer waiting on it. |
| WX-O4a / WX-O4b | Verify `x509-subject-public-key-info` and `der-asn1` (own Warrant evidence each) before the RSA pilot stage P-b | Owner decision 17, reversing the VER04n OWN-4 recommendation. Then adopt `x509-rsa-public-key`, then P-b. |
| WD-100b | `warrant-corpus-conformance-v1` follow-up: Windows reserved device names refused; `win32` case folding is ASCII-only with non-ASCII compared exactly | **Done on branch `claude/warrant-conformance-reserved-names` (PR pending).** 64 containment cases added (77 to 141; fixture 461 cases; widened after security review: colons, stem trim, superscripts, CONIN$/CONOUT$), checker reference and string-level second opinion updated; Python, TypeScript and Go `relativeInside` fold ASCII only and refuse win32 reserved names and trailing dot/space components; the Rust loader's `lexical_containment` likewise (`effect_certificate.json` regenerated, capabilities unchanged); each lane's `ct-compare` manifest and labels regenerated. VER04i 9.1 marked implemented. Not done: the every-host *artifact-path acceptance* wording has no lane consumer yet (the Rust registry-path validator already refuses these names); a lane that adds an artifact loader must apply `refused_component` on every host. Owner decision 20. |

WX implementation continues under the existing WX-001 owner and IDs; this
shard does not create a duplicate feature owner. WX-H/E prerequisites merged
in #19121; WX-D1 S0 errata merged as #19204 at
`d214abc36793908274a21a15282d39fb0694dd16`, and S1 checker/ground evaluator
merged as #19209 at `df621fd51d3633a0cf9fff353372374efd617d5d`. D1 S2a
evidence-generator/corpus translation remains outstanding, so WD-003's
ct-compare Warrant v0 corpus is unchanged.
The tracked missing language features remain WX-D (bounded sequence/model
operations, then higher-order list operations and bounded unrolling), WX-F
(stateful sequence drivers), and WX-G (`inout`/post-state `ensures`), followed
by D2 and D3 in the existing VER04k order. These close the current
unrolled-model and stateful/inout workarounds; they are feature work, separate
from extending WD-003's portable corpus profile. PR #19227 merged as
`173110a5254d53761ee4ba82656bb4ae7f7c6c30`; it is a wasm-leb128 adopter that
changes legacy Rust assurance behavior, not a WD-004a consumer-registry
implementation.

## Gaps handed to WD-004b1 by WD-101

> **Resolved by WD-004b1 (branch `claude/wd-004b1-lane-loader`):** the loader contract, the `planned` to `active` flip (and the
> TypeScript and Go rows), the labels sidecar and the `test_lane_evidence.rs` pin. Still open: recomputing
> `source_sha256`/`runner.src_sha256` (WD-004b2), and a CLI that calls the loader instead of the runners' ports of the rules.

- **Lane artifact loader contract.** The checked-in manifest is `code/packages/python/ct-compare/evidence/test-manifest.json`
  (`schema: warrant-lane-evidence-v1`, 29 rows, sorted by id). It was checked with a scratch build of WD-004a's own
  `validate_lane_evidence` against a registry copy flipped to `active`: accepted, and a one-field edit rejected. That
  validator is a library function with no CLI, so the BUILD instead runs the runner's Python port of the same rules
  (`validate-manifest`); a loader should make the Rust check callable and replace the port as the gate.
- **`planned` to `active`.** Flip the `python/ct-compare` row in `consumers.json` (and `active_consumer_count`) when the
  loader lands, and update `test_lane_evidence.rs`, which asserts the checked-in row is `planned`.
- **`toolchain.version` is excluded from the drift check** (`check` compares everything else, byte formatting included).
  A report-side freshness check must not treat it as evidence, and `source_sha256`/`runner.src_sha256` are the
  `warrant-src-v1` framing over `src/coding_adventures_ct_compare/` and the runner package respectively.
- **`corpus` is the directory** (`code/specs/fixtures/ct-compare-warrant-v1`), `spec` the repo-relative spec path; both are
  re-derived by the runner, not trusted.
- **Honest labels.** The lane proves behaviour over the enumerated domains through the binding only. Proof label is
  `unchecked`, property/fuzz `not-run`, and constant-time is "required by spec; not established in this lane".
- **Binding fidelity.** The `errors=` row maps by exception class, so any `ValueError` from `ct_select_bytes` reads as
  `CT_SELECT_LENGTH_MISMATCH`; the corpus cannot distinguish them.

## Follow-ups from the WD-101 security review

- **Dedicated exception types per port (proposed).** The Python `ct-compare` raises a plain `ValueError` for the
  length mismatch and also for unrelated input errors, so the exact-class binding still cannot tell them apart.
  Each port should raise a dedicated exception type per declared error (for example `SelectLengthMismatch`),
  mapped in `evidence.bindings`. Changing ct-compare's public exceptions is a library change, not part of WD-101.
- **LP-012.** The per-suite `full`/`bounded-set`/`sample` split now exists for the Python lane as the sidecar
  `evidence/domain-labels.json`; the Rust manifests and `ca-assurance-report` still sum them as "exhaustive".
- **Sidecar in WD-004b1.** The lane evidence row schema is closed, so labels are a sidecar; a loader should read it
  and the report should quote `cases_by_label`.

## Runtime jail (CAP-RT-1xx) for Python: deliberately not in WD-101

VER04i section 12 expects each lane's corpus run to repeat under a VER08b sandbox derived from
`required_capabilities.json` and to record a `sandbox` field. WD-101 neither arms a Python filter nor writes that field,
so the lane cannot be `release-ready` (VER04i 12). The `python/warrant-corpus` runner declares the capabilities it needs
(fs read/write, env read of `WARRANT_EVIDENCE_*`, one child process for `sabotage`); a Python jail has to allow exactly
those for the runner and none for the library, arm after interpreter start-up, and re-run the same corpus with the same
binding. Tracked under CAP-RT-1xx (Python); not started.

## Receipts and status

- PR #19264's first Ubuntu attempt exposed two Rust 1.100 stable compatibility
  issues in unchanged prerequisites. Cargo 1.99 emitted test executables into
  `target/debug/deps`; Cargo 1.100 emits separate targets under
  `target/debug/build/<package>/<fingerprint>/out`, so `test-bootstrap`'s
  same-directory sibling scan failed. The fix uses Cargo `CARGO_BIN_EXE_*`
  paths while retaining the existing harness-free self-corpus test and BUILD
  sabotage pipeline. The same Ubuntu job found a new Clippy
  `needless_range_loop` warning in `bit-blaster::bv::mul`; iterating operand
  bits with `enumerate` preserves the shift/AND construction. Both findings
  were reproduced locally on Rust 1.100.0; package tests and Clippy also pass
  on the previous stable toolchain 1.99.1. This records the observed fix, not a
  guarantee about future stable-toolchain changes.
- PR #19264's follow-up Windows job 131411314817 newly selected the LANG native
  executable smoke tests and failed to link the generated PE: MSVC reported
  unresolved `WaitOnAddress` and `WakeByAddressSingle/All` imports from Rust
  `std`. Microsoft Learn identifies `Synchronization.lib` as the import
  library for these APIs; the `twig-aot` MSVC/LldLink command already supplied
  `kernel32.lib` but omitted that library. The current change adds it to the
  shared system-import list. The previous Windows jobs had skipped this LANG
  step, so this is a newly exercised dependency gap, not a confirmed toolchain
  regression. The exact-head hosted Windows acceptance remains required.

- PR #19177 exact merge receipt: integrated head `7d2c92409b15fb19ea82a6cb892eaed44067975c`, run 43144592253 attempt 1, gate job 130981923321, merged at 2026-11-12T05:34:56Z as `d746f3b16bbbc78781ade0987abd88bc5a953e37`. The gate waited queued for about 36 minutes after all substantive jobs passed, then completed successfully; no retry was needed. Do not use earlier-head successes as evidence for this receipt.
- PR #19236 exact merge receipt: head `0c3676504666241a2815b0ecff4b6fbc924cd316`, CI run 43235519556 attempt 2 (14 successful jobs, 13 expected skips, gate 131346046024 passed), Swift Apps run 43235520086 attempt 2 (both jobs passed), Books run 43235520098 (2 passed, 5 skipped), merged at 2026-11-12T17:05:10Z as `28572c410df730098f7bf92c390c07237aa80591`. Attempt 1 Windows job 131243804408 failed with Node `spawnSync ETIMEDOUT` at `discovery.test.ts:551` and `resolver.test.ts:478`; the one authorized failed-only retry job 131304632677 passed. Swift Apps Hello World job 131241710684 was cancelled before runner acquisition; annotations reported hosted macOS arm64 capacity constraints. Its one job-specific retry 131316867122 passed while Notifications job 131316870768 remained successful. No source changes or assertions were made for these retries.
- The Windows TypeScript ETIMEDOUT recurred across two heads: job 130798040371 on `2ad2f78b...` and job 131243804408 on PR #19236 head `0c367650...`; both had a successful same-head retry. No root cause is established. The failing fixtures request a 5-second wall-time limit, and the conformance contract includes adapter plus child process tree time. `plan.test.ts` uses `max(fixture.limits.wall_time_ms, 30_000)` for a process-free plan check, but no spec defines a launch-free execution timer. Do not raise the 5-second process-tree bound to mask CLI startup time. Track clarification/diagnosis under separate `VER-CI002`, not macOS-only `VER-CI001`.
- The initial Lua failures were Ubuntu archive mirror/DNS resolution failures in the two Linux Lua setup paths. The fix routes both prerequisite installs through the existing retrying apt wrapper while preserving the package list and step timeouts. This is a targeted response to those observed failures, not evidence of a global Ubuntu archive outage.
- A transient macOS orientation timeout is tracked by the existing VER-CI001 owner; do not open a duplicate issue here.
- PR #19177 changed `.github/workflows/ci.yml` to route the two Linux Lua apt installs through the repository retry wrapper. The current selector treats a workflow-file change as a Mosaic Swift runtime change, so it selected the required macOS SwiftUI/runtime validation; the lane passed. No existing owner for this broad workflow-change fallback was found. Track precise workflow-to-lane routing as a lower-priority follow-up after WD-004a; do not omit required jobs or alter the current workflow in WD-004a.

## Backlog reconciliation discovery

Main's newer backlog snapshot regressed WD-001/WD-002/WD-003/WD-004 to
`in PR` / `waiting WD-001` / `waiting WD-002` states despite #19054 and #19078
being merged and #19177 active. It also reverted WX-001 to a prerequisite
branch status after those prerequisites had merged as #19121. These are stale
state differences between branch snapshots; this record does not assign a
cause. Preserve dated snapshots as historical evidence, use this shard as the
current operational record, and link to it from VER-BACKLOG and VER04i rather
than copying detailed receipts into both. Do not duplicate WX-001 or VER-CI001
ownership.

The canonical package inventory at
[`package_parity_report.py`](../../scripts/package_parity_report.py) shows
`gf256` in 17 of the 18 VER04i rollout lanes. The complete OCaml package tree
has no `gf256` port, and no existing owner was found. Track a narrow OCaml
`gf256` package-port prerequisite after the first real ct-compare lane runner
and before a separate GF256 portable-corpus rollout. This is package-parity
work, not a Warrant syntax, compiler, or report port. The missing Warrant
language features remain WX-D, WX-F, and WX-G under their existing WX owner and
IDs.

## Follow-ups from WD-102

> **Registry row and `--registry`: done in WD-004b1** (rows are `active`; the TypeScript and Go BUILD lines now pass `--registry`).

- **Registry row for the TypeScript lane.** Add `typescript/ct-compare` (`planned`) to
  `code/specs/fixtures/ct-compare-warrant-v1/consumers.json` and its count, with the Rust tests that pin the checked-in
  registry (`test_lane_evidence.rs`), then add `--registry` to the TypeScript `validate-manifest` BUILD line. It was left
  out because the registry belongs to WD-004a's Rust validator and this branch does not run Rust.
- **Shared runner fixes found by the second lane.** The Python runner hard-coded `toolchain.name == "python"` in the label
  digest; fixed in WD-102 (see `lessons.d`). Any further lane should validate its manifest with *both* existing runners
  before it is called shared.
- **Dedicated error class per declared error.** `ctSelectBytes` throws a plain `RangeError` for the length mismatch and
  for an out-of-range byte, so the exact-prototype binding cannot tell them apart; the corpus passes only `Uint8Array`s.
  Same recommendation as the Python lane: one named error class per declared error.
- **LP-012.** The TypeScript lane writes the same `domain-labels.json` sidecar (65,537 `full`, 506,572 `bounded-set`).
- **Layer B.** TypeScript is the lane with property and fuzz engines (VER05a, VER06b); native suites remain WD-2xx.

## Follow-ups from WD-103

- **Registry row for the Go lane.** Same as WD-102: add `go/ct-compare` (`planned`) to
  `code/specs/fixtures/ct-compare-warrant-v1/consumers.json` and its count, with the Rust tests that pin it, then add
  `--registry` to the Go `validate-manifest` BUILD line. The registry belongs to WD-004a's Rust validator.
- **The adapters are code, not data.** `go/ct-compare/cmd/warrant-evidence` translates the Warrant `Bool` choice into the
  `int` bit `CTSelectBytes` takes and the library's length-mismatch panic into the `ErrLengthMismatch` sentinel. They are
  digested with the source and are what a reviewer reads (VER04i 11, item 3). A dedicated error return from
  `CTSelectBytes` (a library change) would remove the panic translation.
- **No `-trimpath`.** The provenance check needs the real source path of each export and fails closed otherwise; a lane
  BUILD must not pass `-trimpath`.
- **Sandbox (CAP-RT-1xx, Go).** Not armed: the expected strategy is an OS sandbox around the test binary.
- **Layer B.** Go has no property or fuzz engine corpus in the Warrant sense yet; native suites remain WD-2xx.
- **Sibling-lane note.** The older lanes' build-tool language-filter probes used `"go"` as a language with no consumers;
  that stopped being true here and they now probe `"ruby"`.
