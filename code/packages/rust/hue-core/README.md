# hue-core

Philips Hue CLIP v2 resource and mapping primitives for the smart-home runtime.

This crate contains no network I/O. It gives later Hue client and integration
packages a typed surface for:

- Hue resource kinds and ids
- CLIP v2 resource paths
- event stream path constants
- structured Hue command intents
- Hue command planning from normalized D23 state deltas
- direct and grouped-light color-temperature command projection
- Hue command plans that retain generated requests and ignored capabilities for
  reconciliation telemetry, including projection-coverage summaries
- Hue command summaries for payload-free command planning and read-model
  telemetry, including light-surface, surface-breadth, and capability-breadth
  helper predicates
- Hue application registration requests and discovered-bridge pairing plans,
  including payload-free pairing-plan readiness summaries
- Hue application registration local-HTTP request plans, response parsing, and
  no-secret Vault handoff metadata and summaries for physical-presence pairing
- typed Hue bridge resources for paired bridge identity/health refresh
- typed Hue device resources and service references
- typed Hue grouped-light resources for room/zone aggregate lights
- typed Hue room, zone, and scene resources
- typed Hue motion and button resources for sensor/input surfaces
- discovery-to-`Bridge` projection
- Hue light/device-to-normalized-model projection
- Hue scene-to-normalized-`Scene` projection
- Hue motion/button-to-normalized-`Entity` projection
- Hue scene summaries and scene-set rollups for recall planning and read-model
  telemetry
- Hue light, grouped-light, motion, and button state update-to-`StateDelta`
  projection
- Hue state update summaries for read models and event-stream telemetry
- unified Hue state update streams and rollups across light, grouped-light,
  motion, and button resources, including mixed-surface, owner-coverage, and
  partial-state helper predicates
- Hue snapshot and scene desired-state values keyed by canonical D23 capability
  ids such as `light.on_off` and `light.brightness`
- Hue mDNS and cloud-fallback discovery observations normalized into D23
  `DiscoveryRecord` bridge candidates
- Hue discovery worker runs that bundle mDNS/cloud observations, per-source
  failures, and generic D23 worker metadata for runtime catalog ingest
- Hue discovery worker runs from generic mDNS scan results, preserving
  malformed datagram failures alongside valid bridge advertisements
- Hue discovery worker runs from aggregate mDNS worker scan reports, preserving
  interface-level scan failures without moving network I/O into Hue mapping code
- discovery-record-to-pairing-plan handoff for local physical-presence Hue
  pairing flows
- integration descriptor metadata and payload-free summaries for Chief of Staff
  discovery
- integration package summaries that join descriptor, command-flow, and local
  pairing readiness
- Hue package release-readiness summaries for catalog publish checks across
  worker, command-flow, local-pairing, event-stream, and physical-presence gates
- Hue package spec summaries for catalog/spec handoff checks across canonical
  identity, CLIP v2 endpoints, registration headers, and runtime model surface
- Hue package spec gap summaries that route blocked package specs to release,
  identity, CLIP v2 transport, or runtime model review
- Hue catalog package readiness summaries that roll package/spec, release,
  identity, transport, runtime model, and pairing-handoff gates into one
  payload-free catalog handoff check
- Hue catalog package gap summaries that route blocked catalog handoffs to
  spec, release, identity, transport/runtime, or pairing-handoff review
- Hue catalog/spec handoff summaries that count accepted catalog, spec,
  release, and runtime/pairing review gates
- Hue package publish-gate summaries that route blocked handoffs to catalog,
  release, or runtime/pairing review queues
- Hue package lifecycle summaries that count release, spec, catalog, handoff,
  and publish stages as one ordered package readiness view
- Hue package review-queue summaries that turn lifecycle stage blockers into
  release, spec, catalog, handoff, and publish acceptance queues
- Hue package acceptance summaries that combine lifecycle completion,
  review-queue clearance, and publish-gate readiness into one package
  acceptance gate
- Hue package release handoff summaries that turn accepted package state into
  final release/manual-review handoff checks
