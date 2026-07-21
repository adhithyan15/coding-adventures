# Changelog

All notable changes to `task-wasm` are documented here.

## [0.1.0] - Unreleased

### Added

- **View-layer exports** (Phase 3 PR-6 of `code/specs/task-app-view-layer.md`), all acting
  on the active project and returning **render-ready** data:
  - `table` — the sheet: columns (label + kind) and grouped rows whose cells carry both
    the typed value and the engine-formatted display string.
  - `view_selection` — the ordered, grouped task ids for a view (filter → sort → group).
  - `calendar` — dated events for a view over an inclusive `[start, end]` day range.
  - Each takes `{ view, projectStart }` (calendar also `{ start, end }`); a parse failure
    or an empty workspace answers with an error envelope rather than trapping.
- **Label and priority ops**: `upsert_label`, `delete_label`, `set_task_labels`,
  `set_priority`.
- `task-engine.mjs` gains the matching camelCase methods (`table`, `viewSelection`,
  `calendar`, `upsertLabel`, `setTaskLabels`, `setPriority`, …).

### Changed

- **The ABI now holds a whole `Workspace`, not a single `ProjectState`** (Phase 2 PR-4 of
  `code/specs/task-app-workspace.md`) — projects are first-class, plural, and nestable
  over the wire, while the existing single-project surface is untouched.
  - **Per-project ops/queries act on the *active project*** (the first root), so a
    single-project host — including the shipped web app — behaves exactly as before.
    `create_task`, `todos`, `gantt`, `schedule`, `checklist`, … are unchanged.
  - **New workspace operations**: `create_project` / `rename_project` / `delete_project`
    / `nest_project` / `unnest_project`, `create_task_in` (workspace-global id
    uniqueness), `move_task`, `link_cross_project_dependency` /
    `unlink_cross_project_dependency`, `upsert_shared_resource` /
    `delete_shared_resource`.
  - **New workspace queries**: `workspace` (the whole workspace) and `workspace_schedule`
    (one CPM schedule across every project, with per-project rollups).
  - **`snapshot`/`load` are workspace-level, with migration**: `load` accepts either a
    workspace snapshot or a **pre-workspace bare-`ProjectState`** snapshot (wrapped into a
    one-project workspace), so data persisted by the Phase-1 web app keeps loading. The
    shapes are unambiguous (`projects` vs. `tasks`), so `Workspace` is tried first.
  - JS accessor (`task-engine.mjs`) gains the matching camelCase methods
    (`createProject`, `moveTask`, `linkCrossProjectDependency`, `workspaceSchedule`, …).
  - No panics across the FFI boundary: an empty workspace answers per-project calls with
    an error envelope; all new ops return the same `{ok}`/`{ok:false,error,code}` shape.
  - 4 new native tests (9 total): active-project targeting, snapshot/load migration,
    cross-project scheduling, and workspace-op error envelopes. wasm32 build + JS smoke
    pass.

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
