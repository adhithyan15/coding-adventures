# thread-mle

Thread Mesh Link Establishment roles, TLVs, and attach-state primitives.

This crate starts the D27 Thread control-plane layer above 6LoWPAN:

- device role model
- MLE command ids
- common MLE TLV ids
- MLE message/TLV parsing and encoding
- payload-free MLE message and message-batch summaries for attach,
  parent-selection, status, unknown-command, and diagnostic TLV context
- scan-mask and mode bit helpers
- typed Status TLV helpers
- typed Leader Data TLV helpers and opaque Network Data extraction
- Thread Network Data TLV parsing/encoding with stable-bit preservation
- typed Prefix TLV projection with nested Network Data sub-TLVs
- compact Network Data summaries for top-level TLV, prefix, stability, routing,
  service, and unknown-TLV coverage
- Thread Network Data readiness summaries for prefix, routing, stable-data,
  service/context, and unknown-TLV coverage checks
- Thread Network Data TLV handoff summaries for stable TLV, routing TLV,
  service/context TLV, and unknown-TLV review gates
- typed Connectivity TLV helpers for route-cost and active-router diagnostics
- Thread diagnostic snapshots that combine neighbor health with leader,
  connectivity, partition, and prefix data from MLE messages
- supervisor action projections that turn diagnostic drift into stable repair
  intents for runtime supervisors
- neighbor table summaries for cheap runtime/read-model projections of parent,
  child, router, stale-neighbor, and parent-candidate state
- attach readiness summaries that combine MLE parent-selection traffic with
  neighbor parent/candidate state
- attach action summaries that turn attach readiness into parent-selection,
  response-wait, neighbor-refresh, status-review, and unknown-command queues
- attach completion summaries that combine attach action clearance with
  supervision-plan clearance for runtime handoff checks
- attach route-handoff summaries that combine attach completion with Thread
  Network Data readiness and routing-anchor checks
- attach route audit summaries that turn route-handoff readiness into final
  route audit checks
- attach route signoff summaries that turn route-audit readiness into final
  route signoff checks
- attach route completion summaries that turn route-signoff readiness into
  final route completion checks
- attach route publication summaries that turn route-completion readiness into
  final route publication checks
- attach route verification summaries that turn route-publication readiness into
  final route verification checks
- attach route validation summaries that turn route-verification readiness into
  final route validation checks
- attach route certification summaries that turn route-validation readiness into
  final route certification checks
- attach route approval summaries that turn route-certification readiness into
  final route approval checks
- attach route activation summaries that turn route-approval readiness into
  final route activation checks
- attach route rollout summaries that turn route-activation readiness into
  final route rollout checks
- attach route adoption summaries that turn route-rollout readiness into
  final route adoption checks
- attach route acceptance summaries that turn route-adoption readiness into
  final route acceptance checks
- attach route distribution summaries that turn route-acceptance readiness into
  final route distribution checks
- attach route export summaries that turn route-distribution readiness into
  final route export checks
- attach route import summaries that turn route-export readiness into final
  route import checks
- attach route ingest summaries that turn route-import readiness into final
  route ingest checks
- attach route load summaries that turn route-ingest readiness into final route
  load checks
- attach route restore summaries that turn route-load readiness into final
  route restore checks
- attach route recovery summaries that turn route-restore readiness into final
  route recovery checks
- attach route replay summaries that turn route-recovery readiness into final
  route replay checks
- deterministic parent/child attach-state skeleton
- neighbor table primitives for parent/child/router relationships, link margin,
  timeout freshness, and parent-candidate selection

It does not yet implement UDP, CoAP, DTLS, commissioning, border-router routing
policy, or real radio behavior.

## Dependencies

- `sixlowpan`

## Development

```bash
bash BUILD
```