- Hue package release queue summaries that turn release handoff readiness into
  final queue entry checks for publish/release coordination
- Hue package release coordination summaries that combine release queue,
  handoff, acceptance, review, and publish gates for final coordination checks
- Hue package release dispatch summaries that turn coordinated release state
  into final dispatch readiness checks
- Hue package release operator summaries that turn dispatch readiness into
  final operator-facing release checks
- Hue package release audit summaries that turn operator readiness into final
  audit/signoff checks
- Hue package release signoff summaries that turn audit readiness into final
  package release signoff checks
- Hue package release closure summaries that turn signoff readiness into final
  package release closure checks
- Hue package release archive summaries that turn closure readiness into final
  package release archive checks
- Hue package release archive signoff summaries that turn archive readiness
  into final package release archive signoff checks
- Hue package release archive closure summaries that turn archive signoff
  readiness into final package release archive closure checks
- Hue package release archive handoff summaries that turn archive closure
  readiness into final package release archive handoff checks
- Hue package release archive dispatch summaries that turn archive handoff
  readiness into final package release archive dispatch checks
- Hue package release archive operator summaries that turn archive dispatch
  readiness into final operator-facing archive release checks
- Hue package release archive supervisor summaries that turn archive operator
  readiness into final supervised archive release checks
- Hue package release archive completion summaries that turn archive supervisor
  readiness into final archive completion checks
- Hue package release archive publication summaries that turn archive completion
  readiness into final archive publication checks
- Hue package release archive verification summaries that turn archive
  publication readiness into final archive verification checks
- Hue package release archive validation summaries that turn archive
  verification readiness into final archive validation checks
- Hue package release archive certification summaries that turn archive
  validation readiness into final archive certification checks
- Hue package release archive approval summaries that turn archive
  certification readiness into final archive approval checks
- Hue package release archive activation summaries that turn archive approval
  readiness into final archive activation checks
- Hue package release archive rollout summaries that turn archive activation
  readiness into final archive rollout checks
- Hue package release archive adoption summaries that turn archive rollout
  readiness into final archive adoption checks
- Hue package release archive acceptance summaries that turn archive adoption
  readiness into final archive acceptance checks
- Hue package release archive distribution summaries that turn archive
  acceptance readiness into final archive distribution checks
- Hue package release archive export summaries that turn archive distribution
  readiness into final archive export checks
- Hue package release archive import summaries that turn archive export
  readiness into final archive import checks
- Hue package release archive ingest summaries that turn archive import
  readiness into final archive ingest checks
- Hue package release archive load summaries that turn archive ingest
  readiness into final archive load checks
- Hue package release archive restore summaries that turn archive load
  readiness into final archive restore checks
- Hue package release archive recovery summaries that turn archive restore
  readiness into final archive recovery checks
- Hue package release archive replay summaries that turn archive recovery
  readiness into final archive replay checks
- Hue package release archive reconciliation summaries that turn archive replay
  readiness into final archive reconciliation checks
- Hue package release archive settlement summaries that turn archive
  reconciliation readiness into final archive settlement checks
- Hue package release archive finalization summaries that turn archive
  settlement readiness into final archive finalization checks
- Hue package release archive confirmation summaries that turn archive
  finalization readiness into final archive confirmation checks
- Hue package release archive attestation summaries that turn archive
  confirmation readiness into final archive attestation checks
- Hue package release archive evidence summaries that turn archive attestation
  readiness into final archive evidence checks
- Hue package release archive evidence ledger summaries that turn archive
  evidence readiness into ledger-level release archive evidence checks
- Hue package release readiness evidence summaries that combine package release
  readiness with release archive evidence ledger checks
- Hue package release evidence index summaries that group readiness, archive,
  closeout, and operations evidence into a compact release evidence index
- Hue package release archive notarization summaries that turn release evidence
  index readiness into final archive notarization checks

## Dependencies

- `serde_json`
- `smart-home-core`
- `smart-home-discovery`
- `smart-home-local-http`

## Development

```bash
bash BUILD
```
