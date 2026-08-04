# smart-home-home-assistant-history

`smart-home-home-assistant-history` migrates historical state from a running
Home Assistant instance into D23's durable device-event replay history.

The CLI reads the versioned topology/current-state export produced by
`smart-home-home-assistant-export`, authenticates to `/api/websocket`, and
requests `history/history_during_period` in bounded entity batches. It then:

- validates every returned entity against the migration topology;
- produces deterministic event identifiers and a stable source fingerprint;
- preserves source state, attributes, and timestamps as event metadata;
- applies events chronologically through `SmartHomeRuntime::apply_device_event`;
- skips identical events on repeat apply;
- restores the topology export's current state after replay;
- emits a durable runtime snapshot containing replayable registry events.

The access token is read only from `HOME_ASSISTANT_TOKEN`.
History artifacts retain source attributes for audit and may therefore contain
sensitive household data; store and transfer them with the same protections as
the Home Assistant source database.

```sh
HOME_ASSISTANT_TOKEN='...' cargo run \
  -p smart-home-home-assistant-history \
  --bin smart-home-import-home-assistant-history -- \
  home-assistant-export.json \
  wss://home.example/api/websocket \
  2026-01-01T00:00:00Z \
  2026-02-01T00:00:00Z \
  history-migration.json \
  --dry-run
```

Remove `--dry-run` to include the topology-seeded durable runtime snapshot and
history receipt. Use `--batch-size`, `--timeout-ms`, or `--collected-at-ms` to
override the bounded defaults.

## Validation

```sh
bash smart-home-home-assistant-history/BUILD
cargo clippy -p smart-home-home-assistant-history --all-targets -- -D warnings
```
