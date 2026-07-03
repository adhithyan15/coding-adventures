# smart-home-testkit

Deterministic fixtures and fake streams for D23 smart-home runtime and
integration tests.

This crate does not open sockets, touch radios, read files, or call cloud APIs.
It gives future Hue, MQTT, Zigbee, Z-Wave, Thread, Matter, and runtime packages
a shared way to build:

- normalized bridge/device/entity/scene fixtures
- normalized Hue discovery records, built through `hue-core` mDNS mapping, and
  discovery-seeded runtime fixtures
- deterministic Hue mDNS scan, scan-report, and discovery worker-run fixtures
  that feed the runtime discovery catalog without opening network sockets
- deterministic Hue discovery worker schedules that exercise runtime due-run
  planning and scheduled ingest without opening network sockets
- scripted mDNS worker scan executors that run runtime-produced scan plans into
  deterministic reports without opening network sockets
- registry seeding helpers for installing fixture records into
  `smart-home-registry`
- confirmed, stale, and optimistic state snapshots
- deterministic device events
- scripted fake event streams with disconnect and gap markers
- non-consuming scripted fake event stream summaries for supervision assertions
- Hue SSE event-stream specs, connected stream states, and drivers that apply
  scripts to both event-stream supervision state and `smart-home-runtime`
- fake command buses with queued command/result pairs
- fake local HTTP responses that can match planned requests without sockets
- a deterministic Hue pairing fixture path from fake local HTTP response to
  runtime pairing completion without raw secrets in audit metadata
- non-consuming fake local HTTP server summaries for response-shape assertions
- fake MQTT broker publications with retained-message and metadata markers
- non-consuming fake MQTT broker summaries for publication-shape assertions
- scripted MQTT subscriptions with delivery matching by topic filter and QoS
- read-only fake local HTTP response queries by method, URL, status, metadata,
  observation time, sort, and limit
- read-only fake MQTT broker queries by topic, prefix, retained flag, metadata,
  observation time, sort, and limit
- simple logical clocks for freshness and supervision tests

## Dependencies

- `smart-home-core`
- `hue-core`
- `smart-home-discovery`
- `smart-home-event-streams`
- `smart-home-local-http`
- `smart-home-registry`
- `smart-home-runtime`

## Development

```bash
bash BUILD
```
