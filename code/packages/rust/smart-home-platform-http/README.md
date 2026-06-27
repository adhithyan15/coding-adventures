# smart-home-platform-http

Home Assistant-compatible local HTTP API routes for the D23 smart-home platform.

This crate builds a `web-core::WebApp` over the repo's own embeddable HTTP
server stack. It does not open sockets by itself and it does not bypass the
runtime model. Instead, it projects a `SmartHomeRuntime` registry snapshot into
stable local API responses for:

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
- `/api/smart_home/entities`
- `/api/smart_home/entities/:entity_id`
- `/api/smart_home/rooms`
- `/api/smart_home/events`
- `/api/smart_home/command_results`
- `/api/smart_home/authorization_decisions`
- `/api/smart_home/desired_states`
- `POST /api/smart_home/desired_states/:entity_id`
- `DELETE /api/smart_home/desired_states/:entity_id`
- `/api/smart_home/state_history`

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
records, room topology summaries, checkpointed event-log entries,
command-result audit records, authorization decisions, and desired-state
supervision targets. State-history reads expose registry-backed device events
with Home Assistant entity aliases, state deltas, and timestamp filters; the
Home Assistant-style history route accepts `filter_entity_id`.

## Dependencies

- smart-home-core
- smart-home-runtime
- web-core

## Development

```bash
bash BUILD
```

## Fixture Controller

Run the deterministic Hue fixture controller locally:

```bash
cargo run -p smart-home-platform-http --example hue_fixture_controller -- 127.0.0.1:8123
```

Then query it from another shell:

```bash
curl http://127.0.0.1:8123/api/
curl http://127.0.0.1:8123/api/states
curl 'http://127.0.0.1:8123/api/smart_home/entities?domain=light&commandable=true'
curl 'http://127.0.0.1:8123/api/smart_home/rooms?sort=scene_count&state_gaps_only=true'
curl 'http://127.0.0.1:8123/api/smart_home/command_results?limit=10'
curl 'http://127.0.0.1:8123/api/smart_home/state_history?entity_id=light.entity_light_1'
curl 'http://127.0.0.1:8123/api/history/period?filter_entity_id=light.entity_light_1'
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
