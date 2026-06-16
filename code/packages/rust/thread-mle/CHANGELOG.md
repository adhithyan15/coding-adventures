# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-05-06

### Added

- Thread role, MLE command, TLV, scan-mask, and mode primitives.
- Payload-free MLE message and message-batch summaries for attach,
  parent-selection, status, unknown-command, and diagnostic TLV context.
- Typed MLE Status TLV helpers for validating and extracting raw status codes.
- MLE message parser/encoder and a deterministic parent/child attach-state
  skeleton for simulator-first Thread work.
- Typed Leader Data TLV helpers plus opaque Network Data extraction from MLE
  messages for diagnostics and simulator fixtures.
- Thread Network Data TLV parsing/encoding with stable-bit preservation and
  typed Prefix TLV projection for prefix diagnostics.
- Compact Thread Network Data summaries for top-level TLV, prefix, stability,
  routing, service, and unknown-TLV coverage.
- Thread Network Data readiness summaries for prefix, routing, stable-data,
  service/context, and unknown-TLV coverage checks.
- Thread Network Data TLV handoff summaries for stable TLV, routing TLV,
  service/context TLV, and unknown-TLV review gates.
- Typed Connectivity TLV helpers for route-cost, link-quality, active-router,
  and sleepy-end-device diagnostic fields.
- Thread diagnostic snapshots that combine neighbor, leader, connectivity,
  partition, and prefix state for D23-facing health reads.
- Thread supervisor action projections that classify diagnostic drift into
  stable repair intents for runtime supervisors.
- Neighbor table summaries for cheap runtime/read-model projections of parent,
  child, router, stale-neighbor, and parent-candidate state.
- Thread attach readiness summaries that combine MLE parent-selection traffic
  with neighbor parent/candidate state.
- Thread attach action summaries that turn attach readiness into
  parent-selection, response-wait, neighbor-refresh, status-review, and
  unknown-command queues.
- Thread attach completion summaries that combine attach action clearance with
  supervision-plan clearance for runtime handoff checks.
- Thread attach route-handoff summaries that combine attach completion with
  Network Data readiness, routing surface, and parent/router anchor checks.
- Thread attach route audit summaries that turn route-handoff readiness into
  final route audit checks.
- Thread attach route signoff summaries that turn route-audit readiness into
  final route signoff checks.
- Thread attach route completion summaries that turn route-signoff readiness
  into final route completion checks.
- Thread attach route publication summaries that turn route-completion readiness
  into final route publication checks.
- Thread attach route verification summaries that turn route-publication
  readiness into final route verification checks.
- Thread attach route validation summaries that turn route-verification
  readiness into final route validation checks.
- Thread attach route certification summaries that turn route-validation
  readiness into final route certification checks.
- Thread attach route approval summaries that turn route-certification
  readiness into final route approval checks.
- Thread attach route activation summaries that turn route-approval readiness
  into final route activation checks.
- Thread attach route rollout summaries that turn route-activation readiness
  into final route rollout checks.
- Thread attach route adoption summaries that turn route-rollout readiness
  into final route adoption checks.
- Thread attach route acceptance summaries that turn route-adoption readiness
  into final route acceptance checks.
- Thread attach route distribution summaries that turn route-acceptance
  readiness into final route distribution checks.
- Thread attach route export summaries that turn route-distribution readiness
  into final route export checks.
- Thread attach route import summaries that turn route-export readiness into
  final route import checks.
- Thread attach route ingest summaries that turn route-import readiness into
  final route ingest checks.
- Thread attach route load summaries that turn route-ingest readiness into
  final route load checks.
- Thread attach route restore summaries that turn route-load readiness into
  final route restore checks.
- Thread attach route recovery summaries that turn route-restore readiness into
  final route recovery checks.
- Thread attach route replay summaries that turn route-recovery readiness into
  final route replay checks.
- Neighbor table primitives for parent/child/router relationships, stale
  timeout expiry, link margin tracking, and parent-candidate selection.
