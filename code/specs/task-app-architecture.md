# task-app — Application Architecture

> Part of the [task-app spec series](task-app-overview.md). Defines the crate layering, the
> props/event contract, and the FFI/UI seams. This replicates the **Engram** architecture
> (`engram-core` → `engram-core-wasm` → `engram-capi`+`engram-wasm` → Mosaic host adapters) exactly;
> where a decision has a proven Engram precedent, we cite it.

## The layer cake

```
task-core            pure domain: ProjectState + reduce() + CPM scheduler + formula/calendar modules
  │                  no I/O, no clock (now: u64 injected), serde behind a feature, forbid(unsafe)
  └ task-core-wasm   THE FACADE (platform-free, forbid(unsafe)): TaskSession = ProjectState +
        │            view-model state; &str-in / JSON-String-out (panic-caught); dispatch() command
        │            bus; get_props(ctx) → flat kebab slot map; handle_event() → combined envelope
        ├ task-capi  extern "C" over TaskSession — native shells; + task-host-cli sidecar
        └ task-wasm  linear-memory WASM ABI over TaskSession — browser / Electron; + JS loader
```

`task-capi` and `task-wasm` are **siblings**, each depending only on `task-core-wasm` and adding an
ABI + marshalling. All behavior — every prop, every event — lives once in the facade. Porting to a
platform is a new thin ABI/adapter, never a change to core logic. (Engram precedent:
`engram-capi`/`engram-wasm` both wrap `engram-core-wasm`, not `engram-core`.)

Only the two ABI crates contain `unsafe`; everything below `forbid(unsafe_code)`.

## `task-core` — the domain core

Public surface (engram-core `reducer.rs` pattern):

```rust
pub struct ProjectState { /* see data-model spec */ }

pub enum TaskCommand { /* the full mutation catalog, below */ }

/// Pure reducer: immutable in, new state out. Never mutates in place.
pub fn reduce(state: &ProjectState, cmd: TaskCommand) -> ProjectState;

/// Pure read-models (not commands) — the scheduler & projections:
pub fn schedule(state: &ProjectState) -> ScheduleResult;        // CPM: dates, slack, critical, conflicts
pub fn level_resources(state: &ProjectState, opts: LevelOpts) -> ScheduleResult;
pub fn recompute_fields(state: &ProjectState) -> FieldValues;   // formula/rollup via symbolic-vm
pub fn checklist_view(state, view) -> ChecklistProjection;      // flattenVisibleItems analogue
pub fn kanban_view(state, view) -> KanbanProjection;
pub fn gantt_view(state, view) -> GanttProjection;
// … one per ViewShape …
```

### Command catalog (`TaskCommand`)

Grouped; each variant carries typed ids/values (not strings):

- **Task**: `CreateTask`, `RenameTask`, `SetNotes`, `DeleteTask`, `Reparent{task,new_parent,index}`,
  `Reorder`, `SetKind`, `ToggleCollapsed`.
- **Progress/workflow**: `SetStatus`, `ToggleCompleted`, `SetPercentComplete`.
- **Schedule**: `SetDuration`, `SetWork`, `SetTaskType`, `SetConstraint`, `SetDeadline`,
  `SetTaskCalendar`, `SetActuals`.
- **Dependencies**: `LinkDependency{pred,succ,kind,lag}`, `UnlinkDependency`, `SetDependencyKind`,
  `SetLag`.
- **Generic links**: `AddLink`, `RemoveLink`.
- **Resources/assignments**: `CreateResource`, `EditResource`, `DeleteResource`, `Assign`,
  `SetAssignmentUnits`, `SetContour`, `Unassign`.
- **Calendars**: `CreateCalendar`, `SetWorkWeek`, `AddException`, `RemoveException`, `SetProjectCalendar`.
- **Fields**: `AddFieldDef`, `EditFieldDef`, `DeleteFieldDef`, `SetFieldValue`.
- **Decision (checklist)**: `SetDecisionQuestion`, `AnswerDecision{task,answer}`, `ClearDecision`.
- **Baselines**: `CaptureBaseline`, `DeleteBaseline`.
- **Views**: `CreateView`, `EditView`, `DeleteView`.
- **Bulk/state**: `LoadState`, `ImportProject`, `Undo`, `Redo` (history via snapshot stack).

### Recompute triggers

Each command declares what it invalidates, so the facade recomputes minimally:

