# smart-home-platform-http

Home Assistant-compatible local HTTP API routes for the D23 smart-home platform.

This crate builds a `web-core::WebApp` over the repo's own embeddable HTTP
server stack. It does not open sockets by itself and it does not bypass the
runtime model. Instead, it projects a `SmartHomeRuntime` registry snapshot into
stable local API responses for:

- `/`
- `/dashboard`
- `/smart-home`
- `/api/`
- `/api/config`
- `/api/states`
- `/api/states/:entity_id`
- `/api/services`
- `/api/services/:domain/:service`
- `/api/events`
- `/api/history/period`
- `/api/history/period/:start_time`
- `POST /api/states/:entity_id`
- `/api/smart_home/runtime`
- `/api/smart_home/health`
- `/api/smart_home/readiness`
- `/api/smart_home/controller_handoff`
- `/api/smart_home/dashboard`
- `/api/smart_home/dashboard_manifest`
- `/api/smart_home/bootstrap`
- `/api/smart_home/smoke`
- `/api/smart_home/smoke_script`
- `/api/smart_home/api`
- `/api/smart_home/states`
- `/api/smart_home/states/:entity_id`
- `/api/smart_home/services`
- `/api/smart_home/services/:domain/:service`
- `/api/smart_home/entities`
- `/api/smart_home/entities/:entity_id`
- `/api/smart_home/capabilities`
- `/api/smart_home/devices`
- `/api/smart_home/devices/:device_id`
- `/api/smart_home/bridges`
- `/api/smart_home/bridges/:bridge_id`
- `/api/smart_home/pairing_sessions`
- `/api/smart_home/pairing_sessions/:session_id`
- `/api/smart_home/rooms`
- `/api/smart_home/rooms/:room_id`
- `/api/smart_home/scenes`
- `/api/smart_home/scenes/:scene_id`
- `/api/smart_home/events`
- `/api/smart_home/events/:sequence`
- `/api/smart_home/command_results`
- `/api/smart_home/command_results/:command_id`
- `/api/smart_home/capability_grants`
- `/api/smart_home/capability_grants/:grant_id`
- `/api/smart_home/authorization_decisions`
- `/api/smart_home/command_authorization`
- `/api/smart_home/desired_state_authorization`
- `/api/smart_home/scene_authorization`
- `/api/smart_home/service_authorization/:domain/:service`
- `/api/smart_home/authorization_decisions/:decision_index`
- `/api/smart_home/desired_states`
- `POST /api/smart_home/desired_states/:entity_id`
- `DELETE /api/smart_home/desired_states/:entity_id`
- `/api/smart_home/state_history`
- `/api/smart_home/state_history/:event_id`
- `/api/smart_home/automations`
- `/api/smart_home/automation_audit`

`POST /api/services/:domain/:service` accepts Home Assistant-style JSON targets
and dispatches through `SmartHomeRuntime::execute_command_tool`, so runtime
capability grants still decide whether a local API caller can mutate devices.
Targets can use either the D23 entity ids, such as `entity-light-1`, or the
Home Assistant-style aliases exposed in state attributes, such as
`light.entity_light_1`.

Local controller state writes are represented as runtime desired-state targets,
not observed-state rewrites. `POST /api/smart_home/desired_states/:entity_id`
accepts a `desired_state` capability map and `POST /api/states/:entity_id`
accepts a small Home Assistant-style state body for lights, locks, and climate
entities. Both routes call `SmartHomeRuntime::execute_set_desired_state_tool`;
`DELETE /api/smart_home/desired_states/:entity_id` calls the matching clear
tool.

