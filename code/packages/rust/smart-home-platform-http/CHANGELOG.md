# Changelog

All notable changes to this package will be documented in this file.

## Unreleased

- Added a Home Assistant-compatible read-only local API `web-core::WebApp` for
  config, state, services, and events over smart-home runtime snapshots.
- Added a live runtime-backed API constructor with `POST /api/services/:domain/:service`
  dispatch through runtime command authorization and command results.
- Added dashboard-ready runtime read routes for snapshot pending work,
  entity/capability registry records, room topology summaries, event-log
  replay, command-result audit, authorization-decision audit, and desired-state
  supervision targets.
- Added detail lookups for runtime event-log entries and command-result audit
  records so dashboard clients can drill into a specific audit row.
- Added command-result audit filters for command id, bridge id, and correlation
  id.
- Added command-result and authorization-decision audit sort controls, plus
  authorization-decision principal filtering for dashboard audit views.
- Added indexed authorization-decision audit rows and a matching detail lookup
  for dashboard drill-down flows.
- Added a state-history event detail lookup for registry-backed event drill-down
  by event id.
- Added a dashboard overview route that composes runtime health, topology,
  inventory, desired-state, and audit summaries for local controllers.
- Added an embedded browser dashboard shell served by the repo HTTP stack for
  fixture-controller startup, readiness, state, and light command flows.
- Extended the embedded dashboard shell with scene activation,
  desired-state clearing, and recent state-history previews over existing
  runtime-authorized API routes.
- Added browser dashboard light brightness controls that discover commandable
  ranged capabilities and dispatch through the existing `set_brightness`
  service path.
- Added browser dashboard service and authorized API catalog panels backed by
  the native local-controller discovery routes.
- Added browser dashboard desired-state target controls for light on/off and
  brightness so the local controller can supervise intended state through the
  existing runtime-authorized native API.
- Added a native readiness checklist route with actionable links for registry,
  topology, state coverage, event bus, discovery, supervisor, authorization,
  and desired-state checks.
- Added a dashboard bootstrap route that bundles startup links, API discovery,
  state-gap records, and recent audit summaries for local controllers.
- Added a dashboard-ready API route catalog with surface, method, mutating, and
  authorization filters for local-controller clients.
- Added a native service catalog route with command affordances, Home
  Assistant target aliases, and service detail lookups for dashboards.
- Added native current-state registry routes with state confidence, source, and
  staleness filters plus entity detail lookups.
- Added native scene registry routes with Home Assistant aliases, room/action
  projections, metadata, and detail lookups.
- Added a compact runtime health probe for local-controller readiness,
  pending-work, topology, and event-bus status checks.
- Added a dashboard-ready capability catalog route that groups entity
  capabilities, Home Assistant aliases, rooms, devices, and service affordances.
- Added device and bridge inventory routes with platform topology rollups for
  local-controller dashboards.
- Added a state-history read route over registry-backed device events with
  entity-alias, event-type, and timestamp filtering.
- Added Home Assistant-style `/api/history/period` routes backed by the same
  runtime state-history projection.
- Added runtime-authorized desired-state set and clear routes, including a
  Home Assistant-style `POST /api/states/:entity_id` controller surface that
  creates desired targets instead of rewriting observed state.
- Added a Hue fixture controller example that serves the API through the repo
  HTTP server for manual local smoke tests.
