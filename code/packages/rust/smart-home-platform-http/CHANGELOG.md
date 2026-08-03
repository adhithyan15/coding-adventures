# Changelog

All notable changes to this package will be documented in this file.

## Unreleased

- Added `lan_tcp` bridge transport projection and request parsing.
- Added `lan_udp` bridge transport projection and request parsing.
- Added camera entity labels and Home Assistant-compatible `camera` domains to
  the local API projection.
- Added an operational browser dashboard for native migrated views, runtime
  inventory and health, automation definitions and audit, pairing sessions,
  state history, and command/authorization audit. The durable controller can
  load validated raw or applied dashboard manifests before binding.
- Added native dashboard-manifest and pairing-session list/detail routes.
- Added a restart-safe automation runtime to the production controller, a local
  schedule worker, and native definition, evaluation, dry-run, and audit API
  routes with atomic persistence rollback.
- Added a production `smart-home-local-controller` binary that restores the
  durable runtime store, uses a live clock, and atomically persists authorized
  desired-state and service mutations before returning success.
- Added persistence-failure rollback and local-folder restart tests so an HTTP
  mutation either survives a fresh store instance or remains unapplied.
- Added a Home Assistant-compatible read-only local API `web-core::WebApp` for
  config, state, services, and events over smart-home runtime snapshots.
- Added a local-controller smoke script export generated from the runtime smoke
  plan so fixture-controller launches can run the dashboard/API/readiness and
  command probes without hand-copying curls.
- Added a live runtime-backed API constructor with `POST /api/services/:domain/:service`
  dispatch through runtime command authorization and command results.
- Added a read-only command-authorization preview route and browser dashboard
  Auth actions so local-controller users can inspect runtime grants before
  dispatching light commands.
- Added a read-only desired-state authorization preview route and browser
  dashboard Auth actions so target set/clear controls expose runtime grant
  boundaries before mutating desired-state supervision targets.
- Added a read-only scene authorization preview route and browser dashboard
  Auth action so scene activation can show every runtime command grant before
  dispatching commands.
- Added a read-only service authorization preview route and browser dashboard
  Auth action so Home Assistant service calls can show their expanded command
  grants before dispatching commands.
- Added runtime audit-row links on event-log entries, command-result records,
  and authorization decisions so local-controller clients can pivot between
  command, event, grant, state-history, and policy detail routes without
  rebuilding endpoint paths.
- Added a controller handoff manifest route that groups the local-controller
  readiness evidence across repo HTTP, browser dashboard, fixture smoke,
  state/history/event, command/service/scene, and runtime authorization
  surfaces.
- Added browser dashboard action feedback that renders accepted command,
  scene, and desired-state mutation responses with command-result and
  desired-state readback links in the detail panel.
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
- Added a browser dashboard capability catalog panel with URL-backed
  capability id, commandability, and observability filters.
- Increased the fixture-controller/test HTTP server pending-write budget so the
  embedded dashboard shell is served over the repo TCP stack as it grows.
- Added URL-backed browser dashboard service-catalog controls for service name,
  capability, target entity, and target scene filters.
- Added URL-backed browser dashboard scene-catalog controls for scene scope and
  target entity filters.
- Added browser dashboard desired-state target controls for light on/off and
  brightness so the local controller can supervise intended state through the
  existing runtime-authorized native API.
- Added URL-backed browser dashboard desired-state filters for entity,
  requester, and capability scoping.
- Added browser dashboard command-result and authorization-decision audit
  panels backed by the existing local-controller audit routes.
- Added browser dashboard room, device, and bridge topology panels backed by
  the existing local-controller inventory routes.
- Added URL-backed browser dashboard topology filters for device bridge,
  manufacturer, health, and bridge integration, transport, and health.
- Added a dashboard-ready room detail route and matching browser room inspect
  action.
- Expanded the dashboard-ready room detail route with member devices, entities,
  and scenes.
- Added room-scoped entity/state filters and navigation links on room detail
  responses.
- Added room-scoped state-history, runtime event-log, and command-result audit
  filters with matching room detail links.
- Added a browser dashboard room filter that scopes inventory, state,
  state-history, event-log, and command-result panels through native room-aware
  routes.
- Added entity-centered dashboard links for state, desired state, state history,
  entity-scoped events, and owning bridge command-result audit trails, plus an
  entity-alias filter for runtime event-log reads.
- Added a browser dashboard detail panel that renders inspected native route
  responses with endpoint, HTTP status, and formatted JSON.
- Added a browser dashboard event-log panel backed by the existing native
  runtime event stream route.
- Added browser dashboard detail actions for history, event-log,
  command-result, and authorization audit rows.
- Added browser dashboard detail actions for entity state/registry records,
  services, scenes, devices, and bridges.
- Added browser dashboard filters for entity state, event-log, command-result,
  and authorization audit views, plus local text search across dashboard rows.
- Added browser dashboard filter permalinks that mirror search, room, state,
  event, command, and authorization filters into URL query parameters.
- Added dashboard-ready capability grant inventory and detail routes for
  inspecting runtime authorization boundaries.
- Added a browser dashboard capability-grants panel with status, scope, and
  principal filters plus authorization-row links to active principal grants.
- Added a browser dashboard authorization principal filter so decision-log
  views can be scoped to one local API caller from the dashboard URL.
- Added browser dashboard command-result identity filters for command id, bridge
  id, and correlation id so audit views can be reopened from URL state.
- Added a browser dashboard activity entity filter for scoping state history
  and runtime event panels through existing entity-aware activity routes.
- Added a browser dashboard history event-type filter for reopening state
  history views scoped to one device event class from URL state.
- Added state-history time-window filters so smart-home `from_ms`/`to_ms` and
  Home Assistant period `start_time`/`end_time` requests share the same route.
- Added state-history bridge filtering so `bridge_id` route-catalog entries are
  enforced by the runtime history query.
- Added received-time upper-bound filtering for state history with
  `received_at_or_before_ms`.
- Added observed-time upper-bound filtering for state history with
  `observed_at_or_before_ms`.
- Added sequence-window filtering for runtime event-log and command-result
  audit routes with `to_sequence`.
- Added browser dashboard sequence-window filters for runtime event-log and
  command-result audit views backed by URL state.
- Added browser dashboard state-history bridge and observed/received time-window
  filters backed by URL state.
- Added browser dashboard API catalog filters for route surface, method,
  category, mutation, and runtime authorization.
- Added fixture-controller launch help, smoke-test URLs, and example-level tests
  to keep the local controller startup path usable.
- Added a machine-readable local-controller smoke plan route that lists safe
  HTTP probes plus a runtime-authorized command probe for fixture-controller
  verification.
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