The `GET /api/smart_home/*` routes expose dashboard-ready read models for the
same runtime: pending-work snapshot counts, entity and capability registry
records, a compact health probe, a capability catalog grouped across entities,
device and bridge inventory, room topology summaries with member detail lookups
and room-scoped links, a readiness checklist with actionable links, a controller
handoff manifest that groups platform readiness evidence across the repo HTTP
stack, browser dashboard, fixture controller, state/history/events,
commands/services/scenes, and runtime authorization boundaries, a dashboard
overview, a bootstrap payload that composes startup links, route discovery, state
gaps, and recent audit summaries, an API route catalog with
surface/method/authorization filters, checkpointed event-log entries with detail
lookups, a native service catalog for command affordances and Home Assistant
target aliases, a native current-state registry with confidence/source/staleness
filters plus room filters and detail lookups, a scene registry with room/action
projections, command-result audit records with
command, bridge, correlation, room, status, and sort filters, indexed
authorization decisions with principal, outcome, and sort filters, read-only
command-authorization previews that evaluate the local API principal's tool and
device-command grants without dispatching commands, read-only desired-state
authorization previews that evaluate set/clear tool grants without mutating
targets, capability grant
inventory/detail routes with principal, status, scope, capability, entity, and
sort filters, and desired-state supervision targets.
State-history reads expose registry-backed device events with Home Assistant
entity aliases, room filters, state deltas, timestamp filters, and event-id
detail lookups; the Home Assistant-style history route accepts
`filter_entity_id` and room filters.
Runtime event-log reads accept Home Assistant entity aliases through
`entity_id`, letting dashboard clients drill from an entity into its device,
state-expiration, and desired-state drift events.
Runtime event-log entries, command-result records, and authorization decisions
also include native `links` objects for their related command, event-window,
state-history, entity, bridge, principal-grant, and authorization preview
routes, so local-controller clients can pivot across audit surfaces without
reconstructing endpoint paths.

`GET /api/smart_home/smoke` exposes a machine-readable local-controller smoke
plan with safe GET probes, command and desired-state authorization previews,
and a single runtime-authorized Home Assistant-style command probe. It lets
scripts discover the dashboard/API/readiness checks and the exact request body
to use for fixture-controller verification without scraping launch text.
`GET /api/smart_home/smoke_script` renders that same plan as a copy-pasteable
`sh` script using `curl`; set `SMART_HOME_BASE_URL` or `CURL` to override the
defaults when the fixture controller runs on a custom address.

The browser routes serve an embedded operational dashboard over the same
`web-core::WebApp`. The shell loads bootstrap, readiness, native dashboard
manifests, state, scene,
desired-state, room, device, bridge, state-history, command-result audit,
runtime event-log, automation definitions and audit, pairing sessions,
authorization audit, capability-grant inventory, service
catalog, capability catalog, API catalog, and audit summary data from the
native API routes and sends light on/off, light brightness, scene, and
desired-state set/clear actions through the existing Home
Assistant-compatible and native service endpoints, preserving runtime
authorization. Entity action cards expose command-authorization preview buttons
for light on/off and brightness controls before dispatch and desired-state
authorization preview buttons before target set/clear actions. After command,
scene, or desired-state mutations, the shell renders the accepted response in
the detail panel with command-result, correlation, current-state, desired-state,
or history readback links as applicable. Entity, service, scene, device, bridge,
history, event-log, command-result, authorization, capability catalog, and
capability-grant
rows/cards expose read-only detail buttons that fetch the matching native detail
route and show formatted JSON plus the endpoint/status in a dedicated detail
panel. Authorization rows also link to the active grants for their principal so
denied/allowed decisions can be checked against the runtime policy boundary.
Entity cards and state-gap rows also link to their current state, registry
detail, desired-state target, state history, entity-scoped runtime events, and
owning bridge command-result audit trail. The browser shell also exposes
filters for room, entity domain/state/control status, device
bridge/manufacturer/health, bridge integration/transport/health, capability
catalog capability/commandability/observability, desired-state
entity/requester/capability, scene catalog scope/entity, service catalog
service/capability/target-entity/target-scene, API catalog
surface/method/category/mutation/authorization, runtime event kind/activity
entity, history event type, history bridge and observed/received time windows,
command-result status/id/bridge/correlation, runtime event-log sequence
windows, command-result sequence windows, authorization outcome/principal, and
capability-grant status/scope/principal,
with server-backed room and topology scoping across inventory, state, history,
event-log, and command-result panels plus server-backed capability catalog,
desired-state supervision, scene catalog, service catalog, activity/history,
command audit, authorization, and capability-grant scoping and local text
search across the rendered dashboard rows. Those filter selections are mirrored
into URL query parameters and restored on page load or browser navigation, so
local-controller room, topology, capability, desired-state, scene, service,
activity, history, audit, and grant-boundary views can be shared or reopened
directly.
State-history routes also accept numeric observed-time windows through
`from_ms`/`to_ms` or
`observed_at_or_after_ms`/`observed_at_or_before_ms` on
`/api/smart_home/state_history` and `start_time`/`end_time` on Home Assistant
period routes, and can scope runtime history to a single bridge through
`bridge_id`. Runtime history can also be bounded by ingestion time with
`received_at_or_after_ms` and `received_at_or_before_ms`.
Runtime event-log and command-result audit routes accept sequence windows with
`from_sequence` and `to_sequence`, allowing local-controller activity panels to
reopen bounded replay slices without fetching the full audit tail.

