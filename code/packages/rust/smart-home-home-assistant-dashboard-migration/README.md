# smart-home-home-assistant-dashboard-migration

`smart-home-home-assistant-dashboard-migration` collects live Lovelace
dashboards after the Home Assistant topology has been reviewed and exports a
repository-owned dashboard manifest. It uses the concrete
`lovelace/dashboards/list`, `lovelace/config`, and
`lovelace/resources/list` WebSocket commands.

The manifest contract is owned by `smart-home-dashboard-core`, so migration
output and the operational local dashboard share one validated schema.

Standard entity, light, thermostat, sensor, tile, button, entities, glance,
and history-graph cards are compiled to native entity-control, entity-list,
and history widgets. Vertical stacks, horizontal stacks, grids, and section
views are flattened without changing source order. Entity references are
accepted only when present in the reviewed topology export and become the same
`ha:<entity-id>` identifiers used by the executable runtime migration.

Custom cards, custom resources, unsupported actions, and unknown entity
references are never approximated. They are retained as durable diagnostics
for manual replacement. A failed fetch for a dashboard that Home Assistant
listed is an error and blocks applied output; use `--dry-run` to retain the
complete review plan.

`HOME_ASSISTANT_TOKEN` is read only from the environment and is never written
to artifacts or diagnostics.

```sh
HOME_ASSISTANT_TOKEN='...' cargo run \
  -p smart-home-home-assistant-dashboard-migration \
  --bin smart-home-migrate-home-assistant-dashboards -- \
  home-assistant-enriched.json \
  wss://home.example/api/websocket \
  smart-home-dashboards.json
```

## Validation

```sh
bash smart-home-home-assistant-dashboard-migration/BUILD
cargo clippy -p smart-home-home-assistant-dashboard-migration --all-targets -- -D warnings
```