| Invalidates | Commands (examples) |
|---|---|
| **Schedule cache** (→ re-run CPM) | duration/work/type/constraint/calendar edits, link CRUD, reparent, assignment edits |
| **Formula cache** (→ recompute affected fields) | `SetFieldValue`, any schedule change formulas read (`[finish]` etc.) |
| **Neither** | rename, notes, view CRUD, collapse toggle |

`directed-graph.affected_nodes` bounds both recomputations to the transitive impact set.

## `task-core-wasm` — the facade (the contract layer)

The keystone: the single place the UI contract is defined. (Engram precedent:
`engram-core-wasm/src/lib.rs`.)

```rust
pub struct TaskSession {
    state: ProjectState,
    // view-model state the core doesn't track:
    active_view: ViewId,
    selection: Vec<TaskId>,
    editor: Option<TaskEditorState>,
    filters: FilterState,
}
impl TaskSession {
    pub fn new() -> Self; pub fn new_demo() -> Self;
    pub fn snapshot(&self) -> String;                 // JSON of ProjectState
    pub fn load_snapshot(&mut self, json: &str) -> String;
    pub fn dispatch(&mut self, command_json: &str) -> String;      // {"ok":true,"state":…}
    pub fn get_props(&self, ctx_json: &str) -> String;             // flat kebab-case slot map
    pub fn handle_event(&mut self, event_json: &str, now: u64) -> String; // combined envelope
    pub fn export_backup(&self, now: u64) -> String;
    pub fn import_backup(&mut self, json: &str) -> String;
}
```