## Dependencies

- embeddable-http-server
- smart-home-dashboard-core
- smart-home-core
- smart-home-runtime
- smart-home-runtime-store
- storage-local-folder
- tcp-runtime
- web-core

## Development

```bash
bash BUILD
```

## Durable Local Controller

Run the production local controller against a durable runtime folder:

```bash
cargo run -p smart-home-platform-http --bin smart-home-local-controller -- \
  --data-dir "$HOME/.coding-adventures/smart-home" \
  --dashboard-manifest "$HOME/.coding-adventures/smart-home/dashboards.json" \
  --bind 127.0.0.1:8123
```

`SMART_HOME_DATA_DIR` supplies the data folder when `--data-dir` is omitted.
`SMART_HOME_DASHBOARD_MANIFEST` supplies the applied migration artifact or raw
native manifest when `--dashboard-manifest` is omitted. Invalid and dry-run
artifacts are rejected before the controller binds.
The controller loads the latest `smart-home-runtime-store` snapshot before it
binds, restores automation definitions, consumed trigger occurrences, and
automation audit, uses wall-clock request timestamps, and saves the local API
capability grant. A local worker evaluates schedule triggers every 500 ms.
Accepted desired-state, service, automation-definition, and automation-execution
mutations are persisted synchronously before the API returns success. A failed
write restores the exact pre-request runtime and automation engine and returns
HTTP 503, so clients never observe a successful mutation that only exists in
memory.

The native automation surface is:

- `GET /api/smart_home/automations`
- `POST /api/smart_home/automations`
- `POST /api/smart_home/automations/evaluate`
- `GET /api/smart_home/automation_audit`

The evaluation endpoint accepts `{"dry_run":true}` for a schedule preview or
`{"dry_run":false,"event":{...}}` for a normalized device-event trigger.

The controller can start with an empty folder. Discovery, pairing, and
integration owners can populate the same durable runtime store before launch;
on later launches the API serves the restored topology, state, event, command,
pairing, desired-state, authorization, automation definitions, idempotency
state, and automation audit.

## Fixture Controller

Run the deterministic Hue fixture controller locally:

```bash
cargo run -p smart-home-platform-http --example hue_fixture_controller
```

The controller defaults to `127.0.0.1:8123`, accepts either a positional bind
address or `--bind`, and prints dashboard, health, readiness, API catalog,
smoke-test URLs, and a generated smoke-script URL after the repo HTTP server
binds. It raises the repo TCP
runtime pending-write budget for the embedded dashboard response while leaving
request limits at the default HTTP-server settings:

```bash
cargo run -p smart-home-platform-http --example hue_fixture_controller -- --bind 127.0.0.1:8123
cargo run -p smart-home-platform-http --example hue_fixture_controller -- --help
```

Then query it from another shell:

