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
| Python L0 runner | Execute portable corpus cases against the Python binding, with sabotage, manifest, and report | After WD-004b1/WD-004b2; then continue the per-lane WD-1xx sequence. |

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
