# smart-home-home-assistant-definitions

`smart-home-home-assistant-definitions` enriches a reviewed Home Assistant
topology export with live scene and automation definitions. It is the second
stage after `smart-home-home-assistant-export` and its output remains directly
readable by `smart-home-home-assistant-migration`.

The collector uses two administrator-authenticated Home Assistant APIs:

- `automation/config` over `/api/websocket` for each discovered automation;
- `GET /api/config/scene/config/{config-id}` for editable Home Assistant scenes.

Only the migration runtime's executable subset is collected: state and simple
time-pattern triggers, state conditions, and bounded light, switch, lock,
thermostat, or scene actions. Definitions with templates, delays, multi-trigger
semantics, device/area targets, or unsupported services are skipped with a
durable diagnostic instead of being approximated.

The input export supplies the reviewed entity registry and source instance ID.
The enriched output adds a `definition_collection` report at the top level;
serde readers for the original export ignore that extra field, so the artifact
can be passed directly to the existing migration CLI.

`HOME_ASSISTANT_TOKEN` is read only from the environment. The token is never a
command-line argument, artifact field, or diagnostic value.

```sh
HOME_ASSISTANT_TOKEN='...' cargo run \
  -p smart-home-home-assistant-definitions \
  --bin smart-home-collect-home-assistant-definitions -- \
  home-assistant-export.json \
  wss://home.example/api/websocket \
  https://home.example \
  home-assistant-enriched.json
```

The REST and WebSocket base URLs are separate so reverse proxies and local test
hosts can expose them on different origins. HTTP and HTTPS responses are size
bounded; socket operations time out after 30 seconds by default.

## Validation

```sh
bash smart-home-home-assistant-definitions/BUILD
cargo clippy -p smart-home-home-assistant-definitions --all-targets -- -D warnings
```
