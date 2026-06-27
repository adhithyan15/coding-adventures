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

`POST /api/services/:domain/:service` accepts Home Assistant-style JSON targets
and dispatches through `SmartHomeRuntime::execute_command_tool`, so runtime
capability grants still decide whether a local API caller can mutate devices.
Targets can use either the D23 entity ids, such as `entity-light-1`, or the
Home Assistant-style aliases exposed in state attributes, such as
`light.entity_light_1`.

## Dependencies

- smart-home-core
- smart-home-runtime
- web-core

## Development

```bash
bash BUILD
```