- **Method shape**: `&str` in, JSON `String` out, every call wrapped in a panic-catcher so a bug
  becomes `{"ok":false,"error":…}` instead of trapping the FFI boundary. Standard success envelope
  `{"ok":true,"<key>":<payload>}`. (Engram's `catch_json`/`ok_with`.)
- **`dispatch`** deserializes a `FacadeCommand`, lowers it into a core `TaskCommand`, and `reduce`s.
- **`get_props(ctx)`** builds a flat object of **kebab-case slot keys** from the current state +
  active view + selection — e.g. `"app-title"`, `"view-shape"`, `"task-count"`,
  `"gantt-bars"` (list), `"selected-task-name"`, `"schedule-conflicts"` (list),
  `"kanban-columns"`. Names line up 1:1 with the Mosaic `.mil` slots.
- **`handle_event(event, now)`** parses an event envelope into a typed `TaskAppEvent`
  (`SelectTask`, `Toggle`, `AnswerDecision`, `MoveCard{task,to_status}`, `EditDuration`,
  `RequestOpenFile`, `RequestSave`, …), mutates via `dispatch`/view-model, and returns **one combined
  envelope**: `{ ok, event, hostIntent?, state, props }` — the freshly recomputed props *and* an
  optional out-of-band host intent.
- **`hostIntent`** is the escape hatch for things the sandbox can't do — file open/save,
  OS dialogs. `host_intent_for_event` emits e.g. `{"type":"openProject","accept":[".taskproj",".json"]}`
  or `{"type":"saveProject","extension":".taskproj"}`. The host performs the side effect and feeds
  bytes back through a dedicated method. (Engram precedent: Anki import/export intents.)

The facade also owns **snapshot persistence policy** (what to persist after each non-error event),
which host adapters wrap with `storage-core` / localStorage.

## `task-capi` — C ABI (native shells)

(Engram precedent: `engram-capi`.) `crate-type = ["cdylib","staticlib","rlib"]`, plus a
`[[bin]] task-host-cli`.

```c
TaskSession* task_session_new(void);
void         task_session_free(TaskSession*);
char*        task_dispatch(TaskSession*, const char* command_json);
char*        task_get_props(TaskSession*, const char* ctx_json);
char*        task_handle_event(TaskSession*, const char* event_json, uint64_t now);
char*        task_snapshot(TaskSession*);
char*        task_load_snapshot(TaskSession*, const char* json);
char*        task_import_project(TaskSession*, const uint8_t* data, size_t len); // binary payloads
void         task_string_free(char*);   // caller frees every returned string
```

Opaque handle + NUL-terminated UTF-8 strings; a `with_session` helper null-checks and maps 1:1 onto
the facade. Binary payloads (project file import) use `(ptr,len)` byte pairs. Events flow back the
same way any call returns — request/response, no callbacks; `hostIntent` rides inside the
`handle_event` JSON. Ships a generated `include/task.h` for the SwiftUI module map.

## `task-wasm` — WASM ABI (browser / Electron)

(Engram precedent: `engram-wasm`, mirroring the repo's `spreadsheet-wasm` convention.) Hand-rolled
`extern "C"` over `wasm32-unknown-unknown` linear memory — not wasm-bindgen:

- exports `alloc(len)` / `dealloc(ptr,len)`; inputs are `(ptr,len)` byte pairs; outputs are
  **length-prefixed** (`[u32 LE len][UTF-8 JSON]`).
- one `thread_local! { static SESSION: RefCell<TaskSession> }`, with `reset()`/`reset_demo()`.
- exports mirror the facade: `dispatch`, `snapshot`, `load_snapshot`, `get_props`, `handle_event`,
  `import_project`, `export_backup`.
- a hand-written JS loader `task-mosaic-host-wasm.mjs` (typed `.d.ts`) exposes
  `createTaskEngine(wasmBytes, opts) → TaskEngine` with camelCase methods and, crucially,
  `engine.createMosaicHost({now, onHostIntent}) → { platform, getProps, handleEvent }` — where the
  Rust kebab-case slot keys are surfaced to the Mosaic host contract.

## Mosaic UI package (`code/programs/mosaic/task-app`)

A **UI** package — no product logic. (Engram precedent: `engram-app` package.)

- `TaskApp.mil` — interface: `slot`s (props, typed) and `emit`s (events, some with typed args like
  `onMoveCard(task:text, toStatus:text)`), names aligned to the facade's kebab-case prop keys.
- `TaskApp.mll` + `TaskApp.touch.mll` — desktop and touch layout variants.
- `TaskApp.light.msl` + `TaskApp.dark.msl` — theme stylesheets (`--theme` at build).
- `mosaic-package.toml` — `exports = ["TaskApp"]`; depends on reusable
  `code/packages/mosaic/mosaic-pkg-*` components; declares `[host_assets]` (the adapter files).
- `scripts/build-all.ps1` — the assembly line: build `task-wasm` (→ wasm + JS loader) and
  `cargo build -p task-capi` (→ dll/dylib/so + static lib + `task-host-cli`); then per backend run
  `mosaic-compile pkg … --backend X --emit-project`; then install the right artifact — wasm+loader
  for web/react/electron, `task_capi.{dll,dylib,so}` for qt/xaml/flutter/compose, static lib +
  `task.h` + module.modulemap for swiftui.

### Reusable components (`code/packages/mosaic/mosaic-pkg-*`)

`checklist-runner`, `todo-list`, `task-board` (kanban), `gantt-view`, `flowchart-view`,
`task-detail`, `field-editor`, `calendar-view`. Each is an independently-adoptable Mosaic component,
authored once, emitted to every backend.

### Host adapters (`host/{web,electron,qt,swiftui,compose,flutter,xaml}`)

Thin per-backend glue. The **getProps/handleEvent contract** is the seam: Mosaic-generated UI calls
`window.mosaicHost.getProps({component})` to fill slots and `window.mosaicHost.handleEvent(...)` on
`emit`. The web adapter builds that host object from `engine.createMosaicHost(...)`:
`getProps → task_get_props → {props}`; `handleEvent → task_handle_event → {props, hostIntent}`;
re-render from `props`, and route any `hostIntent` through `onHostIntent` (file open/save), then
persist `snapshot()` via `withSnapshotPersistence` (localStorage / `~/.task-app/snapshot.v1.json`).
Native shells (Qt/XAML/SwiftUI/Compose/Flutter) link `task-capi` and marshal identically.

## Contract-parity testing

Following Engram's smoke tests: assert that every generated host shell (web/react/electron and the
native project shells) exposes the **same** slot keys and event envelope as the `TaskSession` facade
— i.e. the `.mil` interface stays in lockstep with `get_props`/`handle_event`. This catches drift
between the Rust contract and the Mosaic UI at build time, on every platform.

## Phase-1 seam (how the two tracks meet)

Track B (facade + wasm + Mosaic + web/electron) can start against a **minimal** `task-core` (tasks +
checklist/todo projections, no scheduling) so the pipeline is proven end-to-end early; as Track A
lands the real CPM scheduler + resources, the facade's read-models (`schedule`, `gantt_view`, …)
light up without changing the ABI or the UI contract. The contract-parity tests guard the seam.
