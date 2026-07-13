# Changelog

All notable changes to `task-wasm` are documented here.

## [0.1.0] - Unreleased

### Added

- **Linear-memory WASM ABI over the pure `task-core` engine** — the browser/Electron
  boundary for task-app. Follows the repo's `*-wasm` convention (`alloc`/`dealloc`,
  `(ptr, len)` in, length-prefixed out) and holds one global `ProjectState` per page.
- **No facade / no dispatch bus** — deliberately. One export per engine operation and
  query, so each backend can call the engine natively (see `task-app-architecture.md`).
  Operations: `create_task`, `rename_task`, `delete_task`, `reparent`, `set_kind`,
  `set_completed`, `set_percent_complete`, `set_status`, `set_schedule`, `set_duration`,
  `set_constraint`, `set_deadline`, `link_dependency`, `unlink_dependency`, `add_link`,
  `upsert_resource`, `assign`, `upsert_field`, `set_field_value`, `set_decision`,
  `answer_decision`, `set_project_name`. Queries: `checklist`, `todos`, `kanban`,
  `gantt`, `flowchart`, `schedule`. Lifecycle: `reset`, `snapshot`, `load`.
- **JSON envelopes** — `{ ok: true }` / `{ ok: true, data }` / `{ ok: false, error, code }`.
  Rejected operations and malformed input come back as typed errors, never a trap.
- **JavaScript accessor** (`js/task-engine.mjs`) — dependency-free loader exposing one
  camelCase method per export; plus a node smoke test (`js/smoke.mjs`).
- **Build scripts** (`build-wasm.sh` / `.ps1`) → `pkg/task_engine.wasm`. The engine,
  including the `symbolic-vm` formula CAS, compiles cleanly to `wasm32-unknown-unknown`.

### Notes

- `task-core` gained `serde` derives on the scheduler's public output types
  (`ScheduleResult`, `Conflict`, `ConflictKind`) so the `schedule` query can serialise.
- ABI logic is unit-tested natively (5 tests: op round-trip, error envelopes, gantt
  criticality, snapshot/load, empty-input safety); `forbid`-clean, no clippy warnings.
