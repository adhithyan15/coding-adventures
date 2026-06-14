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

## Dependencies

- `serde_json`
- `smart-home-core`
- `smart-home-discovery`
- `smart-home-local-http`

## Development

```bash
bash BUILD
```