```bash
curl http://127.0.0.1:8123/
curl http://127.0.0.1:8123/api/
curl http://127.0.0.1:8123/api/states
curl http://127.0.0.1:8123/api/smart_home/health
curl http://127.0.0.1:8123/api/smart_home/readiness
curl http://127.0.0.1:8123/api/smart_home/controller_handoff
curl http://127.0.0.1:8123/api/smart_home/dashboard
curl http://127.0.0.1:8123/api/smart_home/dashboard_manifest
curl http://127.0.0.1:8123/api/smart_home/bootstrap
curl http://127.0.0.1:8123/api/smart_home/smoke
curl http://127.0.0.1:8123/api/smart_home/smoke_script
SMART_HOME_BASE_URL=http://127.0.0.1:8123 sh -c "$(curl -fsS http://127.0.0.1:8123/api/smart_home/smoke_script)"
curl 'http://127.0.0.1:8123/api/smart_home/api?surface=home_assistant&method=POST'
curl 'http://127.0.0.1:8123/api/smart_home/api?mutating=true&authorized=true'
curl 'http://127.0.0.1:8123/api/smart_home/states?domain=light&stale=true'
curl 'http://127.0.0.1:8123/api/smart_home/states?room_id=kitchen&stale=true'
curl 'http://127.0.0.1:8123/api/smart_home/states/light.entity_light_1'
curl 'http://127.0.0.1:8123/api/smart_home/services?domain=light'
curl 'http://127.0.0.1:8123/api/smart_home/services?domain=light&service=turn_on&entity_id=light.entity_light_1'
curl 'http://127.0.0.1:8123/api/smart_home/services?capability_id=light.on_off'
curl 'http://127.0.0.1:8123/api/smart_home/services/light/turn_on'
curl 'http://127.0.0.1:8123/api/smart_home/entities?domain=light&commandable=true'
curl 'http://127.0.0.1:8123/api/smart_home/entities?room_id=kitchen'
curl 'http://127.0.0.1:8123/api/smart_home/capabilities?domain=light&commandable=true'
curl 'http://127.0.0.1:8123/api/smart_home/devices?room_id=kitchen&health=online'
curl 'http://127.0.0.1:8123/api/smart_home/bridges?integration_id=hue&transport=lan_http'
curl 'http://127.0.0.1:8123/api/smart_home/pairing_sessions?status=pending_user_presence'
curl 'http://127.0.0.1:8123/api/smart_home/rooms?sort=scene_count&state_gaps_only=true'
curl 'http://127.0.0.1:8123/api/smart_home/rooms/kitchen'
curl 'http://127.0.0.1:8123/api/smart_home/scenes?room_id=kitchen&scope=room'
curl 'http://127.0.0.1:8123/api/smart_home/scenes/scene.scene_kitchen_bright'
curl 'http://127.0.0.1:8123/api/smart_home/scene_authorization?scene_id=scene.scene_kitchen_bright'
curl 'http://127.0.0.1:8123/api/smart_home/events?limit=12'
curl 'http://127.0.0.1:8123/api/smart_home/events?room_id=kitchen&limit=12'
curl 'http://127.0.0.1:8123/api/smart_home/events?entity_id=light.entity_light_1&limit=12'
curl 'http://127.0.0.1:8123/api/smart_home/events/0'
curl 'http://127.0.0.1:8123/api/smart_home/command_results?limit=10'
curl 'http://127.0.0.1:8123/api/smart_home/command_results?room_id=kitchen&limit=10'
curl 'http://127.0.0.1:8123/api/smart_home/command_results?bridge_id=bridge-1'
curl 'http://127.0.0.1:8123/api/smart_home/command_results?sort=status_then_newest'
curl 'http://127.0.0.1:8123/api/smart_home/command_authorization?entity_id=light.entity_light_1&command_type=turn_on'
curl 'http://127.0.0.1:8123/api/smart_home/desired_state_authorization?entity_id=light.entity_light_1&operation=set'
curl 'http://127.0.0.1:8123/api/smart_home/desired_state_authorization?entity_id=light.entity_light_1&operation=clear'
curl 'http://127.0.0.1:8123/api/smart_home/service_authorization/light/turn_on?entity_id=light.entity_light_1&brightness_pct=75'
curl 'http://127.0.0.1:8123/api/smart_home/capability_grants?principal_id=agent:home-assistant-local-api&status=active'
curl 'http://127.0.0.1:8123/api/smart_home/capability_grants/grant:agent:home-assistant-local-api:local-api-full-access'
curl 'http://127.0.0.1:8123/api/smart_home/authorization_decisions?principal_id=agent:home-assistant-local-api&sort=oldest_first'
curl 'http://127.0.0.1:8123/api/smart_home/authorization_decisions/0'
curl 'http://127.0.0.1:8123/api/smart_home/state_history?entity_id=light.entity_light_1'
curl 'http://127.0.0.1:8123/api/smart_home/state_history?room_id=kitchen'
curl 'http://127.0.0.1:8123/api/smart_home/state_history/event-light-1-on'
curl 'http://127.0.0.1:8123/api/history/period?filter_entity_id=light.entity_light_1'
curl 'http://127.0.0.1:8123/api/history/period?room_id=kitchen'
curl -X POST http://127.0.0.1:8123/api/services/light/turn_on \
  -H 'Content-Type: application/json' \
  -d '{"entity_id":"light.entity_light_1","brightness_pct":75}'
curl -X POST http://127.0.0.1:8123/api/smart_home/desired_states/light.entity_light_1 \
  -H 'Content-Type: application/json' \
  -d '{"desired_state":{"light.on_off":true,"light.brightness":80},"requested_by":"agent:dashboard"}'
curl -X POST http://127.0.0.1:8123/api/states/light.entity_light_1 \
  -H 'Content-Type: application/json' \
  -d '{"state":"on","attributes":{"brightness":191}}'
curl -X DELETE http://127.0.0.1:8123/api/smart_home/desired_states/light.entity_light_1
```
