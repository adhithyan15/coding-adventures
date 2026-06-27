# smart-home-platform-http

Home Assistant-compatible local HTTP API routes for the D23 smart-home platform.

This crate builds a `web-core::WebApp` over the repo's own embeddable HTTP
server stack. It does not open sockets by itself and it does not bypass the
runtime model. Instead, it projects a `SmartHomeRuntime` registry snapshot into
stable local API responses for:

- `/api/config`
- `/api/states`
- `/api/states/:entity_id`
- `/api/services`
- `/api/events`

Mutation routes such as service calls will be wired through runtime command and
desired-state paths in later slices.

## Dependencies

- smart-home-core
- smart-home-runtime
- web-core

## Development

```bash
bash BUILD
```
