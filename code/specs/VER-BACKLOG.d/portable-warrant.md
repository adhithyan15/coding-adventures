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
| WD-004b1 | Add the safe lane artifact loader, registry join, raw digest checks, and static readiness checks | After WD-004a; preserve the legacy Rust loader and make no source/build/toolchain authenticity claim beyond the named checks. |
| WD-004b2 | Integrate lane-aware package reports, CLI/check behavior, and honest absent rungs | After WD-004b1; keep Rust-only proof/effect attestations isolated from non-Rust lanes. |
| WD-101 (Python L0 runner) | First non-Rust lane: `python/warrant-corpus` runner, `python/ct-compare` binding and committed lane manifest, BUILD drift/sabotage/validate lines, repository-input registration | Delivered on branch `claude/warrant-python-lane` (not yet merged). 12 examples; 9 exhaustive suites, 572,109 cases enumerated in Python over finite domains (`Bytes[347]` x `Bytes[347]`, `Bytes<0/1/2/4/16>`, `U64[377]` x `U64[377]`), every `inputs_sha256`/`expected_sha256`/call count equal to the Rust-emitted corpus; 8 negative controls, each caught by a required claim; each `sabotage` child exits 1 with `not ok`. The consumer row stays `planned`: `ca-assurance-report`'s own test pins the checked-in registry to `planned`, and flipping it belongs with WD-004b1. Not delivered: report integration, sandbox arming (CAP-RT-1xx Python), native (layer B) suites, the shared `warrant-corpus-conformance-v1` fixture of VER04i section 9. |
| WD-102 (TypeScript L0 runner) | Second non-Rust lane: `typescript/warrant-corpus` runner (`@coding-adventures/warrant-corpus`, zero runtime dependencies, runs as TypeScript by Node type stripping), `typescript/ct-compare` binding and committed lane manifest and label sidecar, BUILD and BUILD_windows `check`/`sabotage`/`validate-manifest` lines, repository-input registration | Delivered on branch `claude/warrant-typescript-lane` (not yet merged). Same corpus as WD-101: 12 examples; 9 exhaustive suites, 572,109 cases enumerated in TypeScript (65,537 `full`, 506,572 `bounded-set`), every `inputs_sha256`/`expected_sha256`/call count equal to the Rust-emitted corpus and every manifest row equal to the Python lane's; 8 negative controls, each caught by a required claim; each `sabotage` child exits 1 with `not ok`. `U64`/`Nat` are `bigint`, `Bytes` results must be an exact `Uint8Array` and a `fresh` result may not alias an argument (VER04i 5.2); the corpus is read by a strict JSON reader. 183 tests, line coverage 99.7%. Same open items as WD-101: the consumer registry has no `typescript/ct-compare` row (the checked-in registry lists only `python/ct-compare` and `ca-assurance-report` pins it), so BUILD validates the manifest without `--registry`; no sandbox arming (CAP-RT-1xx TypeScript/Node); no layer B (native) suites; no `warrant-corpus-conformance-v1` fixture. |

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
