# Portable Warrant implementation queue

**Authority:** current queue, status, receipts, and discoveries for the
portable Warrant work described by [VER04i](../VER04i-polyglot-warrant-evidence.md).
The main [VER-BACKLOG](../VER-BACKLOG.md) keeps the compact program index;
this shard owns operational detail. Historical rollout snapshots in VER04i
remain historical and must not be read as the current queue.

## Current queue

| Order | Work | State / dependency |
|---|---|---|
| WD-003 | Rust Warrant v0 corpus producer, independent Python semantic and digest oracle, byte-stable regeneration and falsification, consumer/build routing | PR #19177 includes the #19209 Warrant WX-D1 S1 main integration. Local corpus regeneration/oracle, source-routing, and Rust package checks pass after integration; exact integrated-head hosted checks are pending. The implementation is explicitly unsharded and does not execute a Python lane binding. The focused Linux Lua apt-retry routing fix is included. |
| WD-004a | Define the strict per-lane manifest schema and consumer registry | After WD-003 lands. Establish the manifest and consumer contract before report integration. |
| WD-004b | Generalize `ca-assurance-report` beyond `rust/`, with honest not-live rungs | After WD-004a. |
| Python L0 runner | Execute portable corpus cases against the Python binding, with sabotage, manifest, and report | After WD-004a and WD-004b; then continue the per-lane WD-1xx sequence. |

WX implementation continues under the existing WX owner and IDs; this shard
does not create a duplicate WX-001 owner. WX-H and WX-E prerequisites merged
in #19121. WX-D1 S1 (checker and ground evaluator) merged as #19209; the S0
errata PR #19204 remains open. D1 S2a evidence-generator/corpus translation is
still outstanding, so WD-003's ct-compare Warrant v0 corpus remains unchanged.
WX-F/G may proceed alongside the remaining D1 work, with sequence operations
that rely on D1 constructors following D1; D2 follows D1 and the F/G baseline,
then D3 follows D2 and the corresponding WIR lowering. The 13 open owner
questions remain in VER04k section 9.

## Receipts and status

- PR #19177 attempt 5: run 43052542252 (gate job 130885644344) passed at the old exact head `2ad2f78b3854b0869dfc532ca7d0d9b36e7f23ab`. The integration/rebase work has not been pushed or committed; merged-head checks remain pending. Do not treat this receipt as evidence for a newer head.
- The initial Lua failures were Ubuntu archive mirror/DNS resolution failures in the two Linux Lua setup paths. The fix routes both prerequisite installs through the existing retrying apt wrapper while preserving the package list and step timeouts. This is a targeted response to those observed failures, not evidence of a global Ubuntu archive outage.
- A transient macOS orientation timeout is tracked by the existing VER-CI001 owner; do not open a duplicate issue here.
- Windows TypeScript job 130798040371 reported `ETIMEDOUT` once. Record as an observation only; there is not enough evidence to assign a cause or a new owner.

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
