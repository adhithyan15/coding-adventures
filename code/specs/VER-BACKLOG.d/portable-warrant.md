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
| WD-004a | Define the strict per-lane manifest schema, planned consumer registry, and corpus structural validator | Active next slice. Spec/schema first; then isolated `ca-assurance-report` parser/tests and honest source/build routing. No real lane execution, report output, or proof credit. |
| WD-004b | Generalize `ca-assurance-report` beyond `rust/`, with honest not-live rungs | After WD-004a. |
| Python L0 runner | Execute portable corpus cases against the Python binding, with sabotage, manifest, and report | After WD-004a and WD-004b; then continue the per-lane WD-1xx sequence. |

WX implementation continues under the existing WX-001 owner and IDs; this
shard does not create a duplicate feature owner. WX-H/E prerequisites merged
in #19121, and WX-D1 S1 (checker and ground evaluator) merged as #19209. The S0
errata PR #19204 remains open. D1 S2a evidence-generator/corpus translation is
still outstanding, so WD-003's ct-compare Warrant v0 corpus remains unchanged.
The tracked missing language features remain WX-D (bounded sequence/model
operations, then higher-order list operations and bounded unrolling), WX-F
(stateful sequence drivers), and WX-G (`inout`/post-state `ensures`), followed
by D2 and D3 in the existing VER04k order. These close the current
unrolled-model and stateful/inout workarounds; they are feature work, separate
from extending WD-003's portable corpus profile. PR #19227 is a separate
wasm-leb128 adopter that changes legacy Rust assurance behavior, not a WD-004a
consumer-registry implementation.

## Receipts and status

- PR #19177 exact merge receipt: integrated head `7d2c92409b15fb19ea82a6cb892eaed44067975c`, run 43144592253 attempt 1, gate job 130981923321, merged at 2026-11-12T05:34:56Z as `d746f3b16bbbc78781ade0987abd88bc5a953e37`. The gate waited queued for about 36 minutes after all substantive jobs passed, then completed successfully; no retry was needed. Do not use earlier-head successes as evidence for this receipt.
- The initial Lua failures were Ubuntu archive mirror/DNS resolution failures in the two Linux Lua setup paths. The fix routes both prerequisite installs through the existing retrying apt wrapper while preserving the package list and step timeouts. This is a targeted response to those observed failures, not evidence of a global Ubuntu archive outage.
- A transient macOS orientation timeout is tracked by the existing VER-CI001 owner; do not open a duplicate issue here.
- Windows TypeScript job 130798040371 reported `ETIMEDOUT` once. Record as an observation only; there is not enough evidence to assign a cause or a new owner.
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
