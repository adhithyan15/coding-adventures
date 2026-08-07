# smart-home-home-assistant-migration

`smart-home-home-assistant-migration` turns a versioned Home Assistant export
into normalized D23 topology, state, scenes, and durable automation
definitions.

The export contract is intentionally reviewable JSON. Areas, devices, entities,
and current states can be collected through Home Assistant's registry and state
APIs. Scene targets and the supported automation subset are represented
explicitly so unsupported behavior is diagnosed instead of silently changed.

The planner:

- gives every imported record a deterministic, source-prefixed identifier;
- preserves Home Assistant identifiers and source metadata;
- maps known domains to D23 capabilities and retains unknown domains as
  observe-only entities;
- validates area, device, entity, scene, condition, and action references;
- expands supported service actions into durable D23 automation definitions;
- blocks apply when any reference or behavior cannot be migrated safely.

The CLI writes either a dry-run plan or an applied runtime/automation snapshot
with a deterministic source fingerprint and import receipt. Output is written
through a temporary sibling and renamed atomically.

```sh
cargo run -p smart-home-home-assistant-migration --bin smart-home-import-home-assistant -- \
  home-assistant-export.json migration-artifact.json --dry-run

cargo run -p smart-home-home-assistant-migration --bin smart-home-import-home-assistant -- \
  home-assistant-export.json migration-artifact.json
```

Production Matter commissioning, Thread border routing, Home Assistant history,
and dashboard migration remain separate slices.

## Validation

```sh
./smart-home-home-assistant-migration/BUILD
cargo clippy -p smart-home-home-assistant-migration --all-targets -- -D warnings
```
