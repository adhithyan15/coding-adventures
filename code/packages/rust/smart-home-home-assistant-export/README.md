# smart-home-home-assistant-export

`smart-home-home-assistant-export` collects live topology and current state from
Home Assistant into the versioned export contract consumed by
`smart-home-home-assistant-migration`.

The collector connects to Home Assistant's `/api/websocket` endpoint,
authenticates with a long-lived access token, and requests:

- the area registry;
- the device registry;
- the entity registry;
- all current states.

Results are normalized into stable order. State records missing from the entity
registry receive synthetic entity records so transient and integration-owned
state is not silently dropped. Scene definitions, automation definitions, and
history are not inferred from state-only data and remain explicit follow-up
collection boundaries.

The token is read from `HOME_ASSISTANT_TOKEN`; it is never accepted as a
command-line argument or written to the export.

```sh
HOME_ASSISTANT_TOKEN='...' cargo run \
  -p smart-home-home-assistant-export \
  --bin smart-home-export-home-assistant -- \
  wss://home.example/api/websocket \
  my-home-instance \
  home-assistant-export.json
```

Use `--exported-at-ms <timestamp>` when a reproducible source timestamp is
required. Socket reads and writes time out after 30 seconds by default; use
`--timeout-ms <milliseconds>` to set a different bound.

## Validation

```sh
bash smart-home-home-assistant-export/BUILD
cargo clippy -p smart-home-home-assistant-export --all-targets -- -D warnings
```
